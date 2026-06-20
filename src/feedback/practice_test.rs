//! Practice test — compare decision output against proxy prediction.
//!
//! Computes the deviation Δ between a decision and its predicted consequence.
//! Δ = w_v · δ_v + w_p · δ_p, where δ_v is value mismatch (0 or 1) and
//! δ_p is phase difference.

use crate::core::value::TritValue;
use crate::sandbox::SandboxOutput;

use super::{ConsequencePrediction, CorrectionHint, PracticeTestResult};

/// Weight for value mismatch in deviation computation.
const VALUE_WEIGHT: f64 = 0.6;

/// Weight for phase difference in deviation computation.
const PHASE_WEIGHT: f64 = 0.4;

/// Tolerance for considering a match "close enough."
const MATCH_TOLERANCE: f64 = 0.15;

/// Stateless practice test comparator.
pub struct PracticeTest;

impl PracticeTest {
    /// Compare a decision output against a proxy prediction.
    ///
    /// Returns `Matched` if Δ < tolerance, `Deviated` with correction hint
    /// if Δ ≥ tolerance, or `Erroneous` if the deviation is extreme (Δ > 0.8).
    pub fn compare(
        decision: &SandboxOutput,
        prediction: &ConsequencePrediction,
    ) -> PracticeTestResult {
        let decision_value = TritValue::from(decision.final_value_code);

        // Value mismatch: 1.0 if values differ, 0.0 if same
        let delta_v = if decision_value != prediction.expected_value {
            1.0
        } else {
            0.0
        };

        // Phase difference: absolute difference
        let delta_p = (decision.final_phase_raw - prediction.expected_phase).abs();

        // Weighted deviation
        let delta = VALUE_WEIGHT * delta_v + PHASE_WEIGHT * delta_p;

        if delta < MATCH_TOLERANCE {
            PracticeTestResult::Matched {
                confidence: prediction.confidence,
            }
        } else if delta > 0.8 {
            PracticeTestResult::Erroneous {
                reason: format!(
                    "Extreme deviation Δ={:.3}: decision={:?}/{} vs expected={:?}/{}",
                    delta,
                    decision_value,
                    decision.final_phase_raw,
                    prediction.expected_value,
                    prediction.expected_phase
                ),
                severity: super::CorrectionSeverity::Severe,
            }
        } else {
            let correction = CorrectionHint {
                suggested_value: if delta_v > 0.0 {
                    Some(prediction.expected_value)
                } else {
                    None
                },
                suggested_phase: if delta_p > 0.1 {
                    Some(prediction.expected_phase)
                } else {
                    None
                },
                reason: format!(
                    "Deviation Δ={:.3}: δ_v={:.3}, δ_p={:.3}",
                    delta, delta_v, delta_p
                ),
            };
            PracticeTestResult::Deviated { delta, correction }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output(value_code: i8, phase: f64) -> SandboxOutput {
        SandboxOutput {
            scenario_id: "test".into(),
            final_value: match value_code {
                1 => "True".into(),
                0 => "Hold".into(),
                -1 => "False".into(),
                _ => "Unknown".into(),
            },
            final_value_code: value_code,
            final_frame: "Science".into(),
            final_phase_raw: phase,
            interrupts: vec![],
            policy_action: "Commit".into(),
            reflexive_alert: None,
            attention_cmd: None,
            receiver_estimate: None,
            hold_state: None,
        }
    }

    fn prediction(value: TritValue, phase: f64) -> ConsequencePrediction {
        ConsequencePrediction {
            expected_value: value,
            expected_phase: phase,
            confidence: 0.6,
            reasoning: "test".into(),
        }
    }

    #[test]
    fn exact_match_is_matched() {
        let out = output(1, 0.9);
        let pred = prediction(TritValue::True, 0.9);
        let result = PracticeTest::compare(&out, &pred);
        assert!(matches!(result, PracticeTestResult::Matched { .. }));
    }

    #[test]
    fn small_phase_diff_is_matched() {
        let out = output(1, 0.85);
        let pred = prediction(TritValue::True, 0.9);
        let result = PracticeTest::compare(&out, &pred);
        // Δ = 0.6*0 + 0.4*0.05 = 0.02 < 0.15 → Matched
        assert!(matches!(result, PracticeTestResult::Matched { .. }));
    }

    #[test]
    fn value_mismatch_is_deviated() {
        let out = output(1, 0.9);
        let pred = prediction(TritValue::False, 0.9);
        let result = PracticeTest::compare(&out, &pred);
        // Δ = 0.6*1 + 0.4*0 = 0.6 → Deviated
        assert!(matches!(result, PracticeTestResult::Deviated { .. }));
    }

    #[test]
    fn large_phase_diff_is_deviated() {
        let out = output(1, 0.9);
        let pred = prediction(TritValue::True, 0.1);
        let result = PracticeTest::compare(&out, &pred);
        // Δ = 0.6*0 + 0.4*0.8 = 0.32 → Deviated
        assert!(matches!(result, PracticeTestResult::Deviated { .. }));
    }

    #[test]
    fn extreme_deviation_is_erroneous() {
        let out = output(1, 1.0);
        let pred = prediction(TritValue::False, 0.0);
        let result = PracticeTest::compare(&out, &pred);
        // Δ = 0.6*1 + 0.4*1.0 = 1.0 > 0.8 → Erroneous
        assert!(matches!(result, PracticeTestResult::Erroneous { .. }));
    }

    #[test]
    fn deviated_includes_correction_hint() {
        let out = output(1, 0.9);
        let pred = prediction(TritValue::False, 0.9);
        let result = PracticeTest::compare(&out, &pred);
        match result {
            PracticeTestResult::Deviated { correction, .. } => {
                assert_eq!(correction.suggested_value, Some(TritValue::False));
                assert!(correction.suggested_phase.is_none());
            }
            _ => panic!("expected Deviated"),
        }
    }

    #[test]
    fn phase_only_deviation_suggests_phase_correction() {
        let out = output(1, 0.9);
        let pred = prediction(TritValue::True, 0.3);
        let result = PracticeTest::compare(&out, &pred);
        match result {
            PracticeTestResult::Deviated { correction, .. } => {
                assert_eq!(correction.suggested_value, None);
                assert!(correction.suggested_phase.is_some());
            }
            _ => panic!("expected Deviated"),
        }
    }
}
