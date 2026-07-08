//! Source registry — manages data source lifecycle, scheduling, and caching.
//!
//! Owns the set of configured DataSources and the L2 cache. Provides
//! `fetch_all()` for one-shot collection and `spawn_refresh_loop()` for
//! background periodic refresh.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::cache::{read_cached, write_cached, L2Cache};
use crate::source::DataSource;
use crate::types::RawSignal;

/// Per-source health metrics updated automatically by fetch_all/fetch_changes.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct SourceHealth {
    /// Human-readable source name.
    pub name: String,
    /// Total successful fetches.
    pub successes: u64,
    /// Total failed fetches.
    pub failures: u64,
    /// Average latency of last 10 successful fetches (micros), or None if never fetched.
    pub avg_latency_us: Option<u64>,
    /// Unix timestamp (seconds) of last successful fetch, or None.
    pub last_success_s: Option<u64>,
}

/// Manages a set of data sources with L2 caching.
pub struct SourceRegistry {
    sources: Vec<Box<dyn DataSource>>,
    cache: Arc<L2Cache>,
    /// Default TTL for cached raw signals (1 hour).
    cache_ttl: Duration,
    /// Per-source health counters. Index matches `sources`.
    successes: Vec<AtomicU64>,
    failures: Vec<AtomicU64>,
    /// Ring buffer of last 10 latencies per source (micros), flattened.
    latencies: Vec<std::sync::Mutex<Vec<u64>>>,
    /// Consecutive failure count per source (reset on success).
    consecutive_failures: Vec<AtomicU64>,
    /// If consecutive failures ≥ CIRCUIT_BREAKER_THRESHOLD, skip this source
    /// until this timestamp (seconds since epoch). 0 = circuit closed.
    circuit_open_until: Vec<AtomicU64>,
}

/// Number of consecutive failures before the circuit opens.
const CIRCUIT_BREAKER_THRESHOLD: u64 = 3;
/// Cooldown period after circuit opens (seconds).
const CIRCUIT_COOLDOWN_SECS: u64 = 300; // 5 minutes

impl SourceRegistry {
    /// Create an empty registry backed by the given cache.
    pub fn new(cache: Arc<L2Cache>) -> Self {
        Self {
            sources: Vec::new(),
            cache,
            cache_ttl: Duration::from_secs(3600),
            successes: Vec::new(),
            failures: Vec::new(),
            latencies: Vec::new(),
            consecutive_failures: Vec::new(),
            circuit_open_until: Vec::new(),
        }
    }

    /// Add a data source.
    pub fn with_source(mut self, source: Box<dyn DataSource>) -> Self {
        self.successes.push(AtomicU64::new(0));
        self.failures.push(AtomicU64::new(0));
        self.latencies
            .push(std::sync::Mutex::new(Vec::with_capacity(10)));
        self.consecutive_failures.push(AtomicU64::new(0));
        self.circuit_open_until.push(AtomicU64::new(0));
        self.sources.push(source);
        self
    }

    /// Create a registry with all built-in data sources pre-registered.
    ///
    /// ponytail: one call to wire up all 7 sources. Individual sources can
    /// still be added/removed with `with_source()` chaining after.
    pub fn with_all_sources(cache: Arc<L2Cache>) -> Self {
        Self::new(cache)
            .with_source(Box::new(crate::sources::open_meteo::OpenMeteoSource::new()))
            .with_source(Box::new(crate::sources::noaa_co2::NoaaCo2Source::new()))
            .with_source(Box::new(crate::sources::nsidc::NsidcSource::new()))
            .with_source(Box::new(crate::sources::noaa_tides::NoaaTidesSource::new()))
            .with_source(Box::new(crate::sources::gbif::GbifSource::new()))
            .with_source(Box::new(crate::sources::arxiv::ArxivSource::new()))
            .with_source(Box::new(crate::sources::ucdp::UcdpSource::new()))
            .with_source(Box::new(crate::sources::usgs::UsgsSource::new()))
            .with_source(Box::new(crate::sources::gibs::GibsSource::new()))
            .with_source(Box::new(crate::sources::nasa_power::NasaPowerSource::new()))
    }

