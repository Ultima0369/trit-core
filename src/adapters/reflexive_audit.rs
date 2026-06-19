//! Reflexive audit adapter: wraps the existing reflexive auditor.
//!
//! Migrated from `src/reflexive/auditor.rs`. Implements `CognitiveModule`
//! so it can be mounted/unmounted by the Hook Manager.

use crate::adapters::{CognitiveModule, ModuleId, ModuleInput, ModuleOutput, ModuleState};
use crate::feedback::FeedbackSignal;
use crate::hook::{HookContext, UnmountReason};
use crate::reflexive::{AuditReport, ReflexiveAuditor};

pub struct ReflexiveAuditModule {
    id: ModuleId,
    state: ModuleState,
    auditor: ReflexiveAuditor,
}

impl ReflexiveAuditModule {
    pub fn new() -> Self {
        ReflexiveAuditModule {
            id: ModuleId::new("reflexive_audit"),
            state: ModuleState::Unmounted,
            auditor: ReflexiveAuditor::new(),
        }
    }
}

impl Default for ReflexiveAuditModule {
    fn default() -> Self {
        Self::new()
    }
}

impl CognitiveModule for ReflexiveAuditModule {
    fn id(&self) -> ModuleId {
        self.id.clone()
    }

    fn name(&self) -> &'static str {
        "reflexive_audit"
    }

    fn process_signals(&mut self, _input: &ModuleInput, _ctx: &HookContext) -> ModuleOutput {
        let report = self.auditor.audit_last_decision();
        let (confidence, summary, warnings) = match &report {
            AuditReport::Clean => (
                0.9,
                "Reflexive audit: decision path is clean".to_string(),
                vec![],
            ),
            AuditReport::ForcedCollapse {
                reason,
                recommendation,
            } => (
                0.4,
                format!("Forced collapse detected: {}", reason),
                vec![recommendation.clone()],
            ),
            AuditReport::ExplanationImpulse { reason } => (
                0.3,
                format!("Explanation impulse detected: {}", reason),
                vec!["Consider returning Hold instead of forcing an answer".into()],
            ),
        };

        ModuleOutput {
            results: vec![],
            confidence,
            explanation_impulse_detected: matches!(report, AuditReport::ExplanationImpulse { .. }),
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
        0.7
    }
}
