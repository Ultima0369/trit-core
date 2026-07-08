//! Anomaly detection for time-series data.
//!
//! Two complementary detectors:
//! - **AnomalyDetector**: z-score sliding window (catches unknown patterns)
//! - **ThresholdDetector**: fixed safety bounds (catches known dangerous zones)
//!
//! ponytail: pure statistics, no ML. Stateless, call on demand.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::timeseries::{TimeSeriesPoint, TimeSeriesStore};

/// An anomaly score attached to a time-series point.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnomalyResult {
    /// The original point.
    pub point: TimeSeriesPoint,
    /// Z-score of this point relative to the reference window.
    /// None if the window had fewer than 2 points (insufficient for stddev).
    pub z_score: Option<f64>,
    /// Whether this point is considered anomalous (|z_score| > threshold).
    pub is_anomalous: bool,
}

/// Configuration for z-score anomaly detection.
#[derive(Debug, Clone)]
pub struct AnomalyConfig {
    /// Number of recent points to use as the reference window.
    pub window_size: usize,
    /// |z-score| above this threshold flags an anomaly.
    pub threshold: f64,
}

impl Default for AnomalyConfig {
    fn default() -> Self {
        Self {
            window_size: 30,
            threshold: 3.0,
        }
    }
}

/// Stateless z-score anomaly detector.
///
/// ponytail: uses the TimeSeriesStore's query methods to fetch the window,
/// then computes mean/stddev inline. No internal state.
pub struct AnomalyDetector {
    config: AnomalyConfig,
}

impl AnomalyDetector {
    pub fn new(config: AnomalyConfig) -> Self {
        Self { config }
    }

    /// Score a single point against a reference window from the store.
    ///
    /// The reference window is the `config.window_size` most recent points
    /// for the same parameter, excluding the point itself.
    pub fn score(&self, point: &TimeSeriesPoint, store: &TimeSeriesStore) -> AnomalyResult {
        let window: Vec<f64> = store
            .query_parameter(&point.parameter)
            .iter()
            .filter(|p| p.timestamp < point.timestamp)
            .map(|p| p.value)
            .rev()
            .take(self.config.window_size)
            .collect();

        if window.len() < 2 {
            return AnomalyResult {
                point: point.clone(),
                z_score: None,
                is_anomalous: false,
            };
        }

        let n = window.len() as f64;
        let mean = window.iter().sum::<f64>() / n;
        let variance = window.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0);
        let stddev = variance.sqrt();

        if stddev < 1e-12 {
            // Near-constant series — any deviation is anomalous
            let is_anomalous = (point.value - mean).abs() > 1e-9;
            return AnomalyResult {
                point: point.clone(),
                z_score: if is_anomalous {
                    Some(f64::INFINITY)
                } else {
                    Some(0.0)
                },
                is_anomalous,
            };
        }

