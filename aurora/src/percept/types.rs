use chrono::{DateTime, Utc};
use serde::Deserialize;
use trit_core::TritWord;

/// Specification for a synthetic signal, deserializable from JSON input.
///
/// Moved from `pipeline::analysis::SignalSpec` during the BC Architecture
/// Hardening (2026-07-08). This is a perception input parameter — it describes
/// the signal to be perceived, not how the analysis pipeline operates.
#[derive(Debug, Clone, Deserialize)]
pub struct SignalSpec {
    pub freq: f64,
    pub sample_rate: f64,
    pub duration_secs: f64,
    pub noise_std: f64,
}

/// A batch of TritWord signals extracted from raw input by a perception provider.
///
/// ## 流沙 (Flowing Sands) Design Philosophy
///
/// This struct embodies three principles from the 流沙 philosophy:
///
/// - **璇玑 (Armillary Sphere)**: `signals` are faithful rotations of raw input —
///   no meaning attached, no interpretation embedded. Each signal is a pure
///   spectral decomposition of what was perceived.
///
/// - **棱镜 (Prism)**: each signal is one spectral band — a Frame, a Value,
///   a Phase. The user sees what their angle reveals. No band is "the truth."
///
/// - **微风 (Breeze)**: no summary, no suggestion, no trace. Signals pass
///   through the perception layer and dissolve. Only the user's own reaction
///   to the data remains.
///
/// There is deliberately NO `summary` field — would violate 零文字 (不解释).
/// There is deliberately NO `suggested_scenario` field — would violate 棱镜 (不引导).
/// Scenario recognition is Trit-Core's job, not the perception provider's.
///
/// The only text field is `raw_data_layer` — it describes physical measurements
/// (the territory), never interpretations (the map).
#[derive(Debug, Clone)]
pub struct PerceptBatch {
    /// Extracted ternary signals — the prismatic decomposition of raw input.
    ///
    /// Each signal is one spectral band: a Frame, a Value, a Phase.
    /// No signal carries an explanation of "why" — only "what."
    pub signals: Vec<TritWord>,

    /// Provider name for audit trail (e.g. "claude-opus-4-8", "local-llm").
    pub source: String,

    /// Perception timestamp (UTC).
    pub timestamp: DateTime<Utc>,

    /// Provider-reported confidence, range 0.0–1.0.
    ///
    /// This is a signal-quality marker, not a truth claim.
    /// Trit-Core may override decisions regardless of confidence.
    pub confidence: f64,

    /// Pure physical data layer description (optional).
    ///
    /// When the input contains references to measurable physical quantities
    /// (temperature, wind speed, population density, CO₂ levels, elevation,
    /// precipitation, etc.), this field records those quantities as raw
    /// data points.
    ///
    /// Format: free-form text describing physical measurements only.
    ///
    /// **MUST NOT contain**: advice, interpretation, suggestions, conclusions,
    /// imperative language, or any form of "you should."
    ///
    /// Example: `"surface_temp:28.4C wind:12km/h_NE humidity:65%"`
    ///
    /// This describes the territory, not the map.
    pub raw_data_layer: Option<String>,
}

impl PerceptBatch {
    /// Create an empty batch (used by FFTProvider when no text input is relevant).
    pub fn empty(source: impl Into<String>) -> Self {
        Self {
            signals: Vec::new(),
            source: source.into(),
            timestamp: Utc::now(),
            confidence: 1.0,
            raw_data_layer: None,
        }
    }
}
