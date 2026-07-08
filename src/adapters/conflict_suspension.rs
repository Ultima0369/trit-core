//! Conflict suspension adapter — cross-frame conflict detection.
//!
//! Detects when signals from different frames are in opposition and
//! recommends suspending judgment (Hold) rather than forcing a collapse.
//! This is the module that operationalizes "a conflict detected is a
//! conflict preserved, not resolved prematurely."

use crate::adapters::{adapter_lifecycle_no_unmount, CognitiveModule, ModuleInput, ModuleOutput};
use crate::core::interrupt::{ConflictType, MetaInterrupt};
use crate::core::TritValue;
use crate::hook::module_registry::{ModuleId, ModuleState};
use crate::hook::HookContext;

/// Cognitive module for conflict suspension and frame tension detection.
pub struct ConflictSuspension {
    state: ModuleState,
    /// How many conflicts have been suspended in this session.
    suspension_count: usize,
}

impl ConflictSuspension {
    /// Create a new conflict suspension module.
    pub fn new() -> Self {
        ConflictSuspension {
            state: ModuleState::Idle,
            suspension_count: 0,
        }
    }

    /// Number of conflicts suspended.
    pub fn suspension_count(&self) -> usize {
        self.suspension_count
    }
}

impl Default for ConflictSuspension {
    fn default() -> Self {
        Self::new()
    }
}

impl CognitiveModule for ConflictSuspension {
    adapter_lifecycle_no_unmount!();

    fn id(&self) -> ModuleId {
        ModuleId::ConflictSuspension
    }

