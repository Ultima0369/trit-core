//! Coupling adapter — external system tuning and signal integrity.
//!
//! Acts as the interface gate between trit-core and external systems.
//! In MVP, this is a signal integrity validator: it checks that all
//! input signals have valid frame-value combinations and flags
//! out-of-distribution values.

use crate::adapters::{CognitiveModule, ModuleInput, ModuleOutput};
use crate::core::{Frame, TritValue};
use crate::hook::module_registry::{ModuleId, ModuleState};
use crate::hook::HookContext;
use crate::meta::{ConflictType, MetaInterrupt};

/// Cognitive module for coupling with external systems.
pub struct CouplingAdapter {
    state: ModuleState,
    /// How many integrity violations have been detected.
    violation_count: usize,
}

impl CouplingAdapter {
    /// Create a new coupling adapter module.
    pub fn new() -> Self {
        CouplingAdapter {
            state: ModuleState::Idle,
            violation_count: 0,
        }
    }

    /// Number of integrity violations detected.
    pub fn violation_count(&self) -> usize {
        self.violation_count
    }
}

impl Default for CouplingAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl CognitiveModule for CouplingAdapter {
    fn id(&self) -> ModuleId {
        ModuleId::CouplingAdapter
    }

    fn name(&self) -> &'static str {
        "coupling_adapter"
    }

    fn process(&mut self, input: &ModuleInput, _ctx: &HookContext) -> ModuleOutput {
        self.state = ModuleState::Processing;

        if input.signals.is_empty() {
            self.state = ModuleState::Completed;
            return ModuleOutput::new(
                TritValue::Hold,
                0.5,
                "coupling: no signals — nothing to validate",
            );
        }

        let mut interrupts = Vec::new();
        let mut violations: Vec<String> = Vec::new();

        for signal in &input.signals {
            // Violation 1: Unknown values — out of distribution.
            if signal.value() == TritValue::Unknown {
                violations.push(format!(
                    "Unknown value in {:?} frame — out of distribution, cannot couple",
                    signal.frame()
                ));
            }

            // Violation 2: Absolute frame must remain Hold per invariants.
            if signal.frame() == Frame::Absolute && signal.value() != TritValue::Hold {
                violations.push(
                    "Absolute frame with non-Hold value — violates system invariant".to_string(),
                );
            }

            // Violation 3: Meta frame in external input — Meta is system-internal only.
            if signal.frame() == Frame::Meta {
                violations.push(
                    "Meta frame in external signal — Meta is system-internal, not valid for external input".to_string(),
                );
            }
        }

        // Build interrupts.
        for violation in &violations {
            interrupts.push(MetaInterrupt::new(
                ConflictType::OutOfScope,
                format!("coupling: {}", violation),
            ));
        }

        self.state = ModuleState::Completed;

        if violations.is_empty() {
            ModuleOutput::new(
                TritValue::True,
                0.9,
                format!(
                    "coupling: {} signal(s) validated — all frame-value combinations valid",
                    input.signals.len()
                ),
            )
        } else {
            self.violation_count += violations.len();
            ModuleOutput::new(
                TritValue::Hold,
                0.3,
                format!(
                    "coupling: {} integrity violation(s) — {}",
                    violations.len(),
                    violations.join("; ")
                ),
            )
            .with_interrupts(interrupts)
        }
    }

    fn on_mount(&mut self) {
        self.state = ModuleState::Idle;
    }

    fn on_unmount(&mut self) {
        self.violation_count = 0;
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

    fn word(frame: Frame, value: TritValue) -> crate::core::TritWord {
        crate::core::TritWord::new(value, Phase::neutral(), frame)
    }

    #[test]
    fn empty_input_hold() {
        let mut m = CouplingAdapter::new();
        let input = ModuleInput {
            signals: vec![],
            interrupts: vec![],
            attention_cmd: None,
        };
        let out = m.process(&input, &HookContext::default());
        assert_eq!(out.recommendation, TritValue::Hold);
    }

    #[test]
    fn valid_signals_pass() {
        let mut m = CouplingAdapter::new();
        let input = ModuleInput {
            signals: vec![
                word(Frame::Science, TritValue::True),
                word(Frame::Individual, TritValue::False),
            ],
            interrupts: vec![],
            attention_cmd: None,
        };
        let out = m.process(&input, &HookContext::default());
        assert_eq!(out.recommendation, TritValue::True);
        assert!(out.confidence > 0.8);
    }

    #[test]
    fn unknown_value_violation() {
        let mut m = CouplingAdapter::new();
        let input = ModuleInput {
            signals: vec![word(Frame::Science, TritValue::Unknown)],
            interrupts: vec![],
            attention_cmd: None,
        };
        let out = m.process(&input, &HookContext::default());
        assert_eq!(out.recommendation, TritValue::Hold);
        assert!(!out.interrupts.is_empty());
        assert!(out
            .interrupts
            .iter()
            .any(|i| matches!(i.conflict, ConflictType::OutOfScope)));
    }

    #[test]
    fn absolute_frame_hold_is_valid() {
        // Absolute frame with Hold is the only valid state (enforced by TritWord constructor).
        let mut m = CouplingAdapter::new();
        let input = ModuleInput {
            signals: vec![crate::core::TritWord::hold(crate::core::Frame::Absolute)],
            interrupts: vec![],
            attention_cmd: None,
        };
        let out = m.process(&input, &HookContext::default());
        // Hold in Absolute frame is valid — passes validation.
        assert_eq!(out.recommendation, TritValue::True);
    }

    #[test]
    fn meta_frame_in_external_input() {
        let mut m = CouplingAdapter::new();
        let input = ModuleInput {
            signals: vec![word(Frame::Meta, TritValue::True)],
            interrupts: vec![],
            attention_cmd: None,
        };
        let out = m.process(&input, &HookContext::default());
        assert_eq!(out.recommendation, TritValue::Hold);
        assert!(out.trace.contains("Meta"));
    }

    #[test]
    fn violation_count_resets_on_unmount() {
        let mut m = CouplingAdapter::new();
        let input = ModuleInput {
            signals: vec![word(Frame::Science, TritValue::Unknown)],
            interrupts: vec![],
            attention_cmd: None,
        };
        m.process(&input, &HookContext::default());
        assert_eq!(m.violation_count(), 1);

        m.on_unmount();
        assert_eq!(m.violation_count(), 0);
    }

    #[test]
    fn module_id_and_name() {
        let m = CouplingAdapter::new();
        assert_eq!(m.id(), ModuleId::CouplingAdapter);
        assert_eq!(m.name(), "coupling_adapter");
    }
}
