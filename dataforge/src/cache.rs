//! L2 disk-persistent cache for raw signals.
//!
//! Same design as src-tauri's L2Cache: key-value filesystem cache with
//! TTL-based staleness, atomic writes (tmp + rename), and LRU eviction.
//!
//! ponytail: this is a self-contained copy of the L2 cache pattern from
//! src-tauri. dataforge does not depend on src-tauri, so it brings its own.
//! The pattern is proven (5440 files, concurrent put same key, etc.).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, RwLock};
use std::time::{Duration, SystemTime};

/// L2 disk cache — read path lock-free, clear exclusive.
pub struct L2Cache {
    base_dir: PathBuf,
    max_bytes: AtomicU64,
    hits: AtomicU64,
    misses: AtomicU64,
    evict_lock: Mutex<()>,
    clear_lock: RwLock<()>,
    write_count: AtomicU64,
    tmp_seq: AtomicU64,
}

const EVICT_CHECK_INTERVAL: u64 = 100;

impl L2Cache {
    /// Create or open a disk cache under `base_dir`.
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

    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        let _guard = self.clear_lock.read().unwrap_or_else(|e| e.into_inner());
        let path = self.path_for(key);
        match fs::read(&path) {
            Ok(data) if !data.is_empty() => {
                self.hits.fetch_add(1, Ordering::Relaxed);
                let path_clone = path.clone();
                std::thread::spawn(move || {
                    let _ = touch_file(&path_clone);
                });
                Some(data)
            }
            _ => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                None
            }
        }
    }

    pub fn put(&self, key: &str, data: &[u8]) -> io::Result<()> {
        let _guard = self.clear_lock.read().unwrap_or_else(|e| e.into_inner());
        let path = self.path_for(key);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let seq = self.tmp_seq.fetch_add(1, Ordering::Relaxed);
        let tmp = path.with_extension(format!("tmp.{}.{}", std::process::id(), seq));
        fs::write(&tmp, data)?;
        if let Err(e) = fs::rename(&tmp, &path) {
            let _ = fs::remove_file(&tmp);
            return Err(e);
        }

        let count = self.write_count.fetch_add(1, Ordering::Relaxed) + 1;
        if count.is_multiple_of(EVICT_CHECK_INTERVAL) {
            let current = self.total_bytes();
            if current > self.max_bytes.load(Ordering::Relaxed) {
                let _ = self.evict_lru();
            }
        }
        Ok(())
    }

    pub fn total_bytes(&self) -> u64 {
        dir_size(&self.base_dir)
    }

    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed) as f64;
        let total = hits + self.misses.load(Ordering::Relaxed) as f64;
        if total == 0.0 {
            1.0
        } else {
            hits / total
        }
    }

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

    fn path_for(&self, key: &str) -> PathBuf {
        self.base_dir.join(key)
    }

    fn evict_lru(&self) -> io::Result<()> {
        let _guard = self.evict_lock.lock().unwrap_or_else(|e| e.into_inner());

        let mut entries: Vec<(PathBuf, SystemTime)> = Vec::new();
        collect_files_by_atime(&self.base_dir, &mut entries);
        if entries.is_empty() {
            return Ok(());
        }

        let target = self.max_bytes.load(Ordering::Relaxed) * 3 / 4;
        let mut current = self.total_bytes();
        entries.sort_unstable_by_key(|(_, atime)| *atime);

        for (path, _) in &entries {
            if current <= target {
                break;
            }
            if let Ok(meta) = fs::metadata(path) {
                current = current.saturating_sub(meta.len());
                let _ = fs::remove_file(path);
            }
        }
        clean_empty_dirs(&self.base_dir);
        Ok(())
    }
}

fn touch_file(path: &Path) -> io::Result<()> {
    let now = SystemTime::now();
    let file = fs::File::open(path)?;
    let accessed = fs::metadata(path)?.accessed().unwrap_or(now);
    let times = fs::FileTimes::new()
        .set_accessed(accessed)
        .set_modified(now);
    let _ = file.set_times(times);
    Ok(())
}

