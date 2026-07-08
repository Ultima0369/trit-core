//! Prism engine — signal decomposition for raw observations.
//!
//! The prism takes [`RawSignal`]s from dataforge and decomposes each one
//! through an LLM into a [`PerceptBatch`] of [`TritWord`] signals.
//!
//! ## Core concepts
//!
//! - **SourceWeights**: credibility profiles for each data source, used to
//!   inform (not determine) the LLM's signal extraction.
//! - **PrismEngine**: orchestrates RawSignal → prompt → LLM → PerceptBatch.
//! - **Multi-source collision**: when the same event appears in multiple
//!   sources, each source is perceived independently, and the resulting
//!   TritWord[] batches are merged for cross-frame conflict detection
//!   downstream in trit-core.
//!
//! ## 流沙 Philosophy
//!
//! The prism does NOT interpret. It decomposes. It splits. It passes through.
//! The LLM is asked to extract signals, not opinions. The SourceWeights are
//! metadata, not judgment. trit-core makes the ternary decision.
//!
//! ## Design
//!
//! - prism depends on trit-core types (TritWord, Frame, Phase) — shared via
//!   the aurora crate's trit_core dependency.
//! - prism depends on dataforge types (RawSignal, DataCategory) — shared via
//!   the aurora crate's dataforge dependency.
//! - prism depends on datacore for normalization and anomaly detection.
//! - prism uses the existing PerceptChain for LLM degradation.

use std::collections::HashMap;
use std::time::Duration;

use dataforge::{DataCategory, RawSignal};
use trit_core::{Frame, Phase, TritValue, TritWord};

use crate::percept::{PerceptBatch, PerceptChain, PerceptError};

// ── Source credibility profiles ──────────────────────────────────────

/// Credibility profile for a data source.
///
/// These are prior weights, not truth claims. They inform the LLM prompt
/// but do NOT override the LLM's signal extraction. A high-credibility
/// source can still produce conflicting signals; the ternary engine
/// resolves that.
#[derive(Debug, Clone)]
pub struct SourceProfile {
    /// Prior credibility [0.0, 1.0] based on:
    /// - Method transparency (is the raw data public?)
    /// - Peer review (is this an institutional source?)
    /// - Historical consistency (has this source been reliable?)
    pub credibility: f64,
    /// Data category.
    pub category: DataCategory,
    /// Expected update frequency.
    pub update_frequency: Duration,
}

impl SourceProfile {
    pub fn new(credibility: f64, category: DataCategory) -> Self {
        Self {
            credibility: credibility.clamp(0.0, 1.0),
            category,
            update_frequency: Duration::from_secs(3600),
        }
    }

    pub fn with_frequency(mut self, freq: Duration) -> Self {
        self.update_frequency = freq;
        self
    }
}

/// Registry of source credibility weights.
#[derive(Debug, Clone, Default)]
pub struct SourceWeights {
    weights: HashMap<String, SourceProfile>,
}

impl SourceWeights {
    pub fn new() -> Self {
        Self {
            weights: HashMap::new(),
        }
    }

    /// Pre-populate with known data sources.
    ///
    /// Credibility estimates are conservative priors based on institutional
    /// transparency and peer-review status. These should be periodically
    /// reviewed and recalibrated against actual signal quality.
    pub fn with_defaults() -> Self {
        let mut w = Self::new();
        w.insert("NOAA GML", SourceProfile::new(0.95, DataCategory::Climate));
        w.insert(
            "Open-Meteo",
            SourceProfile::new(0.90, DataCategory::Climate),
        );
        w.insert("GBIF", SourceProfile::new(0.85, DataCategory::Ecology));
        w.insert(
            "arXiv",
            SourceProfile::new(0.70, DataCategory::ScientificResearch)
                .with_frequency(Duration::from_secs(3600)),
        );
        w.insert(
            "UCDP GED",
            SourceProfile::new(0.80, DataCategory::Geopolitical),
        );
        w.insert("USGS", SourceProfile::new(0.95, DataCategory::Other));
        w.insert("NSIDC", SourceProfile::new(0.90, DataCategory::Climate));
        w.insert(
            "NOAA Tides",
            SourceProfile::new(0.85, DataCategory::Ecology),
        );
        w.insert(
            "NASA GIBS",
            SourceProfile::new(0.75, DataCategory::Satellite),
        );
        w.insert(
            "NASA POWER",
            SourceProfile::new(0.90, DataCategory::Climate),
        );
        w
    }