        let z = (point.value - mean) / stddev;
        AnomalyResult {
            point: point.clone(),
            z_score: Some(z),
            is_anomalous: z.abs() > self.config.threshold,
        }
    }

    /// Detect anomalies in the rate of change (first derivative).
    ///
    /// Computes the sequence of deltas between consecutive points for a parameter,
    /// builds a z-score from the reference window of deltas, and flags points
    /// where the change rate is anomalous. Useful for detecting sudden accelerations
    /// (e.g. CO₂ rising faster than normal).
    ///
    /// ponytail: O(n) scan over sorted points, then z-score on deltas.
    /// Assumes roughly uniform sampling — raw delta (not /dt) is used to avoid
    /// floating-point noise on small numbers.
    pub fn score_rate_of_change(
        &self,
        parameter: &str,
        store: &TimeSeriesStore,
    ) -> Vec<AnomalyResult> {
        let mut points: Vec<&TimeSeriesPoint> =
            store.query_parameter(parameter).into_iter().collect();
        points.sort_by_key(|p| p.timestamp);

        let mut results = Vec::new();
        // First point has no delta
        if !points.is_empty() {
            results.push(AnomalyResult {
                point: points[0].clone(),
                z_score: None,
                is_anomalous: false,
            });
        }

        let mut deltas: Vec<f64> = Vec::new();
        for i in 1..points.len() {
            let delta = points[i].value - points[i - 1].value;
            deltas.push(delta);

            // Reference window: previous deltas (exclude current)
            let window: Vec<f64> = deltas
                .iter()
                .rev()
                .skip(1)
                .take(self.config.window_size)
                .copied()
                .collect();

            if window.len() < 2 {
                results.push(AnomalyResult {
                    point: points[i].clone(),
                    z_score: None,
                    is_anomalous: false,
                });
                continue;
            }

            let n = window.len() as f64;
            let mean = window.iter().sum::<f64>() / n;
            let variance = window.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0);
            let stddev = variance.sqrt();

            if stddev < 1e-12 {
                results.push(AnomalyResult {
                    point: points[i].clone(),
                    z_score: if delta.abs() > 1e-9 {
                        Some(f64::INFINITY)
                    } else {
                        Some(0.0)
                    },
                    is_anomalous: delta.abs() > 1e-9,
                });
                continue;
            }

            let z = (delta - mean) / stddev;
            results.push(AnomalyResult {
                point: points[i].clone(),
                z_score: Some(z),
                is_anomalous: z.abs() > self.config.threshold,
            });
        }

        results
    }

    /// Score all points in a store for a given parameter.
    pub fn score_parameter(&self, parameter: &str, store: &TimeSeriesStore) -> Vec<AnomalyResult> {
        let mut points: Vec<&TimeSeriesPoint> =
            store.query_parameter(parameter).into_iter().collect();
        // Sort by timestamp for sequential scoring
        points.sort_by_key(|p| p.timestamp);

        // Build a temporary store that grows as we iterate
        let mut running = TimeSeriesStore::new();
        let mut results = Vec::new();

        for p in points {
            let result = self.score(p, &running);
            results.push(result);
            // Add this point to the running store for subsequent scoring
            // ponytail: clone the point into the running store via a synthetic NormalizedSignal
            // Since insert_signal needs a NormalizedSignal, we use the simpler approach:
            // manually add to running store's internal state
            running.points.push(p.clone());
        }

        results
    }

    /// Score all parameters in a store, returning results grouped by parameter.
    pub fn score_all(&self, store: &TimeSeriesStore) -> Vec<AnomalyResult> {
        let mut all_results = Vec::new();
        for param in store.parameters() {
            all_results.extend(self.score_parameter(param, store));
        }
        all_results
    }

    /// Export anomaly results as JSON string.
    ///
    /// ponytail: AnomalyResult derives Serialize, so this is trivial.
    pub fn results_to_json(results: &[AnomalyResult]) -> Result<String, serde_json::Error> {
        serde_json::to_string(results)
    }
}

/// Fixed-threshold anomaly detection for known dangerous zones.
///
/// Each parameter has a `[min, max]` safety bound. Any value outside these
/// bounds is flagged as anomalous. This complements AnomalyDetector:
/// thresholds catch known hazards (CO₂ > 430 ppm), z-score catches
/// unexpected deviations (sudden spike in a normally stable series).
///
/// ponytail: a HashMap of parameter→[min, max]. Add rules as domain knowledge
/// accumulates. No calibration phase needed — bounds are explicit.
pub struct ThresholdDetector {
    /// Per-parameter safety bounds: `parameter → (min, max)`.
    /// None values mean "no upper/lower bound".
    bounds: HashMap<String, (Option<f64>, Option<f64>)>,
}

impl ThresholdDetector {
    /// Create a new detector with no rules.
    pub fn new() -> Self {
        Self {
            bounds: HashMap::new(),
        }
    }

