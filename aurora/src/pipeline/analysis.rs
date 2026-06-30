//! Analysis pipeline link: SignalAnalysis BC → TernaryDecision BC.
//!
//! Generates a synthetic signal, detects its fundamental frequency via FFT,
//! maps frequency and user state to TritWords, and evaluates a ternary decision.

use crate::bc::relationship_annotation::ContactProfile;
use crate::bc::signal_analysis::{FftWaveletEngine, FrequencySpectrum, TimeSeries};
use crate::bc::ternary_decision::{DecisionRecord, DecisionSession, TritDecisionEngine};
use crate::bc::BcError;
use crate::wavelet::sine_wave;
use serde::Deserialize;
use std::str::FromStr;
use truncore::core::{Frame, Phase, PhaseTracker, Trend, TritValue, TritWord};

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
    /// Number of contact-derived TritWords that participated in the decision.
    pub contact_count: usize,
}

/// Accumulated phase trajectory across analysis runs.
///
/// Holds a [`PhaseTracker`] for the Embodied frame and the overall
/// decision phase, updated each time [`run_analysis`] is called.
#[derive(Debug, Clone)]
pub struct PhaseTrajectory {
    /// Tracker for the Embodied frame phase (from frequency analysis).
    pub embodied: PhaseTracker,
    /// Tracker for the overall decision result phase.
    pub decision: PhaseTracker,
    /// Number of analysis runs recorded.
    pub runs: u64,
}

impl PhaseTrajectory {
    /// Create a new trajectory seeded from the first analysis report.
    pub fn new(report: &AnalysisReport) -> Self {
        // The Embodied signal is the first element in input_signals
        let embodied_phase = report
            .decision
            .input_signals
            .iter()
            .find(|w| w.frame() == Frame::Embodied)
            .map(|w| w.phase())
            .unwrap_or_else(Phase::neutral);

        let decision_phase = report.decision.result.phase();

        PhaseTrajectory {
            embodied: PhaseTracker::new(embodied_phase),
            decision: PhaseTracker::new(decision_phase),
            runs: 1,
        }
    }

    /// Feed a new analysis report into the trajectory.
    pub fn update(&mut self, report: &AnalysisReport) {
        let embodied_phase = report
            .decision
            .input_signals
            .iter()
            .find(|w| w.frame() == Frame::Embodied)
            .map(|w| w.phase())
            .unwrap_or_else(Phase::neutral);

        let decision_phase = report.decision.result.phase();

        self.embodied.update(embodied_phase);
        self.decision.update(decision_phase);
        self.runs += 1;
    }

    /// Summary of the current trend state.
    pub fn summary(&self) -> TrajectorySummary {
        TrajectorySummary {
            runs: self.runs,
            embodied_trend: self.embodied.trend(),
            embodied_velocity: self.embodied.velocity(),
            decision_trend: self.decision.trend(),
            decision_velocity: self.decision.velocity(),
            trend_reliable: self.embodied.trend_reliable() && self.decision.trend_reliable(),
        }
    }
}

/// Readable summary of phase trajectory.
#[derive(Debug, Clone)]
pub struct TrajectorySummary {
    pub runs: u64,
    pub embodied_trend: Trend,
    pub embodied_velocity: f64,
    pub decision_trend: Trend,
    pub decision_velocity: f64,
    pub trend_reliable: bool,
}

/// Convert loaded contacts to TritWords for decision input.
///
/// Each contact's frame annotations are mapped to TritWords.
/// Annotations with invalid phase values or unknown frame names
/// are skipped with a warning to stderr.
pub fn contacts_to_tritwords(contacts: &[ContactProfile]) -> Vec<TritWord> {
    let mut words = Vec::new();
    for contact in contacts {
        for ann in &contact.frames {
            match Phase::new(ann.phase) {
                Ok(phase) => {
                    let value = if phase.inner() >= 0.5 {
                        TritValue::True
                    } else {
                        TritValue::False
                    };
                    let frame = match Frame::from_str(&ann.frame) {
                        Ok(f) => f,
                        Err(_) => {
                            eprintln!(
                                "warning: unknown frame '{}' for contact {}, skipping",
                                ann.frame, contact.name
                            );
                            continue;
                        }
                    };
                    words.push(TritWord::new(value, phase, frame));
                }
                Err(_) => {
                    eprintln!(
                        "warning: invalid phase {} for contact {}, skipping",
                        ann.phase, contact.name
                    );
                }
            }
        }
    }
    words
}

