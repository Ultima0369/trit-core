//! Critical thinking: logical consistency, boundary condition verification, counterfactual reasoning.
//! Stub — implementation in Phase 3.

use crate::adapters::{CognitiveModule, ModuleId, ModuleInput, ModuleOutput, ModuleState};
use crate::feedback::FeedbackSignal;
use crate::hook::{HookContext, UnmountReason};

pub struct CriticalThinking {
    id: ModuleId,
    state: ModuleState,
}

impl CriticalThinking {
    pub fn new() -> Self {
        CriticalThinking {
            id: ModuleId::new("critical_thinking"),
            state: ModuleState::Unmounted,
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
        self.id.clone()
    }
    fn name(&self) -> &'static str {
        "critical_thinking"
    }
    fn process_signals(&mut self, _input: &ModuleInput, _ctx: &HookContext) -> ModuleOutput {
        ModuleOutput {
            results: vec![],
            confidence: 0.5,
            explanation_impulse_detected: false,
            summary: "Critical thinking: stub — no analysis performed".into(),
            warnings: vec![],
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
        0.5
    }
}
