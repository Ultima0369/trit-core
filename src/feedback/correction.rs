//! Correction trigger — threshold-based feedback signal emission.
//!
//! Evaluates practice test results against correction and re-entry
//! thresholds. Builds a [`FeedbackSignal`] when correction is warranted.

use crate::hook::ScenarioType;

use super::{CorrectionSeverity, FeedbackSignal, PracticeTestResult};

/// Stateless correction trigger.
pub struct CorrectionTrigger;

impl CorrectionTrigger {
    /// Evaluate whether a correction signal should be emitted.
    ///
    /// Returns `Some(FeedbackSignal)` if the deviation warrants correction
    /// (delta ≥ correction_threshold), or `None` if the decision is
    /// acceptable as-is.
    pub fn evaluate(
        result: &PracticeTestResult,
        severity: CorrectionSeverity,
        source_decision_id: &str,
        correction_threshold: f64,
        reentry_threshold: f64,
    ) -> Option<FeedbackSignal> {
        let delta = match result {
            PracticeTestResult::Matched { .. } => return None,
            PracticeTestResult::Deviated { delta, .. } => *delta,
            PracticeTestResult::Erroneous { .. } => 1.0,
        };

        // Only emit signal if delta exceeds correction threshold
        if delta < correction_threshold {
            return None;
        }

        let recommended_scenario = if delta >= reentry_threshold {
            Some(ScenarioType::ReflexiveAudit)
        } else {
            None
        };

        Some(FeedbackSignal {
            test_result: result.clone(),
            source_decision_id: source_decision_id.to_string(),
            deviation_delta: delta,
            recommended_scenario,
            anchor_violations: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::value::TritValue;

    #[test]
    fn matched_returns_none() {
        let result = PracticeTestResult::Matched { confidence: 0.9 };
        let signal =
            CorrectionTrigger::evaluate(&result, CorrectionSeverity::Mild, "test", 0.3, 0.5);
        assert!(signal.is_none());
    }

    #[test]
    fn mild_deviation_below_threshold_returns_none() {
        let result = PracticeTestResult::Deviated {
            delta: 0.15,
            correction: super::super::CorrectionHint {
                suggested_value: None,
                suggested_phase: None,
                reason: "test".into(),
            },
        };
        let signal =
            CorrectionTrigger::evaluate(&result, CorrectionSeverity::Mild, "test", 0.3, 0.5);
        assert!(signal.is_none());
    }

    #[test]
    fn moderate_deviation_above_threshold_emits_signal() {
        let result = PracticeTestResult::Deviated {
            delta: 0.35,
            correction: super::super::CorrectionHint {
                suggested_value: Some(TritValue::Hold),
                suggested_phase: None,
                reason: "test".into(),
            },
        };
        let signal = CorrectionTrigger::evaluate(
            &result,
            CorrectionSeverity::Moderate,
            "scenario_1",
            0.3,
            0.5,
        );
        assert!(signal.is_some());
        let s = signal.unwrap();
        assert_eq!(s.source_decision_id, "scenario_1");
        assert_float_eq!(s.deviation_delta, 0.35);
        assert!(s.recommended_scenario.is_none()); // below reentry threshold
    }

    #[test]
    fn severe_deviation_recommends_reflexive_audit() {
        let result = PracticeTestResult::Deviated {
            delta: 0.7,
            correction: super::super::CorrectionHint {
                suggested_value: Some(TritValue::Hold),
                suggested_phase: Some(0.5),
                reason: "test".into(),
            },
        };
        let signal = CorrectionTrigger::evaluate(
            &result,
            CorrectionSeverity::Severe,
            "test",
            0.3,
            0.5,
        );
        assert!(signal.is_some());
        let s = signal.unwrap();
        assert_eq!(
            s.recommended_scenario,
            Some(ScenarioType::ReflexiveAudit)
        );
    }

    #[test]
    fn erroneous_always_emits_signal() {
        let result = PracticeTestResult::Erroneous {
            reason: "critical failure".into(),
            severity: CorrectionSeverity::Severe,
        };
        let signal = CorrectionTrigger::evaluate(
            &result,
            CorrectionSeverity::Severe,
            "test",
            0.3,
            0.5,
        );
        assert!(signal.is_some());
        assert_float_eq!(signal.unwrap().deviation_delta, 1.0);
    }

    #[test]
    fn custom_thresholds_are_respected() {
        let result = PracticeTestResult::Deviated {
            delta: 0.4,
            correction: super::super::CorrectionHint {
                suggested_value: None,
                suggested_phase: None,
                reason: "test".into(),
            },
        };
        // With correction_threshold=0.5, delta=0.4 should NOT trigger
        let signal = CorrectionTrigger::evaluate(
            &result,
            CorrectionSeverity::Moderate,
            "test",
            0.5,
            0.7,
        );
        assert!(signal.is_none());
    }
}
