//! Self-knowledge: "knowing oneself" as a precondition for knowing others.
//!
//! This module provides [`SelfKnowledge`], a minimal model of the system's
//! own response patterns. By comparing an incoming signal against known
//! patterns, the system can estimate the likely state of a receiver that
//! shares similar cognitive architecture.
//!
//! ## Feedback loop (v0.3.0)
//!
//! `calibrate()` is now called after each pipeline run. Clean decisions
//! (no interrupts, computable result) strengthen matching patterns with
//! a positive phase delta. Conflicted decisions (interrupts + Hold) weaken
//! patterns with a negative delta. The confidence ceiling in
//! `infer_receiver_state()` grows with the calibration count, from a
//! floor of 0.2 up to a ceiling of 0.95.

use serde::{Deserialize, Serialize};

use crate::core::frame::Frame;
use crate::core::phase::Phase;
use crate::core::value::TritValue;
use crate::core::word::TritWord;

/// A recorded response pattern: "when I see X under conditions C, I tend Y".
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ResponsePattern {
    /// Frame of the input that triggers this pattern.
    pub frame: Frame,
    /// Value tendency associated with this pattern.
    pub value: TritValue,
    /// Phase tendency.
    pub phase: f64,
    /// Context label (e.g. "stress", "rest", "conflict").
    pub context: String,
}

/// A trigger signature: a compact description of a stimulus that reliably
/// evokes a known response.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct TriggerSignature {
    /// Frame involved in the trigger.
    pub frame: Frame,
    /// Keywords or labels associated with the trigger.
    pub labels: Vec<String>,
    /// Expected phase response.
    pub phase_response: f64,
}

/// A calibration event recording how a pattern was updated.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CalibrationEvent {
    /// Pattern that was updated.
    pub pattern: ResponsePattern,
    /// Signed delta applied to the phase.
    pub phase_delta: f64,
    /// Human-readable reason.
    pub reason: String,
}

/// Estimate of a receiver's likely cognitive state.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ReceiverEstimate {
    /// Estimated trit value the receiver would produce.
    pub estimated_value: TritValue,
    /// Estimated phase of the receiver.
    pub estimated_phase: f64,
    /// Confidence of the estimate in `[0.0, 1.0]`.
    pub confidence: f64,
    /// Frames the receiver is likely attending to.
    pub attended_frames: Vec<Frame>,
}

/// Self-knowledge model containing the system's own patterns and triggers.
///
/// ## Confidence scaling
///
/// The confidence ceiling in `infer_receiver_state()` scales with the
/// number of calibration events recorded:
///
/// | Calibrations | Confidence ceiling |
/// |-------------|-------------------|
/// | 0           | 0.6               |
/// | 1–9         | 0.7               |
/// | 10–49       | 0.8               |
/// | 50–99       | 0.9               |
/// | 100+        | 0.95              |
///
/// The floor is always 0.2 (when no matching pattern is found).
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct SelfKnowledge {
    /// Known response patterns.
    pub own_response_patterns: Vec<ResponsePattern>,
    /// Known trigger signatures.
    pub known_triggers: Vec<TriggerSignature>,
    /// History of calibrations.
    pub calibration_history: Vec<CalibrationEvent>,
}

impl SelfKnowledge {
    /// Create an empty self-knowledge model.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a model seeded with common human-like response patterns.
    pub fn with_human_defaults() -> Self {
        Self {
            own_response_patterns: vec![
                ResponsePattern {
                    frame: Frame::Embodied,
                    value: TritValue::Hold,
                    phase: 0.5,
                    context: "high_arousal".to_string(),
                },
                ResponsePattern {
                    frame: Frame::FirstPerson,
                    value: TritValue::True,
                    phase: 0.7,
                    context: "autonomy".to_string(),
                },
                ResponsePattern {
                    frame: Frame::Relational,
                    value: TritValue::Hold,
                    phase: 0.5,
                    context: "trust_uncertainty".to_string(),
                },
            ],
            known_triggers: vec![
                TriggerSignature {
                    frame: Frame::Embodied,
                    labels: vec!["heart_rate".to_string(), "gsr".to_string()],
                    phase_response: 0.8,
                },
                TriggerSignature {
                    frame: Frame::Relational,
                    labels: vec!["betrayal".to_string(), "trust".to_string()],
                    phase_response: 0.4,
                },
            ],
            calibration_history: vec![],
        }
    }