    fn name(&self) -> &'static str {
        "conflict_suspension"
    }

    fn process(&mut self, input: &ModuleInput, _ctx: &HookContext) -> ModuleOutput {
        self.state = ModuleState::Processing;

        if input.signals.len() < 2 {
            self.state = ModuleState::Completed;
            return ModuleOutput::new(
                TritValue::True,
                0.8,
                "conflict_suspension: single signal — no cross-frame tension possible",
            );
        }

        // Group signals by frame and detect value disagreements across frames.
        let mut frame_values: std::collections::HashMap<crate::core::Frame, Vec<TritValue>> =
            std::collections::HashMap::new();

        for signal in &input.signals {
            frame_values
                .entry(signal.frame())
                .or_default()
                .push(signal.value());
        }

        let frames: Vec<crate::core::Frame> = frame_values.keys().copied().collect();

        // Check for cross-frame opposition: one frame is predominantly True,
        // another is predominantly False.
        let mut conflicts: Vec<String> = Vec::new();
        for i in 0..frames.len() {
            for j in (i + 1)..frames.len() {
                let vals_a = &frame_values[&frames[i]];
                let vals_b = &frame_values[&frames[j]];

                let a_true_ratio = vals_a.iter().filter(|v| **v == TritValue::True).count() as f64
                    / vals_a.len() as f64;
                let b_false_ratio = vals_b.iter().filter(|v| **v == TritValue::False).count()
                    as f64
                    / vals_b.len() as f64;

                // Strong opposition: >60% True in one frame, >60% False in another
                if a_true_ratio > 0.6 && b_false_ratio > 0.6 {
                    conflicts.push(format!(
                        "{:?} predominantly True ({:.0}%) vs {:?} predominantly False ({:.0}%)",
                        frames[i],
                        a_true_ratio * 100.0,
                        frames[j],
                        b_false_ratio * 100.0
                    ));
                }

                // Check the reverse direction too
                let a_false_ratio = vals_a.iter().filter(|v| **v == TritValue::False).count()
                    as f64
                    / vals_a.len() as f64;
                let b_true_ratio = vals_b.iter().filter(|v| **v == TritValue::True).count() as f64
                    / vals_b.len() as f64;

                if a_false_ratio > 0.6 && b_true_ratio > 0.6 {
                    conflicts.push(format!(
                        "{:?} predominantly False ({:.0}%) vs {:?} predominantly True ({:.0}%)",
                        frames[i],
                        a_false_ratio * 100.0,
                        frames[j],
                        b_true_ratio * 100.0
                    ));
                }
            }
        }

        self.state = ModuleState::Completed;

        if conflicts.is_empty() {
            ModuleOutput::new(
                TritValue::True,
                0.8,
                format!(
                    "conflict_suspension: {} frame(s), no cross-frame opposition",
                    frames.len()
                ),
            )
        } else {
            self.suspension_count += conflicts.len();
            let interrupts: Vec<MetaInterrupt> = conflicts
                .iter()
                .map(|c| MetaInterrupt::new(ConflictType::FrameMismatch, c.clone()))
                .collect();

            ModuleOutput::new(
                TritValue::Hold,
                0.6,
                format!(
                    "conflict_suspension: {} cross-frame conflict(s) suspended — {}",
                    conflicts.len(),
                    conflicts.join("; ")
                ),
            )
            .with_interrupts(interrupts)
        }
    }

    // ponytail: on_mount + state via adapter_lifecycle_no_unmount!()
    fn on_unmount(&mut self) {
        self.suspension_count = 0;
        self.state = ModuleState::Completed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::phase::Phase;
    use crate::core::Frame;

    fn word(frame: Frame, value: TritValue) -> crate::core::TritWord {
        crate::core::TritWord::new(value, Phase::neutral(), frame)
    }

    #[test]
    fn single_signal_no_conflict() {
        let mut m = ConflictSuspension::new();
        let input = ModuleInput {
            signals: vec![word(Frame::Science, TritValue::True)],
            interrupts: vec![],
            attention_cmd: None,
        };
        let out = m.process(&input, &HookContext::default());
        assert_eq!(out.recommendation, TritValue::True);
    }

    #[test]
    fn same_frame_no_cross_frame_conflict() {
        let mut m = ConflictSuspension::new();
        let input = ModuleInput {
            signals: vec![
                word(Frame::Science, TritValue::True),
                word(Frame::Science, TritValue::True),
            ],
            interrupts: vec![],
            attention_cmd: None,
        };
        let out = m.process(&input, &HookContext::default());
        assert_eq!(out.recommendation, TritValue::True);
    }

    #[test]
    fn cross_frame_opposition_detected() {
        let mut m = ConflictSuspension::new();
        let input = ModuleInput {
            signals: vec![
                word(Frame::Science, TritValue::True),
                word(Frame::Science, TritValue::True),
                word(Frame::Individual, TritValue::False),
                word(Frame::Individual, TritValue::False),
            ],
            interrupts: vec![],
            attention_cmd: None,
        };
        let out = m.process(&input, &HookContext::default());
        assert_eq!(out.recommendation, TritValue::Hold);
        assert!(!out.interrupts.is_empty());
        assert!(out
            .interrupts
            .iter()
            .any(|i| matches!(i.conflict, ConflictType::FrameMismatch)));
        assert_eq!(m.suspension_count(), 1);
    }

    #[test]
    fn mixed_frames_no_clear_opposition() {
        let mut m = ConflictSuspension::new();
        let input = ModuleInput {
            signals: vec![
                word(Frame::Science, TritValue::True),
                word(Frame::Individual, TritValue::True), // same value, different frame
                word(Frame::Consensus, TritValue::Hold),
            ],
            interrupts: vec![],
            attention_cmd: None,
        };
        let out = m.process(&input, &HookContext::default());
        // All True or Hold — no False opposition → no conflict
        assert_eq!(out.recommendation, TritValue::True);
    }

    #[test]
    fn suspension_count_resets_on_unmount() {
        let mut m = ConflictSuspension::new();
        let input = ModuleInput {
            signals: vec![
                word(Frame::Science, TritValue::True),
                word(Frame::Science, TritValue::True),
                word(Frame::Individual, TritValue::False),
                word(Frame::Individual, TritValue::False),
            ],
            interrupts: vec![],
            attention_cmd: None,
        };
        m.process(&input, &HookContext::default());
        assert_eq!(m.suspension_count(), 1);

        m.on_unmount();
        assert_eq!(m.suspension_count(), 0);
    }

    #[test]
    fn module_id_and_name() {
        let m = ConflictSuspension::new();
        assert_eq!(m.id(), ModuleId::ConflictSuspension);
        assert_eq!(m.name(), "conflict_suspension");
    }
}
