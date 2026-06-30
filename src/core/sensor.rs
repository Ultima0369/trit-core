//! Sensor-aware input types for mind-engineering extensions.
//!
//! Real-world decision systems receive signals from more than text:
//! body state, environmental snapshots, and cognitive-load estimates all
//! carry information that can be mapped into the ternary decision space.
//! This module provides minimal, extensible containers for those signals.

use serde::{Deserialize, Serialize};

/// A multi-modal sensor signal that can be converted into a [`crate::core::word::TritWord`].
///
/// The variants are intentionally coarse: they name the *origin* of the
/// signal rather than prescribing a specific sensor hardware interface.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum SensorSignal {
    /// Body-state measurement such as heart-rate variability, galvanic skin
    /// response, or muscle tension.
    BodyState(BodyState),
    /// Environmental snapshot such as temperature, luminance, noise level,
    /// or social density.
    Environmental(EnvSnapshot),
    /// Cognitive-state estimate such as attention bandwidth or cognitive load.
    Cognitive(CogState),
    /// Original text-based input signal.
    Text(TextInput),
}

/// Body-state signal container.
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct BodyState {
    /// Arousal level in `[0.0, 1.0]`. Higher values indicate stronger
    /// sympathetic activation.
    pub arousal: f64,
    /// Readiness / energy level in `[0.0, 1.0]`.
    pub readiness: f64,
    /// Optional textual label for the measurement (e.g. "hrv", "gsr").
    #[serde(default)]
    pub source: String,
}

/// Environmental snapshot container.
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct EnvSnapshot {
    /// Ambient arousal of the environment in `[0.0, 1.0]` (noise, crowding,
    /// temperature extremes, etc.).
    pub ambient_arousal: f64,
    /// Social density in `[0.0, 1.0]`.
    pub social_density: f64,
    /// Optional textual label for the environment type.
    #[serde(default)]
    pub context_label: String,
}

/// Cognitive-state estimate container.
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct CogState {
    /// Estimated available attention bandwidth in `[0.0, 1.0]`.
    pub attention_bandwidth: f64,
    /// Estimated cognitive load in `[0.0, 1.0]`.
    pub cognitive_load: f64,
    /// Optional label for the cognitive mode (e.g. "deliberative",
    /// "reactive", "ruminative").
    #[serde(default)]
    pub mode: String,
}

/// Text-based input signal, preserving the original external-input origin.
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct TextInput {
    /// Raw text payload.
    pub text: String,
    /// Optional sentiment or stance phase in `[0.0, 1.0]`, where `0.5` is
    /// neutral. When absent, defaults to `0.5`.
    #[serde(default = "default_neutral_phase")]
    pub phase_hint: f64,
}

fn default_neutral_phase() -> f64 {
    0.5
}

/// Context that persists across a decision session and influences how
/// sensor signals are interpreted.
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct EnvironmentalContext {
    /// Time scale at which the current decision is being made.
    #[serde(default)]
    pub temporal_scale: TemporalScale,
    /// Social density of the environment in `[0.0, 1.0]`.
    pub social_density: f64,
    /// Ambient arousal of the environment in `[0.0, 1.0]`.
    pub ambient_arousal: f64,
}

/// Time scale for a decision.
#[derive(Debug, Clone, Copy, PartialEq, Default, Deserialize, Serialize)]
pub enum TemporalScale {
    /// Milliseconds — reactive, reflexive decisions.
    Milliseconds,
    /// Seconds — immediate action selection.
    Seconds,
    /// Minutes — short deliberation.
    Minutes,
    /// Hours — planning horizon.
    Hours,
    /// Days or longer — strategic reflection.
    #[default]
    Days,
}

impl SensorSignal {
    /// Return a human-readable origin label for logging and interrupts.
    pub fn origin_label(&self) -> &'static str {
        match self {
            SensorSignal::BodyState(_) => "body",
            SensorSignal::Environmental(_) => "environment",
            SensorSignal::Cognitive(_) => "cognitive",
            SensorSignal::Text(_) => "text",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensor_signal_origin_labels() {
        assert_eq!(
            SensorSignal::Text(TextInput::default()).origin_label(),
            "text"
        );
        assert_eq!(
            SensorSignal::BodyState(BodyState::default()).origin_label(),
            "body"
        );
    }

    #[test]
    fn environmental_context_defaults() {
        let ctx = EnvironmentalContext::default();
        assert_eq!(ctx.temporal_scale, TemporalScale::Days);
        assert_eq!(ctx.social_density, 0.0);
        assert_eq!(ctx.ambient_arousal, 0.0);
    }
}
