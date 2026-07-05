//! Integration test: SourceRegistry fetch_all with a mock data source.
//!
//! Tests caching behavior and fail-safe semantics without real HTTP.

use async_trait::async_trait;
use dataforge::cache::L2Cache;
use dataforge::error::DataforgeError;
use dataforge::registry::SourceRegistry;
use dataforge::source::DataSource;
use dataforge::types::{DataCategory, RawSignal};
use std::sync::Arc;
use std::time::Duration;

/// A test source that returns exactly one signal per fetch, counting calls.
struct MockSource {
    name: String,
    counter: std::sync::atomic::AtomicU32,
}

impl MockSource {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            counter: std::sync::atomic::AtomicU32::new(0),
        }
    }

    #[allow(dead_code)]
    fn call_count(&self) -> u32 {
        self.counter.load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[async_trait]
impl DataSource for MockSource {
    fn name(&self) -> &str {
        &self.name
    }

    fn category(&self) -> DataCategory {
        DataCategory::Other
    }

    async fn fetch(&self) -> Result<Vec<RawSignal>, DataforgeError> {
        let n = self
            .counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(vec![RawSignal {
            id: format!("mock-{}-{}", self.name, n),
            source_url: "mock://test".into(),
            source_name: self.name.clone(),
            category: DataCategory::Other,
            raw_content: format!("call_{}", n),
            captured_at: chrono::Utc::now(),
            data_period: None,
            location: None,
        }])
    }

    fn fetch_interval(&self) -> Duration {
        Duration::from_secs(60)
    }
}

fn test_dir() -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "dataforge_integration_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos()
    ))
}

#[tokio::test]
async fn fetch_all_caches_on_second_call() {
    let dir = test_dir();
    let cache = Arc::new(L2Cache::new(dir.clone(), 1024 * 1024));

    let mock = MockSource::new("test-source");
    let registry = SourceRegistry::new(Arc::clone(&cache)).with_source(Box::new(mock));

    // First fetch: should call the source live.
    let signals = registry.fetch_all().await;
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].raw_content, "call_0");

    // Second fetch: cache should be fresh, source not called again.
    let signals2 = registry.fetch_all().await;
    assert_eq!(signals2.len(), 1);
    assert_eq!(signals2[0].raw_content, "call_0"); // cached, not call_1

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn fetch_all_respects_stale_cache() {
    let dir = test_dir();
    let cache = Arc::new(L2Cache::new(dir.clone(), 1024 * 1024));

    // Pre-populate cache with backdated data.
    let old_signal = RawSignal {
        id: "old".into(),
        source_url: "mock://old".into(),
        source_name: "test-source".into(),
        category: DataCategory::Other,
        raw_content: "old_data".into(),
        captured_at: chrono::Utc::now(),
        data_period: None,
        location: None,
    };
    let cache_key = "dataforge/sources/test-source.json";
    let mut header = Vec::with_capacity(8 + 1024);
    let old_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .saturating_sub(7200); // 2 hours ago
    header.extend_from_slice(&old_ts.to_le_bytes());
    header.extend_from_slice(&serde_json::to_vec(&vec![old_signal]).unwrap());
    cache.put(&cache_key, &header).unwrap();

    // Registry with TTL = never (stale check uses fetch_all's internal TTL)
    let mock = MockSource::new("test-source");
    let registry = SourceRegistry::new(Arc::clone(&cache)).with_source(Box::new(mock));

    let signals = registry.fetch_all().await;
    // Should return the cached stale data + live fetch for fresh data
    assert!(!signals.is_empty());
    // First signal is the old cached one (stale, but returned)
    assert!(signals.iter().any(|s| s.raw_content == "old_data"));

    let _ = std::fs::remove_dir_all(&dir);
}
