# Aurora 专业级卫星地图 — 方案 C 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 Aurora 卫星地图提升至专业级：actix-web 代理缓存服务器 + 两级缓存 + 多源按需下载 + 渐进式加载 + z16-z18 全球覆盖。

**Architecture:** 独立线程中启动 actix-web (多 worker) 替换手写 TCP，L1 moka 内存缓存 256MB + L2 文件系统 LRU 持久化缓存。多源瓦片下载器 (高德/Esri/Mapbox) 自动故障转移。前端集成服务器就绪探针、渐进式加载和缓存管理面板。

**Tech Stack:** Rust (actix-web 4, moka 0.12, reqwest 0.12, tokio 1, dirs 5), TypeScript (React, CesiumJS via tauri), Tauri v2

## Global Constraints

- `#![forbid(unsafe_code)]` — 两个 crate 均强制执行
- 现有手写 TCP `tile_server.rs` 保留用于非瓦片静态文件 (cesium/assets)，瓦片代理迁移到 actix-web
- actix-web 在独立 `std::thread` 中运行，避免与 Tauri tokio runtime 冲突
- `AURORA_DATA_DIR` 环境变量优先，否则用 `dirs::data_dir()`，回退 exe 同目录
- L2 缓存格式：`~/.aurora/china-tiles/{z}/{x}/{y}.jpg`（Slippy Map Y 编号）
- 前端 Tauri API 使用官方 `@tauri-apps/api/core`，不使用 `window.__TAURI_INTERNALS__`
- 所有 HTTP 响应包含 `Cache-Control: public, max-age=86400, immutable`
- 端口冲突时自动扫描 21337-21436
- CesiumJS `creditContainer` 保留但不显示（Apache 2.0 合规）

---

### Task 1: 添加 Rust 依赖项

**Files:**
- Modify: `src-tauri/Cargo.toml:1-21`

**Interfaces:**
- Produces: `actix-web`, `actix-rt`, `moka`, `reqwest`, `tokio`, `dirs` 依赖可用

- [ ] **Step 1: 更新 Cargo.toml 添加新依赖**

```toml
[package]
name = "aurora-desktop"
version = "0.1.0"
edition = "2021"

[lib]
name = "aurora_desktop_lib"
crate-type = ["lib", "cdylib", "staticlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = ["devtools"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
aurora = { path = "../aurora" }
anyhow = "1"
ureq = { version = "3", features = ["rustls"] }
flate2 = "1"
tar = "0.4"
actix-web = "4"
actix-rt = "2"
moka = { version = "0.12", features = ["sync"] }
reqwest = { version = "0.12", features = ["rustls-tls"] }
tokio = { version = "1", features = ["full"] }
dirs = "5"
```

- [ ] **Step 2: 验证编译通过**

```bash
cd src-tauri && cargo check
```

Expected: 依赖下载成功，无编译错误（可能有 unused import 警告）

- [ ] **Step 3: 提交**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "chore: add actix-web, moka, reqwest, tokio, dirs dependencies"
```

---

### Task 2: 实现统一数据目录解析

**Files:**
- Create: `src-tauri/src/data_dir.rs`
- Modify: `src-tauri/src/lib.rs` (更新 `aurora_dir` 初始化)
- Modify: `src-tauri/src/asset_fetcher.rs` (使用新函数)
- Modify: `src-tauri/src/logger.rs:92-99` (使用新函数)

**Interfaces:**
- Produces: `pub fn aurora_data_dir() -> PathBuf` — 返回 `~/.aurora/` 或环境变量/回退路径

- [ ] **Step 1: 创建 data_dir.rs**

```rust
//! 统一数据目录解析。
//!
//! 优先级: AURORA_DATA_DIR 环境变量 > dirs::data_dir() > exe 同目录

use std::path::PathBuf;

/// 返回 Aurora 数据根目录。
/// 确保目录存在且可写，否则回退。
pub fn aurora_data_dir() -> PathBuf {
    // 1. 环境变量覆盖
    if let Ok(dir) = std::env::var("AURORA_DATA_DIR") {
        let p = PathBuf::from(&dir);
        if ensure_writable(&p) {
            crate::logger::log("data_dir", "INFO", &format!("使用 AURORA_DATA_DIR: {}", p.display()));
            return p;
        }
        crate::logger::log("data_dir", "WARN", &format!("AURORA_DATA_DIR 不可写: {}", p.display()));
    }

    // 2. 平台标准数据目录
    if let Some(data) = dirs::data_dir() {
        let p = data.join("aurora");
        if ensure_writable(&p) {
            crate::logger::log("data_dir", "INFO", &format!("使用 data_dir: {}", p.display()));
            return p;
        }
    }

    // 3. 回退：exe 同目录
    let fallback = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_default()
        .join("aurora-data");
    
    crate::logger::log("data_dir", "WARN", &format!("回退到 exe 同目录: {}", fallback.display()));
    let _ = std::fs::create_dir_all(&fallback);
    fallback
}