    /// Add a response pattern.
    pub fn add_pattern(&mut self, pattern: ResponsePattern) {
        self.own_response_patterns.push(pattern);
    }

    /// Add a trigger signature.
    pub fn add_trigger(&mut self, trigger: TriggerSignature) {
        self.known_triggers.push(trigger);
    }

    /// Return the confidence ceiling based on calibration count.
    ///
    /// More calibrations → higher confidence that the model's patterns
    /// are well-tuned to actual decision outcomes.
    pub fn confidence_ceiling(&self) -> f64 {
        let n = self.calibration_history.len();
        if n >= 100 {
            0.95
        } else if n >= 50 {
            0.9
        } else if n >= 10 {
            0.8
        } else if n >= 1 {
            0.7
        } else {
            0.6
        }
    }

    /// Infer a receiver's likely state from an input signal.
    ///
    /// The heuristic is: "If I had this same input, what would I do?"
    /// The estimate starts from the matching self-pattern, then shifts
    /// toward the input's actual phase/value proportionally to confidence.
    ///
    /// Confidence is clamped to the calibration-derived ceiling, so a
    /// well-calibrated model can express higher confidence in its estimates.
    pub fn infer_receiver_state(&self, input: &TritWord) -> ReceiverEstimate {
        let matching = self
            .own_response_patterns
            .iter()
            .find(|p| p.frame == input.frame());

        let ceiling = self.confidence_ceiling();
        let raw_confidence = if matching.is_some() { ceiling } else { 0.2 };

        let estimated_value = matching.map(|p| p.value).unwrap_or_else(|| input.value());

        let estimated_phase = matching
            .map(|p| (p.phase + input.phase().inner()) / 2.0)
            .unwrap_or_else(|| input.phase().inner());

        let attended_frames = vec![input.frame()];

        ReceiverEstimate {
            estimated_value,
            estimated_phase: Phase::new(estimated_phase)
                .unwrap_or(Phase::neutral())
                .inner(),
            confidence: raw_confidence,
            attended_frames,
        }
    }

    /// Record a calibration event.
    ///
    /// The `phase_delta` is applied to the matching pattern's phase tendency:
    /// - Positive delta (+0.05): the pattern produced a clean result → strengthen.
    /// - Negative delta (-0.05): the pattern produced conflict → weaken.
    pub fn calibrate(&mut self, pattern: ResponsePattern, phase_delta: f64, reason: String) {
        // Apply the delta to the matching pattern in our store.
        if let Some(existing) = self
            .own_response_patterns
            .iter_mut()
            .find(|p| p.frame == pattern.frame && p.context == pattern.context)
        {
            existing.phase = (existing.phase + phase_delta).clamp(0.0, 1.0);
        }

        self.calibration_history.push(CalibrationEvent {
            pattern,
            phase_delta,
            reason,
        });
    }

