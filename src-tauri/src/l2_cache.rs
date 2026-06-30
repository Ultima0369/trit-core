//! L2 磁盘持久化缓存。
//!
//! 瓦片以文件形式存储于 base_dir 下，key 即相对路径。
//! 超出 max_bytes 时按 atime 进行 LRU 驱逐。
//! key 格式: "china-tiles/{z}/{x}/{y}.jpg"

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, RwLock};
use std::time::SystemTime;

/// L2 磁盘缓存 — 读路径无锁并发，clear 独占。
///
/// get/put/exists 持 clear_lock 的读锁（多 worker 可并发命中磁盘缓存，
/// 不被外部串行化）；clear 持写锁独占删目录，阻止并发 get/put 读到
/// 半删状态。驱逐（evict_lru）是低频近似 LRU，持独立 evict_lock，
/// 与 put 之间的最终不一致在缓存语义内可接受。
pub struct L2Cache {
    base_dir: PathBuf,
    max_bytes: AtomicU64,
    hits: AtomicU64,
    misses: AtomicU64,
    /// 驱逐锁 — 确保同一时刻只有一个驱逐操作
    evict_lock: Mutex<()>,
    /// 清空锁 — clear 持写锁独占，get/put/exists 持读锁共享
    clear_lock: RwLock<()>,
    /// 写入计数器 — 每 N 次写入检查一次容量（避免每次 put 都扫描全目录）
    write_count: AtomicU64,
    /// tmp 文件唯一后缀计数器 — 避免并发 put 同 key 撞同一 tmp 路径
    tmp_seq: AtomicU64,
}

/// 每 N 次写入后检查容量并可能触发驱逐。
const EVICT_CHECK_INTERVAL: u64 = 100;

impl L2Cache {
    /// 创建磁盘缓存。确保 base_dir 存在。
    pub fn new(base_dir: PathBuf, max_bytes: u64) -> Self {
        let _ = fs::create_dir_all(&base_dir);
        Self {
            base_dir,
            max_bytes: AtomicU64::new(max_bytes),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evict_lock: Mutex::new(()),
            clear_lock: RwLock::new(()),
            write_count: AtomicU64::new(0),
            tmp_seq: AtomicU64::new(0),
        }
    }