fn is_tmp_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
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

fn clean_empty_dirs(dir: &Path) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                clean_empty_dirs(&path);
                if fs::read_dir(&path)
                    .map(|mut d| d.next().is_none())
                    .unwrap_or(false)
                {
                    let _ = fs::remove_dir(&path);
                }
            }
        }
    }
}

fn dir_size(dir: &Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                total += dir_size(&path);
            } else if let Ok(meta) = entry.metadata() {
                total += meta.len();
            }
        }
    }
    total
}

// ── TTL cache helpers ────────────────────────────────────────────────

/// TTL-cached value: data + expiry.
///
/// 8-byte unix timestamp header, then JSON body.
pub fn read_cached<T: serde::de::DeserializeOwned>(
    l2: &L2Cache,
    key: &str,
    ttl: Duration,
) -> Option<Cached<T>> {
    let bytes = l2.get(key)?;
    if bytes.len() < HEADER_LEN {
        return None;
    }
    let mut ts = [0u8; HEADER_LEN];
    ts.copy_from_slice(&bytes[..HEADER_LEN]);
    let cached_at = u64::from_le_bytes(ts);
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let stale = now.saturating_sub(cached_at) > ttl.as_secs();
    let data: T = serde_json::from_slice(&bytes[HEADER_LEN..]).ok()?;
    Some(Cached { data, stale })
}

/// Write a value to cache with unix timestamp header.
pub fn write_cached(l2: &L2Cache, key: &str, data: &[u8]) {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut out = Vec::with_capacity(HEADER_LEN + data.len());
    out.extend_from_slice(&now.to_le_bytes());
    out.extend_from_slice(data);
    let _ = l2.put(key, &out);
}

const HEADER_LEN: usize = 8;

pub struct Cached<T> {
    pub data: T,
    pub stale: bool,
}

/// Shared HTTP client with polite user-agent.
pub fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Aurora-DataForge/0.1")
        .build()
        .expect("failed to build reqwest client")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn test_dir() -> PathBuf {
        std::env::temp_dir().join(format!(
            "dataforge_test_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos()
        ))
    }

    #[test]
    fn put_get_roundtrip() {
        let dir = test_dir();
        let cache = L2Cache::new(dir.clone(), 1024 * 1024);
        cache.put("test/key.json", b"hello").unwrap();
        assert_eq!(cache.get("test/key.json"), Some(b"hello".to_vec()));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_key_returns_none() {
        let dir = test_dir();
        let cache = L2Cache::new(dir.clone(), 1024 * 1024);
        assert_eq!(cache.get("nope"), None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn ttl_cached_fresh_and_stale() {
        let dir = test_dir();
        let cache = L2Cache::new(dir.clone(), 1024 * 1024);

        // Write current data
        let data = vec![1, 2, 3];
        write_cached(&cache, "ttl-test", &serde_json::to_vec(&data).unwrap());

        // Fresh with long TTL
        let cached = read_cached::<Vec<u8>>(&cache, "ttl-test", Duration::from_secs(3600));
        assert!(cached.is_some());
        assert!(!cached.unwrap().stale);

        // ponytail: TTL=0 makes it stale only if the write happened in a
        // different whole-second than the read. At second granularity they
        // may be the same. Use a small sleep to guarantee staleness.
        std::thread::sleep(Duration::from_secs(2));

        // Write with an old timestamp (manually backdated)
        let mut old = Vec::with_capacity(8 + data.len());
        let old_ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .saturating_sub(10); // 10 seconds ago
        old.extend_from_slice(&old_ts.to_le_bytes());
        old.extend_from_slice(&serde_json::to_vec(&data).unwrap());
        cache.put("ttl-old", &old).unwrap();

        // Now TTL=1s should mark it stale
        let cached = read_cached::<Vec<u8>>(&cache, "ttl-old", Duration::from_secs(1));
        assert!(cached.is_some());
        assert!(
            cached.unwrap().stale,
            "10-second-old cache should be stale with 1s TTL"
        );

        let _ = fs::remove_dir_all(&dir);
    }
}