    /// Calibrate from a pipeline result.
    ///
    /// This is the primary feedback entry point:
    /// - If the result was a clean Commit (no interrupts, computable value),
    ///   the matching pattern is strengthened with a positive delta.
    /// - If the result was Hold with interrupts, the matching pattern is
    ///   weakened with a negative delta.
    /// - Otherwise, no calibration is performed.
    ///
    /// Returns `true` if a calibration event was recorded.
    pub fn calibrate_from_result(
        &mut self,
        frame: Frame,
        result: TritValue,
        phase: f64,
        interrupt_count: usize,
    ) -> bool {
        // Only calibrate on clear signals: clean commit or conflicted hold.
        if result.is_computable() && result != TritValue::Hold && interrupt_count == 0 {
            // Clean decision: strengthen the pattern.
            let pattern = ResponsePattern {
                frame,
                value: result,
                phase,
                context: "calibrated".to_string(),
            };
            self.calibrate(
                pattern,
                0.05,
                format!(
                    "clean {:?} decision with phase {:.3}, no interrupts",
                    result, phase
                ),
            );
            true
        } else if result == TritValue::Hold && interrupt_count > 0 {
            // Conflicted decision: weaken the pattern.
            let pattern = ResponsePattern {
                frame,
                value: TritValue::Hold,
                phase,
                context: "calibrated".to_string(),
            };
            self.calibrate(
                pattern,
                -0.05,
                format!(
                    "conflicted Hold with {} interrupts, phase {:.3}",
                    interrupt_count, phase
                ),
            );
            true
        } else {
            false
        }
    }