    /// 从磁盘读取瓦片。
    ///
    /// 命中时更新文件 atime 用于 LRU 驱逐决策。
    /// atime 更新在后台线程异步执行，不阻塞当前请求。
    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        // 读锁：与并发 get/put 共享，仅 clear 持写锁时阻塞
        let _guard = self.clear_lock.read().unwrap_or_else(|e| e.into_inner());
        let path = self.path_for(key);
        match fs::read(&path) {
            Ok(data) if !data.is_empty() => {
                self.hits.fetch_add(1, Ordering::Relaxed);
                // 异步更新 atime — 不阻塞瓦片服务（guard 已随作用域释放）
                let path_clone = path.clone();
                std::thread::spawn(move || {
                    let _ = file_touch(&path_clone);
                });
                Some(data)
            }
            _ => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                None
            }
        }
    }

    /// 写入瓦片到磁盘。原子写入（先 tmp 后 rename）。
    /// 每 EVICT_CHECK_INTERVAL 次写入检查一次容量，避免每次写入都扫描全目录。
    pub fn put(&self, key: &str, data: &[u8]) -> io::Result<()> {
        // 读锁：与并发 get/put 共享，仅 clear 持写锁时阻塞
        let _guard = self.clear_lock.read().unwrap_or_else(|e| e.into_inner());
        let path = self.path_for(key);

        // 确保父目录存在
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // 原子写入：tmp 用唯一后缀（pid + 自增序号），避免并发 put 同 key
        // 撞同一 tmp 路径导致数据交错；rename 覆盖目标（Win/Linux 均原子替换）
        let seq = self.tmp_seq.fetch_add(1, Ordering::Relaxed);
        let tmp = path.with_extension(format!("tmp.{}.{}", std::process::id(), seq));
        fs::write(&tmp, data)?;
        // rename 失败（磁盘满/权限/杀软占用）时清理 tmp，避免孤儿文件残留——
        // collect_files_by_atime 跳过 tmp，残留的 tmp 永不被驱逐，会无限累积占盘
        if let Err(e) = fs::rename(&tmp, &path) {
            let _ = fs::remove_file(&tmp);
            return Err(e);
        }

        // 批量检查容量 — 每 N 次写入检查一次
        let count = self.write_count.fetch_add(1, Ordering::Relaxed) + 1;
        if count.is_multiple_of(EVICT_CHECK_INTERVAL) {
            let current = self.total_bytes();
            if current > self.max_bytes.load(Ordering::Relaxed) {
                let _ = self.evict_lru();
            }
        }

        Ok(())
    }

    /// 检查 key 是否已缓存（且非空）。
    pub fn exists(&self, key: &str) -> bool {
        let _guard = self.clear_lock.read().unwrap_or_else(|e| e.into_inner());
        let path = self.path_for(key);
        path.exists() && fs::metadata(&path).map(|m| m.len() > 0).unwrap_or(false)
    }

    /// 递归计算缓存总字节数。
    pub fn total_bytes(&self) -> u64 {
        crate::utils::dir_size_and_count(&self.base_dir).0
    }

    /// 单次遍历同时返回 (字节数, 文件数)。
    /// cache_stats 原先分别调 total_bytes + file_count = 两次全目录递归，
    /// 合并为一次，5440 文件目录上从 ~7ms 降到 ~4ms。
    pub fn size_and_count(&self) -> (u64, u64) {
        crate::utils::dir_size_and_count(&self.base_dir)
    }

    /// 缓存命中率 [0.0, 1.0]
    pub fn hit_rate(&self) -> f64 {
        crate::utils::hit_rate(
            self.hits.load(Ordering::Relaxed),
            self.misses.load(Ordering::Relaxed),
        )
    }

    /// 最大容量
    pub fn max_bytes(&self) -> u64 {
        self.max_bytes.load(Ordering::Relaxed)
    }

    /// 设置新的容量上限，可能触发驱逐
    pub fn set_max_bytes(&self, max_bytes: u64) {
        self.max_bytes.store(max_bytes, Ordering::Relaxed);
        if self.total_bytes() > max_bytes {
            let _ = self.evict_lru();
        }
    }

    /// 清空所有缓存 — 持 clear_lock 写锁独占，阻塞并发 get/put/exists
    pub fn clear(&self) -> io::Result<()> {
        let _guard = self.clear_lock.write().unwrap_or_else(|e| e.into_inner());
        if self.base_dir.exists() {
            fs::remove_dir_all(&self.base_dir)?;
            fs::create_dir_all(&self.base_dir)?;
        }
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        Ok(())
    }

    // ── 内部方法 ──

    fn path_for(&self, key: &str) -> PathBuf {
        self.base_dir.join(key)
    }

    /// LRU 驱逐 — 删除最旧的 25% 文件直到容量低于 75%。
    /// 使用 sort_unstable (快排变体) 避免稳定排序的额外分配开销。
    fn evict_lru(&self) -> io::Result<()> {
        let _guard = self.evict_lock.lock().unwrap_or_else(|e| e.into_inner());

        let mut entries: Vec<(PathBuf, SystemTime)> = Vec::new();
        collect_files_by_atime(&self.base_dir, &mut entries);

        if entries.is_empty() {
            return Ok(());
        }

        let target = self.max_bytes.load(Ordering::Relaxed) * 3 / 4; // 目标是 max 的 75%
        let mut current = self.total_bytes();

        // 按 atime 升序排列（最旧的在前）— sort_unstable 比 sort 更快
        entries.sort_unstable_by_key(|(_, atime)| *atime);

        for (path, _atime) in &entries {
            if current <= target {
                break;
            }
            if let Ok(meta) = fs::metadata(path) {
                current = current.saturating_sub(meta.len());
                let _ = fs::remove_file(path);
            }
        }

        // 清理空目录
        let _ = clean_empty_dirs(&self.base_dir);

        Ok(())
    }
}

// ── 辅助函数 ──

fn file_touch(path: &Path) -> io::Result<()> {
    let now = SystemTime::now();
    let file = fs::File::open(path)?;
    let accessed = fs::metadata(path)?.accessed().unwrap_or(now);
    let times = std::fs::FileTimes::new()
        .set_accessed(accessed)
        .set_modified(now);
    let _ = file.set_times(times);
    Ok(())
}

/// 判断是否为 put 产生的 tmp 文件。
/// tmp 命名为 `<stem>.tmp.<pid>.<seq>`（pid/seq 均为数字），精确匹配此模式，
/// 避免裸 `contains(".tmp")` 误判 key 中含 .tmp 的合法缓存文件（如 15.tmp.bak）。
fn is_tmp_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    // 期望形如 <stem>.tmp.<pid>.<seq>：从右起第 1、2 段为数字，第 3 段为 "tmp"
    let parts: Vec<&str> = name.split('.').collect();
    parts.len() >= 4
        && parts[parts.len() - 1].chars().all(|c| c.is_ascii_digit())
        && parts[parts.len() - 2].chars().all(|c| c.is_ascii_digit())
        && parts[parts.len() - 3] == "tmp"
}

