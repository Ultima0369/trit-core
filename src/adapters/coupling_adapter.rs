//! Stub module — implementation in Phase 3.

use crate::adapters::{CognitiveModule, ModuleId, ModuleInput, ModuleOutput, ModuleState};
use crate::feedback::FeedbackSignal;
use crate::hook::{HookContext, UnmountReason};

pub struct CouplingAdapter {
    id: ModuleId,
    state: ModuleState,
}

impl CouplingAdapter {
    pub fn new() -> Self {
        CouplingAdapter {
            id: ModuleId::new("coupling_adapter"),
            state: ModuleState::Unmounted,
        }
    }
}

impl Default for CouplingAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl CognitiveModule for CouplingAdapter {
    fn id(&self) -> ModuleId {
        self.id.clone()
    }
    fn name(&self) -> &'static str {
        "coupling_adapter"
    }
    fn process_signals(&mut self, _input: &ModuleInput, _ctx: &HookContext) -> ModuleOutput {
        ModuleOutput {
            results: vec![],
            confidence: 0.5,
            explanation_impulse_detected: false,
            summary: "coupling_adapter: stub".into(),
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
