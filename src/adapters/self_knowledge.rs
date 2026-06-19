//! Self-knowledge adapter: wraps the existing self-knowledge model.
//!
//! Migrated from `src/knowledge/self_model.rs`. Implements `CognitiveModule`
//! so it can be mounted/unmounted by the Hook Manager.

use crate::adapters::{CognitiveModule, ModuleId, ModuleInput, ModuleOutput, ModuleState};
use crate::feedback::FeedbackSignal;
use crate::hook::{HookContext, UnmountReason};
use crate::knowledge::SelfKnowledge;

pub struct SelfKnowledgeModule {
    id: ModuleId,
    state: ModuleState,
    knowledge: SelfKnowledge,
}

impl SelfKnowledgeModule {
    pub fn new() -> Self {
        SelfKnowledgeModule {
            id: ModuleId::new("self_knowledge"),
            state: ModuleState::Unmounted,
            knowledge: SelfKnowledge::with_human_defaults(),
        }
    }
}

impl Default for SelfKnowledgeModule {
    fn default() -> Self {
        Self::new()
    }
}

impl CognitiveModule for SelfKnowledgeModule {
    fn id(&self) -> ModuleId {
        self.id.clone()
    }

    fn name(&self) -> &'static str {
        "self_knowledge"
    }

    fn process_signals(&mut self, input: &ModuleInput, _ctx: &HookContext) -> ModuleOutput {
        if let Some(last) = input.signals.last() {
            let estimate = self.knowledge.infer_receiver_state(last);
            ModuleOutput {
                results: vec![],
                confidence: estimate.confidence,
                explanation_impulse_detected: false,
                summary: format!(
                    "Receiver estimate: {:?} @ {:.2} confidence (attending: {:?})",
                    estimate.estimated_value, estimate.confidence, estimate.attended_frames
                ),
                warnings: if estimate.confidence < 0.3 {
                    vec!["Low confidence receiver estimate".into()]
                } else {
                    vec![]
                },
            }
        } else {
            ModuleOutput {
                results: vec![],
                confidence: 0.0,
                explanation_impulse_detected: false,
                summary: "Self-knowledge: no signals to analyze".into(),
                warnings: vec![],
            }
        }
    }

    fn on_mount(&mut self) {
        self.state = ModuleState::Active;
    }

    fn on_unmount(&mut self, _reason: UnmountReason) {
        self.state = ModuleState::Unmounted;
    }

    fn state(&self) -> ModuleState {
        self.state
    }

    fn calibrate(&mut self, _feedback: &FeedbackSignal) -> f64 {
        0.6
    }
}