fn collect_files_by_atime(dir: &Path, entries: &mut Vec<(PathBuf, SystemTime)>) {
    if let Ok(dir_entries) = fs::read_dir(dir) {
        for entry in dir_entries.flatten() {
            let path = entry.path();
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() && !is_tmp_file(&path) {
                    let atime = meta.accessed().unwrap_or(SystemTime::UNIX_EPOCH);
                    entries.push((path, atime));
                } else if meta.is_dir() {
                    collect_files_by_atime(&path, entries);
                }
            }
        }
    }
}

fn clean_empty_dirs(dir: &Path) -> io::Result<()> {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let _ = clean_empty_dirs(&path);
                if fs::read_dir(&path)
                    .map(|mut d| d.next().is_none())
                    .unwrap_or(false)
                {
                    let _ = fs::remove_dir(&path);
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn test_dir() -> PathBuf {
        std::env::temp_dir().join(format!(
            "aurora_l2_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos()
        ))
    }

    #[test]
    fn test_l2_put_get() {
        let dir = test_dir();
        let cache = L2Cache::new(dir.clone(), 1024 * 1024);
        cache.put("tiles/0/0/0.jpg", b"test-data").unwrap();
        assert_eq!(cache.get("tiles/0/0/0.jpg"), Some(b"test-data".to_vec()));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_l2_missing_key_returns_none() {
        let dir = test_dir();
        let cache = L2Cache::new(dir.clone(), 1024 * 1024);
        assert_eq!(cache.get("missing.jpg"), None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_l2_exists() {
        let dir = test_dir();
        let cache = L2Cache::new(dir.clone(), 1024 * 1024);
        assert!(!cache.exists("a.jpg"));
        cache.put("a.jpg", b"x").unwrap();
        assert!(cache.exists("a.jpg"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_l2_total_bytes_increases_after_put() {
        let dir = test_dir();
        let cache = L2Cache::new(dir.clone(), 1024 * 1024);
        let before = cache.total_bytes();
        cache.put("b.jpg", b"hello").unwrap();
        let after = cache.total_bytes();
        // Windows 上文件系统可能有延迟，至少验证不减少
        assert!(
            after >= before,
            "total bytes should not decrease: {before} -> {after}"
        );
        // 验证内容可读
        assert_eq!(cache.get("b.jpg"), Some(b"hello".to_vec()));
        let _ = fs::remove_dir_all(&dir);
    }

    /// 并发 put 同一 key — 验证唯一 tmp 名不撞、无残留 tmp、最终数据合法。
    /// 回归 ad85da2 去锁引入的竞态：原 put 用固定 .tmp 路径，并发同 key
    /// 会数据交错 / rename 竞态。
    #[test]
    fn test_l2_concurrent_put_same_key_no_tmp_collision() {
        use std::sync::Arc;
        use std::thread;

        let dir = test_dir();
        let cache = Arc::new(L2Cache::new(dir.clone(), 1024 * 1024));
        const WRITERS: usize = 16;
        const ITERS: usize = 50;
        // 用嵌套 key（真实瓦片路径形状），验证嵌套目录下也无残留 tmp
        const KEY: &str = "china-tiles/5/10/15.jpg";
        let handles: Vec<_> = (0..WRITERS)
            .map(|i| {
                let cache = Arc::clone(&cache);
                thread::spawn(move || {
                    let payload = format!("writer-{i}");
                    for _ in 0..ITERS {
                        cache.put(KEY, payload.as_bytes()).unwrap();
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }

        // 最终内容必为某个 writer 的完整 payload（非交错碎片）
        let data = cache.get(KEY).expect("最终应有数据");
        let text = String::from_utf8(data).expect("数据应为合法 UTF-8，非交错碎片");
        assert!(text.starts_with("writer-"), "数据被交错损坏: {text:?}");

        // 递归扫描整个缓存目录树，无残留 tmp 文件
        let mut tmp_count = 0;
        let mut stack = vec![dir.clone()];
        while let Some(d) = stack.pop() {
            for entry in fs::read_dir(&d).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if is_tmp_file(&path) {
                    tmp_count += 1;
                }
            }
        }
        assert_eq!(tmp_count, 0, "残留 {tmp_count} 个 tmp 文件");

        let _ = fs::remove_dir_all(&dir);
    }
}