    /// Number of calibration events recorded.
    pub fn calibration_count(&self) -> usize {
        self.calibration_history.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_defaults_have_patterns() {
        let knowledge = SelfKnowledge::with_human_defaults();
        assert!(!knowledge.own_response_patterns.is_empty());
        assert!(!knowledge.known_triggers.is_empty());
    }

    #[test]
    fn infer_receiver_from_first_person() {
        let knowledge = SelfKnowledge::with_human_defaults();
        let input = TritWord::tru(Frame::FirstPerson);
        let estimate = knowledge.infer_receiver_state(&input);
        assert_eq!(estimate.estimated_value, TritValue::True);
        assert!(estimate.confidence > 0.5);
        assert!(estimate.attended_frames.contains(&Frame::FirstPerson));
    }

    #[test]
    fn infer_receiver_without_pattern_uses_input() {
        let knowledge = SelfKnowledge::new();
        let input = TritWord::fals(Frame::Science);
        let estimate = knowledge.infer_receiver_state(&input);
        assert_eq!(estimate.estimated_value, TritValue::False);
        assert!(estimate.confidence < 0.5);
    }

    // ── confidence ceiling ────────────────────────────────────────

    #[test]
    fn confidence_ceiling_zero_calibrations() {
        let knowledge = SelfKnowledge::new();
        assert_float_eq!(knowledge.confidence_ceiling(), 0.6);
    }

    #[test]
    fn confidence_ceiling_grows_with_calibrations() {
        let mut knowledge = SelfKnowledge::new();
        // 0 → 0.6
        assert_float_eq!(knowledge.confidence_ceiling(), 0.6);

        // 1 calibration → 0.7
        knowledge.calibrate(
            ResponsePattern {
                frame: Frame::Science,
                value: TritValue::True,
                phase: 0.5,
                context: "test".to_string(),
            },
            0.05,
            "test".to_string(),
        );
        assert_float_eq!(knowledge.confidence_ceiling(), 0.7);

        // 10 calibrations → 0.8
        for i in 0..9 {
            knowledge.calibrate(
                ResponsePattern {
                    frame: Frame::Science,
                    value: TritValue::True,
                    phase: 0.5,
                    context: format!("test{}", i),
                },
                0.05,
                "bulk".to_string(),
            );
        }
        assert_float_eq!(knowledge.confidence_ceiling(), 0.8);
    }

    #[test]
    fn confidence_ceiling_max_is_095() {
        let mut knowledge = SelfKnowledge::new();
        for i in 0..100 {
            knowledge.calibrate(
                ResponsePattern {
                    frame: Frame::Science,
                    value: TritValue::True,
                    phase: 0.5,
                    context: format!("bulk{}", i),
                },
                0.01,
                "bulk".to_string(),
            );
        }
        assert_float_eq!(knowledge.confidence_ceiling(), 0.95);
    }

    // ── calibrate_from_result ─────────────────────────────────────

    #[test]
    fn calibrate_from_clean_result_strengthens() {
        let mut knowledge = SelfKnowledge::new();
        knowledge.add_pattern(ResponsePattern {
            frame: Frame::Science,
            value: TritValue::True,
            phase: 0.5,
            context: "calibrated".to_string(),
        });

        let did_calibrate = knowledge.calibrate_from_result(
            Frame::Science,
            TritValue::True,
            0.8,
            0, // no interrupts
        );
        assert!(did_calibrate);
        assert_eq!(knowledge.calibration_count(), 1);

        // The matching pattern should have its phase increased by 0.05
        let pattern = knowledge
            .own_response_patterns
            .iter()
            .find(|p| p.frame == Frame::Science && p.context == "calibrated")
            .unwrap();
        assert_float_eq!(pattern.phase, 0.55);
    }

    #[test]
    fn calibrate_from_conflicted_result_weakens() {
        let mut knowledge = SelfKnowledge::new();
        knowledge.add_pattern(ResponsePattern {
            frame: Frame::Individual,
            value: TritValue::Hold,
            phase: 0.5,
            context: "calibrated".to_string(),
        });

        let did_calibrate = knowledge.calibrate_from_result(
            Frame::Individual,
            TritValue::Hold,
            0.3,
            3, // 3 interrupts
        );
        assert!(did_calibrate);
        assert_eq!(knowledge.calibration_count(), 1);

        // The matching pattern should have its phase decreased by 0.05
        let pattern = knowledge
            .own_response_patterns
            .iter()
            .find(|p| p.frame == Frame::Individual && p.context == "calibrated")
            .unwrap();
        assert_float_eq!(pattern.phase, 0.45);
    }

    #[test]
    fn calibrate_from_neutral_result_skips() {
        let mut knowledge = SelfKnowledge::new();
        // Hold with 0 interrupts is neither clean nor conflicted
        let did_calibrate = knowledge.calibrate_from_result(
            Frame::Science,
            TritValue::Hold,
            0.5,
            0, // Hold but no interrupts → skip
        );
        assert!(!did_calibrate);
        assert_eq!(knowledge.calibration_count(), 0);
    }

    #[test]
    fn calibrate_from_unknown_skips() {
        let mut knowledge = SelfKnowledge::new();
        // Unknown is not computable and not Hold → skip
        let did_calibrate =
            knowledge.calibrate_from_result(Frame::Science, TritValue::Unknown, 0.5, 0);
        assert!(!did_calibrate);
        assert_eq!(knowledge.calibration_count(), 0);
    }

    #[test]
    fn calibrate_phase_clamped_to_range() {
        let mut knowledge = SelfKnowledge::new();
        knowledge.add_pattern(ResponsePattern {
            frame: Frame::Science,
            value: TritValue::True,
            phase: 0.98,
            context: "calibrated".to_string(),
        });

        // +0.05 would push to 1.03, should clamp to 1.0
        knowledge.calibrate_from_result(Frame::Science, TritValue::True, 0.98, 0);
        let pattern = knowledge
            .own_response_patterns
            .iter()
            .find(|p| p.frame == Frame::Science && p.context == "calibrated")
            .unwrap();
        assert_float_eq!(pattern.phase, 1.0);
    }

    #[test]
    fn infer_confidence_scales_with_calibrations() {
        let mut knowledge = SelfKnowledge::with_human_defaults();
        // 0 calibrations → ceiling 0.6
        let est0 = knowledge.infer_receiver_state(&TritWord::tru(Frame::FirstPerson));
        assert_float_eq!(est0.confidence, 0.6);

        // Add 10 calibrations → ceiling 0.8
        for i in 0..10 {
            knowledge.calibrate(
                ResponsePattern {
                    frame: Frame::FirstPerson,
                    value: TritValue::True,
                    phase: 0.5,
                    context: format!("bulk{}", i),
                },
                0.01,
                "bulk".to_string(),
            );
        }
        let est10 = knowledge.infer_receiver_state(&TritWord::tru(Frame::FirstPerson));
        assert_float_eq!(est10.confidence, 0.8);
    }
}
