//! Stub module — implementation in Phase 3.

use crate::adapters::{CognitiveModule, ModuleId, ModuleInput, ModuleOutput, ModuleState};
use crate::feedback::FeedbackSignal;
use crate::hook::{HookContext, UnmountReason};

pub struct EngineeringArchitecture {
    id: ModuleId,
    state: ModuleState,
}

impl EngineeringArchitecture {
    pub fn new() -> Self {
        EngineeringArchitecture {
            id: ModuleId::new("engineering_architecture"),
            state: ModuleState::Unmounted,
        }
    }
}

impl Default for EngineeringArchitecture {
    fn default() -> Self {
        Self::new()
    }
}

impl CognitiveModule for EngineeringArchitecture {
    fn id(&self) -> ModuleId {
        self.id.clone()
    }
    fn name(&self) -> &'static str {
        "engineering_architecture"
    }
    fn process_signals(&mut self, _input: &ModuleInput, _ctx: &HookContext) -> ModuleOutput {
        ModuleOutput {
            results: vec![],
            confidence: 0.5,
            explanation_impulse_detected: false,
            summary: "engineering_architecture: stub".into(),
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
