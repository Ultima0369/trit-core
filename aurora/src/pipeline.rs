//! End-to-end Aurora pipeline: signal → frequency → decision → attention.

use crate::attention::AttentionManager;
use crate::decision::{detect_conflict, embodied_from_frequency, individual_from_user_state};
use crate::wavelet::{sine_wave, WaveletEngine};
use serde::Deserialize;
use truncore::adapters::AttentionCmd;
use truncore::core::TritWord;
use truncore::meta::MetaInterrupt;

/// Specification for a synthetic signal, deserializable from JSON input.
#[derive(Debug, Clone, Deserialize)]
pub struct SignalSpec {
    pub freq: f64,
    pub sample_rate: f64,
    pub duration_secs: f64,
    pub noise_std: f64,
}

/// Structured report produced by the end-to-end pipeline.
#[derive(Debug, Clone)]
pub struct DecisionReport {
    pub input_freq: f64,
    pub detected_freq: f64,
    pub embodied: TritWord,
    pub individual: TritWord,
    pub result: TritWord,
    pub interrupt: Option<MetaInterrupt>,
    /// Attention scheduler command (None if Continue).
    pub attention_cmd: Option<AttentionCmd>,
    /// Current ASI score.
    pub asi: f64,
    /// Number of reminders in this session.
    pub reminder_count: usize,
}

/// Run the full M0 pipeline on a synthetic signal specification.
///
/// Now includes attention scheduling: after the decision, the attention
/// manager runs one cycle and records any reminders.
pub fn run_pipeline(
    spec: &SignalSpec,
    frequency_threshold: f64,
    user_feels_normal: bool,
    attention: &mut AttentionManager,
) -> Result<DecisionReport, Box<dyn std::error::Error>> {
    let signal = sine_wave(
        spec.freq,
        spec.sample_rate,
        spec.duration_secs,
        spec.noise_std,
    );
    let engine = WaveletEngine::new(spec.sample_rate);
    let wavelet_result = engine.analyze(&signal)?;

    let embodied = embodied_from_frequency(wavelet_result.fundamental_freq, frequency_threshold);
    let individual = individual_from_user_state(user_feels_normal);
    let (result, interrupt) = detect_conflict(&embodied, &individual);

    // Run attention scheduler on the decision signals
    let attention_cmd = attention.run_cycle(&[embodied, individual, result]);

    Ok(DecisionReport {
        input_freq: spec.freq,
        detected_freq: wavelet_result.fundamental_freq,
        embodied,
        individual,
        result,
        interrupt,
        attention_cmd,
        asi: attention.session().asi(),
        reminder_count: attention.session().reminder_count(),
    })
}