    pub fn insert(&mut self, name: &str, profile: SourceProfile) {
        self.weights.insert(name.to_string(), profile);
    }

    pub fn get(&self, name: &str) -> Option<&SourceProfile> {
        self.weights.get(name)
    }

    /// How many sources are registered.
    pub fn len(&self) -> usize {
        self.weights.len()
    }

    pub fn is_empty(&self) -> bool {
        self.weights.is_empty()
    }
}

// ── Prism engine ─────────────────────────────────────────────────────

/// The prism engine: RawSignal → LLM → PerceptBatch.
///
/// Wraps a PerceptChain (cloud → local → FFT degradation) and adds
/// source credibility metadata to each prompt.
pub struct PrismEngine {
    /// Priority-ordered percept chain for LLM degradation.
    chain: PerceptChain,
    /// Source credibility registry.
    source_weights: SourceWeights,
}

impl PrismEngine {
    /// Create a prism engine with the given percept chain and source weights.
    pub fn new(chain: PerceptChain, source_weights: SourceWeights) -> Self {
        Self {
            chain,
            source_weights,
        }
    }

    /// Perceive a single RawSignal through the LLM chain.
    ///
    /// Builds a prompt with source credibility metadata prepended,
    /// then delegates to the PerceptChain for LLM signal extraction.
    ///
    /// When the chain is empty or all providers are unavailable, falls back
    /// to structured decomposition: the raw signal's numeric data is mapped
    /// directly to Instrumental-frame TritWords without LLM interpretation.
    pub fn perceive_one(&self, signal: &RawSignal) -> Result<PerceptBatch, PerceptError> {
        let prompt = self.build_prompt(signal);
        match self.chain.perceive_or_degrade(&prompt) {
            Ok(batch) => Ok(batch),
            Err(PerceptError::AllUnavailable) => {
                tracing::debug!(
                    source = %signal.source_name,
                    "percept chain unavailable, degrading to structured decomposition"
                );
                Ok(self.degrade_to_structured(signal))
            }
            Err(e) => Err(e),
        }
    }

    /// Degrade path: when no LLM is available, extract TritWords directly
    /// from the RawSignal's structured fields.
    ///
    /// Each RawSignal becomes one Instrumental-frame TritWord. The phase is
    /// derived from the source credibility weight — higher credibility →
    /// higher phase (more confident True/False rather than Hold).
    fn degrade_to_structured(&self, signal: &RawSignal) -> PerceptBatch {
        let profile = self.source_weights.get(&signal.source_name);
        let credibility = profile.map(|p| p.credibility).unwrap_or(0.5);

        // Map DataCategory → Frame for structured decomposition.
        // ponytail: direct mapping, no interpretation. Instrumental for
        // sensor data (Climate/Ecology), Science for preprints, Consensus
        // for geopolitical reports.
        let frame = match signal.category {
            DataCategory::Climate | DataCategory::Ecology => Frame::Instrumental,
            DataCategory::ScientificResearch => Frame::Science,
            DataCategory::Geopolitical => Frame::Consensus,
            DataCategory::Satellite => Frame::Instrumental,
            DataCategory::Other => Frame::Individual,
        };

        // Parse a numeric value from the raw_content if possible.
        // ponytail: simple keyword scan — the structured fields in raw_content
        // (e.g. "co2_ppm:432.34", "anomaly_c:+1.23") are parseable without an LLM.
        let (value, phase_val) = Self::extract_signal_value(&signal.raw_content, credibility);

        let phase = Phase::new_clamped(phase_val).quantize(1e-6);
        let word = TritWord::new(value, phase, frame);

        PerceptBatch {
            signals: vec![word],
            source: format!("{}/structured", signal.source_name),
            timestamp: chrono::Utc::now(),
            confidence: credibility,
            raw_data_layer: Some(signal.raw_content.clone()),
        }
    }

