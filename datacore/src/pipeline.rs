//! Pipeline — end-to-end data flow from acquisition to anomaly detection.
//!
//! Connects dataforge's SourceRegistry to datacore's normalize→timeseries→anomaly
//! chain. One call runs the full acquisition-to-insight pipeline.
//!
//! ponytail: this is the single entry point for "give me everything." It fetches
//! all sources, normalizes, stores, detects anomalies, and returns structured
//! results ready for aurora or the Tauri frontend.

use serde::Serialize;

use dataforge::{SourceHealth, SourceRegistry};

use crate::anomaly::{
    AnomalyConfig, AnomalyDetector, AnomalyResult, ThresholdAlert, ThresholdDetector,
};
use crate::normalize::SignalNormalizer;
use crate::timeseries::TimeSeriesStore;

/// Result of a full pipeline run.
#[derive(Debug, Serialize)]
pub struct PipelineResult {
    /// All raw signals collected from sources.
    pub raw_count: usize,
    /// Successfully normalized signals.
    pub normalized_count: usize,
    /// Time-series data points stored.
    pub point_count: usize,
    /// Anomalies detected (z-score).
    pub anomaly_count: usize,
    /// Anomalous results (only those flagged as anomalous by z-score).
    pub anomalies: Vec<AnomalyResult>,
    /// Threshold violations detected.
    pub threshold_alerts: Vec<ThresholdAlert>,
    /// Per-source health metrics.
    pub health: Vec<SourceHealth>,
    /// Time-series data grouped by parameter (JSON-ready for charts).
    pub timeseries_json: serde_json::Value,
    /// Anomaly results as JSON string.
    pub anomalies_json: String,
}

impl PipelineResult {
    /// Generate a human-readable Markdown report from pipeline results.
    ///
    /// ponytail: pure String formatting, no template engine. Suitable for
    /// piping to a file, email, or dashboard widget.
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        // Header
        md.push_str("# Datacore Monitor Report\n\n");
        md.push_str(&format!(
            "**Generated**: {}\n\n",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ));

        // Pipeline summary
        md.push_str("## Pipeline Summary\n\n");
        md.push_str("| Metric | Value |\n");
        md.push_str("|--------|-------|\n");
        md.push_str(&format!("| Raw signals collected | {} |\n", self.raw_count));
        md.push_str(&format!("| Normalized | {} |\n", self.normalized_count));
        md.push_str(&format!("| Data points stored | {} |\n", self.point_count));
        md.push_str(&format!("| Z-score anomalies | {} |\n", self.anomaly_count));
        md.push_str(&format!(
            "| Threshold violations | {} |\n\n",
            self.threshold_alerts.len()
        ));

        // Source health
        md.push_str("## Source Health\n\n");
        md.push_str("| Source | Successes | Failures | Avg Latency (ms) |\n");
        md.push_str("|--------|-----------|----------|------------------|\n");
        for h in &self.health {
            let avg_ms = h
                .avg_latency_us
                .map(|us| format!("{:.1}", us as f64 / 1000.0))
                .unwrap_or_else(|| "N/A".into());
            md.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                h.name, h.successes, h.failures, avg_ms,
            ));
        }
        md.push('\n');

        // Threshold violations (most actionable first)
        if !self.threshold_alerts.is_empty() {
            md.push_str("## 🚨 Threshold Violations\n\n");
            md.push_str("| Parameter | Value | Reason | Source |\n");
            md.push_str("|-----------|-------|--------|--------|\n");
            for a in &self.threshold_alerts {
                md.push_str(&format!(
                    "| {} | {:.3} | {} | {} |\n",
                    a.point.parameter, a.point.value, a.reason, a.point.source,
                ));
            }
            md.push('\n');
        }

        // Z-score anomalies
        if !self.anomalies.is_empty() {
            md.push_str("## 🔍 Statistical Anomalies (z-score)\n\n");
            md.push_str("| Parameter | Value | Z-Score | Source |\n");
            md.push_str("|-----------|-------|---------|--------|\n");
            for a in &self.anomalies {
                let z = a
                    .z_score
                    .map(|z| format!("{z:.2}"))
                    .unwrap_or_else(|| "N/A".into());
                md.push_str(&format!(
                    "| {} | {:.3} | {} | {} |\n",
                    a.point.parameter, a.point.value, z, a.point.source,
                ));
            }
            md.push('\n');
        }

        // All-clear if nothing found
        if self.threshold_alerts.is_empty() && self.anomalies.is_empty() {
            md.push_str("## ✅ All Clear\n\n");
            md.push_str("No anomalies or threshold violations detected.\n\n");
        }

        md
    }
}

/// End-to-end pipeline wrapping acquisition, normalization, storage, and anomaly detection.
pub struct Pipeline {
    normalizer: SignalNormalizer,
    store: TimeSeriesStore,
    detector: AnomalyDetector,
    thresholds: ThresholdDetector,
}

