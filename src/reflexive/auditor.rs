//! Reflexive audit: a slow half-beat self-check after each decision.
//!
//! The reflexive auditor looks back at the decision path and asks:
//!
//! 1. How was this output assembled?
//! 2. Was any step driven by an "explanation impulse" rather than evidence?
//! 3. Could a Hold have been chosen instead of a forced collapse?
//!
//! This is the algorithmic form of "knowing oneself" — the system inspects
//! its own conflict trace before finalizing an answer.

use chrono::Utc;

use crate::core::frame::Frame;
use crate::core::word::TritWord;
use crate::meta::{ConflictType, MetaInterrupt};
use crate::sandbox::output::SandboxOutput;

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

/// Alert produced by the reflexive guard when it intervenes in a pipeline.
#[derive(Debug, Clone, PartialEq)]
pub struct ReflexiveAlert {
    /// Human-readable reason for the alert.
    pub reason: String,
    /// Suggested action.
    pub recommendation: String,
}

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
    ///
    /// Returns `ForcedCollapse` if there are unresolved frame-mismatch
    /// interrupts but the final output is a forced True/False. Returns
    /// `ExplanationImpulse` if the attention trace shows rapid back-and-forth
    /// on the same theme. Otherwise returns `Clean`.
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

    /// Run an automatic post-audit on a concrete sandbox output.
    ///
    /// Returns a `MetaInterrupt` if the reflexive guard detects that a
    /// forced True/False decision was made while unresolved cross-frame
    /// conflicts remain in the trace. Returns `None` otherwise.
    pub fn auto_post_audit(&mut self, decision: &SandboxOutput) -> Option<MetaInterrupt> {
        let unresolved_conflicts = self
            .decision_chain
            .iter()
            .filter(|i| matches!(i.conflict, ConflictType::FrameMismatch))
            .count();

        let is_forced = decision.final_value_code == 1 || decision.final_value_code == -1;

        if unresolved_conflicts > 0 && is_forced {
            let alert = ReflexiveAlert {
                reason: format!(
                    "Forced {} output with {} unresolved frame conflict(s)",
                    decision.final_value, unresolved_conflicts
                ),
                recommendation: "Reflexive guard suggests reviewing for Hold.".to_string(),
            };
            let interrupt = MetaInterrupt::new(
                ConflictType::PolicyViolation,
                format!("{}: {}", alert.reason, alert.recommendation),
            );
            self.decision_chain.push(interrupt.clone());
            return Some(interrupt);
        }

        None
    }

    /// Build a `TritWord` summarizing the reflexive posture of the auditor.
    ///
    /// Useful for downstream attention/knowledge modules.
    pub fn reflexive_posture(&self) -> TritWord {
        match self.audit_last_decision() {
            AuditReport::Clean => TritWord::tru(Frame::Meta),
            _ => TritWord::hold(Frame::Meta),
        }
    }

    fn has_explanation_impulse(&self) -> bool {
        // Simple heuristic: if the same attention label appears more than
        // twice in the trace, treat it as loop entrainment / explanation impulse.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::value::TritValue;

    fn sample_output(value: &str, code: i8) -> SandboxOutput {
        SandboxOutput {
            scenario_id: "test".to_string(),
            final_value: value.to_string(),
            final_value_code: code,
            final_frame: "Meta".to_string(),
            final_phase_raw: 0.5,
            interrupts: vec![],
            policy_action: "Test".to_string(),
            reflexive_alert: None,
            attention_cmd: None,
            receiver_estimate: None,
            hold_state: None,
        }
    }

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
        let out = sample_output("True", 1);
        let alert = auditor.auto_post_audit(&out);
        assert!(alert.is_some());
        assert!(matches!(
            auditor.audit_last_decision(),
            AuditReport::ForcedCollapse { .. }
        ));
    }

    #[test]
    fn hold_output_does_not_trigger_alert() {
        let mut auditor = ReflexiveAuditor::new();
        auditor.record_interrupt(MetaInterrupt::new(
            ConflictType::FrameMismatch,
            "conflict".to_string(),
        ));
        let out = sample_output("Hold", 0);
        assert!(auditor.auto_post_audit(&out).is_none());
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
}