    /// Extract a TritValue and phase from raw_content text.
    ///
    /// Looks for `key:number` patterns and compares against known baselines.
    /// Returns (True/False, phase) — True means "value is in a healthy range",
    /// False means "value indicates concern". The phase encodes confidence.
    fn extract_signal_value(raw: &str, credibility: f64) -> (TritValue, f64) {
        let phase = credibility.clamp(0.0, 1.0);

        // CO2 ppm: >420 is concerning (Mauna Loa baseline ~415)
        if let Some(ppm) = extract_number_after(raw, "co2_ppm") {
            if ppm > 420.0 {
                return (TritValue::False, phase);
            }
            return (TritValue::True, phase);
        }
        // Temperature anomaly: >1.5°C is concerning (Paris threshold)
        if let Some(anomaly) = extract_number_after(raw, "anomaly_c") {
            if anomaly > 1.5 {
                return (TritValue::False, phase);
            }
            if anomaly < -1.5 {
                return (TritValue::False, phase);
            }
            return (TritValue::True, phase);
        }
        // Generic anomaly
        if let Some(anomaly) = extract_number_after(raw, "anomaly") {
            if anomaly.abs() > 2.0 {
                return (TritValue::False, phase);
            }
            return (TritValue::True, phase);
        }
        // Deaths / conflict
        if let Some(deaths) = extract_number_after(raw, "deaths") {
            if deaths > 0.0 {
                return (TritValue::False, phase);
            }
            return (TritValue::True, phase);
        }
        // Generic value extraction
        if let Some(val) = extract_number_after(raw, "value") {
            if val > 0.0 {
                return (TritValue::True, phase);
            }
            return (TritValue::False, phase);
        }

        // No parseable number → Hold (can't determine from this data)
        (TritValue::Hold, 0.5)
    }

    /// Perceive a batch of RawSignals. Each is independently decomposed.
    ///
    /// Failures on individual signals are logged and skipped — the prism
    /// is fail-safe: a bad source doesn't block good ones.
    pub fn perceive_batch(&self, signals: &[RawSignal]) -> Vec<(RawSignal, PerceptBatch)> {
        signals
            .iter()
            .filter_map(|sig| match self.perceive_one(sig) {
                Ok(batch) => Some((sig.clone(), batch)),
                Err(e) => {
                    tracing::warn!(
                        source = %sig.source_name,
                        error = %e,
                        "prism perception failed for signal, skipping"
                    );
                    None
                }
            })
            .collect()
    }

