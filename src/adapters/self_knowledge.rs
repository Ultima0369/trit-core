//! Self-knowledge adapter — model of the system's own response patterns.
//!
//! By comparing an incoming signal against known patterns, the system
//! can estimate the likely state of a receiver that shares similar
//! cognitive architecture. The feedback loop calibrates these patterns
//! over time.

use serde::{Deserialize, Serialize};

use crate::adapters::{
    adapter_lifecycle, CognitiveModule, FeedbackSignal, ModuleInput, ModuleOutput,
};
use crate::core::frame::Frame;
use crate::core::phase::Phase;
use crate::core::value::TritValue;
use crate::core::word::TritWord;
use crate::hook::module_registry::{ModuleId, ModuleState};
use crate::hook::HookContext;

// ── Response pattern ────────────────────────────────────────────────

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

// ── Trigger signature ───────────────────────────────────────────────

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

// ── Calibration event ───────────────────────────────────────────────

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

// ── Receiver estimate ───────────────────────────────────────────────

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

// ── Self-knowledge (inner engine) ───────────────────────────────────

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
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct SelfKnowledge {
    /// Known response patterns.
    pub own_response_patterns: Vec<ResponsePattern>,
    /// Known trigger signatures.
    pub known_triggers: Vec<TriggerSignature>,
    /// History of calibrations.
    pub calibration_history: Vec<CalibrationEvent>,
    /// Lifecycle state for CognitiveModule.
    #[serde(skip, default)]
    state: ModuleState,
}

