//! End-to-end Aurora pipeline: signal → frequency → decision.

use crate::decision::{detect_conflict, embodied_from_frequency, individual_from_user_state};
use crate::wavelet::{sine_wave, WaveletEngine};
use serde::Deserialize;
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
}

/// Run the full M0 pipeline on a synthetic signal specification.
pub fn run_pipeline(
    spec: &SignalSpec,
    frequency_threshold: f64,
    user_feels_normal: bool,
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

    Ok(DecisionReport {
        input_freq: spec.freq,
        detected_freq: wavelet_result.fundamental_freq,
        embodied,
        individual,
        result,
        interrupt,
    })
}
