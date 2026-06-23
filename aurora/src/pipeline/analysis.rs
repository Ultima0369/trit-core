//! Analysis pipeline link: SignalAnalysis BC → TernaryDecision BC.
//!
//! Generates a synthetic signal, detects its fundamental frequency via FFT,
//! maps frequency and user state to TritWords, and evaluates a ternary decision.

use crate::bc::signal_analysis::{FftWaveletEngine, FrequencySpectrum, TimeSeries, WaveletEngine};
use crate::bc::ternary_decision::{DecisionPort, DecisionRecord, DecisionSession, TritDecisionEngine};
use crate::bc::BcError;
use crate::wavelet::sine_wave;
use serde::Deserialize;
use truncore::core::{Frame, TritWord};

/// Specification for a synthetic signal, deserializable from JSON input.
#[derive(Debug, Clone, Deserialize)]
pub struct SignalSpec {
    pub freq: f64,
    pub sample_rate: f64,
    pub duration_secs: f64,
    pub noise_std: f64,
}

/// Structured report from the analysis pipeline link.
#[derive(Debug, Clone)]
pub struct AnalysisReport {
    /// The frequency spectrum detected by FFT analysis.
    pub spectrum: FrequencySpectrum,
    /// The ternary decision record.
    pub decision: DecisionRecord,
}

/// Map a detected frequency to an Embodied-frame TritWord.
///
/// Frequencies above the threshold indicate a physically-embodied signal
/// (e.g., real-time communication), producing a True in the Embodied frame.
/// Frequencies below the threshold indicate a non-embodied signal,
/// producing a False in the Embodied frame.
pub fn frequency_to_embodied(freq: f64, threshold: f64) -> TritWord {
    if freq > threshold {
        TritWord::tru(Frame::Embodied)
    } else {
        TritWord::fals(Frame::Embodied)
    }
}

/// Map a user's self-reported state to an Individual-frame TritWord.
///
/// `true` (feels normal) → True in Individual frame.
/// `false` (feels off) → False in Individual frame.
pub fn user_state_to_individual(feels_normal: bool) -> TritWord {
    if feels_normal {
        TritWord::tru(Frame::Individual)
    } else {
        TritWord::fals(Frame::Individual)
    }
}

/// Run the analysis pipeline link.
///
/// 1. Generate a synthetic sine wave from the signal spec.
/// 2. Create a TimeSeries and analyze it via FFT → FrequencySpectrum.
/// 3. Map frequency → Embodied TritWord, user state → Individual TritWord.
/// 4. Evaluate the ternary decision via TritDecisionEngine.
///
/// Returns an [`AnalysisReport`] containing both the spectrum and the decision.
pub fn run_analysis(
    spec: &SignalSpec,
    frequency_threshold: f64,
    user_feels_normal: bool,
) -> Result<AnalysisReport, BcError> {
    // Step 1: Generate synthetic signal
    let signal = sine_wave(spec.freq, spec.sample_rate, spec.duration_secs, spec.noise_std);

    // Step 2: Analyze via FFT
    let ts = TimeSeries::new(spec.sample_rate, signal)?;
    let engine = FftWaveletEngine;
    let spectrum = engine.analyze(&ts)?;

    // Step 3: Map to TritWords
    let embodied = frequency_to_embodied(spectrum.fundamental_hz, frequency_threshold);
    let individual = user_state_to_individual(user_feels_normal);

    // Step 4: Evaluate ternary decision
    let decision_engine = TritDecisionEngine;
    let mut session = DecisionSession::new("analysis_session".into());
    let decision = decision_engine.evaluate(&mut session, &[embodied, individual], "General")?;

    Ok(AnalysisReport { spectrum, decision })
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── frequency_to_embodied ────────────────────────────────────────────

    #[test]
    fn frequency_above_threshold_is_embodied_true() {
        let result = frequency_to_embodied(5.0, 3.0);
        assert_eq!(result.value(), truncore::core::TritValue::True);
        assert_eq!(result.frame(), Frame::Embodied);
    }

    #[test]
    fn frequency_below_threshold_is_embodied_false() {
        let result = frequency_to_embodied(1.0, 3.0);
        assert_eq!(result.value(), truncore::core::TritValue::False);
        assert_eq!(result.frame(), Frame::Embodied);
    }

    #[test]
    fn frequency_equal_to_threshold_is_embodied_false() {
        // Strictly greater than threshold for True; equal → False
        let result = frequency_to_embodied(3.0, 3.0);
        assert_eq!(result.value(), truncore::core::TritValue::False);
    }

    // ── user_state_to_individual ─────────────────────────────────────────

    #[test]
    fn user_feels_normal_is_individual_true() {
        let result = user_state_to_individual(true);
        assert_eq!(result.value(), truncore::core::TritValue::True);
        assert_eq!(result.frame(), Frame::Individual);
    }

    #[test]
    fn user_feels_off_is_individual_false() {
        let result = user_state_to_individual(false);
        assert_eq!(result.value(), truncore::core::TritValue::False);
        assert_eq!(result.frame(), Frame::Individual);
    }

    // ── run_analysis ─────────────────────────────────────────────────────

    #[test]
    fn run_analysis_detects_2_5hz() {
        let spec = SignalSpec {
            freq: 2.5,
            sample_rate: 100.0,
            duration_secs: 1.0,
            noise_std: 0.0,
        };
        let report = run_analysis(&spec, 1.0, true).unwrap();

        // 2.5 Hz should be detected within ±0.5 Hz
        assert!(
            (report.spectrum.fundamental_hz - 2.5).abs() < 0.5,
            "expected ~2.5 Hz, got {}",
            report.spectrum.fundamental_hz
        );
        // Both signals same frame (Embodied + Individual → cross-frame) → Hold
        assert!(report.decision.is_hold());
        assert!(report.decision.has_conflicts());
    }

    #[test]
    fn run_analysis_cross_frame_produces_hold() {
        let spec = SignalSpec {
            freq: 2.5,
            sample_rate: 100.0,
            duration_secs: 1.0,
            noise_std: 0.0,
        };
        // user_feels_normal=true → Individual True, freq > threshold → Embodied True
        // Cross-frame → Hold
        let report = run_analysis(&spec, 1.0, true).unwrap();
        assert!(report.decision.is_hold());
        assert!(report.decision.has_conflicts());
    }

    #[test]
    fn run_analysis_high_freq_above_threshold() {
        let spec = SignalSpec {
            freq: 10.0,
            sample_rate: 200.0,
            duration_secs: 1.0,
            noise_std: 0.01,
        };
        let report = run_analysis(&spec, 5.0, true).unwrap();
        // 10 Hz > 5 Hz threshold → Embodied True
        // Cross-frame with Individual True → Hold
        assert!(report.decision.is_hold());
    }

    #[test]
    fn run_analysis_low_freq_below_threshold() {
        let spec = SignalSpec {
            freq: 1.0,
            sample_rate: 100.0,
            duration_secs: 1.0,
            noise_std: 0.0,
        };
        // user_feels_normal=false → Individual False, freq < threshold → Embodied False
        // Both False, same frame? No — Embodied and Individual are different frames → Hold
        let report = run_analysis(&spec, 3.0, false).unwrap();
        assert!(report.decision.is_hold());
    }
}