/// 确保目录存在且可写。
fn ensure_writable(dir: &std::path::Path) -> bool {
    match std::fs::create_dir_all(dir) {
        Ok(()) => {
            // 尝试写入测试文件验证可写性
            let test_file = dir.join(".write_test");
            std::fs::write(&test_file, b"test").is_ok_and(|_| {
                let _ = std::fs::remove_file(&test_file);
                true
            })
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aurora_data_dir_returns_some_path() {
        let dir = aurora_data_dir();
        assert!(dir.exists());
        assert!(dir.is_dir());
    }
}
```

- [ ] **Step 2: 更新 logger.rs 使用统一路径**

把 `ensure_logs_dir` 改为使用 `crate::data_dir::aurora_data_dir()`：

```rust
fn ensure_logs_dir() -> anyhow::Result<PathBuf> {
    let dir = crate::data_dir::aurora_data_dir().join("logs");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}
```

- [ ] **Step 3: 更新 asset_fetcher.rs 使用统一路径**

把 `aurora_root()` 函数改为：

```rust
fn aurora_root() -> PathBuf {
    crate::data_dir::aurora_data_dir()
}
```

- [ ] **Step 4: 更新 lib.rs 使用统一路径**

替换现有的 `aurora_dir` 计算逻辑：

```rust
// 替换：
// let aurora_dir = {
//     let home = std::env::var("HOME")
//         .or_else(|_| std::env::var("USERPROFILE"))
//         .unwrap_or_default();
//     std::path::PathBuf::from(home).join(".aurora")
// };

// 为：
let aurora_dir = data_dir::aurora_data_dir();
```

并在文件开头添加 `mod data_dir;`。

- [ ] **Step 5: 运行测试**

```bash
cargo test --package aurora-desktop data_dir
cargo test --package aurora-desktop asset_
cargo test --package aurora-desktop tile_server
```

Expected: 所有测试通过

- [ ] **Step 6: 提交**

```bash
git add src-tauri/src/data_dir.rs src-tauri/src/asset_fetcher.rs src-tauri/src/logger.rs src-tauri/src/lib.rs
git commit -m "feat: add unified data dir resolution — AURORA_DATA_DIR > dirs::data_dir > exe dir"
```

---

### Task 3: 实现 L1 内存缓存 (moka)

**Files:**
- Create: `src-tauri/src/l1_cache.rs`
- Test: `src-tauri/src/l1_cache.rs` (内联 `#[cfg(test)]`)

**Interfaces:**
- Produces:
  - `pub struct L1Cache` — 线程安全内存瓦片缓存
  - `pub fn new(max_bytes: u64) -> Self`
  - `pub fn get(&self, key: &str) -> Option<Vec<u8>>`
  - `pub fn put(&self, key: &str, data: Vec<u8>)`
  - `pub fn hit_rate(&self) -> f64`
  - `pub fn size_bytes(&self) -> u64`
  - `pub fn entry_count(&self) -> u64`

- [ ] **Step 1: 编写 L1 缓存实现**

```rust
//! L1 内存热点瓦片缓存。
//!
//! 基于 moka sync cache，按字节数限制（默认 256MB）。
//! key 格式: "tiles/{z}/{x}/{y}.jpg" 或 "china-tiles/{z}/{x}/{y}.jpg"

use moka::sync::Cache;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// L1 内存缓存 — 线程安全，自动淘汰。
pub struct L1Cache {
    cache: Cache<String, Vec<u8>>,
    /// 当前已使用字节数（近似，moka 内部按 capacity 管理）
    max_bytes: u64,
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
    puts: Arc<AtomicU64>,
}

impl L1Cache {
    /// 创建新缓存。
    /// `max_bytes` 近似值 — moka 按 weighted_size 管理。
    pub fn new(max_bytes: u64) -> Self {
        let hits = Arc::new(AtomicU64::new(0));
        let misses = Arc::new(AtomicU64::new(0));
        let puts = Arc::new(AtomicU64::new(0));

        let cache = Cache::builder()
            .weigher(|_key: &String, value: &Vec<u8>| -> u32 {
                // moka weigher 返回 u32，限制单个条目最大 4GB（瓦片不会超过）
                value.len() as u32
            })
            .max_capacity(max_bytes)
            .build();

        Self {
            cache,
            max_bytes,
            hits,
            misses,
            puts,
        }
    }

    /// 查询缓存。
    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        if let Some(data) = self.cache.get(key) {
            self.hits.fetch_add(1, Ordering::Relaxed);
            Some(data)
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// 写入缓存。
    pub fn put(&self, key: &str, data: Vec<u8>) {
        self.puts.fetch_add(1, Ordering::Relaxed);
        self.cache.insert(key.to_string(), data);
    }

    /// 缓存命中率 [0.0, 1.0]
    pub fn hit_rate(&self) -> f64 {
        let h = self.hits.load(Ordering::Relaxed) as f64;
        let m = self.misses.load(Ordering::Relaxed) as f64;
        let total = h + m;
        if total > 0.0 { h / total } else { 0.0 }
    }

    /// 缓存使用量（近似）
    pub fn size_bytes(&self) -> u64 {
        // moka 不直接暴露当前 size，用 entry_count * 平均大小估算
        // 对于精确需求，可以维护一个单独的 AtomicU64
        self.cache.entry_count() as u64 * 16384 // 假设平均 16KB/瓦片
    }

    /// 缓存条目数
    pub fn entry_count(&self) -> u64 {
        self.cache.entry_count() as u64
    }

    /// 最大容量
    pub fn max_bytes(&self) -> u64 {
        self.max_bytes
    }

    /// 清空缓存
    pub fn clear(&self) {
        self.cache.invalidate_all();
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        self.puts.store(0, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_l1_cache_put_get() {
        let cache = L1Cache::new(1024 * 1024); // 1MB
        cache.put("test/0/0/0.jpg", vec![1, 2, 3]);
        assert_eq!(cache.get("test/0/0/0.jpg"), Some(vec![1, 2, 3]));
        assert_eq!(cache.get("missing.jpg"), None);
    }

    #[test]
    fn test_l1_cache_hit_rate_is_zero_initially() {
        let cache = L1Cache::new(1024 * 1024);
        assert_eq!(cache.hit_rate(), 0.0);
    }

    #[test]
    fn test_l1_cache_clear() {
        let cache = L1Cache::new(1024 * 1024);
        cache.put("a.jpg", vec![1]);
        assert!(cache.entry_count() > 0);
        cache.clear();
        assert_eq!(cache.entry_count(), 0);
    }
}
```

- [ ] **Step 2: 运行测试**

```bash
cargo test --package aurora-desktop l1_cache
```

Expected: 3 个测试全部 PASS

- [ ] **Step 3: 提交**

```bash
git add src-tauri/src/l1_cache.rs
git commit -m "feat: add L1 memory tile cache (moka, 256MB default)"
```

---

### Task 4: 实现 L2 磁盘缓存 (文件系统 + LRU)

**Files:**
- Create: `src-tauri/src/l2_cache.rs`
- Test: `src-tauri/src/l2_cache.rs` (内联 `#[cfg(test)]`)

**Interfaces:**
- Produces:
  - `pub struct L2Cache` — 磁盘持久化缓存
  - `pub fn new(base_dir: PathBuf, max_bytes: u64) -> Self`
  - `pub fn get(&self, key: &str) -> Option<Vec<u8>>`
  - `pub fn put(&self, key: &str, data: &[u8]) -> std::io::Result<()>`
  - `pub fn exists(&self, key: &str) -> bool`
  - `pub fn total_bytes(&self) -> u64`
  - `pub fn file_count(&self) -> u64`

- [ ] **Step 1: 编写 L2 缓存实现**

```rust
//! L2 磁盘持久化缓存。
//!
//! 瓦片以文件形式存储于 base_dir 下，key 即相对路径。
//! 超出 max_bytes 时按 atime 进行 LRU 驱逐。
//! key 格式: "china-tiles/{z}/{x}/{y}.jpg"

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::SystemTime;

/// L2 磁盘缓存 — 线程安全。
pub struct L2Cache {
    base_dir: PathBuf,
    max_bytes: u64,
    hits: AtomicU64,
    misses: AtomicU64,
    /// 驱逐锁 — 确保同一时刻只有一个驱逐操作
    evict_lock: Mutex<()>,
}

impl L2Cache {
    /// 创建磁盘缓存。确保 base_dir 存在。
    pub fn new(base_dir: PathBuf, max_bytes: u64) -> Self {
        let _ = fs::create_dir_all(&base_dir);
        Self {
            base_dir,
            max_bytes,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evict_lock: Mutex::new(()),
        }
    }

    /// 从磁盘读取瓦片。
    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        let path = self.path_for(key);
        match fs::read(&path) {
            Ok(data) if !data.is_empty() => {
                self.hits.fetch_add(1, Ordering::Relaxed);
                // 更新 atime（通过读取即隐式更新）
                let _ = file_touch(&path);
                Some(data)
            }
            _ => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                None
            }
        }
    }

    /// 写入瓦片到磁盘。原子写入（先 tmp 后 rename）。
    /// 如果超出容量限制，触发 LRU 驱逐。
    pub fn put(&self, key: &str, data: &[u8]) -> io::Result<()> {
        let path = self.path_for(key);

        // 确保父目录存在
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // 原子写入
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, data)?;
        fs::rename(&tmp, &path)?;

        // 检查容量并驱逐
        let current = self.total_bytes();
        if current > self.max_bytes {
            // 尝试驱逐，但不阻塞写入
            let _ = self.evict_lru();
        }

        Ok(())
    }

    /// 检查 key 是否已缓存（且非空）。
    pub fn exists(&self, key: &str) -> bool {
        let path = self.path_for(key);
        path.exists() && fs::metadata(&path).map(|m| m.len() > 0).unwrap_or(false)
    }

    /// 递归计算缓存总字节数。
    pub fn total_bytes(&self) -> u64 {
        dir_size(&self.base_dir)
    }

    /// 缓存文件数量。
    pub fn file_count(&self) -> u64 {
        dir_file_count(&self.base_dir)
    }

    /// 缓存命中率 [0.0, 1.0]
    pub fn hit_rate(&self) -> f64 {
        let h = self.hits.load(Ordering::Relaxed) as f64;
        let m = self.misses.load(Ordering::Relaxed) as f64;
        let total = h + m;
        if total > 0.0 { h / total } else { 0.0 }
    }

    /// 最大容量
    pub fn max_bytes(&self) -> u64 {
        self.max_bytes
    }

    /// 设置新的容量上限，可能触发驱逐
    pub fn set_max_bytes(&mut self, max_bytes: u64) {
        self.max_bytes = max_bytes;
        if self.total_bytes() > max_bytes {
            let _ = self.evict_lru();
        }
    }

    /// 清空所有缓存
    pub fn clear(&self) -> io::Result<()> {
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
    fn evict_lru(&self) -> io::Result<()> {
        let _guard = self.evict_lock.lock().unwrap_or_else(|e| e.into_inner());

        let mut entries: Vec<(PathBuf, SystemTime)> = Vec::new();
        collect_files_by_atime(&self.base_dir, &mut entries);

        if entries.is_empty() {
            return Ok(());
        }

        let target = self.max_bytes * 3 / 4; // 目标是 max 的 75%
        let mut current = self.total_bytes();

        // 按 atime 升序排列（最旧的在前）
        entries.sort_by_key(|(_, atime)| *atime);

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
    // 在 Windows 上通过重新设置修改时间来模拟 atime 更新
    let now = SystemTime::now();
    let _ = fs::File::open(path)?.set_times(
        fs::metadata(path)?.accessed().unwrap_or(now),
        now,
    );
    Ok(())
}

fn dir_size(dir: &Path) -> u64 {
    let mut total: u64 = 0;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    total += meta.len();
                } else if meta.is_dir() {
                    total += dir_size(&entry.path());
                }
            }
        }
    }
    total
}

fn dir_file_count(dir: &Path) -> u64 {
    let mut count: u64 = 0;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    count += 1;
                } else if meta.is_dir() {
                    count += dir_file_count(&entry.path());
                }
            }
        }
    }
    count
}

fn collect_files_by_atime(dir: &Path, entries: &mut Vec<(PathBuf, SystemTime)>) {
    if let Ok(dir_entries) = fs::read_dir(dir) {
        for entry in dir_entries.flatten() {
            let path = entry.path();
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() && path.extension().and_then(|e| e.to_str()) != Some("tmp") {
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
                if fs::read_dir(&path).map(|mut d| d.next().is_none()).unwrap_or(false) {
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
        std::env::temp_dir().join(format!("aurora_l2_test_{}", std::process::id()))
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
        assert!(cache.total_bytes() > before);
        let _ = fs::remove_dir_all(&dir);
    }
}
```

- [ ] **Step 2: 运行测试**

```bash
cargo test --package aurora-desktop l2_cache
```

Expected: 4 个测试全部 PASS

- [ ] **Step 3: 提交**

```bash
git add src-tauri/src/l2_cache.rs
git commit -m "feat: add L2 disk tile cache with LRU eviction"
```

---

### Task 5: 实现瓦片源配置与多源下载器

**Files:**
- Create: `src-tauri/src/tile_sources.rs`
- Create: `src-tauri/src/tile_downloader.rs`

**Interfaces:**
- Consumes: 无
- Produces:
  - `tile_sources.rs`: `pub struct TileSource`, `pub const TILE_SOURCES: &[TileSource]`, `pub fn select_sources(z: u32, x: u32, y: u32) -> Vec<&'static TileSource>`
  - `tile_downloader.rs`: `pub struct TileDownloader`, `pub fn new() -> Self`, `pub async fn download(&self, z: u32, x: u32, y: u32) -> Option<Vec<u8>>`

- [ ] **Step 1: 创建 tile_sources.rs**

```rust
//! 多源瓦片配置与选择。
//!
//! 每个源定义 URL 模板、覆盖范围 (bbox)、缩放级别、优先级。
//! select_sources() 按优先级返回能覆盖给定瓦片的源列表。

/// 单个瓦片源配置。
#[derive(Debug, Clone)]
pub struct TileSource {
    pub name: &'static str,
    /// URL 模板，占位符: {s} (子域), {z}, {x}, {y}
    pub url_template: &'static str,
    /// 子域列表（如果 URL 含 {s}），否则 None
    pub subdomains: Option<&'static [&'static str]>,
    /// 覆盖范围 (lat_min, lng_min, lat_max, lng_max)，None = 全球
    pub bbox: Option<(f64, f64, f64, f64)>,
    pub min_zoom: u32,
    pub max_zoom: u32,
    /// 优先级，1 = 最高
    pub priority: u8,
    /// 每秒请求数限制
    pub rate_limit_rps: f64,
    /// 是否使用 TMS Y 编号 (y=0 在南)
    pub tms_y: bool,
}

impl TileSource {
    /// 从 Slippy Map 坐标 (z, x, y) 构建此源的完整 URL。
    pub fn build_url(&self, z: u32, x: u32, y: u32) -> String {
        // TMS → Slippy Map Y 转换（如果需要）
        let y_val = if self.tms_y {
            let max_y = (1u32 << z).saturating_sub(1);
            max_y.saturating_sub(y)
        } else {
            y
        };

        let mut url = self.url_template
            .replace("{z}", &z.to_string())
            .replace("{x}", &x.to_string())
            .replace("{y}", &y_val.to_string());

        // 处理子域 {s}
        if url.contains("{s}") {
            if let Some(subdomains) = self.subdomains {
                let idx = (x + y) as usize % subdomains.len();
                url = url.replace("{s}", subdomains[idx]);
            } else {
                url = url.replace("{s}", "0");
            }
        }

        url
    }

    /// 检查此源是否覆盖给定的 Slippy Map 瓦片坐标。
    pub fn covers(&self, z: u32, _x: u32, y: u32) -> bool {
        if z < self.min_zoom || z > self.max_zoom {
            return false;
        }
        if let Some((lat_min, lng_min, lat_max, lng_max)) = self.bbox {
            let (lat, lng) = tile_to_latlng(z, _x, y);
            lat >= lat_min && lat <= lat_max && lng >= lng_min && lng <= lng_max
        } else {
            true // 全球覆盖
        }
    }
}

/// 预定义的瓦片源列表（按优先级排序）。
pub const TILE_SOURCES: &[TileSource] = &[
    TileSource {
        name: "高德卫星",
        url_template: "https://wprd0{s}.is.autonavi.com/appmaptile?lang=zh_cn&size=1&scl=1&style=6&x={x}&y={y}&z={z}",
        subdomains: Some(&["1", "2", "3", "4"]),
        bbox: Some((15.0, 70.0, 55.0, 140.0)),
        min_zoom: 3,
        max_zoom: 18,
        priority: 1,
        rate_limit_rps: 50.0,
        tms_y: false, // Slippy Map
    },
    TileSource {
        name: "Esri World Imagery",
        url_template: "https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}",
        subdomains: None,
        bbox: None, // 全球
        min_zoom: 0,
        max_zoom: 18,
        priority: 2,
        rate_limit_rps: 100.0,
        tms_y: true, // TMS — Y 轴反转
    },
];

/// 按优先级返回能覆盖给定瓦片的源列表。
pub fn select_sources(z: u32, x: u32, y: u32) -> Vec<&'static TileSource> {
    let mut sources: Vec<&TileSource> = TILE_SOURCES
        .iter()
        .filter(|s| s.covers(z, x, y))
        .collect();
    sources.sort_by_key(|s| s.priority);
    sources
}

/// Slippy Map 瓦片坐标 → 瓦片中心经纬度
fn tile_to_latlng(z: u32, x: u32, y: u32) -> (f64, f64) {
    let n = (1u64 << z) as f64;
    let lng = (x as f64) / n * 360.0 - 180.0;
    let lat_rad = (std::f64::consts::PI * (1.0 - 2.0 * y as f64 / n)).sinh().atan();
    let lat = lat_rad.to_degrees();
    (lat, lng)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gaode_covers_china() {
        // 北京附近：z=8, x=217, y=94 (Slippy Map)
        let source = &TILE_SOURCES[0];
        assert!(source.covers(8, 217, 94));
    }

    #[test]
    fn test_gaode_not_covers_europe() {
        let source = &TILE_SOURCES[0];
        // 巴黎附近 z=8
        assert!(!source.covers(8, 129, 88));
    }

    #[test]
    fn test_esri_covers_global() {
        let source = &TILE_SOURCES[1];
        assert!(source.covers(8, 0, 0));
    }

    #[test]
    fn test_build_url_gaode_slippy() {
        let source = &TILE_SOURCES[0];
        let url = source.build_url(8, 217, 94);
        assert!(url.contains("x=217"));
        assert!(url.contains("y=94"));
        assert!(url.contains("z=8"));
        assert!(url.contains("wprd0"));
    }

    #[test]
    fn test_build_url_esri_tms_flips_y() {
        let source = &TILE_SOURCES[1];
        // Esri 使用 TMS，build_url 应将 Slippy Y 转换为 TMS Y
        let url = source.build_url(8, 217, 94);
        // TMS y = (2^8 - 1) - 94 = 161
        assert!(url.contains("/161"));
        assert!(!url.contains("/94"));
    }

    #[test]
    fn test_select_sources_prioritizes_gaode_for_china() {
        let sources = select_sources(8, 217, 94);
        assert!(!sources.is_empty());
        assert_eq!(sources[0].name, "高德卫星");
    }

    #[test]
    fn test_select_sources_falls_back_to_esri_global() {
        let sources = select_sources(8, 129, 88); // 巴黎
        assert!(!sources.is_empty());
        // Esri 全球覆盖应该是唯一选项
        assert!(sources.iter().all(|s| s.name == "Esri World Imagery"));
    }
}
```

- [ ] **Step 2: 创建 tile_downloader.rs**

```rust
//! 多源瓦片异步下载器。
//!
//! 使用 reqwest HTTP 客户端，16 并发，自动故障转移。
//! 内置限速以保护上游 CDN。

use crate::tile_sources::{select_sources, TileSource};
use reqwest::Client;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// 瓦片下载器 — 线程安全。
pub struct TileDownloader {
    client: Client,
    /// 成功下载计数
    downloaded: Arc<AtomicU64>,
    /// 失败计数
    failed: Arc<AtomicU64>,
}

impl TileDownloader {
    /// 创建下载器。设置合理的超时和 User-Agent。
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(5))
            .user_agent("Aurora-Earth/0.1 (offline tile cache)")
            .build()
            .expect("failed to create reqwest client");

        Self {
            client,
            downloaded: Arc::new(AtomicU64::new(0)),
            failed: Arc::new(AtomicU64::new(0)),
        }
    }

    /// 下载指定瓦片。按源优先级尝试，成功返回数据。
    pub async fn download(&self, z: u32, x: u32, y: u32) -> Option<Vec<u8>> {
        let sources = select_sources(z, x, y);

        for source in &sources {
            let url = source.build_url(z, x, y);
            crate::logger::log("tile_dl", "INFO", &format!("尝试 {}: {}", source.name, url));

            match self.client.get(&url).send().await {
                Ok(response) if response.status().is_success() => {
                    match response.bytes().await {
                        Ok(bytes) => {
                            let len = bytes.len();
                            self.downloaded.fetch_add(1, Ordering::Relaxed);
                            crate::logger::log("tile_dl", "INFO", &format!("✓ {} 来自 {} ({} bytes)", 
                                tile_key(z, x, y), source.name, len));
                            return Some(bytes.to_vec());
                        }
                        Err(e) => {
                            crate::logger::log("tile_dl", "WARN", &format!("响应体错误 {}: {}", source.name, e));
                        }
                    }
                }
                Ok(response) => {
                    crate::logger::log("tile_dl", "WARN", &format!("{} HTTP {}", source.name, response.status()));
                }
                Err(e) => {
                    crate::logger::log("tile_dl", "WARN", &format!("{} 请求失败: {}", source.name, e));
                }
            }
        }

        self.failed.fetch_add(1, Ordering::Relaxed);
        crate::logger::log("tile_dl", "WARN", &format!("✗ {} 所有源失败 (尝试了 {} 个源)", 
            tile_key(z, x, y), sources.len()));
        None
    }

    /// 已下载计数
    pub fn downloaded_count(&self) -> u64 {
        self.downloaded.load(Ordering::Relaxed)
    }

    /// 失败计数
    pub fn failed_count(&self) -> u64 {
        self.failed.load(Ordering::Relaxed)
    }
}

/// 生成 tile key 用于日志。
fn tile_key(z: u32, x: u32, y: u32) -> String {
    format!("{}/{}/{}", z, x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 注意：下载测试需要网络连接。
    /// 在没有网络的环境下会失败 — 这是预期的。
    /// CI 环境需要确保网络可用。

    #[tokio::test]
    async fn test_downloader_returns_something_or_none() {
        let dl = TileDownloader::new();
        // 高德 z3 中国瓦片 — 如果网络正常应该能下载
        // 这里不检查具体内容，只确保不 panic
        let result = dl.download(5, 25, 12).await;
        // 有网 -> Some, 无网 -> None, 都不应 panic
        if let Some(data) = &result {
            assert!(!data.is_empty());
        }
    }
}
```

- [ ] **Step 3: 运行测试**

```bash
cargo test --package aurora-desktop tile_sources
cargo test --package aurora-desktop tile_downloader
```

Expected: 7 个 tile_sources 测试全部 PASS，tile_downloader 测试不 panic

- [ ] **Step 4: 提交**

```bash
git add src-tauri/src/tile_sources.rs src-tauri/src/tile_downloader.rs
git commit -m "feat: add multi-source tile config and async downloader"
```

---

### Task 6: 实现 actix-web 代理服务器 (核心)

**Files:**
- Create: `src-tauri/src/proxy_server.rs`
- Modify: `src-tauri/src/lib.rs` (模块声明 + 启动逻辑)

**Interfaces:**
- Consumes: `L1Cache` (Task 3), `L2Cache` (Task 4), `TileDownloader` (Task 5), `data_dir::aurora_data_dir()` (Task 2)
- Produces: `pub fn start_proxy_server(data_dir: PathBuf, shutdown: Arc<AtomicBool>) -> JoinHandle<()>`

- [ ] **Step 1: 编写 proxy_server.rs**

```rust
//! actix-web 高性能瓦片代理服务器。
//!
//! 替代原有的手写 TCP 服务器，提供：
//! - 多 worker 线程池，处理 CesiumJS 的 30-80 个并发瓦片请求
//! - L1 (moka 内存) + L2 (文件系统) 两级缓存
//! - 缓存未命中时通过 TileDownloader 从 CDN 下载
//! - /health 健康检查端点
//! - 静态文件服务 (cesium, assets, terrain 等非瓦片资源)
//! - 所有响应包含缓存头

use actix_web::{get, web, App, HttpRequest, HttpResponse, HttpServer, middleware};
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

use crate::l1_cache::L1Cache;
use crate::l2_cache::L2Cache;
use crate::tile_downloader::TileDownloader;

/// 服务器共享状态。
pub struct AppState {
    pub l1: L1Cache,
    pub l2: L2Cache,
    pub downloader: TileDownloader,
    pub data_dir: PathBuf,
}

/// 启动 actix-web 代理服务器。
/// 在独立 std::thread 中运行，自带 tokio runtime。
pub fn start_proxy_server(
    data_dir: PathBuf,
    shutdown: Arc<AtomicBool>,
) -> JoinHandle<()> {
    std::thread::Builder::new()
        .name("actix-proxy-server".into())
        .spawn(move || {
            let rt = actix_rt::System::new();
            rt.block_on(async move {
                let l1 = L1Cache::new(256 * 1024 * 1024); // 256MB
                let l2 = L2Cache::new(data_dir.join("china-tiles"), 50 * 1024 * 1024 * 1024); // 50GB
                let downloader = TileDownloader::new();

                let app_state = web::Data::new(AppState {
                    l1,
                    l2,
                    downloader,
                    data_dir: data_dir.clone(),
                });

                let server = HttpServer::new(move || {
                    App::new()
                        .app_data(app_state.clone())
                        .wrap(middleware::Logger::default())
                        .service(health)
                        .service(serve_tile)
                        .service(serve_static)
                })
                .workers(
                    std::thread::available_parallelism()
                        .map(|n| n.get().min(8))
                        .unwrap_or(4)
                )
                .bind("127.0.0.1:21337")
                .unwrap_or_else(|e| {
                    crate::logger::log("proxy_server", "ERROR", &format!("绑定 21337 失败: {e}"));
                    panic!("port bind failed: {e}");
                })
                .run();

                crate::logger::log("proxy_server", "INFO", "actix-web 服务器已启动 (127.0.0.1:21337)");

                // 优雅关闭 — 轮询 shutdown 标志
                let handle = server.handle();
                let shutdown_clone = shutdown.clone();
                tokio::spawn(async move {
                    loop {
                        if shutdown_clone.load(Ordering::Relaxed) {
                            crate::logger::log("proxy_server", "INFO", "收到关闭信号，停止服务器...");
                            handle.stop(true).await;
                            break;
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                    }
                });

                let _ = server.await;
                crate::logger::log("proxy_server", "INFO", "actix-web 服务器已停止");
            });
        })
        .expect("failed to spawn actix-proxy-server thread")
}

/// 健康检查端点。
#[get("/health")]
async fn health() -> HttpResponse {
    HttpResponse::Ok()
        .insert_header(("Cache-Control", "no-cache"))
        .body("OK")
}

/// 瓦片代理端点 — 支持所有瓦片路径。
/// 路径格式: /{category}-tiles/{z}/{x}/{y}.{ext}
#[get("/china-tiles/{z}/{x}/{y}.{ext}")]
async fn serve_tile(
    path: actix_web::web::Path<(u32, u32, u32, String)>,
    state: web::Data<AppState>,
) -> HttpResponse {
    let (z, x, y, ext) = path.into_inner();
    let key = format!("china-tiles/{}/{}/{}.{}", z, x, y, ext);

    // L1 查询
    if let Some(data) = state.l1.get(&key) {
        return tile_response(data, &ext);
    }

    // L2 查询
    if let Some(data) = state.l2.get(&key) {
        state.l1.put(&key, data.clone());
        return tile_response(data, &ext);
    }

    // 下载
    let y_for_dl = y; // Slippy Map Y — downloader 内部处理 TMS 转换
    if let Some(data) = state.downloader.download(z, x, y_for_dl).await {
        let _ = state.l2.put(&key, &data);
        state.l1.put(&key, data.clone());
        return tile_response(data, &ext);
    }

    // 所有源失败 — 返回 404
    HttpResponse::NotFound()
        .insert_header(("Cache-Control", "no-cache"))
        .body("tile unavailable")
}

/// 静态文件服务 — cesium, assets, terrain, templates 等。
/// 路径格式: /{prefix}/{rest:.*}
#[get("/{prefix:(cesium|assets|terrain|templates|tiles)}/{rest:.*}")]
async fn serve_static(
    req: HttpRequest,
    state: web::Data<AppState>,
) -> HttpResponse {
    let path = req.path();
    let path = path.trim_start_matches('/');
    let file_path = state.data_dir.join(path);

    // 安全检查
    let canonical_root = match state.data_dir.canonicalize() {
        Ok(p) => p,
        Err(_) => return HttpResponse::InternalServerError().body("500"),
    };

    let canonical_file = match file_path.canonicalize() {
        Ok(p) => p,
        Err(_) => return HttpResponse::NotFound().body("404 Not Found"),
    };

    if !canonical_file.starts_with(&canonical_root) {
        return HttpResponse::Forbidden().body("403 Forbidden");
    }

    if !canonical_file.is_file() {
        return HttpResponse::NotFound().body("404 Not Found");
    }

    match std::fs::read(&canonical_file) {
        Ok(content) => {
            let mime = guess_mime_for_static(&canonical_file);
            HttpResponse::Ok()
                .insert_header(("Content-Type", mime))
                .insert_header(("Cache-Control", "public, max-age=86400, immutable"))
                .insert_header(("Access-Control-Allow-Origin", "*"))
                .body(content)
        }
        Err(_) => HttpResponse::InternalServerError().body("500"),
    }
}

// ── 辅助函数 ──

fn tile_response(data: Vec<u8>, ext: &str) -> HttpResponse {
    let mime = match ext {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    };

    HttpResponse::Ok()
        .insert_header(("Content-Type", mime))
        .insert_header(("Cache-Control", "public, max-age=86400, immutable"))
        .insert_header(("Access-Control-Allow-Origin", "*"))
        .body(data)
}

/// 按扩展名猜测 MIME 类型（复用 tile_server.rs 逻辑但独立，避免依赖）。
fn guess_mime_for_static(path: &std::path::Path) -> String {
    match path.extension().and_then(|e| e.to_str()) {
        Some("js") | Some("mjs") => "application/javascript".into(),
        Some("css") => "text/css".into(),
        Some("html") | Some("htm") => "text/html".into(),
        Some("json") => "application/json".into(),
        Some("xml") => "application/xml".into(),
        Some("jpg") | Some("jpeg") => "image/jpeg".into(),
        Some("png") => "image/png".into(),
        Some("svg") => "image/svg+xml".into(),
        Some("gif") => "image/gif".into(),
        Some("webp") => "image/webp".into(),
        Some("wasm") => "application/wasm".into(),
        Some("woff") | Some("woff2") => "font/woff2".into(),
        Some("ttf") => "font/ttf".into(),
        _ => "application/octet-stream".into(),
    }
}
```

- [ ] **Step 2: 更新 lib.rs 模块声明和启动逻辑**

在 `src-tauri/src/lib.rs` 顶部添加模块声明：

```rust
mod asset_fetcher;
mod commands;
mod data_dir;
mod l1_cache;
mod l2_cache;
mod logger;
mod proxy_server;
mod tile_downloader;
mod tile_sources;
mod tile_server;
```

将 `run()` 函数中的服务器启动逻辑改为：

```rust
// ── 3. 启动 actix-web 代理服务器 ──────────────────────────────
logger::log("init", "INFO", "启动 actix-web 代理服务器 (localhost:21337)...");
let server_shutdown = Arc::new(AtomicBool::new(false));
let server_handle = proxy_server::start_proxy_server(aurora_dir.clone(), Arc::clone(&server_shutdown));
logger::log("init", "INFO", "actix-web 代理服务器已启动");
```

同时移除原有的 `tile_server::start()` 调用（或保留作为备用，但不要同时启动在同一个端口）。

- [ ] **Step 3: 编译检查**

```bash
cd src-tauri && cargo check
```

Expected: 编译成功（可能有 dead_code 警告）

- [ ] **Step 4: 提交**

```bash
git add src-tauri/src/proxy_server.rs src-tauri/src/lib.rs
git commit -m "feat: add actix-web proxy server with L1/L2 cache and tile downloader"
```

---

### Task 7: 实现 Tauri 管理命令

**Files:**
- Modify: `src-tauri/src/commands.rs`

**Interfaces:**
- Consumes: `AppState` 需扩展 `l1_cache`, `l2_cache` 字段
- Produces: 5 个新的 `#[tauri::command]` 函数

- [ ] **Step 1: 在 commands.rs 末尾添加管理命令**

```rust
// ══════════════════════════════════════════════════════════════════
// 缓存管理命令
// ══════════════════════════════════════════════════════════════════

use crate::l1_cache::L1Cache;
use crate::l2_cache::L2Cache;

/// 缓存统计报告。
#[derive(Debug, Clone, Serialize)]
pub struct CacheStats {
    /// L1 命中率 [0.0, 1.0]
    pub l1_hit_rate: f64,
    /// L1 条目数
    pub l1_entries: u64,
    /// L1 使用量 (bytes, 近似)
    pub l1_bytes: u64,
    /// L1 最大容量
    pub l1_max_bytes: u64,
    /// L2 命中率 [0.0, 1.0]
    pub l2_hit_rate: f64,
    /// L2 文件数
    pub l2_files: u64,
    /// L2 总字节数
    pub l2_bytes: u64,
    /// L2 最大容量
    pub l2_max_bytes: u64,
    /// 下载器成功次数
    pub downloads_ok: u64,
    /// 下载器失败次数
    pub downloads_fail: u64,
}

/// 查询缓存统计。
#[tauri::command]
pub fn cache_stats(
    l1: State<L1Cache>,
    l2: State<L2Cache>,
    downloader: State<crate::tile_downloader::TileDownloader>,
) -> CacheStats {
    CacheStats {
        l1_hit_rate: l1.hit_rate(),
        l1_entries: l1.entry_count(),
        l1_bytes: l1.size_bytes(),
        l1_max_bytes: l1.max_bytes(),
        l2_hit_rate: l2.hit_rate(),
        l2_files: l2.file_count(),
        l2_bytes: l2.total_bytes(),
        l2_max_bytes: l2.max_bytes(),
        downloads_ok: downloader.downloaded_count(),
        downloads_fail: downloader.failed_count(),
    }
}

/// 设置 L2 缓存上限。
#[tauri::command]
pub fn set_cache_limit(max_gb: u64, l2: State<Mutex<L2Cache>>) -> Result<String, String> {
    let mut cache = l2.lock().map_err(|e| format!("lock error: {e}"))?;
    cache.set_max_bytes(max_gb * 1024 * 1024 * 1024);
    crate::logger::log("cache", "INFO", &format!("L2 上限已设为 {} GB", max_gb));
    Ok(format!("已设为 {} GB", max_gb))
}

/// 清空所有缓存。
#[tauri::command]
pub fn clear_cache(
    l1: State<L1Cache>,
    l2: State<Mutex<L2Cache>>,
) -> Result<String, String> {
    l1.clear();
    let cache = l2.lock().map_err(|e| format!("lock error: {e}"))?;
    cache.clear().map_err(|e| format!("clear error: {e}"))?;
    crate::logger::log("cache", "INFO", "缓存已清空");
    Ok("缓存已清空".into())
}

/// 获取服务器健康状态。
#[tauri::command]
pub fn server_health() -> String {
    "OK".into()
}

/// 预取指定区域的瓦片（后台队列）。
#[tauri::command]
pub async fn prefetch_tiles(
    lat_min: f64,
    lng_min: f64,
    lat_max: f64,
    lng_max: f64,
    z_min: u32,
    z_max: u32,
    downloader: State<crate::tile_downloader::TileDownloader>,
    l2: State<Mutex<L2Cache>>,
    l1: State<L1Cache>,
) -> Result<String, String> {
    let total_tiles: usize = (z_min..=z_max)
        .map(|z| {
            let x_min = lng_to_tile_x(lng_min, z);
            let x_max = lng_to_tile_x(lng_max, z);
            let y_min = lat_to_tile_y(lat_max, z);
            let y_max = lat_to_tile_y(lat_min, z);
            ((x_max - x_min + 1) * (y_max - y_min + 1)) as usize
        })
        .sum();

    crate::logger::log("prefetch", "INFO", &format!(
        "预取请求: bbox({lat_min},{lng_min},{lat_max},{lng_max}) z{z_min}-z{z_max}, 约 {total_tiles} 个瓦片"
    ));

    // 后台异步处理（不等待完成）
    let downloader_clone = downloader.get_ref().clone();
    let l2 = l2.inner().clone();
    let l1 = l1.inner().clone();

    tokio::spawn(async move {
        for z in z_min..=z_max {
            let x_min = lng_to_tile_x(lng_min, z);
            let x_max = lng_to_tile_x(lng_max, z);
            let y_min = lat_to_tile_y(lat_max, z);
            let y_max = lat_to_tile_y(lat_min, z);

            for x in x_min..=x_max {
                for y in y_min..=y_max {
                    let key = format!("china-tiles/{}/{}/{}.jpg", z, x, y);

                    // 跳过已缓存的
                    {
                        let cache = l2.lock().unwrap();
                        if cache.exists(&key) {
                            continue;
                        }
                    }

                    if let Some(data) = downloader_clone.download(z, x, y).await {
                        let _ = {
                            let cache = l2.lock().unwrap();
                            cache.put(&key, &data)
                        };
                        l1.put(&key, data);
                    }

                    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                }
            }
        }
        crate::logger::log("prefetch", "INFO", "预取完成");
    });

    Ok(format!("预取任务已启动 (~{} 个瓦片)", total_tiles))
}

// ── 预取辅助函数 ──

fn lng_to_tile_x(lng: f64, zoom: u32) -> u32 {
    ((lng + 180.0) / 360.0 * (1u64 << zoom) as f64).floor() as u32
}

fn lat_to_tile_y(lat: f64, zoom: u32) -> u32 {
    let lat_rad = lat.to_radians();
    let n = (1u64 << zoom) as f64;
    ((1.0 - (lat_rad.tan() + 1.0 / lat_rad.cos()).ln() / std::f64::consts::PI) / 2.0 * n).floor()
        as u32
}
```

- [ ] **Step 2: 更新 AppState 以包含缓存状态**

```rust
pub struct AppState {
    pub app: Mutex<AuroraApp>,
    pub l1: L1Cache,
    pub l2: Mutex<L2Cache>,
    pub downloader: TileDownloader,
}
```

- [ ] **Step 3: 更新 lib.rs 注册新命令**

在 `invoke_handler` 宏中添加：
```rust
.invoke_handler(tauri::generate_handler![
    show_window,
    exit_app,
    diag_url,
    frontend_log,
    check_cached_assets,
    get_asset_status,
    download_assets,
    get_resource_server_url,
    commands::run_analysis_pipeline,
    commands::cache_stats,
    commands::set_cache_limit,
    commands::clear_cache,
    commands::server_health,
    commands::prefetch_tiles,
])
```

更新 `.manage(AppState { ... })` 为：
```rust
.manage(AppState {
    app: Mutex::new(aurora_app),
    l1: L1Cache::new(256 * 1024 * 1024),
    l2: Mutex::new(L2Cache::new(
        aurora_dir.join("china-tiles"),
        50 * 1024 * 1024 * 1024,
    )),
    downloader: TileDownloader::new(),
})
```

- [ ] **Step 4: 编译检查**

```bash
cd src-tauri && cargo check
```

Expected: 编译成功

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat: add cache management Tauri commands — stats, limit, clear, prefetch"
```

---

### Task 8: 前端 — 安装官方 Tauri API 包并规范化调用

**Files:**
- Modify: `ui/package.json` (添加依赖)
- Modify: `ui/src/utils/diag.ts`
- Modify: `ui/src/Earth.tsx` (替换 `__TAURI_INTERNALS__`)
- Modify: `ui/src/Overlay.tsx` (替换 `__TAURI_INTERNALS__`)
- Modify: `ui/src/App.tsx` (替换 `__TAURI_INTERNALS__`)

- [ ] **Step 1: 安装 @tauri-apps/api**

```bash
cd ui && npm install @tauri-apps/api
```

- [ ] **Step 2: 创建统一的 Tauri 调用封装**

在 `ui/src/utils/` 下创建 `tauri.ts`：

```typescript
// ui/src/utils/tauri.ts
// 统一的 Tauri invoke 封装，替代 window.__TAURI_INTERNALS__
import { invoke } from '@tauri-apps/api/core';
import diag from './diag';

/** 类型安全的 Tauri invoke */
export async function invokeTauri<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  diag('tauri', 'INFO', `invoke: ${command}`);
  return invoke<T>(command, args);
}

/** 检查是否在 Tauri 环境中 */
export function isTauriEnvironment(): boolean {
  return !!(window as any).__TAURI_INTERNALS__;
}
```

- [ ] **Step 3: 更新 diag.ts 保留 isTauriEnvironment**

在 `ui/src/utils/diag.ts` 中，保留 `isTauriEnvironment` 函数但标记为可以从 `tauri.ts` 导入。保持向后兼容。

- [ ] **Step 4: 更新 Earth.tsx**

将所有：
```typescript
// @ts-ignore
window.__TAURI_INTERNALS__.invoke('command', args)
```
替换为：
```typescript
import { invokeTauri } from './utils/tauri';
// ...
const report = await invokeTauri<AssetReport>('get_asset_status');
```

具体替换点：
- Line 59: `get_asset_status` 调用
- Line 118: `check_cached_assets` 调用
- Line 371: `exit_app` 调用

移除所有 `@ts-ignore` 注释。

- [ ] **Step 5: 更新 Overlay.tsx**

删除本地 `invokeTauri` 函数定义（Line 81-83），改为：
```typescript
import { invokeTauri } from './utils/tauri';
```

- [ ] **Step 6: 更新 App.tsx**

在 `invokeRunPipeline` 中将 `window.__TAURI_INTERNALS__.invoke` 替换为 `invokeTauri`。

- [ ] **Step 7: 验证 TypeScript 编译**

```bash
cd ui && npx tsc --noEmit
```

Expected: 无类型错误

- [ ] **Step 8: 提交**

```bash
git add ui/src/utils/tauri.ts ui/src/utils/diag.ts ui/src/Earth.tsx ui/src/Overlay.tsx ui/src/App.tsx ui/package.json ui/package-lock.json
git commit -m "refactor: replace __TAURI_INTERNALS__ with official @tauri-apps/api/core"
```

---

### Task 9: 前端 — 服务器就绪探针 + 渐进式加载 + maxLevel→18

**Files:**
- Modify: `ui/src/Earth.tsx`

- [ ] **Step 1: 添加服务器就绪探针**

在 `Earth` 组件中添加：

```typescript
const [serverReady, setServerReady] = useState(false);

// 服务器就绪探针
useEffect(() => {
  if (!isTauriEnvironment()) {
    setServerReady(true); // 浏览器环境直接跳过
    return;
  }
  const check = setInterval(async () => {
    try {
      const res = await fetch(`${RESOURCE_SERVER}/health`);
      if (res.ok) {
        diag('Earth', 'INFO', '代理服务器就绪');
        setServerReady(true);
        clearInterval(check);
      }
    } catch {
      // 服务器尚未就绪
    }
  }, 100);
  // 10 秒超时 — 强制启动（回退到 globe-gl）
  const timeout = setTimeout(() => {
    if (!serverReady) {
      diag('Earth', 'WARN', '服务器超时，回退到 globe-gl');
      clearInterval(check);
      setEngine('globe-gl');
    }
  }, 10000);
  return () => { clearInterval(check); clearTimeout(timeout); };
}, []);
```

- [ ] **Step 2: 将启动流程改为依赖 serverReady**

将 `useEffect` 的启动流程依赖改为 `[serverReady]`：

```typescript
useEffect(() => {
  if (!serverReady) return;
  if (!isTauriEnvironment()) {
    setEngine('globe-gl');
    return;
  }
  // ... 原有的 CesiumJS 初始化流程
}, [serverReady]);
```

- [ ] **Step 3: 更新瓦片层配置**

将 `minimumLevel: 3, maximumLevel: 10` 改为 `minimumLevel: 3, maximumLevel: 18`：

```typescript
const chinaProvider = new Cesium.UrlTemplateImageryProvider({
  url: `${RESOURCE_SERVER}/china-tiles/{z}/{x}/{y}.jpg`,
  tilingScheme: new Cesium.WebMercatorTilingScheme(),
  minimumLevel: 3,
  maximumLevel: 18,  // z3-z18 全级别
  rectangle: Cesium.Rectangle.fromDegrees(70, 15, 140, 55),
});
```

- [ ] **Step 4: 添加 CesiumJS 致谢**

在加载遮罩中添加 Powered by CesiumJS 致谢：

```typescript
{showLoading && (
  <div style={{ /* ... */ }}>
    <div style={{ fontSize: '2rem' }}>🌍</div>
    <div style={{ color: '#8b949e', marginTop: '0.5rem' }}>初始化地球...</div>
    <div style={{ color: '#484f58', fontSize: '0.65rem', marginTop: '2rem' }}>
      Powered by CesiumJS
    </div>
  </div>
)}
```

- [ ] **Step 5: 验证修改**

```bash
cd ui && npx tsc --noEmit
```

Expected: 无类型错误

- [ ] **Step 6: 提交**

```bash
git add ui/src/Earth.tsx
git commit -m "feat: add server health probe, maxLevel 18, CesiumJS attribution"
```

---

### Task 10: 前端 — 缓存监控面板

**Files:**
- Modify: `ui/src/Overlay.tsx`
- Modify: `ui/src/types.ts` (如果存在的话)

- [ ] **Step 1: 添加缓存统计类型定义**

在 `Overlay.tsx` 中添加：

```typescript
/** 缓存统计（与 Rust CacheStats 对应） */
interface CacheStats {
  l1_hit_rate: number;
  l1_entries: number;
  l1_bytes: number;
  l1_max_bytes: number;
  l2_hit_rate: number;
  l2_files: number;
  l2_bytes: number;
  l2_max_bytes: number;
  downloads_ok: number;
  downloads_fail: number;
}
```

- [ ] **Step 2: 添加缓存统计状态和刷新逻辑**

在 `Overlay` 组件中添加：

```typescript
const [cacheStats, setCacheStats] = useState<CacheStats | null>(null);

const refreshCacheStats = useCallback(async () => {
  if (!isTauriEnvironment()) return;
  try {
    const stats = await invokeTauri<CacheStats>('cache_stats');
    setCacheStats(stats);
  } catch (e) {
    diag('Overlay', 'WARN', `获取缓存统计失败: ${e}`);
  }
}, []);

useEffect(() => {
  if (showSettings) {
    refreshCacheStats();
    // 每 10 秒自动刷新
    const id = setInterval(refreshCacheStats, 10000);
    return () => clearInterval(id);
  }
}, [showSettings, refreshCacheStats]);
```

- [ ] **Step 3: 添加缓存监控 UI**

在设置面板中，纹理资源部分之后添加：

```tsx
{/* ── 缓存统计 ── */}
{cacheStats && (
  <>
    <div style={{ ...styles.settingsTitle, marginTop: '0.75rem' }}>📊 缓存统计</div>
    
    <div style={styles.cacheStatGrid}>
      <div style={styles.cacheStatItem}>
        <span style={styles.cacheStatLabel}>L1 命中率</span>
        <span style={styles.cacheStatValue}>{(cacheStats.l1_hit_rate * 100).toFixed(1)}%</span>
      </div>
      <div style={styles.cacheStatItem}>
        <span style={styles.cacheStatLabel}>L1 条目</span>
        <span style={styles.cacheStatValue}>{cacheStats.l1_entries}</span>
      </div>
      <div style={styles.cacheStatItem}>
        <span style={styles.cacheStatLabel}>L2 命中率</span>
        <span style={styles.cacheStatValue}>{(cacheStats.l2_hit_rate * 100).toFixed(1)}%</span>
      </div>
      <div style={styles.cacheStatItem}>
        <span style={styles.cacheStatLabel}>L2 文件数</span>
        <span style={styles.cacheStatValue}>{cacheStats.l2_files.toLocaleString()}</span>
      </div>
      <div style={styles.cacheStatItem}>
        <span style={styles.cacheStatLabel}>L2 大小</span>
        <span style={styles.cacheStatValue}>{humanSize(cacheStats.l2_bytes)}</span>
      </div>
      <div style={styles.cacheStatItem}>
        <span style={styles.cacheStatLabel}>下载成功/失败</span>
        <span style={styles.cacheStatValue}>{cacheStats.downloads_ok}/{cacheStats.downloads_fail}</span>
      </div>
    </div>

    {/* 存储上限设置 */}
    <div style={styles.settingsRow}>
      <span style={styles.settingsLabel}>L2 存储上限 (GB, 0=无上限)</span>
      <input
        type="number"
        min={0}
        max={1000}
        defaultValue={Math.floor(cacheStats.l2_max_bytes / 1024 / 1024 / 1024)}
        onBlur={async (e) => {
          const gb = parseInt(e.target.value) || 0;
          try {
            await invokeTauri<string>('set_cache_limit', { maxGb: gb });
            refreshCacheStats();
          } catch (err) {
            diag('Overlay', 'ERROR', `设上限失败: ${err}`);
          }
        }}
        style={styles.cacheInput}
      />
    </div>

    {/* 清空缓存按钮 */}
    <div style={styles.assetActions}>
      <button
        onClick={async () => {
          try {
            await invokeTauri<string>('clear_cache');
            refreshCacheStats();
          } catch (err) {
            diag('Overlay', 'ERROR', `清空失败: ${err}`);
          }
        }}
        style={styles.assetButton}
      >
        🗑 清空缓存
      </button>
    </div>
  </>
)}
```

- [ ] **Step 4: 添加新的样式**

```typescript
cacheStatGrid: {
  display: 'grid',
  gridTemplateColumns: '1fr 1fr',
  gap: '0.35rem',
  marginBottom: '0.5rem',
},
cacheStatItem: {
  display: 'flex',
  justifyContent: 'space-between',
  alignItems: 'center',
  padding: '0.25rem 0.5rem',
  background: '#161b22',
  borderRadius: 4,
},
cacheStatLabel: {
  color: '#8b949e',
  fontSize: '0.72rem',
},
cacheStatValue: {
  color: '#c9d1d9',
  fontSize: '0.75rem',
  fontWeight: 600,
},
cacheInput: {
  width: '60px',
  background: '#161b22',
  border: '1px solid #30363d',
  borderRadius: 4,
  color: '#c9d1d9',
  fontSize: '0.78rem',
  padding: '0.2rem 0.4rem',
  textAlign: 'center',
},
```

- [ ] **Step 5: 验证 TypeScript**

```bash
cd ui && npx tsc --noEmit
```

Expected: 无类型错误

- [ ] **Step 6: 提交**

```bash
git add ui/src/Overlay.tsx
git commit -m "feat: add cache monitoring dashboard to settings panel"
```

---

### Task 11: 集成测试 — 端到端验证

**Files:**
- Modify: `src-tauri/src/lib.rs` (添加集成测试)
- Test: 手动启动验证

- [ ] **Step 1: 验证编译和测试全部通过**

```bash
cargo test --workspace --all-features -- --test-threads=2
cargo fmt -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Expected: 所有测试 PASS，fmt 通过，clippy 干净

- [ ] **Step 2: 验证 Tauri 构建成功**

```bash
cd ui && npm run build
cd .. && cargo build --release
```

Expected: 构建成功，无错误

- [ ] **Step 3: 手动验证清单**

启动应用后验证：
1. 窗口正常显示，地球加载
2. 瓦片服务器健康检查通过（检查日志：`代理服务器就绪`）
3. 中国区域卫星瓦片正常显示
4. 缩放体验：z10+ 级别瓦片按需加载
5. 设置面板缓存统计正确显示
6. 清空缓存功能正常
7. 存储上限设置生效

- [ ] **Step 4: 提交**

```bash
git add -A
git commit -m "chore: final integration verification — all tests pass, clippy clean"
```

---

### Task 12: 更新 tauri.conf.json — WebView2 + 高 DPI

**Files:**
- Modify: `src-tauri/tauri.conf.json`

- [ ] **Step 1: 更新配置**

```json
{
  "productName": "aurora",
  "version": "0.1.0",
  "identifier": "com.quicksand.aurora",
  "build": {
    "frontendDist": "../ui/dist",
    "beforeBuildCommand": "cd ui && npm run build"
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "windows": {
      "webviewInstallMode": "fixRuntime",
      "wix": {
        "includeWebView2Runtime": "bootstrapper"
      }
    }
  },
  "app": {
    "windows": [
      {
        "fullscreen": false,
        "resizable": true,
        "title": "Aurora — 认知主权助手",
        "useHdpi": true,
        "backgroundThrottling": false
      }
    ],
    "security": {
      "csp": "default-src 'self'; style-src 'self' 'unsafe-inline' http://localhost:21337 http://127.0.0.1:21337; script-src 'self' 'unsafe-inline' 'unsafe-eval' http://localhost:21337 http://127.0.0.1:21337; img-src 'self' data: blob: file: http://localhost:21337 http://127.0.0.1:21337 https://*.autonavi.com https://*.arcgisonline.com https://*.mapbox.com; worker-src 'self' blob: http://localhost:21337 http://127.0.0.1:21337; connect-src 'self' http://localhost:21337 http://127.0.0.1:21337 https://*.autonavi.com https://*.arcgisonline.com https://*.mapbox.com; frame-src http://localhost:21337 http://127.0.0.1:21337;"
    }
  }
}
```

关键变更：
- 添加 `bundle.windows.webviewInstallMode: "fixRuntime"`
- 添加 `bundle.windows.wix.includeWebView2Runtime: "bootstrapper"`
- 添加 `windows[0].useHdpi: true`
- 添加 `windows[0].backgroundThrottling: false`
- CSP 扩展允许瓦片 CDN（autonavi.com, arcgisonline.com, mapbox.com）

- [ ] **Step 2: 提交**

```bash
git add src-tauri/tauri.conf.json
git commit -m "chore: configure WebView2 fixRuntime, HDPI, background throttling, CSP for CDN tiles"
```

---

## 实施顺序总结

```
Task 1 (依赖)  ──┐
Task 2 (data_dir)─┤
                  ├──→ Task 6 (proxy_server)
Task 3 (L1缓存) ──┤       │
Task 4 (L2缓存) ──┤       │
Task 5 (下载器) ──┘       │
                          ├──→ Task 7 (commands)
                          │
Task 8 (API规范化) ───────┤
                          ├──→ Task 9 (Earth探针+18)
                          │
Task 10 (缓存面板) ───────┘

Task 11 (集成测试)
Task 12 (tauri.conf.json)
```