/// Map a detected frequency to an Embodied-frame TritWord with continuous Phase.
///
/// Uses a sigmoid-like mapping based on the ratio of detected frequency to
/// threshold. When freq ≫ threshold, Phase → 1.0 (strongly embodied).
/// When freq ≪ threshold, Phase → 0.0 (weakly embodied).
/// When freq ≈ threshold, Phase → 0.5 (uncertain — in the fluctuation zone).
///
/// The steepness factor k=10 means: at freq/threshold = 1.0±0.3, Phase is in
/// the transition zone [0.05, 0.95]. Outside that, it saturates.
pub fn frequency_to_embodied(freq: f64, threshold: f64) -> TritWord {
    if threshold <= 0.0 {
        return TritWord::hold(Frame::Embodied);
    }
    let ratio = freq / threshold;
    // Logistic: 1 / (1 + exp(-k * (ratio - 1)))
    // ratio=1.0 → 0.5, ratio=2.0 → ~0.999, ratio=0.5 → ~0.007
    const K: f64 = 10.0;
    let phase_val = 1.0 / (1.0 + (-K * (ratio - 1.0)).exp());
    let phase = Phase::new_clamped(phase_val).quantize(1e-6);
    let value = if phase.inner() >= 0.5 {
        TritValue::True
    } else {
        TritValue::False
    };
    TritWord::new(value, phase, Frame::Embodied)
}

/// Map a user's self-reported state to an Individual-frame TritWord.
///
/// `true` (feels normal) → Phase 0.8 (confident True, but not absolute).
/// `false` (feels off) → Phase 0.2 (confident False, but not absolute).
///
/// Self-reported state is never 1.0 or 0.0 because subjective experience
/// is inherently uncertain — the user may not have full access to their
/// own physiological state.
pub fn user_state_to_individual(feels_normal: bool) -> TritWord {
    if feels_normal {
        TritWord::new(TritValue::True, Phase::new_clamped(0.8), Frame::Individual)
    } else {
        TritWord::new(TritValue::False, Phase::new_clamped(0.2), Frame::Individual)
    }
}

/// Run the analysis pipeline link.
///
/// 1. Generate a synthetic sine wave from the signal spec.
/// 2. Create a TimeSeries and analyze it via FFT → FrequencySpectrum.
/// 3. Map frequency → Embodied TritWord, user state → Individual TritWord.
/// 4. Merge contact-derived TritWords into the signal set.
/// 5. Evaluate the ternary decision via TritDecisionEngine.
///
/// Returns an [`AnalysisReport`] containing the spectrum, decision, and contact count.
pub fn run_analysis(
    spec: &SignalSpec,
    frequency_threshold: f64,
    user_feels_normal: bool,
    contact_signals: &[TritWord],
) -> Result<AnalysisReport, BcError> {
    // Step 1: Generate synthetic signal
    let signal = sine_wave(
        spec.freq,
        spec.sample_rate,
        spec.duration_secs,
        spec.noise_std,
    );

    // Step 2: Analyze via FFT
    let ts = TimeSeries::new(spec.sample_rate, signal)?;
    let engine = FftWaveletEngine;
    let spectrum = engine.analyze(&ts)?;

    // Step 3: Map to TritWords
    let embodied = frequency_to_embodied(spectrum.fundamental_hz, frequency_threshold);
    let individual = user_state_to_individual(user_feels_normal);

    // Step 4: Merge all signals (embodied + individual + contacts)
    let mut all_signals = vec![embodied, individual];
    all_signals.extend_from_slice(contact_signals);

    // Step 5: Evaluate ternary decision
    let decision_engine = TritDecisionEngine;
    let mut session = DecisionSession::new("analysis_session".into());
    let decision = decision_engine.evaluate(&mut session, &all_signals, "General")?;

    Ok(AnalysisReport {
        spectrum,
        decision,
        contact_count: contact_signals.len(),
    })
}

