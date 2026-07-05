//! Source registry — manages data source lifecycle, scheduling, and caching.
//!
//! Owns the set of configured DataSources and the L2 cache. Provides
//! `fetch_all()` for one-shot collection and `spawn_refresh_loop()` for
//! background periodic refresh.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::cache::{read_cached, write_cached, L2Cache};
use crate::source::DataSource;
use crate::types::RawSignal;

/// Manages a set of data sources with L2 caching.
pub struct SourceRegistry {
    sources: Vec<Box<dyn DataSource>>,
    cache: Arc<L2Cache>,
    /// Default TTL for cached raw signals (1 hour).
    cache_ttl: Duration,
}

impl SourceRegistry {
    /// Create an empty registry backed by the given cache.
    pub fn new(cache: Arc<L2Cache>) -> Self {
        Self {
            sources: Vec::new(),
            cache,
            cache_ttl: Duration::from_secs(3600),
        }
    }

    /// Add a data source.
    pub fn with_source(mut self, source: Box<dyn DataSource>) -> Self {
        self.sources.push(source);
        self
    }

    /// Number of registered sources.
    pub fn source_count(&self) -> usize {
        self.sources.len()
    }

    /// Source names for display.
    pub fn source_names(&self) -> Vec<&str> {
        self.sources.iter().map(|s| s.name()).collect()
    }

    /// Fetch all sources, reading from cache when fresh.
    ///
    /// Stale cache entries are returned (with a log) rather than blocking
    /// on a live fetch — this keeps the caller responsive. Use
    /// `refresh_all()` for forced live collection.
    pub async fn fetch_all(&self) -> Vec<RawSignal> {
        let mut all = Vec::with_capacity(64);
        for source in &self.sources {
            let cache_key = cache_key_for(source.name());
            // Try cache first
            if let Some(cached) =
                read_cached::<Vec<RawSignal>>(&self.cache, &cache_key, self.cache_ttl)
            {
                let count = cached.data.len();
                all.extend(cached.data);
                if !cached.stale {
                    tracing::debug!(source = source.name(), count, "cache hit");
                    continue;
                }
                // Stale — return old data but fall through to live fetch
                tracing::debug!(source = source.name(), count, "cache stale, using old data");
            }
            // Live fetch
            match source.fetch().await {
                Ok(signals) => {
                    let json = serde_json::to_vec(&signals).unwrap_or_default();
                    write_cached(&self.cache, &cache_key, &json);
                    all.extend(signals);
                }
                Err(e) => {
                    tracing::warn!(
                        source = source.name(),
                        error = %e,
                        "source fetch failed, skipping"
                    );
                }
            }
        }
        all
    }

    /// Force-refresh all sources (ignore cache).
    pub async fn refresh_all(&self) -> Vec<RawSignal> {
        let mut all = Vec::new();
        for source in &self.sources {
            match source.fetch().await {
                Ok(signals) => {
                    let json = serde_json::to_vec(&signals).unwrap_or_default();
                    write_cached(&self.cache, &cache_key_for(source.name()), &json);
                    all.extend(signals);
                }
                Err(e) => {
                    tracing::warn!(
                        source = source.name(),
                        error = %e,
                        "refresh failed, skipping"
                    );
                }
            }
        }
        all
    }

    /// Spawn a background refresh loop that runs every `interval`.
    ///
    /// Runs on a dedicated thread with its own tokio runtime.
    /// Checks `shutdown` flag between cycles.
    pub fn spawn_refresh_loop(
        cache: Arc<L2Cache>,
        sources: Vec<Box<dyn DataSource>>,
        interval: Duration,
        shutdown: Arc<AtomicBool>,
    ) {
        std::thread::Builder::new()
            .name("dataforge-refresher".into())
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(e) => {
                        tracing::error!("dataforge tokio runtime build failed: {e}");
                        return;
                    }
                };

                let registry = SourceRegistry {
                    sources,
                    cache,
                    cache_ttl: interval,
                };

                while !shutdown.load(Ordering::SeqCst) {
                    let signals = rt.block_on(registry.refresh_all());
                    tracing::info!(
                        count = signals.len(),
                        sources = registry.source_count(),
                        "dataforge refresh complete"
                    );

                    let mut slept = 0u64;
                    while slept < interval.as_secs() && !shutdown.load(Ordering::SeqCst) {
                        std::thread::sleep(Duration::from_secs(5));
                        slept += 5;
                    }
                }
                tracing::info!("dataforge refresh loop exited");
            })
            .expect("failed to spawn dataforge-refresher");
    }
}

fn cache_key_for(source_name: &str) -> String {
    format!(
        "dataforge/sources/{}.json",
        source_name.replace(' ', "_").to_lowercase()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DataCategory;
    use async_trait::async_trait;

    /// A test source that returns fixed data.
    struct TestSource {
        name: String,
        signals: Vec<RawSignal>,
    }

    #[async_trait]
    impl DataSource for TestSource {
        fn name(&self) -> &str {
            &self.name
        }
        fn category(&self) -> DataCategory {
            DataCategory::Other
        }
        async fn fetch(&self) -> Result<Vec<RawSignal>, crate::error::DataforgeError> {
            Ok(self.signals.clone())
        }
        fn fetch_interval(&self) -> Duration {
            Duration::from_secs(60)
        }
    }

    fn test_signal(id: &str) -> RawSignal {
        RawSignal {
            id: id.into(),
            source_url: "https://test.example.com".into(),
            source_name: "TestSource".into(),
            category: DataCategory::Other,
            raw_content: "test data".into(),
            captured_at: chrono::Utc::now(),
            data_period: None,
            location: None,
        }
    }

    fn test_cache() -> Arc<L2Cache> {
        let dir = std::env::temp_dir().join(format!(
            "dataforge_reg_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos()
        ));
        Arc::new(L2Cache::new(dir, 1024 * 1024))
    }

    #[tokio::test]
    async fn fetch_all_returns_signals() {
        let cache = test_cache();
        let source = TestSource {
            name: "test-source".into(),
            signals: vec![test_signal("s1"), test_signal("s2")],
        };
        let registry = SourceRegistry::new(cache).with_source(Box::new(source));
        let results = registry.fetch_all().await;
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn source_count_and_names() {
        let cache = test_cache();
        let source = TestSource {
            name: "source-a".into(),
            signals: vec![],
        };
        let registry = SourceRegistry::new(cache).with_source(Box::new(source));
        assert_eq!(registry.source_count(), 1);
        assert_eq!(registry.source_names(), vec!["source-a"]);
    }
}