    /// Create with a set of commonly-used threshold rules for climate monitoring.
    ///
    /// ponytail: these are conservative based on IPCC AR6 and NOAA reference values.
    /// Review periodically against current scientific consensus.
    pub fn with_climate_defaults() -> Self {
        let mut d = Self::new();
        // CO₂: pre-industrial ~280, current ~425, danger > 430 ppm
        d.insert("co2_ppm", None, Some(430.0));
        // Temperature anomaly: Paris Agreement 1.5°C threshold
        d.insert("anomaly_c", Some(-2.0), Some(2.0));
        // T2M (surface temp): >40°C extreme heat, < -50°C extreme cold
        d.insert("t2m_c", Some(-50.0), Some(40.0));
        // Precipitation: >300mm/day is extreme flooding
        d.insert("precip_mm", None, Some(300.0));
        // Sea ice extent: Arctic minimum < 3 million km² is extreme
        d.insert("sea_ice_extent_mkm2", Some(3.0), None);
        // Earthquake magnitude: ≥ 6.0 is significant, ≥ 7.0 major
        d.insert("earthquake_mag", None, Some(6.0));
        // Earthquake depth: < 10km shallow = more destructive
        d.insert("depth_km", Some(10.0), None);
        // Deaths in conflict: any > 0 is concerning
        d.insert("deaths", None, Some(0.0));
        // Water level: ±3m from MSL is anomalous at tide gauges
        d.insert("water_level_m", Some(-3.0), Some(3.0));
        // Wind speed: > 50 m/s (180 km/h) is Category 3+ hurricane
        d.insert("wind_m_s", None, Some(50.0));
        // Solar radiation: > 1361 W/m² solar constant is implausible
        d.insert("solar_w_m2", None, Some(1361.0));
        d
    }

    /// Add a threshold rule for a parameter.
    ///
    /// `min` = lower bound (inclusive). `None` = no lower bound.
    /// `max` = upper bound (inclusive). `None` = no upper bound.
    pub fn insert(&mut self, parameter: &str, min: Option<f64>, max: Option<f64>) {
        self.bounds.insert(parameter.to_string(), (min, max));
    }

    /// Check a single point against its parameter's threshold.
    pub fn check(&self, point: &TimeSeriesPoint) -> Option<ThresholdAlert> {
        let (min, max) = self.bounds.get(&point.parameter)?;
        let mut alert = ThresholdAlert {
            point: point.clone(),
            is_anomalous: false,
            reason: String::new(),
        };

        if let Some(min_val) = min {
            if point.value < *min_val {
                alert.is_anomalous = true;
                alert.reason = format!("value {:.3} < min {min_val}", point.value);
            }
        }
        if let Some(max_val) = max {
            if point.value > *max_val {
                alert.is_anomalous = true;
                if alert.reason.is_empty() {
                    alert.reason = format!("value {:.3} > max {max_val}", point.value);
                } else {
                    alert.reason.push_str(&format!("; also > max {max_val}"));
                }
            }
        }

        if alert.is_anomalous {
            Some(alert)
        } else {
            None
        }
    }

    /// Check all points in a store against their threshold rules.
    ///
    /// Only returns points that violate their thresholds.
    pub fn check_all(&self, store: &TimeSeriesStore) -> Vec<ThresholdAlert> {
        store
            .query_all()
            .iter()
            .filter_map(|p| self.check(p))
            .collect()
    }

    /// Number of threshold rules configured.
    pub fn rule_count(&self) -> usize {
        self.bounds.len()
    }