impl Pipeline {
    /// Create a new pipeline with default anomaly detection config.
    pub fn new() -> Self {
        Self {
            normalizer: SignalNormalizer::new(),
            store: TimeSeriesStore::new(),
            detector: AnomalyDetector::default(),
            thresholds: ThresholdDetector::with_climate_defaults(),
        }
    }

    /// Create with a custom anomaly config.
    pub fn with_config(config: AnomalyConfig) -> Self {
        Self {
            normalizer: SignalNormalizer::new(),
            store: TimeSeriesStore::new(),
            detector: AnomalyDetector::new(config),
            thresholds: ThresholdDetector::with_climate_defaults(),
        }
    }

    /// Run the full pipeline: fetch → normalize → store → detect → export.
    ///
    /// This is the primary integration point. Pass a SourceRegistry (typically
    /// created with `SourceRegistry::with_all_sources()`), and receive a complete
    /// PipelineResult with everything downstream consumers need.
    pub async fn run(&mut self, registry: &SourceRegistry) -> PipelineResult {
        // 1. Acquire: fetch all signals from all sources
        let raw_signals = registry.fetch_all().await;
        let raw_count = raw_signals.len();

        // 2. Normalize: extract structured numeric values
        let normalized = self.normalizer.normalize_batch(&raw_signals);
        let normalized_count = normalized.len();

        // 3. Store: persist to time-series store
        let point_count = self.store.insert_batch(&normalized);

        // 4. Detect: z-score anomalies + threshold violations
        let all_anomalies = self.detector.score_all(&self.store);
        let anomalies: Vec<AnomalyResult> = all_anomalies
            .into_iter()
            .filter(|a| a.is_anomalous)
            .collect();
        let anomaly_count = anomalies.len();
        let threshold_alerts = self.thresholds.check_all(&self.store);

        // 5. Export: JSON for frontend consumption
        let timeseries_json = self.store.to_json_grouped();
        let anomalies_json =
            AnomalyDetector::results_to_json(&anomalies).unwrap_or_else(|_| "[]".into());

        // 6. Health: source metrics
        let health = registry.health();

        PipelineResult {
            raw_count,
            normalized_count,
            point_count,
            anomaly_count,
            anomalies,
            threshold_alerts,
            health,
            timeseries_json,
            anomalies_json,
        }
    }

    /// Run the pipeline with change detection (only new/changed signals).
    ///
    /// Uses `fetch_changes()` instead of `fetch_all()`, comparing against
    /// previously cached data. Faster for polling loops.
    pub async fn run_changes(&mut self, registry: &SourceRegistry) -> PipelineResult {
        let raw_signals = registry.fetch_changes().await;
        let raw_count = raw_signals.len();

        if raw_signals.is_empty() {
            return PipelineResult {
                raw_count: 0,
                normalized_count: 0,
                point_count: 0,
                anomaly_count: 0,
                anomalies: vec![],
                threshold_alerts: vec![],
                health: registry.health(),
                timeseries_json: self.store.to_json_grouped(),
                anomalies_json: "[]".into(),
            };
        }

        let normalized = self.normalizer.normalize_batch(&raw_signals);
        let normalized_count = normalized.len();
        let point_count = self.store.insert_batch(&normalized);

        let all_anomalies = self.detector.score_all(&self.store);
        let anomalies: Vec<AnomalyResult> = all_anomalies
            .into_iter()
            .filter(|a| a.is_anomalous)
            .collect();
        let anomaly_count = anomalies.len();
        let threshold_alerts = self.thresholds.check_all(&self.store);

        let timeseries_json = self.store.to_json_grouped();
        let anomalies_json =
            AnomalyDetector::results_to_json(&anomalies).unwrap_or_else(|_| "[]".into());
        let health = registry.health();

        PipelineResult {
            raw_count,
            normalized_count,
            point_count,
            anomaly_count,
            anomalies,
            threshold_alerts,
            health,
            timeseries_json,
            anomalies_json,
        }
    }

    /// Mutable reference to the time-series store for external queries.
    pub fn store(&self) -> &TimeSeriesStore {
        &self.store
    }

    /// Clear accumulated time-series data (start a fresh observation window).
    pub fn clear_store(&mut self) {
        self.store = TimeSeriesStore::new();
    }

    /// Save current time-series data to a JSON file.
    ///
    /// ponytail: writes the full store as JSON. Overwrites existing file.
    /// Returns the number of points saved.
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<usize> {
        let json = self.store.to_json().map_err(std::io::Error::other)?;
        std::fs::write(path, &json)?;
        Ok(self.store.len())
    }

