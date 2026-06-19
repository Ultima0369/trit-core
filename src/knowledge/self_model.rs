//! Self-knowledge: "knowing oneself" as a precondition for knowing others.
//!
//! This module provides [`SelfKnowledge`], a minimal model of the system's
//! own response patterns. By comparing an incoming signal against known
//! patterns, the system can estimate the likely state of a receiver that
//! shares similar cognitive architecture.

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

    /// Infer a receiver's likely state from an input signal.
    ///
    /// The heuristic is: "If I had this same input, what would I do?"
    /// The estimate starts from the matching self-pattern, then shifts
    /// toward the input's actual phase/value proportionally to confidence.
    pub fn infer_receiver_state(&self, input: &TritWord) -> ReceiverEstimate {
        let matching = self
            .own_response_patterns
            .iter()
            .find(|p| p.frame == input.frame());

        let confidence = if matching.is_some() { 0.6 } else { 0.2 };

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
            confidence,
            attended_frames,
        }
    }

    /// Record a calibration event.
    pub fn calibrate(&mut self, pattern: ResponsePattern, phase_delta: f64, reason: String) {
        self.calibration_history.push(CalibrationEvent {
            pattern,
            phase_delta,
            reason,
        });
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
}
