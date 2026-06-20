//! Reflexive audit adapter — post-decision self-check.
//!
//! Migrated from `src/reflexive/auditor.rs`. Wraps the existing
//! [`ReflexiveAuditor`] in a [`CognitiveModule`] implementation.
//!
//! The reflexive auditor inspects the decision path and asks:
//! 1. How was this output assembled?
//! 2. Was any step driven by an "explanation impulse" rather than evidence?
//! 3. Could a Hold have been chosen instead of a forced collapse?

use chrono::Utc;

use crate::adapters::{CognitiveModule, ModuleInput, ModuleOutput};
use crate::core::frame::Frame;
use crate::core::word::TritWord;
use crate::core::TritValue;
use crate::hook::module_registry::{ModuleId, ModuleState};
use crate::hook::HookContext;
use crate::meta::{ConflictType, MetaInterrupt, PolicyViolation};

// ── Attention event ─────────────────────────────────────────────────

/// A single attention event in the decision trace.
#[derive(Debug, Clone, PartialEq)]
pub struct AttentionEvent {
    /// Human-readable label for what held attention at this moment.
    pub label: String,
    /// Timestamp in UTC.
    pub timestamp: chrono::DateTime<Utc>,
}

impl AttentionEvent {
    /// Create a new attention event.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            timestamp: Utc::now(),
        }
    }
}

// ── Phase shift ─────────────────────────────────────────────────────

/// A recorded phase shift in the decision trace.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhaseShift {
    /// Phase value before the shift.
    pub from: f64,
    /// Phase value after the shift.
    pub to: f64,
    /// Human-readable reason for the shift.
    pub reason: &'static str,
}

impl PhaseShift {
    /// Create a new phase shift record.
    pub fn new(from: f64, to: f64, reason: &'static str) -> Self {
        Self { from, to, reason }
    }
}

// ── Audit report ────────────────────────────────────────────────────

/// Outcome of a reflexive audit.
#[derive(Debug, Clone, PartialEq)]
pub enum AuditReport {
    /// The decision path is clean: no unresolved conflicts were collapsed.
    Clean,
    /// The decision collapsed a conflict that could have been held.
    ForcedCollapse {
        /// Description of the unresolved conflict.
        reason: String,
        /// Recommendation for what should have happened.
        recommendation: String,
    },
    /// The decision was influenced by an attention pattern that looks like
    /// loop entrainment or explanation impulse.
    ExplanationImpulse {
        /// Description of the detected pattern.
        reason: String,
    },
}

// ── Reflexive alert ─────────────────────────────────────────────────

/// Alert produced by the reflexive guard when it intervenes in a pipeline.
#[derive(Debug, Clone, PartialEq)]
pub struct ReflexiveAlert {
    /// Human-readable reason for the alert.
    pub reason: String,
    /// Suggested action.
    pub recommendation: String,
}

// ── Reflexive auditor (inner engine) ────────────────────────────────

/// Reflexive auditor that traces decisions and produces post-hoc audits.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReflexiveAuditor {
    /// Recorded interrupts from the current decision chain.
    pub decision_chain: Vec<MetaInterrupt>,
    /// Recorded attention events.
    pub attention_trace: Vec<AttentionEvent>,
    /// Recorded phase shifts.
    pub phase_history: Vec<PhaseShift>,
}

impl ReflexiveAuditor {
    /// Create a new empty auditor.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an interrupt in the decision chain.
    pub fn record_interrupt(&mut self, interrupt: MetaInterrupt) {
        self.decision_chain.push(interrupt);
    }

    /// Record an attention event.
    pub fn record_attention(&mut self, event: AttentionEvent) {
        self.attention_trace.push(event);
    }

    /// Record a phase shift.
    pub fn record_phase_shift(&mut self, shift: PhaseShift) {
        self.phase_history.push(shift);
    }

    /// Clear all traces. Useful when starting a new decision session.
    pub fn clear(&mut self) {
        self.decision_chain.clear();
        self.attention_trace.clear();
        self.phase_history.clear();
    }

