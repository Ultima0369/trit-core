//! 多源瓦片异步下载器。
//!
//! 使用 reqwest HTTP 客户端，16 并发，自动故障转移。
//!
//! 批量下载方法 `download_batch` 用于首次离线地图预加载，
//! 使用 tokio Semaphore 控制并发度，避免触发服务端限流。

use crate::tile_sources::select_sources;
use reqwest::Client;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// 节奏控制：不要使劲整。
/// - 重试：每个源失败后指数退避（200ms → 400ms），最多 MAX_RETRIES 次
/// - 并发：默认 6（原 16 对 Esri 太猛，易触发限流）
/// - 抖动：源之间错峰，避免同步洪峰
const MAX_RETRIES: u32 = 2;
const BASE_BACKOFF_MS: u64 = 200;
const DEFAULT_CONCURRENCY: usize = 6;

/// 用系统时钟纳秒做 0–50ms 伪随机抖动（不加 rand crate）。
/// ponytail: 抖动只需错峰，无需密码学随机性。
fn jitter_duration() -> Duration {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    Duration::from_millis((nanos % 50) as u64)
}

/// 瓦片下载器 — 线程安全。
#[derive(Clone)]
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
            .pool_max_idle_per_host(DEFAULT_CONCURRENCY) // 连接池复用，与默认并发对齐
            .build()
            .expect("failed to create reqwest client");

        Self {
            client,
            downloaded: Arc::new(AtomicU64::new(0)),
            failed: Arc::new(AtomicU64::new(0)),
        }
    }

    /// 下载指定瓦片。按源优先级尝试，成功返回数据。
    ///
    /// 节奏优先：每个源失败后指数退避重试（200ms → 400ms），最多重试 2 次，
    /// 避免连续高频打 Esri 触发限流。源之间也加微小抖动错峰。
    pub async fn download(&self, z: u32, x: u32, y: u32) -> Option<Vec<u8>> {
        let sources = select_sources(z, x, y);

        for source in &sources {
            let url = source.build_url(z, x, y);
            crate::logger::log("tile_dl", "INFO", &format!("尝试 {}: {}", source.name, url));

            // 指数退避重试：同源失败后等 200ms / 400ms 再试。
            for attempt in 0..=MAX_RETRIES {
                if attempt > 0 {
                    let backoff = Duration::from_millis(BASE_BACKOFF_MS * (1 << (attempt - 1)));
                    tokio::time::sleep(backoff).await;
                }
                match self.client.get(&url).send().await {
                    Ok(response) if response.status().is_success() => match response.bytes().await {
                        Ok(bytes) => {
                            let len = bytes.len();
                            self.downloaded.fetch_add(1, Ordering::Relaxed);
                            crate::logger::log(
                                "tile_dl",
                                "INFO",
                                &format!(
                                    "✓ {} 来自 {} ({} bytes){}",
                                    tile_key(z, x, y),
                                    source.name,
                                    len,
                                    if attempt > 0 { format!(" (重试 {} 次)", attempt) } else { String::new() }
                                ),
                            );
                            return Some(bytes.to_vec());
                        }
                        Err(e) => {
                            crate::logger::log(
                                "tile_dl",
                                "WARN",
                                &format!("响应体错误 {}: {}", source.name, e),
                            );
                        }
                    },
                    Ok(response) => {
                        crate::logger::log(
                            "tile_dl",
                            "WARN",
                            &format!("{} HTTP {} (attempt {})", source.name, response.status(), attempt + 1),
                        );
                    }
                    Err(e) => {
                        crate::logger::log(
                            "tile_dl",
                            "WARN",
                            &format!("{} 请求失败 (attempt {}): {}", source.name, attempt + 1, e),
                        );
                    }
                }
            }
            // 换下一个源前加微小抖动错峰，避免多源同步重打。
            tokio::time::sleep(jitter_duration()).await;
        }

        self.failed.fetch_add(1, Ordering::Relaxed);
        crate::logger::log(
            "tile_dl",
            "WARN",
            &format!(
                "✗ {} 所有源失败 (尝试了 {} 个源)",
                tile_key(z, x, y),
                sources.len()
            ),
        );
        None
    }

    /// 批量并发下载瓦片并写入磁盘。
    ///
    /// `concurrency` 控制同时下载的最大任务数（建议 8-16）。
    /// 使用 tokio Semaphore 控制并发度。
    /// 每个瓦片下载成功后原子写入 `output_dir/{z}/{x}/{y}.jpg`。
    /// 已存在的文件跳过（非 force 模式）。
    ///
    /// 返回: (成功下载数, 失败数, 已跳过数)
    pub async fn download_batch(
        &self,
        tiles: &[(u32, u32, u32)], // [(z, x, y)]
        output_dir: &std::path::Path,
        force: bool,
        concurrency: usize,
        progress_interval: u64, // 每 N 个报告一次进度
    ) -> (u64, u64, u64) {
        use std::fs;

        let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency.max(1)));
        let mut handles = Vec::with_capacity(tiles.len());

        for &(z, x, y) in tiles {
            let permit = Arc::clone(&semaphore);
            let self_clone = self.clone();
            let output_dir = output_dir.to_path_buf();
            // Tuple: (z, x, y, success, skipped) — `success`=wrote a new file;
            // `skipped`=true only when the file already existed (not on failure).
            handles.push(tokio::spawn(async move {
                let _guard = permit.acquire().await;

                let rel_path = format!("{}/{}/{}.jpg", z, x, y);
                let target = output_dir.join(&rel_path);

                // 跳过已存在的文件
                if !force && target.exists() {
                    if let Ok(meta) = fs::metadata(&target) {
                        if meta.len() > 0 {
                            return (z, x, y, true, true); // skipped, not a new download
                        }
                    }
                }

                // 确保父目录存在
                if let Some(parent) = target.parent() {
                    let _ = fs::create_dir_all(parent);
                }

                match self_clone.download_single_tile(z, x, y).await {
                    Some(data) => {
                        // 原子写入
                        let tmp = target.with_extension("tmp");
                        let ok = fs::write(&tmp, &data).is_ok() && {
                            if target.exists() {
                                let _ = fs::remove_file(&target);
                            }
                            fs::rename(&tmp, &target).is_ok()
                        };
                        (z, x, y, ok, false)
                    }
                    None => (z, x, y, false, false), // failed — not skipped
                }
            }));
        }

        let mut downloaded: u64 = 0;
        let mut failed: u64 = 0;
        let mut skipped: u64 = 0;

        for handle in handles {
            match handle.await {
                Ok((_z, _x, _y, true, is_skip)) => {
                    if is_skip {
                        skipped += 1;
                    } else {
                        downloaded += 1;
                        if progress_interval > 0 && downloaded.is_multiple_of(progress_interval) {
                            crate::logger::log(
                                "tile_dl",
                                "INFO",
                                &format!(
                                    "  进度: {} 完成, {} 跳过, {} 失败",
                                    downloaded, skipped, failed
                                ),
                            );
                        }
                    }
                }
                Ok((_z, _x, _y, false, _is_skip)) => {
                    failed += 1;
                    if failed <= 5 {
                        crate::logger::log(
                            "tile_dl",
                            "WARN",
                            &format!("  瓦片 {}/{} 下载失败", _x, _y),
                        );
                    }
                }
                Err(join_err) => {
                    failed += 1;
                    crate::logger::log(
                        "tile_dl",
                        "WARN",
                        &format!("tokio join 错误: {}", join_err),
                    );
                }
            }
        }

        (downloaded, failed, skipped)
    }

    /// 下载单个瓦片，不更新全局计数器（由 batch 层面统一管理）。
    /// 同样指数退避重试，节奏与 download() 一致。
    async fn download_single_tile(&self, z: u32, x: u32, y: u32) -> Option<Vec<u8>> {
        let sources = select_sources(z, x, y);
        for source in &sources {
            let url = source.build_url(z, x, y);
            for attempt in 0..=MAX_RETRIES {
                if attempt > 0 {
                    tokio::time::sleep(Duration::from_millis(BASE_BACKOFF_MS * (1 << (attempt - 1)))).await;
                }
                match self.client.get(&url).send().await {
                    Ok(response) if response.status().is_success() => {
                        if let Ok(bytes) = response.bytes().await {
                            return Some(bytes.to_vec());
                        }
                    }
                    Ok(_) | Err(_) => continue,
                }
            }
            tokio::time::sleep(jitter_duration()).await;
        }
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

    #[tokio::test]
    async fn test_downloader_does_not_panic() {
        let dl = TileDownloader::new();
        // 高德 z5 中国瓦片 — 无论成功与否，不应 panic
        let result = dl.download(5, 25, 12).await;
        if let Some(data) = &result {
            assert!(!data.is_empty());
        }
        // 无网络时 result 为 None，同样不 panic
    }

    #[tokio::test]
    async fn test_download_batch_empty_tiles() {
        let dl = TileDownloader::new();
        let tmp = std::env::temp_dir().join(format!("aurora_batch_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let (ok, fail, skip) = dl.download_batch(&[], &tmp, false, 4, 100).await;
        assert_eq!(ok, 0);
        assert_eq!(fail, 0);
        assert_eq!(skip, 0);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