    /// Run the full datacore pipeline on a batch of RawSignals before perception.
    ///
    /// Normalizes → stores in timeseries → detects anomalies → perceives each.
    /// Anomalous signals get their TritWord phase attenuated (multiplied by 0.5)
    /// to reduce their influence on ternary decisions downstream.
    ///
    /// Returns (perceived batches, anomaly results) for downstream processing.
    ///
    /// ponytail: this is the bridge between dataforge acquisition and aurora
    /// perception. Signals flow through datacore's normalize+timeseries+anomaly
    /// before hitting the prism.
    pub fn pipe_and_perceive(
        &self,
        signals: &[RawSignal],
    ) -> (Vec<PerceptBatch>, Vec<datacore::AnomalyResult>) {
        // 1. Normalize
        let normalizer = datacore::SignalNormalizer::new();
        let normalized = normalizer.normalize_batch(signals);

        // 2. Store in time series
        let mut store = datacore::TimeSeriesStore::new();
        store.insert_batch(&normalized);

        // 3. Detect anomalies: z-score + threshold
        let z_detector = datacore::AnomalyDetector::default();
        let anomalies = z_detector.score_all(&store);

        let t_detector = datacore::ThresholdDetector::with_climate_defaults();
        let threshold_alerts = t_detector.check_all(&store);

        // Build a set of (parameter, timestamp) keys that are anomalous,
        // for rapid lookup during phase attenuation. Combine both detectors.
        let mut anomalous_keys: std::collections::HashSet<(String, i64)> = anomalies
            .iter()
            .filter(|a| a.is_anomalous)
            .map(|a| {
                (
                    a.point.parameter.clone(),
                    a.point.timestamp.timestamp_micros(),
                )
            })
            .collect();
        // Add threshold violations to the anomalous set
        for alert in &threshold_alerts {
            anomalous_keys.insert((
                alert.point.parameter.clone(),
                alert.point.timestamp.timestamp_micros(),
            ));
        }

        // Build a set of (parameter, timestamp) keys for threshold violations,
        // which get stronger attenuation (×0.25) vs z-score anomalies (×0.5).
        let threshold_keys: std::collections::HashSet<(String, i64)> = threshold_alerts
            .iter()
            .map(|a| {
                (
                    a.point.parameter.clone(),
                    a.point.timestamp.timestamp_micros(),
                )
            })
            .collect();

        // ponytail: attenuation factors.
        // z-score anomaly → phase ×0.5 (uncertain, reduce influence)
        // threshold violation → phase ×0.25 (known danger, strongly reduce)
        const Z_ATTENUATION: f64 = 0.5;
        const THRESHOLD_ATTENUATION: f64 = 0.25;

        // 4. Perceive each raw signal (existing degrade path)
        let batches: Vec<PerceptBatch> = signals
            .iter()
            .filter_map(|sig| match self.perceive_one(sig) {
                Ok(mut batch) => {
                    // Attenuate phase of TritWords from anomalous signals
                    if !anomalous_keys.is_empty() || !threshold_keys.is_empty() {
                        for word in &mut batch.signals {
                            let sig_ts = sig.captured_at.timestamp_micros();
                            let is_z_anomaly = anomalous_keys
                                .iter()
                                .any(|(_, ts)| (ts - sig_ts).abs() < 3_600_000_000);
                            let is_threshold = threshold_keys
                                .iter()
                                .any(|(_, ts)| (ts - sig_ts).abs() < 3_600_000_000);

                            if is_z_anomaly || is_threshold {
                                let factor = if is_threshold {
                                    THRESHOLD_ATTENUATION
                                } else {
                                    Z_ATTENUATION
                                };
                                let current_phase = word.phase().inner();
                                let attenuated = Phase::new_clamped(current_phase * factor);
                                if let Ok(attenuated_word) = word.with_phase(attenuated) {
                                    *word = attenuated_word;
                                }
                            }
                        }
                    }
                    Some(batch)
                }
                Err(e) => {
                    tracing::warn!(
                        source = %sig.source_name,
                        error = %e,
                        "pipe_and_perceive: perception failed, skipping"
                    );
                    None
                }
            })
            .collect();

        (batches, anomalies)
    }

    /// Flatten a batch perception into a single Vec<TritWord> for trit-core.
    ///
    /// Each PerceptBatch contributes its signals. The source name is NOT
    /// attached to individual TritWords (TritWord has no source field) —
    /// instead, the caller should track which batch each signal came from
    /// if per-source traceability is needed.
    pub fn flatten_signals(batches: &[(RawSignal, PerceptBatch)]) -> Vec<TritWord> {
        batches
            .iter()
            .flat_map(|(_, batch)| batch.signals.clone())
            .collect()
    }

    /// Build the prompt for a RawSignal.
    ///
    /// Prepends source credibility metadata as structured context,
    /// followed by the raw data. The LLM's percept_system.txt already
    /// instructs it to decompose without interpretation.
    fn build_prompt(&self, signal: &RawSignal) -> String {
        let profile = self.source_weights.get(&signal.source_name);
        let credibility = profile
            .map(|p| format!("{:.2}", p.credibility))
            .unwrap_or_else(|| "unknown".into());
        let category = profile
            .map(|p| p.category.to_string())
            .unwrap_or_else(|| "unknown".into());

        let mut prompt = String::new();
        prompt.push_str(&format!(
            "[source: {}, credibility: {}, category: {}]\n",
            signal.source_name, credibility, category
        ));
        if let Some(ref period) = signal.data_period {
            prompt.push_str(&format!("[data period: {}]\n", period));
        }
        if let Some(ref loc) = signal.location {
            prompt.push_str(&format!(
                "[location: lat={:.4}, lng={:.4}]\n",
                loc.lat, loc.lng
            ));
        }
        prompt.push_str(&format!("[data: \"{}\"]\n", signal.raw_content));
        prompt
    }
}

