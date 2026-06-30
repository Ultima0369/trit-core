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
    /// 当前已使用字节数（近似，moka 内部按 weigher 管理）
    max_bytes: u64,
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
}

impl L1Cache {
    /// 创建新缓存。
    /// `max_bytes` 近似值 — moka 按 weigher 管理逐出。
    pub fn new(max_bytes: u64) -> Self {
        let hits = Arc::new(AtomicU64::new(0));
        let misses = Arc::new(AtomicU64::new(0));

        let cache = Cache::builder()
            .weigher(|_key: &String, value: &Vec<u8>| -> u32 { value.len() as u32 })
            .max_capacity(max_bytes)
            .build();

        Self {
            cache,
            max_bytes,
            hits,
            misses,
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
        self.cache.insert(key.to_string(), data);
    }

    /// 缓存命中率 [0.0, 1.0]
    pub fn hit_rate(&self) -> f64 {
        crate::utils::hit_rate(
            self.hits.load(Ordering::Relaxed),
            self.misses.load(Ordering::Relaxed),
        )
    }

    /// 缓存使用量（精确值 — 基于 weigher 的加权总字节数）。
    /// moka 内部追踪每个 entry 的 weigher 返回值 (value.len() as u32)，
    /// weighted_size() 返回所有 entry 的加权值总和。
    pub fn size_bytes(&self) -> u64 {
        self.cache.weighted_size()
    }

    /// 缓存条目数
    pub fn entry_count(&self) -> u64 {
        self.cache.entry_count()
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
        // moka 内部异步处理，entry_count 可能延迟更新
        // 直接验证 get 能命中即可
        assert_eq!(cache.get("a.jpg"), Some(vec![1]));
        cache.clear();
        assert_eq!(cache.get("a.jpg"), None);
    }
}