impl SelfKnowledge {
    /// Create an empty self-knowledge model.
    pub fn new() -> Self {
        Self {
            state: ModuleState::Idle,
            ..Default::default()
        }
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
            state: ModuleState::Idle,
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
    pub fn calibrate_pattern(
        &mut self,
        pattern: ResponsePattern,
        phase_delta: f64,
        reason: String,
    ) {
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
    /// - Clean Commit (no interrupts, computable value) → strengthen (+0.05)
    /// - Hold with interrupts → weaken (-0.05)
    /// - Otherwise → skip
    ///
    /// Returns `true` if a calibration event was recorded.
    pub fn calibrate_from_result(
        &mut self,
        frame: Frame,
        result: TritValue,
        phase: f64,
        interrupt_count: usize,
    ) -> bool {
        if result.is_computable() && result != TritValue::Hold && interrupt_count == 0 {
            self.calibrate_clean(frame, result, phase);
            true
        } else if result == TritValue::Hold && interrupt_count > 0 {
            self.calibrate_conflicted(frame, phase, interrupt_count);
            true
        } else {
            false
        }
    }

    fn calibrate_clean(&mut self, frame: Frame, result: TritValue, phase: f64) {
        self.calibrate_pattern(
            ResponsePattern {
                frame,
                value: result,
                phase,
                context: "calibrated".into(),
            },
            0.05,
            format!(
                "clean {:?} decision with phase {:.3}, no interrupts",
                result, phase
            ),
        );
    }

    fn calibrate_conflicted(&mut self, frame: Frame, phase: f64, interrupt_count: usize) {
        self.calibrate_pattern(
            ResponsePattern {
                frame,
                value: TritValue::Hold,
                phase,
                context: "calibrated".into(),
            },
            -0.05,
            format!(
                "conflicted Hold with {} interrupts, phase {:.3}",
                interrupt_count, phase
            ),
        );
    }

    /// Number of calibration events recorded.
    pub fn calibration_count(&self) -> usize {
        self.calibration_history.len()
    }
}

impl CognitiveModule for SelfKnowledge {
    adapter_lifecycle!();

    fn id(&self) -> ModuleId {
        ModuleId::SelfKnowledge
    }

    fn name(&self) -> &'static str {
        "self_knowledge"
    }

    fn process(&mut self, input: &ModuleInput, ctx: &HookContext) -> ModuleOutput {
        self.state = ModuleState::Processing;

        if input.signals.is_empty() {
            self.state = ModuleState::Completed;
            return ModuleOutput::new(
                TritValue::Hold,
                0.3,
                "self_knowledge: no signals to infer from",
            );
        }

        // Infer receiver state for each input signal and aggregate.
        let estimates: Vec<ReceiverEstimate> = input
            .signals
            .iter()
            .map(|s| self.infer_receiver_state(s))
            .collect();

        let avg_confidence: f64 =
            estimates.iter().map(|e| e.confidence).sum::<f64>() / estimates.len() as f64;

        // If we have a previous iteration with a Hold result, calibrate.
        if let Some(ref prev) = ctx.previous_iteration {
            if matches!(prev.arbitration, crate::meta::ArbitrationResult::Hold) {
                // Calibrate from conflicted result for each signal frame.
                for signal in &input.signals {
                    self.calibrate_from_result(
                        signal.frame(),
                        TritValue::Hold,
                        signal.phase().inner(),
                        prev.interrupt_count,
                    );
                }
            }
        }

        // Recommendation: if confidence is high, True; otherwise Hold.
        let (recommendation, confidence) = if avg_confidence >= 0.6 {
            (TritValue::True, avg_confidence)
        } else {
            (TritValue::Hold, avg_confidence.max(0.3))
        };

        self.state = ModuleState::Completed;

        ModuleOutput::new(
            recommendation,
            confidence,
            format!(
                "self_knowledge: inferred from {} signals, avg_confidence={:.3}",
                input.signals.len(),
                avg_confidence
            ),
        )
    }

    // ponytail: lifecycle generated by adapter_lifecycle!()

    fn calibrate(&mut self, feedback: &FeedbackSignal) -> f64 {
        // Use feedback to calibrate patterns.
        let is_matched = matches!(
            feedback.test_result,
            crate::feedback::PracticeTestResult::Matched { .. }
        );

        let deviation = feedback.deviation_delta;

        if is_matched && deviation < 0.01 {
            // Positive feedback: strengthen the most recent pattern.
            if let Some(last_pattern) = self.own_response_patterns.last().cloned() {
                self.calibrate_pattern(last_pattern, 0.03, feedback.source_decision_id.clone());
            }
            0.03
        } else if deviation > 0.0 {
            // Negative feedback: weaken proportionally to deviation.
            let delta = -(deviation * 0.05).min(0.1);
            if let Some(last_pattern) = self.own_response_patterns.last().cloned() {
                self.calibrate_pattern(last_pattern, delta, feedback.source_decision_id.clone());
            }
            delta.abs()
        } else {
            0.0
        }
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
        assert_float_eq!(knowledge.confidence_ceiling(), 0.6);

        knowledge.calibrate_pattern(
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

        for i in 0..9 {
            knowledge.calibrate_pattern(
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
            knowledge.calibrate_pattern(
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

        let did_calibrate =
            knowledge.calibrate_from_result(Frame::Science, TritValue::True, 0.8, 0);
        assert!(did_calibrate);
        assert_eq!(knowledge.calibration_count(), 1);

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

        let did_calibrate =
            knowledge.calibrate_from_result(Frame::Individual, TritValue::Hold, 0.3, 3);
        assert!(did_calibrate);
        assert_eq!(knowledge.calibration_count(), 1);

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
        let did_calibrate =
            knowledge.calibrate_from_result(Frame::Science, TritValue::Hold, 0.5, 0);
        assert!(!did_calibrate);
        assert_eq!(knowledge.calibration_count(), 0);
    }

    #[test]
    fn calibrate_from_unknown_skips() {
        let mut knowledge = SelfKnowledge::new();
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
        let est0 = knowledge.infer_receiver_state(&TritWord::tru(Frame::FirstPerson));
        assert_float_eq!(est0.confidence, 0.6);

        for i in 0..10 {
            knowledge.calibrate_pattern(
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

    // ── CognitiveModule tests ─────────────────────────────────────

    #[test]
    fn self_knowledge_module_id() {
        let module = SelfKnowledge::new();
        assert_eq!(module.id(), ModuleId::SelfKnowledge);
        assert_eq!(module.name(), "self_knowledge");
    }

    #[test]
    fn self_knowledge_module_empty_input() {
        let mut module = SelfKnowledge::with_human_defaults();
        let input = ModuleInput {
            signals: vec![],
            interrupts: vec![],
            attention_cmd: None,
        };
        let ctx = HookContext::default();
        let out = module.process(&input, &ctx);
        assert_eq!(out.recommendation, TritValue::Hold);
        assert!(out.confidence < 0.5);
    }

    #[test]
    fn self_knowledge_module_with_signals() {
        let mut module = SelfKnowledge::with_human_defaults();
        let input = ModuleInput {
            signals: vec![TritWord::tru(Frame::FirstPerson)],
            interrupts: vec![],
            attention_cmd: None,
        };
        let ctx = HookContext::default();
        let out = module.process(&input, &ctx);
        // With human defaults, FirstPerson → True pattern exists → high confidence
        assert_eq!(out.recommendation, TritValue::True);
        assert!(out.confidence > 0.5);
    }

    #[test]
    fn self_knowledge_module_calibrate_from_feedback() {
        let mut module = SelfKnowledge::with_human_defaults();
        let initial_count = module.calibration_count();

        let fb = FeedbackSignal {
            test_result: crate::feedback::PracticeTestResult::Matched { confidence: 0.9 },
            source_decision_id: "validated".into(),
            deviation_delta: 0.0,
            recommended_scenario: None,
            anchor_violations: vec![],
        };
        let delta = module.calibrate(&fb);
        assert!(delta > 0.0);
        assert_eq!(module.calibration_count(), initial_count + 1);
    }

    #[test]
    fn self_knowledge_module_calibrate_negative_feedback() {
        let mut module = SelfKnowledge::with_human_defaults();
        let initial_count = module.calibration_count();

        let fb = FeedbackSignal {
            test_result: crate::feedback::PracticeTestResult::Deviated {
                delta: 0.5,
                correction: crate::feedback::CorrectionHint {
                    suggested_value: None,
                    suggested_phase: None,
                    reason: "test".into(),
                },
            },
            source_decision_id: "deviated".into(),
            deviation_delta: 0.5,
            recommended_scenario: None,
            anchor_violations: vec![],
        };
        let delta = module.calibrate(&fb);
        assert!(delta > 0.0);
        assert_eq!(module.calibration_count(), initial_count + 1);
    }

    #[test]
    fn self_knowledge_module_lifecycle() {
        let mut module = SelfKnowledge::new();
        assert_eq!(module.state(), ModuleState::Idle);

        module.on_unmount();
        assert_eq!(module.state(), ModuleState::Completed);

        module.on_mount();
        assert_eq!(module.state(), ModuleState::Idle);
    }
}
