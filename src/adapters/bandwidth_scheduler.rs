//! Bandwidth scheduler adapter: wraps the existing attention scheduler.
//!
//! Migrated from `src/attention/scheduler.rs`. Implements `CognitiveModule`
//! so it can be mounted/unmounted by the Hook Manager.

use crate::adapters::{CognitiveModule, ModuleId, ModuleInput, ModuleOutput, ModuleState};
use crate::attention::{AttentionCmd, AttentionScheduler};
use crate::feedback::FeedbackSignal;
use crate::hook::{HookContext, UnmountReason};

pub struct BandwidthScheduler {
    id: ModuleId,
    state: ModuleState,
    scheduler: AttentionScheduler,
}

impl BandwidthScheduler {
    pub fn new(bandwidth: f64) -> Self {
        BandwidthScheduler {
            id: ModuleId::new("bandwidth_scheduler"),
            state: ModuleState::Unmounted,
            scheduler: AttentionScheduler::new(bandwidth),
        }
    }
}

impl Default for BandwidthScheduler {
    fn default() -> Self {
        Self::new(0.5)
    }
}

impl CognitiveModule for BandwidthScheduler {
    fn id(&self) -> ModuleId {
        self.id.clone()
    }

    fn name(&self) -> &'static str {
        "bandwidth_scheduler"
    }

    fn process_signals(&mut self, input: &ModuleInput, _ctx: &HookContext) -> ModuleOutput {
        let cmd = self.scheduler.suggest_reprioritization(&input.signals);
        let (summary, warnings) = match &cmd {
            AttentionCmd::ShiftTo(target) => {
                (format!("Attention shift suggested: {:?}", target), vec![])
            }
            AttentionCmd::HoldCurrent => (
                "Attention scheduler suggests holding current processing".into(),
                vec!["Bandwidth below threshold".into()],
            ),
            AttentionCmd::Recalibrate => (
                "Attention scheduler suggests recalibration".into(),
                vec!["Cognitive load imbalance detected".into()],
            ),
            AttentionCmd::Continue => ("Attention: continue".into(), vec![]),
        };

        ModuleOutput {
            results: vec![],
            confidence: self.scheduler.bandwidth,
            explanation_impulse_detected: false,
            summary,
            warnings,
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
        self.scheduler.bandwidth
    }
}