    /// List all parameters with threshold rules.
    pub fn parameters(&self) -> Vec<&str> {
        self.bounds.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for ThresholdDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// A threshold violation alert.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThresholdAlert {
    /// The point that violated the threshold.
    pub point: TimeSeriesPoint,
    /// Whether the value is outside the safe bounds.
    pub is_anomalous: bool,
    /// Human-readable explanation (e.g. "value 435.0 > max 430").
    pub reason: String,
}

impl Default for AnomalyDetector {
    fn default() -> Self {
        Self::new(AnomalyConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalize::{NormalizedSignal, SignalValue};
    use chrono::Utc;
    use dataforge::DataCategory;

    fn make_signal(
        parameter: &str,
        value: f64,
        timestamp: chrono::DateTime<Utc>,
    ) -> NormalizedSignal {
        NormalizedSignal {
            signal_id: format!("id_{}", timestamp.timestamp_nanos_opt().unwrap_or(0)),
            source_name: "test".into(),
            category: DataCategory::Climate,
            captured_at: timestamp,
            location: None,
            values: vec![SignalValue {
                name: parameter.into(),
                value,
                unit: "test".into(),
            }],
        }
    }

    #[test]
    fn no_anomaly_in_steady_series() {
        let mut store = TimeSeriesStore::new();
        let t0 = Utc::now();
        for i in 0..35 {
            let t = t0 + chrono::Duration::hours(i);
            store.insert_signal(&make_signal("v", 100.0 + (i as f64 * 0.1), t));
        }

        let detector = AnomalyDetector::default();
        let results = detector.score_parameter("v", &store);
        // None should be anomalous — steady drift of 0.1/hour
        assert!(
            results.iter().all(|r| !r.is_anomalous),
            "steady series should have no anomalies"
        );
    }

    #[test]
    fn spike_is_anomalous() {
        let mut store = TimeSeriesStore::new();
        let t0 = Utc::now();
        // 30 steady points
        for i in 0..30 {
            let t = t0 + chrono::Duration::hours(i);
            store.insert_signal(&make_signal("v", 100.0, t));
        }
        // Then a spike
        let spike_t = t0 + chrono::Duration::hours(31);
        store.insert_signal(&make_signal("v", 150.0, spike_t));

        let detector = AnomalyDetector::default();
        let results = detector.score_parameter("v", &store);
        let spike_result = results.last().unwrap();
        assert!(spike_result.is_anomalous);
        assert!(spike_result.z_score.unwrap() > 5.0);
    }

    #[test]
    fn too_few_points_returns_none_zscore() {
        let mut store = TimeSeriesStore::new();
        let t = Utc::now();
        store.insert_signal(&make_signal("v", 100.0, t));

        let detector = AnomalyDetector::default();
        let results = detector.score_parameter("v", &store);
        assert_eq!(results.len(), 1);
        assert!(results[0].z_score.is_none());
        assert!(!results[0].is_anomalous);
    }

    #[test]
    fn near_constant_series_flags_any_deviation() {
        let mut store = TimeSeriesStore::new();
        let t0 = Utc::now();
        for i in 0..30 {
            let t = t0 + chrono::Duration::hours(i);
            store.insert_signal(&make_signal("v", 42.0, t));
        }
        // Slight deviation
        let dev_t = t0 + chrono::Duration::hours(31);
        store.insert_signal(&make_signal("v", 42.001, dev_t));

        let detector = AnomalyDetector::default();
        let results = detector.score_parameter("v", &store);
        assert!(results.last().unwrap().is_anomalous);
    }

    #[test]
    fn rate_of_change_detects_sudden_acceleration() {
        let mut store = TimeSeriesStore::new();
        let t0 = Utc::now();
        // 30 hours of steady 0.1/hour increase
        for i in 0..30 {
            let t = t0 + chrono::Duration::hours(i);
            store.insert_signal(&make_signal("v", 100.0 + i as f64 * 0.1, t));
        }
        // Then a sudden 10.0 jump in one hour
        let jump_t = t0 + chrono::Duration::hours(31);
        store.insert_signal(&make_signal("v", 103.0 + 10.0, jump_t));

        let detector = AnomalyDetector::new(AnomalyConfig {
            window_size: 30,
            threshold: 3.0,
        });
        let results = detector.score_rate_of_change("v", &store);
        let last = results.last().unwrap();
        assert!(last.is_anomalous, "sudden jump should be anomalous rate");
        assert!(last.z_score.unwrap() > 3.0);
    }

    #[test]
    fn rate_of_change_steady_series_not_anomalous() {
        let mut store = TimeSeriesStore::new();
        let t0 = Utc::now();
        // Add tiny noise so stddev is non-zero (real data always has noise)
        let noise = [
            0.01, -0.01, 0.02, -0.02, 0.005, -0.005, 0.0, 0.01, -0.01, 0.0,
        ];
        for i in 0..65 {
            let t = t0 + chrono::Duration::hours(i);
            let jitter = noise[i as usize % noise.len()];
            store.insert_signal(&make_signal("v", 100.0 + i as f64 * 0.1 + jitter, t));
        }
        let detector = AnomalyDetector::default();
        let results = detector.score_rate_of_change("v", &store);
        let anomalies_after_warmup: Vec<_> =
            results.iter().skip(30).filter(|r| r.is_anomalous).collect();
        assert!(
            anomalies_after_warmup.is_empty(),
            "steady rate with noise should produce no anomalies after window warmup, got {}",
            anomalies_after_warmup.len()
        );
    }

    #[test]
    fn results_to_json_is_valid() {
        let mut store = TimeSeriesStore::new();
        let t = chrono::Utc::now();
        store.insert_signal(&make_signal("v", 100.0, t));
        store.insert_signal(&make_signal("v", 150.0, t + chrono::Duration::hours(1)));

        let detector = AnomalyDetector::default();
        let results = detector.score_parameter("v", &store);
        let json = AnomalyDetector::results_to_json(&results).unwrap();
        assert!(json.contains("z_score"));
        assert!(json.contains("is_anomalous"));
        let _parsed: serde_json::Value = serde_json::from_str(&json).expect("should be valid JSON");
    }

    #[test]
    fn threshold_detects_co2_above_430() {
        let detector = ThresholdDetector::with_climate_defaults();
        let point = TimeSeriesPoint {
            parameter: "co2_ppm".into(),
            value: 435.0,
            timestamp: chrono::Utc::now(),
            source: "test".into(),
            location: None,
        };
        let alert = detector.check(&point).unwrap();
        assert!(alert.is_anomalous);
        assert!(alert.reason.contains("max"));
    }

    #[test]
    fn threshold_passes_co2_below_430() {
        let detector = ThresholdDetector::with_climate_defaults();
        let point = TimeSeriesPoint {
            parameter: "co2_ppm".into(),
            value: 415.0,
            timestamp: chrono::Utc::now(),
            source: "test".into(),
            location: None,
        };
        assert!(detector.check(&point).is_none());
    }

    #[test]
    fn threshold_detects_sea_ice_below_3mkm2() {
        let detector = ThresholdDetector::with_climate_defaults();
        let point = TimeSeriesPoint {
            parameter: "sea_ice_extent_mkm2".into(),
            value: 2.5,
            timestamp: chrono::Utc::now(),
            source: "test".into(),
            location: None,
        };
        let alert = detector.check(&point).unwrap();
        assert!(alert.is_anomalous);
        assert!(alert.reason.contains("min"));
    }

    #[test]
    fn threshold_unknown_parameter_returns_none() {
        let detector = ThresholdDetector::with_climate_defaults();
        let point = TimeSeriesPoint {
            parameter: "unknown_param".into(),
            value: 999.0,
            timestamp: chrono::Utc::now(),
            source: "test".into(),
            location: None,
        };
        assert!(detector.check(&point).is_none());
    }

    #[test]
    fn threshold_with_climate_defaults_has_rules() {
        let detector = ThresholdDetector::with_climate_defaults();
        assert!(detector.rule_count() >= 11);
        assert!(detector.parameters().contains(&"co2_ppm"));
        assert!(detector.parameters().contains(&"sea_ice_extent_mkm2"));
        assert!(detector.parameters().contains(&"wind_m_s"));
    }
}