/// Run the analysis link with additional percept signals from external providers.
///
/// This is an overload of [`run_analysis`] that accepts percept signals
/// (from LLMs or other perception providers) and merges them into the
/// signal vector alongside embodied, individual, and contact signals
/// before ternary evaluation.
///
/// The original [`run_analysis`] function is unchanged and still available.
pub fn run_analysis_from_percept(
    spec: &SignalSpec,
    frequency_threshold: f64,
    user_feels_normal: bool,
    contact_signals: &[TritWord],
    percept_signals: &[TritWord],
) -> Result<AnalysisReport, BcError> {
    // Step 1: Generate synthetic signal
    let signal = sine_wave(
        spec.freq,
        spec.sample_rate,
        spec.duration_secs,
        spec.noise_std,
    );

    // Step 2: Analyze via FFT
    let ts = TimeSeries::new(spec.sample_rate, signal)?;
    let engine = FftWaveletEngine;
    let spectrum = engine.analyze(&ts)?;

    // Step 3: Map to TritWords
    let embodied = frequency_to_embodied(spectrum.fundamental_hz, frequency_threshold);
    let individual = user_state_to_individual(user_feels_normal);

    // Step 4: Merge all signals (embodied + individual + contacts + percept)
    let mut all_signals = vec![embodied, individual];
    all_signals.extend_from_slice(contact_signals);
    all_signals.extend_from_slice(percept_signals);

    // Step 5: Evaluate ternary decision
    let decision_engine = TritDecisionEngine;
    let mut session = DecisionSession::new("analysis_session".into());
    let decision = decision_engine.evaluate(&mut session, &all_signals, "General")?;

    Ok(AnalysisReport {
        spectrum,
        decision,
        contact_count: contact_signals.len(),
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── frequency_to_embodied ────────────────────────────────────────────

    #[test]
    fn frequency_above_threshold_is_embodied_true() {
        let result = frequency_to_embodied(5.0, 3.0);
        assert_eq!(result.value(), TritValue::True);
        assert_eq!(result.frame(), Frame::Embodied);
        // ratio=1.67 → phase > 0.99 (strongly embodied)
        assert!(result.phase().inner() > 0.99);
    }

    #[test]
    fn frequency_below_threshold_is_embodied_false() {
        let result = frequency_to_embodied(1.0, 3.0);
        assert_eq!(result.value(), TritValue::False);
        assert_eq!(result.frame(), Frame::Embodied);
        // ratio=0.33 → phase < 0.01 (weakly embodied)
        assert!(result.phase().inner() < 0.01);
    }

    #[test]
    fn frequency_equal_to_threshold_is_neutral_phase() {
        // ratio=1.0 → phase ≈ 0.5 (in the fluctuation zone)
        let result = frequency_to_embodied(3.0, 3.0);
        assert_eq!(result.frame(), Frame::Embodied);
        // At exactly threshold, phase is 0.5 (logistic midpoint)
        let phase = result.phase().inner();
        assert!(phase > 0.49 && phase < 0.51, "expected ~0.5, got {phase}");
    }

    #[test]
    fn frequency_near_threshold_is_in_transition_zone() {
        // ratio=1.3 → phase ≈ 0.95 (leaning True but not saturated)
        let result = frequency_to_embodied(3.9, 3.0);
        assert_eq!(result.value(), TritValue::True);
        let phase = result.phase().inner();
        assert!(phase > 0.9 && phase < 1.0, "expected 0.9–1.0, got {phase}");
    }

    #[test]
    fn frequency_zero_threshold_returns_hold() {
        let result = frequency_to_embodied(5.0, 0.0);
        assert_eq!(result.value(), TritValue::Hold);
    }

    // ── user_state_to_individual ─────────────────────────────────────────

    #[test]
    fn user_feels_normal_is_individual_true() {
        let result = user_state_to_individual(true);
        assert_eq!(result.value(), TritValue::True);
        assert_eq!(result.frame(), Frame::Individual);
        // Self-reported normal → Phase 0.8 (confident, not absolute)
        assert!((result.phase().inner() - 0.8).abs() < 0.01);
    }

    #[test]
    fn user_feels_off_is_individual_false() {
        let result = user_state_to_individual(false);
        assert_eq!(result.value(), TritValue::False);
        assert_eq!(result.frame(), Frame::Individual);
        // Self-reported off → Phase 0.2 (confident, not absolute)
        assert!((result.phase().inner() - 0.2).abs() < 0.01);
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
        let report = run_analysis(&spec, 1.0, true, &[]).unwrap();

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
        let report = run_analysis(&spec, 1.0, true, &[]).unwrap();
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
        let report = run_analysis(&spec, 5.0, true, &[]).unwrap();
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
        let report = run_analysis(&spec, 3.0, false, &[]).unwrap();
        assert!(report.decision.is_hold());
    }

    // ── contacts_to_tritwords ───────────────────────────────────────────

    #[test]
    fn contacts_to_tritwords_empty() {
        let contacts: Vec<ContactProfile> = vec![];
        let words = contacts_to_tritwords(&contacts);
        assert!(words.is_empty());
    }

    #[test]
    fn contacts_to_tritwords_maps_frame_and_phase() {
        let mut profile = ContactProfile::new(
            "c1".into(),
            "Alice".into(),
            crate::bc::relationship_annotation::RelationLabel::Friend,
        );
        profile.annotate_frame(
            crate::bc::relationship_annotation::FrameAnnotation::new(
                "Embodied".into(),
                "高频".into(),
                0.8,
            )
            .unwrap(),
        );

        let contacts = vec![profile];
        let words = contacts_to_tritwords(&contacts);
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].frame(), Frame::Embodied);
        assert_eq!(words[0].value(), TritValue::True);
    }

    #[test]
    fn contacts_to_tritwords_low_phase_is_false() {
        let mut profile = ContactProfile::new(
            "c1".into(),
            "Bob".into(),
            crate::bc::relationship_annotation::RelationLabel::Colleague,
        );
        profile.annotate_frame(
            crate::bc::relationship_annotation::FrameAnnotation::new(
                "Individual".into(),
                "低频".into(),
                0.3,
            )
            .unwrap(),
        );

        let contacts = vec![profile];
        let words = contacts_to_tritwords(&contacts);
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].value(), TritValue::False);
    }

    #[test]
    fn run_analysis_with_contacts_includes_contact_count() {
        let mut profile = ContactProfile::new(
            "c1".into(),
            "Alice".into(),
            crate::bc::relationship_annotation::RelationLabel::Friend,
        );
        profile.annotate_frame(
            crate::bc::relationship_annotation::FrameAnnotation::new(
                "Science".into(),
                "test".into(),
                0.7,
            )
            .unwrap(),
        );

        let contact_signals = contacts_to_tritwords(&[profile]);
        assert_eq!(contact_signals.len(), 1);

        let spec = SignalSpec {
            freq: 2.5,
            sample_rate: 100.0,
            duration_secs: 1.0,
            noise_std: 0.0,
        };
        let report = run_analysis(&spec, 1.0, true, &contact_signals).unwrap();
        assert_eq!(report.contact_count, 1);
    }

    #[test]
    fn run_analysis_without_contacts_has_zero_contact_count() {
        let spec = SignalSpec {
            freq: 2.5,
            sample_rate: 100.0,
            duration_secs: 1.0,
            noise_std: 0.0,
        };
        let report = run_analysis(&spec, 1.0, true, &[]).unwrap();
        assert_eq!(report.contact_count, 0);
    }

    // ── PhaseTrajectory ──────────────────────────────────────────────────

    #[test]
    fn trajectory_starts_with_single_run() {
        let spec = SignalSpec {
            freq: 2.5,
            sample_rate: 100.0,
            duration_secs: 1.0,
            noise_std: 0.0,
        };
        let report = run_analysis(&spec, 1.0, true, &[]).unwrap();
        let traj = PhaseTrajectory::new(&report);
        assert_eq!(traj.runs, 1);
        assert!(!traj.embodied.trend_reliable()); // only 1 observation
    }

    #[test]
    fn trajectory_tracks_trend_across_runs() {
        let spec_high = SignalSpec {
            freq: 10.0, // ratio=10 → phase ~1.0
            sample_rate: 200.0,
            duration_secs: 1.0,
            noise_std: 0.0,
        };
        let spec_low = SignalSpec {
            freq: 0.5, // ratio=0.5 → phase ~0.007
            sample_rate: 100.0,
            duration_secs: 1.0,
            noise_std: 0.0,
        };

        // Start with high frequency
        let report1 = run_analysis(&spec_high, 1.0, true, &[]).unwrap();
        let mut traj = PhaseTrajectory::new(&report1);

        // Feed a low frequency — embodied phase drops sharply
        let report2 = run_analysis(&spec_low, 1.0, true, &[]).unwrap();
        traj.update(&report2);

        // After a big drop, trend should be TowardFalse
        assert_eq!(traj.embodied.trend(), Trend::TowardFalse);
        assert!(traj.embodied.velocity() < 0.0);
    }

    #[test]
    fn trajectory_summary_returns_all_fields() {
        let spec = SignalSpec {
            freq: 2.5,
            sample_rate: 100.0,
            duration_secs: 1.0,
            noise_std: 0.0,
        };
        let report = run_analysis(&spec, 1.0, true, &[]).unwrap();
        let traj = PhaseTrajectory::new(&report);
        let summary = traj.summary();
        assert_eq!(summary.runs, 1);
        assert!(!summary.trend_reliable);
    }
}