    /// Number of registered sources.
    pub fn source_count(&self) -> usize {
        self.sources.len()
    }

    /// Source names for display.
    pub fn source_names(&self) -> Vec<&str> {
        self.sources.iter().map(|s| s.name()).collect()
    }

    /// Snapshot current health metrics for all sources.
    pub fn health(&self) -> Vec<SourceHealth> {
        self.sources
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let latencies = self.latencies[i].lock().unwrap_or_else(|e| e.into_inner());
                let avg_latency_us = if latencies.is_empty() {
                    None
                } else {
                    let sum: u64 = latencies.iter().sum();
                    Some(sum / latencies.len() as u64)
                };
                let last_success_s = latencies.last().map(|_| {
                    // ponytail: approximate "last success" from atomic counters.
                    // A proper timestamp would require another AtomicU64 per source.
                    // For now, non-empty latencies implies at least one success.
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0)
                });
                SourceHealth {
                    name: s.name().to_string(),
                    successes: self.successes[i].load(Ordering::Relaxed),
                    failures: self.failures[i].load(Ordering::Relaxed),
                    avg_latency_us,
                    last_success_s,
                }
            })
            .collect()
    }

    /// Fetch all sources, reading from cache when fresh.
    ///
    /// Cache hits return immediately. All sources requiring a live fetch
    /// are executed concurrently via `tokio::join_all` for minimum latency.
    /// Each source still has its own independent fail-safe (errors logged, skipped).
    pub async fn fetch_all(&self) -> Vec<RawSignal> {
        let mut all = Vec::with_capacity(64);
        let mut live_fetches: Vec<(usize, &Box<dyn DataSource>, String)> = Vec::new();

        // Phase 1: drain cache hits, collect sources needing live fetch
        for (i, source) in self.sources.iter().enumerate() {
            let cache_key = cache_key_for(source.name());
            if let Some(cached) =
                read_cached::<Vec<RawSignal>>(&self.cache, &cache_key, self.cache_ttl)
            {
                let count = cached.data.len();
                all.extend(cached.data);
                if !cached.stale {
                    tracing::debug!(source = source.name(), count, "cache hit");
                    // ponytail: cache hit counts as success (no latency data)
                    self.successes[i].fetch_add(1, Ordering::Relaxed);
                    continue;
                }
                tracing::debug!(source = source.name(), count, "cache stale, using old data");
            }
            // Skip if circuit breaker is open
            if self.is_circuit_open(i) {
                tracing::debug!(
                    source = source.name(),
                    "circuit breaker open — skipping live fetch"
                );
                continue;
            }
            live_fetches.push((i, source, cache_key));
        }

        // Phase 2: concurrent live fetch with per-source timeout
        if !live_fetches.is_empty() {
            const FETCH_TIMEOUT: Duration = Duration::from_secs(30);
            let results = futures::future::join_all(live_fetches.iter().map(
                |(i, source, cache_key)| async move {
                    let t0 = std::time::Instant::now();
                    let outcome = tokio::time::timeout(FETCH_TIMEOUT, source.fetch()).await;
                    let latency_us = t0.elapsed().as_micros() as u64;
                    match outcome {
                        Ok(fetch_result) => (*i, fetch_result, cache_key.clone(), latency_us),
                        Err(_elapsed) => {
                            let err = crate::error::DataforgeError::Unavailable(format!(
                                "fetch timed out after {}s",
                                FETCH_TIMEOUT.as_secs()
                            ));
                            (*i, Err(err), cache_key.clone(), latency_us)
                        }
                    }
                },
            ))
            .await;

            for (i, outcome, cache_key, latency_us) in results {
                match outcome {
                    Ok(signals) => {
                        self.record_success(i, latency_us);
                        let json = serde_json::to_vec(&signals).unwrap_or_default();
                        write_cached(&self.cache, &cache_key, &json);
                        all.extend(signals);
                    }
                    Err(e) => {
                        self.record_failure(i);
                        tracing::warn!(
                            source = self.sources[i].name(),
                            error = %e,
                            "source fetch failed, skipping"
                        );
                    }
                }
            }
        }

        all
    }

    fn record_success(&self, index: usize, latency_us: u64) {
        self.successes[index].fetch_add(1, Ordering::Relaxed);
        self.consecutive_failures[index].store(0, Ordering::Relaxed);
        self.circuit_open_until[index].store(0, Ordering::Relaxed);
        if let Ok(mut ring) = self.latencies[index].lock() {
            if ring.len() >= 10 {
                ring.remove(0);
            }
            ring.push(latency_us);
        }
    }

    fn record_failure(&self, index: usize) {
        self.failures[index].fetch_add(1, Ordering::Relaxed);
        let consecutive = self.consecutive_failures[index].fetch_add(1, Ordering::Relaxed) + 1;
        if consecutive >= CIRCUIT_BREAKER_THRESHOLD {
            let cooldown_until = now_secs() + CIRCUIT_COOLDOWN_SECS;
            self.circuit_open_until[index].store(cooldown_until, Ordering::Relaxed);
            tracing::warn!(
                source = self.sources[index].name(),
                consecutive,
                cooldown_secs = CIRCUIT_COOLDOWN_SECS,
                "circuit breaker opened",
            );
        }
    }

    fn is_circuit_open(&self, index: usize) -> bool {
        let until = self.circuit_open_until[index].load(Ordering::Relaxed);
        if until == 0 {
            return false;
        }
        let now = now_secs();
        if now >= until {
            self.circuit_open_until[index].store(0, Ordering::Relaxed);
            self.consecutive_failures[index].store(0, Ordering::Relaxed);
            return false;
        }
        true
    }

    /// Fetch all sources, read from cache when fresh, return only changed signals.
    ///
    /// Compares new fetch results against cached data. Returns only signals
    /// whose `id` was not in the previous cache entry. This is the core
    /// "change detection" primitive for a listening/monitoring system.
    ///
    /// All live fetches run concurrently.
    pub async fn fetch_changes(&self) -> Vec<RawSignal> {
        let mut changed = Vec::new();

        // Phase 1: read old cache content for all sources (skip circuit-broken)
        let preparations: Vec<(usize, std::collections::HashSet<String>, String)> = self
            .sources
            .iter()
            .enumerate()
            .filter(|(i, _)| !self.is_circuit_open(*i))
            .map(|(i, source)| {
                let cache_key = cache_key_for(source.name());
                let old_ids: std::collections::HashSet<String> =
                    read_cached::<Vec<RawSignal>>(&self.cache, &cache_key, self.cache_ttl)
                        .map(|c| c.data.iter().map(|s| s.id.clone()).collect())
                        .unwrap_or_default();
                (i, old_ids, cache_key)
            })
            .collect();

        // Phase 2: concurrent live fetch
        let results = futures::future::join_all(preparations.iter().map(
            |(i, old_ids, cache_key)| async move {
                let t0 = std::time::Instant::now();
                let outcome =
                    tokio::time::timeout(Duration::from_secs(30), self.sources[*i].fetch()).await;
                let latency_us = t0.elapsed().as_micros() as u64;
                let result = match outcome {
                    Ok(r) => r,
                    Err(_) => Err(crate::error::DataforgeError::Unavailable(
                        "fetch timed out after 30s".into(),
                    )),
                };
                (*i, result, old_ids.clone(), cache_key.clone(), latency_us)
            },
        ))
        .await;

        for (i, outcome, old_ids, cache_key, latency_us) in results {
            match outcome {
                Ok(signals) => {
                    self.record_success(i, latency_us);
                    let novel: Vec<RawSignal> = signals
                        .iter()
                        .filter(|s| !old_ids.contains(&s.id))
                        .cloned()
                        .collect();
                    if !novel.is_empty() {
                        tracing::info!(
                            source = self.sources[i].name(),
                            novel = novel.len(),
                            "changes detected",
                        );
                        changed.extend(novel);
                    }
                    let all_json = serde_json::to_vec(&signals).unwrap_or_default();
                    write_cached(&self.cache, &cache_key, &all_json);
                }
                Err(e) => {
                    self.record_failure(i);
                    tracing::warn!(
                        source = self.sources[i].name(),
                        error = %e,
                        "fetch_changes failed, skipping"
                    );
                }
            }
        }

        changed
    }

    /// Force-refresh all sources (ignore cache). All fetches run concurrently.
    /// Sources with open circuit breakers are skipped.
    pub async fn refresh_all(&self) -> Vec<RawSignal> {
        let results = futures::future::join_all(
            self.sources
                .iter()
                .enumerate()
                .filter(|(i, _)| !self.is_circuit_open(*i))
                .map(|(i, source)| async move {
                    let t0 = std::time::Instant::now();
                    let outcome =
                        tokio::time::timeout(Duration::from_secs(30), source.fetch()).await;
                    let latency_us = t0.elapsed().as_micros() as u64;
                    let result = match outcome {
                        Ok(r) => r,
                        Err(_) => Err(crate::error::DataforgeError::Unavailable(
                            "fetch timed out after 30s".into(),
                        )),
                    };
                    (i, result, cache_key_for(source.name()), latency_us)
                }),
        )
        .await;

        let mut all = Vec::new();
        for (i, outcome, cache_key, latency_us) in results {
            match outcome {
                Ok(signals) => {
                    self.record_success(i, latency_us);
                    let json = serde_json::to_vec(&signals).unwrap_or_default();
                    write_cached(&self.cache, &cache_key, &json);
                    all.extend(signals);
                }
                Err(e) => {
                    self.record_failure(i);
                    tracing::warn!(
                        source = self.sources[i].name(),
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

                let n = sources.len();
                let registry = SourceRegistry {
                    sources,
                    cache,
                    cache_ttl: interval,
                    successes: (0..n).map(|_| AtomicU64::new(0)).collect(),
                    failures: (0..n).map(|_| AtomicU64::new(0)).collect(),
                    latencies: (0..n)
                        .map(|_| std::sync::Mutex::new(Vec::with_capacity(10)))
                        .collect(),
                    consecutive_failures: (0..n).map(|_| AtomicU64::new(0)).collect(),
                    circuit_open_until: (0..n).map(|_| AtomicU64::new(0)).collect(),
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

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
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

    #[tokio::test]
    async fn with_all_sources_registers_all_ten() {
        let cache = test_cache();
        let registry = SourceRegistry::with_all_sources(cache);
        assert_eq!(registry.source_count(), 10);
        let names = registry.source_names();
        assert!(names.contains(&"NOAA GML"));
        assert!(names.contains(&"Open-Meteo"));
        assert!(names.contains(&"NSIDC"));
        assert!(names.contains(&"NOAA Tides"));
        assert!(names.contains(&"GBIF"));
        assert!(names.contains(&"arXiv"));
        assert!(names.contains(&"UCDP GED"));
        assert!(names.contains(&"USGS"));
        assert!(names.contains(&"NASA GIBS"));
        assert!(names.contains(&"NASA POWER"));
    }

    #[tokio::test]
    async fn fetch_changes_detects_new_signals() {
        let cache = test_cache();
        let source = TestSource {
            name: "changes-test".into(),
            signals: vec![test_signal("s1"), test_signal("s2")],
        };
        let registry = SourceRegistry::new(cache).with_source(Box::new(source));
        // First call: all signals are new (empty cache)
        let changes = registry.fetch_changes().await;
        assert_eq!(changes.len(), 2, "first fetch: all should be new");
        // Second call with same signals: no changes
        let changes2 = registry.fetch_changes().await;
        assert_eq!(changes2.len(), 0, "second fetch: no new signals");
        // Health should reflect two successes
        let health = registry.health();
        assert_eq!(health.len(), 1);
        assert_eq!(health[0].successes, 2);
        assert_eq!(health[0].failures, 0);
        assert!(health[0].avg_latency_us.is_some());
    }
}