    /// Audit the most recent decision path.
    pub fn audit_last_decision(&self) -> AuditReport {
        let unresolved_conflicts: Vec<_> = self
            .decision_chain
            .iter()
            .filter(|i| matches!(i.conflict, ConflictType::FrameMismatch))
            .collect();

        if !unresolved_conflicts.is_empty() && self.has_explanation_impulse() {
            return AuditReport::ExplanationImpulse {
                reason: format!(
                    "{} unresolved frame conflict(s) co-occur with loop-like attention trace",
                    unresolved_conflicts.len()
                ),
            };
        }

        if !unresolved_conflicts.is_empty() {
            return AuditReport::ForcedCollapse {
                reason: format!(
                    "{} unresolved frame conflict(s) were present in the decision chain",
                    unresolved_conflicts.len()
                ),
                recommendation: "Consider returning Hold instead of forcing True/False."
                    .to_string(),
            };
        }

        AuditReport::Clean
    }

    /// Build a `TritWord` summarizing the reflexive posture of the auditor.
    pub fn reflexive_posture(&self) -> TritWord {
        match self.audit_last_decision() {
            AuditReport::Clean => TritWord::tru(Frame::Meta),
            _ => TritWord::hold(Frame::Meta),
        }
    }

    fn has_explanation_impulse(&self) -> bool {
        if self.attention_trace.len() < 4 {
            return false;
        }
        let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for event in &self.attention_trace {
            *counts.entry(event.label.as_str()).or_insert(0) += 1;
        }
        counts.values().any(|&c| c >= 3)
    }
}

// ── ReflexiveAuditModule (CognitiveModule wrapper) ──────────────────

/// Cognitive module wrapping the [`ReflexiveAuditor`].
///
/// Implements [`CognitiveModule`] so the Hook Manager can mount/unmount
/// it based on scenario needs.
pub struct ReflexiveAuditModule {
    inner: ReflexiveAuditor,
    state: ModuleState,
}

impl ReflexiveAuditModule {
    /// Create a new reflexive audit module.
    pub fn new() -> Self {
        ReflexiveAuditModule {
            inner: ReflexiveAuditor::new(),
            state: ModuleState::Idle,
        }
    }

    /// Create from an existing auditor.
    pub fn from_auditor(auditor: ReflexiveAuditor) -> Self {
        ReflexiveAuditModule {
            inner: auditor,
            state: ModuleState::Idle,
        }
    }

    /// Access the inner auditor.
    pub fn inner(&self) -> &ReflexiveAuditor {
        &self.inner
    }
}

impl Default for ReflexiveAuditModule {
    fn default() -> Self {
        Self::new()
    }
}

impl CognitiveModule for ReflexiveAuditModule {
    fn id(&self) -> ModuleId {
        ModuleId::ReflexiveAudit
    }

