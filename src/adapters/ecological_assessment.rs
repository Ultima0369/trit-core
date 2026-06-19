//! Stub module — implementation in Phase 3.

use crate::adapters::{CognitiveModule, ModuleId, ModuleInput, ModuleOutput, ModuleState};
use crate::feedback::FeedbackSignal;
use crate::hook::{HookContext, UnmountReason};

pub struct EcologicalAssessment {
    id: ModuleId,
    state: ModuleState,
}

impl EcologicalAssessment {
    pub fn new() -> Self {
        EcologicalAssessment {
            id: ModuleId::new("ecological_assessment"),
            state: ModuleState::Unmounted,
        }
    }
}

impl Default for EcologicalAssessment {
    fn default() -> Self {
        Self::new()
    }
}

impl CognitiveModule for EcologicalAssessment {
    fn id(&self) -> ModuleId {
        self.id.clone()
    }
    fn name(&self) -> &'static str {
        "ecological_assessment"
    }
    fn process_signals(&mut self, _input: &ModuleInput, _ctx: &HookContext) -> ModuleOutput {
        ModuleOutput {
            results: vec![],
            confidence: 0.5,
            explanation_impulse_detected: false,
            summary: "ecological_assessment: stub".into(),
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