/// Extract a numeric value following a key in text like "key:number".
///
/// ponytail: simple colon-split + parse — no regex needed. Handles
/// whitespace and sign.
fn extract_number_after(text: &str, key: &str) -> Option<f64> {
    let prefix = format!("{}:", key);
    let pos = text.find(&prefix)?;
    let rest = &text[pos + prefix.len()..];
    // Take the first contiguous chunk that looks like a number
    let num_str = rest.split_ascii_whitespace().next()?;
    // Strip trailing punctuation
    let cleaned: String = num_str
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-' || *c == '+')
        .collect();
    cleaned.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_weights_defaults_have_ten_sources() {
        let weights = SourceWeights::with_defaults();
        assert_eq!(weights.len(), 10);
        assert!(weights.get("NOAA GML").is_some());
        assert!(weights.get("Open-Meteo").is_some());
        assert!(weights.get("NSIDC").is_some());
        assert!(weights.get("NOAA Tides").is_some());
        assert!(weights.get("GBIF").is_some());
        assert!(weights.get("arXiv").is_some());
        assert!(weights.get("UCDP GED").is_some());
        assert!(weights.get("USGS").is_some());
        assert!(weights.get("NASA GIBS").is_some());
        assert!(weights.get("NASA POWER").is_some());
    }

    #[test]
    fn source_profile_credibility_clamped() {
        let p = SourceProfile::new(1.5, DataCategory::Climate);
        assert!((p.credibility - 1.0).abs() < 1e-9);
        let p = SourceProfile::new(-0.5, DataCategory::Ecology);
        assert!((p.credibility - 0.0).abs() < 1e-9);
    }

    #[test]
    fn build_prompt_includes_metadata() {
        let weights = SourceWeights::with_defaults();
        // Use an empty chain — prompt building doesn't need a real chain
        let chain = PerceptChain::new();
        let engine = PrismEngine::new(chain, weights);

        let signal = RawSignal {
            id: "test-1".into(),
            source_url: "https://example.com/data".into(),
            source_name: "NOAA GML".into(),
            category: DataCategory::Climate,
            raw_content: "co2_ppm:432.34".into(),
            captured_at: chrono::Utc::now(),
            data_period: Some("2026-05".into()),
            location: Some(dataforge::types::GeoPoint {
                lat: 19.54,
                lng: -155.58,
            }),
        };

        let prompt = engine.build_prompt(&signal);
        assert!(prompt.contains("NOAA GML"));
        assert!(prompt.contains("credibility: 0.95"));
        assert!(prompt.contains("category: climate"));
        assert!(prompt.contains("co2_ppm:432.34"));
        assert!(prompt.contains("lat=19.5400"));
    }

    #[test]
    fn empty_weights_handles_unknown_source() {
        let weights = SourceWeights::new();
        let chain = PerceptChain::new();
        let engine = PrismEngine::new(chain, weights);

        let signal = RawSignal {
            id: "test-1".into(),
            source_url: "https://unknown.example.com".into(),
            source_name: "UnknownBlog".into(),
            category: DataCategory::Other,
            raw_content: "something happened".into(),
            captured_at: chrono::Utc::now(),
            data_period: None,
            location: None,
        };

        let prompt = engine.build_prompt(&signal);
        assert!(prompt.contains("credibility: unknown"));
        assert!(prompt.contains("UnknownBlog"));
    }

    // ── degrade_to_structured tests ─────────────────────────────────

    #[test]
    fn degrade_extracts_co2_ppm() {
        let signal = RawSignal {
            id: "test".into(),
            source_url: "https://example.com".into(),
            source_name: "NOAA GML".into(),
            category: DataCategory::Climate,
            raw_content: "co2_ppm:432.34".into(),
            captured_at: chrono::Utc::now(),
            data_period: None,
            location: None,
        };
        let weights = SourceWeights::with_defaults();
        let engine = PrismEngine::new(PerceptChain::new(), weights);
        let batch = engine.degrade_to_structured(&signal);
        assert_eq!(batch.signals.len(), 1);
        let word = &batch.signals[0];
        // 432.34 > 420 → False (concerning)
        assert_eq!(word.value(), TritValue::False);
        assert_eq!(word.frame(), Frame::Instrumental);
        assert!((word.phase().inner() - 0.95).abs() < 0.01);
        assert!((batch.confidence - 0.95).abs() < 0.01);
    }

    #[test]
    fn degrade_normal_co2_is_true() {
        let signal = RawSignal {
            id: "test".into(),
            source_url: "https://example.com".into(),
            source_name: "NOAA GML".into(),
            category: DataCategory::Climate,
            raw_content: "co2_ppm:415.0".into(),
            captured_at: chrono::Utc::now(),
            data_period: None,
            location: None,
        };
        let weights = SourceWeights::with_defaults();
        let engine = PrismEngine::new(PerceptChain::new(), weights);
        let batch = engine.degrade_to_structured(&signal);
        assert_eq!(batch.signals[0].value(), TritValue::True);
    }

    #[test]
    fn degrade_temperature_anomaly_above_paris_threshold() {
        let signal = RawSignal {
            id: "test".into(),
            source_url: "https://example.com".into(),
            source_name: "Open-Meteo".into(),
            category: DataCategory::Climate,
            raw_content:
                "station:Mauna Loa lat:19.54 lng:-155.58 temp_mean_c:15.50 anomaly_c:+1.62".into(),
            captured_at: chrono::Utc::now(),
            data_period: None,
            location: None,
        };
        let weights = SourceWeights::with_defaults();
        let engine = PrismEngine::new(PerceptChain::new(), weights);
        let batch = engine.degrade_to_structured(&signal);
        // 1.62 > 1.5 → False
        assert_eq!(batch.signals[0].value(), TritValue::False);
    }

    #[test]
    fn degrade_unparseable_content_is_hold() {
        let signal = RawSignal {
            id: "test".into(),
            source_url: "https://example.com".into(),
            source_name: "GBIF".into(),
            category: DataCategory::Ecology,
            raw_content: "observed species: Panthera tigris count: 3 individuals".into(),
            captured_at: chrono::Utc::now(),
            data_period: None,
            location: None,
        };
        let weights = SourceWeights::with_defaults();
        let engine = PrismEngine::new(PerceptChain::new(), weights);
        let batch = engine.degrade_to_structured(&signal);
        // No parseable key:number → Hold
        assert_eq!(batch.signals[0].value(), TritValue::Hold);
        assert!((batch.signals[0].phase().inner() - 0.5).abs() < 0.01);
    }

    // ── extract_number_after tests ──────────────────────────────────

    #[test]
    fn extract_number_positive_float() {
        assert!((extract_number_after("co2_ppm:432.34", "co2_ppm").unwrap() - 432.34).abs() < 0.01);
    }

    #[test]
    fn extract_number_negative() {
        assert!(
            (extract_number_after("anomaly_c:-1.62 extra", "anomaly_c").unwrap() - (-1.62)).abs()
                < 0.01
        );
    }

    #[test]
    fn extract_number_with_sign() {
        assert!(
            (extract_number_after("anomaly_c:+1.62", "anomaly_c").unwrap() - 1.62).abs() < 0.01
        );
    }

    #[test]
    fn extract_number_missing_key_returns_none() {
        assert!(extract_number_after("no match here", "co2_ppm").is_none());
    }

    #[test]
    fn extract_number_integer() {
        assert!((extract_number_after("deaths:42 people", "deaths").unwrap() - 42.0).abs() < 0.01);
    }

    // ── end-to-end: perceive_batch with empty chain (degradation path) ─

    #[test]
    fn perceive_batch_with_empty_chain_uses_degradation() {
        let engine = PrismEngine::new(PerceptChain::new(), SourceWeights::with_defaults());

        let signals = vec![
            RawSignal {
                id: "s1".into(),
                source_url: "https://example.com".into(),
                source_name: "NOAA GML".into(),
                category: DataCategory::Climate,
                raw_content: "co2_ppm:432.34".into(),
                captured_at: chrono::Utc::now(),
                data_period: Some("2026-06".into()),
                location: None,
            },
            RawSignal {
                id: "s2".into(),
                source_url: "https://example.com".into(),
                source_name: "Open-Meteo".into(),
                category: DataCategory::Climate,
                raw_content: "station:Mauna Loa anomaly_c:+1.62".into(),
                captured_at: chrono::Utc::now(),
                data_period: None,
                location: None,
            },
            RawSignal {
                id: "s3".into(),
                source_url: "https://example.com".into(),
                source_name: "UCDP GED".into(),
                category: DataCategory::Geopolitical,
                raw_content: "deaths:12 country:Sudan".into(),
                captured_at: chrono::Utc::now(),
                data_period: None,
                location: None,
            },
        ];

        let batches = engine.perceive_batch(&signals);
        // All 3 signals should succeed via structured degradation
        assert_eq!(batches.len(), 3);
        assert_eq!(batches[0].0.source_name, "NOAA GML");
        assert_eq!(batches[0].1.source, "NOAA GML/structured");
        assert_eq!(batches[0].1.signals[0].value(), TritValue::False); // 432 > 420

        assert_eq!(batches[1].0.source_name, "Open-Meteo");
        assert_eq!(batches[1].1.signals[0].value(), TritValue::False); // 1.62 > 1.5

        assert_eq!(batches[2].0.source_name, "UCDP GED");
        assert_eq!(batches[2].1.signals[0].frame(), Frame::Consensus);
    }

    #[test]
    fn perceive_batch_flatten_signals_roundtrip() {
        let engine = PrismEngine::new(PerceptChain::new(), SourceWeights::with_defaults());
        let signals = vec![RawSignal {
            id: "s1".into(),
            source_url: "https://example.com".into(),
            source_name: "NOAA GML".into(),
            category: DataCategory::Climate,
            raw_content: "co2_ppm:415.0".into(),
            captured_at: chrono::Utc::now(),
            data_period: None,
            location: None,
        }];
        let batches = engine.perceive_batch(&signals);
        let words = PrismEngine::flatten_signals(&batches);
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].value(), TritValue::True);
        assert_eq!(words[0].frame(), Frame::Instrumental);
    }

    #[test]
    fn pipe_and_perceive_with_co2_data() {
        let engine = PrismEngine::new(PerceptChain::new(), SourceWeights::with_defaults());
        let signals = vec![
            RawSignal {
                id: "co2_1".into(),
                source_url: "https://example.com".into(),
                source_name: "NOAA GML".into(),
                category: DataCategory::Climate,
                raw_content: "co2_ppm:418.00".into(),
                captured_at: chrono::Utc::now(),
                data_period: None,
                location: None,
            },
            RawSignal {
                id: "co2_2".into(),
                source_url: "https://example.com".into(),
                source_name: "NOAA GML".into(),
                category: DataCategory::Climate,
                raw_content: "co2_ppm:419.50".into(),
                captured_at: chrono::Utc::now() + chrono::Duration::hours(1),
                data_period: None,
                location: None,
            },
            RawSignal {
                id: "co2_spike".into(),
                source_url: "https://example.com".into(),
                source_name: "NOAA GML".into(),
                category: DataCategory::Climate,
                raw_content: "co2_ppm:550.00".into(),
                captured_at: chrono::Utc::now() + chrono::Duration::hours(2),
                data_period: None,
                location: None,
            },
        ];

        let (batches, anomalies) = engine.pipe_and_perceive(&signals);
        // All three signals should be perceived (degrade path)
        assert_eq!(batches.len(), 3);
        // The spike should be detected as an anomaly
        assert!(!anomalies.is_empty());
        let spike_anomalies: Vec<_> = anomalies.iter().filter(|a| a.is_anomalous).collect();
        assert!(
            !spike_anomalies.is_empty(),
            "550 ppm spike should be anomalous"
        );

        // Verify that the spike batch has attenuated phase compared to normal
        // ponytail: all signals in the spike batch are attenuated because
        // the batch-level matching at ±1 hour window catches the anomaly.
        let spike_batch = &batches[2]; // third signal = spike
        assert_eq!(spike_batch.signals.len(), 1);
        let spike_phase = spike_batch.signals[0].phase().inner();
        let normal_phase = batches[0].signals[0].phase().inner();
        assert!(
            spike_phase < normal_phase,
            "anomalous signal phase ({spike_phase}) should be lower than normal ({normal_phase})"
        );
    }
}