    /// Restore time-series data from a previously saved JSON file.
    ///
    /// Returns the number of points loaded. Existing store data is NOT cleared —
    /// restored points are appended. Call `clear_store()` first if you want a
    /// fresh start.
    pub fn restore(&mut self, path: &std::path::Path) -> std::io::Result<usize> {
        let json = std::fs::read_to_string(path)?;
        Ok(self.store.from_json(&json))
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dataforge::L2Cache;
    use std::sync::Arc;

    /// Build a test registry with synthetic sources (no HTTP).
    fn test_registry() -> SourceRegistry {
        let cache_dir = std::env::temp_dir().join(format!(
            "datacore_pipeline_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos()
        ));
        let cache = Arc::new(L2Cache::new(cache_dir, 1024 * 1024));
        SourceRegistry::new(cache)
    }

    #[tokio::test]
    async fn pipeline_run_with_empty_registry() {
        let registry = test_registry();
        let mut pipeline = Pipeline::new();
        let result = pipeline.run(&registry).await;
        assert_eq!(result.raw_count, 0);
        assert_eq!(result.anomaly_count, 0);
        assert!(result.timeseries_json.as_object().unwrap().is_empty());
    }

    #[tokio::test]
    async fn pipeline_run_changes_with_empty_signals() {
        let registry = test_registry();
        let mut pipeline = Pipeline::new();
        let result = pipeline.run_changes(&registry).await;
        assert_eq!(result.raw_count, 0);
        assert!(result.anomalies_json == "[]");
    }

    #[test]
    fn markdown_report_contains_sections() {
        let result = PipelineResult {
            raw_count: 10,
            normalized_count: 8,
            point_count: 24,
            anomaly_count: 1,
            anomalies: vec![],
            threshold_alerts: vec![],
            health: vec![],
            timeseries_json: serde_json::Value::Object(serde_json::Map::new()),
            anomalies_json: "[]".into(),
        };
        let md = result.to_markdown();
        assert!(md.contains("# Datacore Monitor Report"));
        assert!(md.contains("## Pipeline Summary"));
        assert!(md.contains("## Source Health"));
        assert!(md.contains("All Clear"));
        assert!(md.contains("Raw signals collected | 10"));
    }

    #[test]
    fn markdown_report_lists_threshold_violations() {
        use crate::anomaly::ThresholdAlert;
        use crate::timeseries::TimeSeriesPoint;

        let result = PipelineResult {
            raw_count: 5,
            normalized_count: 5,
            point_count: 5,
            anomaly_count: 0,
            anomalies: vec![],
            threshold_alerts: vec![ThresholdAlert {
                point: TimeSeriesPoint {
                    parameter: "co2_ppm".into(),
                    value: 435.0,
                    timestamp: chrono::Utc::now(),
                    source: "NOAA GML".into(),
                    location: None,
                },
                is_anomalous: true,
                reason: "value 435 > max 430".into(),
            }],
            health: vec![],
            timeseries_json: serde_json::Value::Object(serde_json::Map::new()),
            anomalies_json: "[]".into(),
        };
        let md = result.to_markdown();
        assert!(md.contains("## 🚨 Threshold Violations"));
        assert!(md.contains("co2_ppm"));
        assert!(md.contains("435"));
        assert!(md.contains("value 435 > max 430"));
    }

    #[test]
    fn save_and_restore_roundtrip() {
        let tmp =
            std::env::temp_dir().join(format!("datacore_pipeline_save_{}", std::process::id()));
        let mut pipeline = Pipeline::new();
        // Insert some data
        let t = chrono::Utc::now();
        use crate::normalize::{NormalizedSignal, SignalValue};
        let sig = NormalizedSignal {
            signal_id: "s1".into(),
            source_name: "test".into(),
            category: dataforge::DataCategory::Climate,
            captured_at: t,
            location: None,
            values: vec![SignalValue {
                name: "co2_ppm".into(),
                value: 432.0,
                unit: "ppm".into(),
            }],
        };
        pipeline.store.insert_signal(&sig);

        // Save
        let saved = pipeline.save(&tmp).unwrap();
        assert_eq!(saved, 1);

        // Restore into a fresh pipeline
        let mut pipeline2 = Pipeline::new();
        let loaded = pipeline2.restore(&tmp).unwrap();
        assert_eq!(loaded, 1);
        assert_eq!(pipeline2.store.len(), 1);

        // Cleanup
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn restore_empty_file_returns_zero() {
        let tmp =
            std::env::temp_dir().join(format!("datacore_pipeline_empty_{}", std::process::id()));
        std::fs::write(&tmp, "[]").unwrap();
        let mut pipeline = Pipeline::new();
        let loaded = pipeline.restore(&tmp).unwrap();
        assert_eq!(loaded, 0);
        let _ = std::fs::remove_file(&tmp);
    }
}