    fn name(&self) -> &'static str {
        "reflexive_audit"
    }

    fn process(&mut self, input: &ModuleInput, ctx: &HookContext) -> ModuleOutput {
        self.state = ModuleState::Processing;

        // Record incoming interrupts into the decision chain.
        for interrupt in &input.interrupts {
            self.inner.record_interrupt(interrupt.clone());
        }

        // If there's a previous iteration with interrupts, record those too.
        if let Some(ref prev) = ctx.previous_iteration {
            // Previous iteration anchor violations suggest forced decisions.
            if prev.anchor_report.is_some() {
                self.inner.record_attention(AttentionEvent::new(
                    "anchor_violation_in_previous_iteration",
                ));
            }
        }

        let report = self.inner.audit_last_decision();

        self.state = ModuleState::Completed;

        match report {
            AuditReport::Clean => {
                ModuleOutput::new(TritValue::True, 0.9, "reflexive audit: decision path clean")
            }
            AuditReport::ForcedCollapse {
                ref reason,
                ref recommendation,
            } => {
                let interrupt = MetaInterrupt::policy_violation(
                    PolicyViolation::ForcedCollapse,
                    format!(
                        "forced collapse: {}. recommendation: {}",
                        reason, recommendation
                    ),
                );
                ModuleOutput::new(
                    TritValue::Hold,
                    0.7,
                    format!("reflexive audit: forced collapse detected — {}", reason),
                )
                .with_interrupts(vec![interrupt])
            }
            AuditReport::ExplanationImpulse { ref reason } => {
                let interrupt = MetaInterrupt::policy_violation(
                    PolicyViolation::Other("explanation impulse".to_string()),
                    format!("explanation impulse: {}", reason),
                );
                ModuleOutput::new(
                    TritValue::Hold,
                    0.5,
                    format!("reflexive audit: explanation impulse — {}", reason),
                )
                .with_interrupts(vec![interrupt])
            }
        }
    }

    fn on_mount(&mut self) {
        self.state = ModuleState::Idle;
    }

    fn on_unmount(&mut self) {
        self.inner.clear();
        self.state = ModuleState::Completed;
    }

    fn state(&self) -> ModuleState {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_audit_when_no_conflicts() {
        let auditor = ReflexiveAuditor::new();
        assert_eq!(auditor.audit_last_decision(), AuditReport::Clean);
    }

    #[test]
    fn forced_collapse_detected() {
        let mut auditor = ReflexiveAuditor::new();
        auditor.record_interrupt(MetaInterrupt::new(
            ConflictType::FrameMismatch,
            "Science vs Individual".to_string(),
        ));
        assert!(matches!(
            auditor.audit_last_decision(),
            AuditReport::ForcedCollapse { .. }
        ));
    }

    #[test]
    fn explanation_impulse_detected() {
        let mut auditor = ReflexiveAuditor::new();
        for _ in 0..4 {
            auditor.record_attention(AttentionEvent::new("ruminate"));
        }
        auditor.record_interrupt(MetaInterrupt::new(
            ConflictType::FrameMismatch,
            "conflict".to_string(),
        ));
        assert!(matches!(
            auditor.audit_last_decision(),
            AuditReport::ExplanationImpulse { .. }
        ));
    }

    #[test]
    fn reflexive_posture_is_hold_when_audit_fails() {
        let mut auditor = ReflexiveAuditor::new();
        auditor.record_interrupt(MetaInterrupt::new(
            ConflictType::FrameMismatch,
            "conflict".to_string(),
        ));
        assert_eq!(auditor.reflexive_posture().value(), TritValue::Hold);
    }

    #[test]
    fn reflexive_posture_is_true_when_clean() {
        let auditor = ReflexiveAuditor::new();
        assert_eq!(auditor.reflexive_posture().value(), TritValue::True);
    }

    // ── CognitiveModule tests ─────────────────────────────────────

    #[test]
    fn reflexive_audit_module_id() {
        let module = ReflexiveAuditModule::new();
        assert_eq!(module.id(), ModuleId::ReflexiveAudit);
        assert_eq!(module.name(), "reflexive_audit");
    }

    #[test]
    fn reflexive_audit_module_clean_process() {
        let mut module = ReflexiveAuditModule::new();
        let input = ModuleInput {
            signals: vec![],
            interrupts: vec![],
            attention_cmd: None,
        };
        let ctx = HookContext::default();
        let out = module.process(&input, &ctx);
        assert_eq!(out.recommendation, TritValue::True);
        assert!(out.confidence > 0.8);
    }

    #[test]
    fn reflexive_audit_module_detects_conflict() {
        let mut module = ReflexiveAuditModule::new();
        let interrupt =
            MetaInterrupt::new(ConflictType::FrameMismatch, "test conflict".to_string());
        let input = ModuleInput {
            signals: vec![],
            interrupts: vec![interrupt],
            attention_cmd: None,
        };
        let ctx = HookContext::default();
        let out = module.process(&input, &ctx);
        assert_eq!(out.recommendation, TritValue::Hold);
        assert!(!out.interrupts.is_empty());
    }

    #[test]
    fn reflexive_audit_module_clear_on_unmount() {
        let mut module = ReflexiveAuditModule::new();
        module.inner.record_interrupt(MetaInterrupt::new(
            ConflictType::FrameMismatch,
            "test".to_string(),
        ));
        assert!(!module.inner.decision_chain.is_empty());

        module.on_unmount();
        assert!(module.inner.decision_chain.is_empty());
        assert_eq!(module.state(), ModuleState::Completed);
    }
}
