//! Consequence review — classify deviation severity from practice test results.
//!
//! Maps the deviation Δ to a [`CorrectionSeverity`] level:
//! - Δ < 0.2 → Mild (record only)
//! - 0.2 ≤ Δ < 0.5 → Moderate (calibrate modules)
//! - Δ ≥ 0.5 → Severe (calibrate + re-enter pipeline)

use super::{CorrectionSeverity, PracticeTestResult};

/// Threshold for Moderate severity.
const MODERATE_THRESHOLD: f64 = 0.2;

/// Threshold for Severe severity.
const SEVERE_THRESHOLD: f64 = 0.5;

/// Stateless consequence review classifier.
pub struct ConsequenceReview;

impl ConsequenceReview {
    /// Classify a practice test result into a severity level.
    ///
    /// Matched results always return Mild. Deviated results are classified
    /// by their delta. Erroneous results always return Severe.
    pub fn classify(result: &PracticeTestResult) -> CorrectionSeverity {
        match result {
            PracticeTestResult::Matched { .. } => CorrectionSeverity::Mild,
            PracticeTestResult::Deviated { delta, .. } => {
                if *delta >= SEVERE_THRESHOLD {
                    CorrectionSeverity::Severe
                } else if *delta >= MODERATE_THRESHOLD {
                    CorrectionSeverity::Moderate
                } else {
                    CorrectionSeverity::Mild
                }
            }
            PracticeTestResult::Erroneous { severity, .. } => *severity,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::value::TritValue;

    #[test]
    fn matched_is_mild() {
        let result = PracticeTestResult::Matched { confidence: 0.9 };
        assert_eq!(ConsequenceReview::classify(&result), CorrectionSeverity::Mild);
    }

    #[test]
    fn small_deviation_is_mild() {
        let result = PracticeTestResult::Deviated {
            delta: 0.1,
            correction: super::super::CorrectionHint {
                suggested_value: None,
                suggested_phase: None,
                reason: "test".into(),
            },
        };
        assert_eq!(ConsequenceReview::classify(&result), CorrectionSeverity::Mild);
    }

    #[test]
    fn moderate_deviation_is_moderate() {
        let result = PracticeTestResult::Deviated {
            delta: 0.35,
            correction: super::super::CorrectionHint {
                suggested_value: None,
                suggested_phase: None,
                reason: "test".into(),
            },
        };
        assert_eq!(
            ConsequenceReview::classify(&result),
            CorrectionSeverity::Moderate
        );
    }

    #[test]
    fn large_deviation_is_severe() {
        let result = PracticeTestResult::Deviated {
            delta: 0.6,
            correction: super::super::CorrectionHint {
                suggested_value: None,
                suggested_phase: None,
                reason: "test".into(),
            },
        };
        assert_eq!(
            ConsequenceReview::classify(&result),
            CorrectionSeverity::Severe
        );
    }

    #[test]
    fn erroneous_is_severe() {
        let result = PracticeTestResult::Erroneous {
            reason: "test".into(),
            severity: CorrectionSeverity::Severe,
        };
        assert_eq!(
            ConsequenceReview::classify(&result),
            CorrectionSeverity::Severe
        );
    }

    #[test]
    fn boundary_at_moderate_threshold() {
        let result = PracticeTestResult::Deviated {
            delta: 0.2,
            correction: super::super::CorrectionHint {
                suggested_value: None,
                suggested_phase: None,
                reason: "test".into(),
            },
        };
        assert_eq!(
            ConsequenceReview::classify(&result),
            CorrectionSeverity::Moderate
        );
    }

    #[test]
    fn boundary_at_severe_threshold() {
        let result = PracticeTestResult::Deviated {
            delta: 0.5,
            correction: super::super::CorrectionHint {
                suggested_value: None,
                suggested_phase: None,
                reason: "test".into(),
            },
        };
        assert_eq!(
            ConsequenceReview::classify(&result),
            CorrectionSeverity::Severe
        );
    }
}
