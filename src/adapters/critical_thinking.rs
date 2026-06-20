//! Critical thinking adapter — logical consistency and counterfactual reasoning.
//!
//! Verifies boundary conditions, detects logical gaps, and generates
//! counterfactuals. When too many frames are in play, confidence drops
//! because the reasoning surface is too large to verify exhaustively.

use crate::adapters::{CognitiveModule, ModuleInput, ModuleOutput};
use crate::core::TritValue;
use crate::hook::module_registry::{ModuleId, ModuleState};
use crate::hook::HookContext;
use crate::meta::{ConflictType, MetaInterrupt, PolicyViolation};

/// Cognitive module for critical thinking and boundary verification.
pub struct CriticalThinking {
    state: ModuleState,
}

impl CriticalThinking {
    /// Create a new critical thinking module.
    pub fn new() -> Self {
        CriticalThinking {
            state: ModuleState::Idle,
        }
    }
}

impl Default for CriticalThinking {
    fn default() -> Self {
        Self::new()
    }
}

impl CognitiveModule for CriticalThinking {
    fn id(&self) -> ModuleId {
        ModuleId::CriticalThinking
    }

    fn name(&self) -> &'static str {
        "critical_thinking"
    }

    fn process(&mut self, input: &ModuleInput, _ctx: &HookContext) -> ModuleOutput {
        self.state = ModuleState::Processing;

        if input.signals.is_empty() {
            self.state = ModuleState::Completed;
            return ModuleOutput::new(TritValue::Hold, 0.3, "critical: no signals to verify");
        }

        let n = input.signals.len();

        // Count distinct frames — too many frames stretches verification.
        let mut frames_seen = std::collections::HashSet::new();
        for signal in &input.signals {
            frames_seen.insert(signal.frame());
        }
        let distinct_frames = frames_seen.len();

        // Detect contradictory pairs: same frame, opposite values, both high phase.
        let mut contradictions = 0usize;
        for i in 0..n {
            for j in (i + 1)..n {
                let a = &input.signals[i];
                let b = &input.signals[j];
                if a.frame() == b.frame()
                    && a.value() != b.value()
                    && a.value().is_computable()
                    && b.value().is_computable()
                    && a.phase().inner() > 0.7
                    && b.phase().inner() > 0.7
                {
                    contradictions += 1;
                }
            }
        }

        // Confidence decays with frame count and contradictions.
        let frame_penalty = if distinct_frames >= 3 { 0.3 } else { 0.0 };
        let contradiction_penalty = (contradictions as f64 * 0.15).min(0.4);
        let confidence = (0.9 - frame_penalty - contradiction_penalty).max(0.2);

        let mut interrupts = Vec::new();

        if contradictions > 0 {
            interrupts.push(MetaInterrupt::policy_violation(
                PolicyViolation::ForcedCollapse,
                format!(
                    "critical: {} contradictory signal pair(s) detected across {} frame(s)",
                    contradictions, distinct_frames
                ),
            ));
        }

        if distinct_frames >= 4 {
            interrupts.push(MetaInterrupt::new(
                ConflictType::FrameMismatch,
                format!(
                    "critical: {} distinct frames — reasoning surface too large to verify exhaustively",
                    distinct_frames
                ),
            ));
        }

        self.state = ModuleState::Completed;

        if !interrupts.is_empty() {
            ModuleOutput::new(
                TritValue::Hold,
                confidence,
                format!(
                    "critical: {} contradiction(s), {} frame(s) — recommend Hold for verification",
                    contradictions, distinct_frames
                ),
            )
            .with_interrupts(interrupts)
        } else if distinct_frames == 1 {
            ModuleOutput::new(
                TritValue::True,
                confidence,
                "critical: single-frame, no contradictions — logically consistent",
            )
        } else {
            ModuleOutput::new(
                TritValue::True,
                confidence,
                format!(
                    "critical: {} frame(s), no contradictions — passes boundary check",
                    distinct_frames
                ),
            )
        }
    }

    fn on_mount(&mut self) {
        self.state = ModuleState::Idle;
    }

    fn on_unmount(&mut self) {
        self.state = ModuleState::Completed;
    }

    fn state(&self) -> ModuleState {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::phase::Phase;
    use crate::core::Frame;

    fn word(frame: Frame, value: TritValue, phase: f64) -> crate::core::TritWord {
        crate::core::TritWord::new(value, Phase::new_clamped(phase), frame)
    }

    #[test]
    fn empty_input_returns_hold() {
        let mut m = CriticalThinking::new();
        let input = ModuleInput {
            signals: vec![],
            interrupts: vec![],
            attention_cmd: None,
        };
        let out = m.process(&input, &HookContext::default());
        assert_eq!(out.recommendation, TritValue::Hold);
    }

    #[test]
    fn single_frame_no_contradictions() {
        let mut m = CriticalThinking::new();
        let input = ModuleInput {
            signals: vec![
                word(Frame::Science, TritValue::True, 0.6),
                word(Frame::Science, TritValue::True, 0.7),
            ],
            interrupts: vec![],
            attention_cmd: None,
        };
        let out = m.process(&input, &HookContext::default());
        assert_eq!(out.recommendation, TritValue::True);
        assert!(out.confidence > 0.7);
    }

    #[test]
    fn contradictory_signals_detected() {
        let mut m = CriticalThinking::new();
        let input = ModuleInput {
            signals: vec![
                word(Frame::Science, TritValue::True, 0.9),
                word(Frame::Science, TritValue::False, 0.85),
            ],
            interrupts: vec![],
            attention_cmd: None,
        };
        let out = m.process(&input, &HookContext::default());
        assert_eq!(out.recommendation, TritValue::Hold);
        assert!(!out.interrupts.is_empty());
    }

    #[test]
    fn many_frames_reduces_confidence() {
        let mut m = CriticalThinking::new();
        let input = ModuleInput {
            signals: vec![
                word(Frame::Science, TritValue::True, 0.5),
                word(Frame::Individual, TritValue::True, 0.5),
                word(Frame::Consensus, TritValue::True, 0.5),
                word(Frame::FirstPerson, TritValue::True, 0.5),
            ],
            interrupts: vec![],
            attention_cmd: None,
        };
        let out = m.process(&input, &HookContext::default());
        // 4 distinct frames should trigger a FrameMismatch interrupt
        assert!(out
            .interrupts
            .iter()
            .any(|i| matches!(i.conflict, ConflictType::FrameMismatch)));
    }

    #[test]
    fn module_id_and_name() {
        let m = CriticalThinking::new();
        assert_eq!(m.id(), ModuleId::CriticalThinking);
        assert_eq!(m.name(), "critical_thinking");
    }

    #[test]
    fn lifecycle() {
        let mut m = CriticalThinking::new();
        assert_eq!(m.state(), ModuleState::Idle);
        m.on_unmount();
        assert_eq!(m.state(), ModuleState::Completed);
        m.on_mount();
        assert_eq!(m.state(), ModuleState::Idle);
    }
}
