//! DecisionEngine facade — ternary decision pipeline (Layer 4).
//!
//! Extracted from `SandboxPipeline`, this module owns the core decision
//! logic: TAND cascade → policy arbitration → reflexive guard → SafeFallback.
//! It does NOT handle validation, OS sampling, attention scheduling,
//! self-knowledge, anchor checks, or calibration — those remain in the
//! sandbox pipeline (Layer 2–3–5 integrator).

use crate::adapters::reflexive_audit::{ReflexiveAlert, ReflexiveAuditor};
use crate::core::frame::Frame;
use crate::core::value::TritValue;
use crate::core::word::TritWord;
use crate::core::TernaryAlgebra;
use crate::meta::{
    ArbitrationResult, ConflictType, Domain, MetaInterrupt, ResolutionPolicy, SafeFallback,
};
use crate::sandbox::SandboxError;

/// Result of a single ternary decision cycle.
#[derive(Debug, Clone)]
pub struct DecisionResult {
    /// The final word after arbitration, reflexive guard, and SafeFallback.
    pub final_word: TritWord,
    /// The policy action taken by arbitration.
    pub policy_action: ArbitrationResult,
    /// All interrupts collected during the decision cycle.
    pub interrupts: Vec<MetaInterrupt>,
    /// Optional alert from the reflexive guard.
    pub reflexive_alert: Option<ReflexiveAlert>,
    /// Whether SafeFallback was triggered.
    pub safe_fallback_triggered: bool,
}

/// Facade for the ternary decision engine (Layer 4 of the 5-layer architecture).
///
/// Owns SafeFallback configuration and optional reflexive auditor.
/// Does NOT own the ternary algebra (stateless), validation logic,
/// attention scheduling, self-knowledge, anchor constraints, or calibration —
/// those remain in [`SandboxPipeline`](crate::sandbox::SandboxPipeline).
pub struct DecisionEngine {
    safe_fallback: SafeFallback,
    reflexive: Option<ReflexiveAuditor>,
    trace_phase: bool,
}

impl DecisionEngine {
    /// Create a new DecisionEngine with default SafeFallback.
    pub fn new() -> Self {
        DecisionEngine {
            safe_fallback: SafeFallback::new(),
            reflexive: None,
            trace_phase: false,
        }
    }

    /// Attach a reflexive auditor for post-decision self-checking.
    pub fn with_reflexive(mut self, auditor: ReflexiveAuditor) -> Self {
        self.reflexive = Some(auditor);
        self
    }

    /// Enable phase-trace collection in the reflexive auditor.
    pub fn with_trace_phase(mut self, enabled: bool) -> Self {
        self.trace_phase = enabled;
        self
    }

    /// Set a custom SafeFallback configuration.
    pub fn with_safe_fallback(mut self, safe_fallback: SafeFallback) -> Self {
        self.safe_fallback = safe_fallback;
        self
    }

    /// Run the full ternary decision cycle:
    ///
    /// 1. TAND cascade over all trits
    /// 2. Policy arbitration (domain-specific)
    /// 3. Reflexive guard (override forced decisions with unresolved conflicts)
    /// 4. SafeFallback (force False in dangerous domains when uncertain)
    ///
    /// # Errors
    ///
    /// Returns `SandboxError::InvalidScenario` if policy arbitration fails.
    pub fn decide(
        &mut self,
        trits: &[TritWord],
        domain: &Domain,
    ) -> Result<DecisionResult, SandboxError> {
        // Stage 1: TAND cascade
        let (current, mut interrupts) = TernaryAlgebra::t_and_n(trits);

        // Stage 2: policy arbitration
        let policy = ResolutionPolicy::new(domain.clone());
        let policy_result = policy
            .arbitrate(trits)
            .map_err(|e| SandboxError::InvalidScenario(format!("arbitration failed: {e}")))?;

        let arbitrated_word = self.resolve_arbitrated_word(&policy_result, &current);

        // Stage 3: reflexive guard
        let reflexive_alert = self.run_reflexive_guard(&policy, &arbitrated_word, &interrupts);

        // Stage 4: SafeFallback
        let force = matches!(&policy_result, ArbitrationResult::ForceCollapse);
        let (final_word, fb_interrupt) = self.safe_fallback.guard_with_force(
            &policy.domain,
            &arbitrated_word,
            interrupts.len(),
            force,
        );

        let safe_fallback_triggered = fb_interrupt.is_some();
        if let Some(int) = fb_interrupt {
            interrupts.push(int);
        }

        // If reflexive guard fired and output is still forced True/False, override to Hold
        let final_word = if reflexive_alert.is_some() && final_word.value().is_computable() {
            TritWord::hold(Frame::Meta)
        } else {
            final_word
        };

        Ok(DecisionResult {
            final_word,
            policy_action: policy_result,
            interrupts,
            reflexive_alert,
            safe_fallback_triggered,
        })
    }

    // ── Private methods ───────────────────────────────────────────

    /// Resolve the word to use after arbitration.
    fn resolve_arbitrated_word(
        &self,
        policy_result: &ArbitrationResult,
        current: &TritWord,
    ) -> TritWord {
        match policy_result {
            ArbitrationResult::Commit(w) => {
                if current.value() == TritValue::Hold && w.value().is_computable() {
                    TritWord::hold(Frame::Meta)
                } else {
                    *w
                }
            }
            ArbitrationResult::Preserve(w) => *w,
            ArbitrationResult::Hold => TritWord::hold(Frame::Meta),
            ArbitrationResult::ForceCollapse => TritWord::hold(Frame::Meta),
            ArbitrationResult::Negotiate => *current,
            ArbitrationResult::DryRun => *current,
        }
    }

    /// Run the reflexive guard — check for forced decisions with unresolved conflicts.
    fn run_reflexive_guard(
        &mut self,
        policy: &ResolutionPolicy,
        arbitrated_word: &TritWord,
        interrupts: &[MetaInterrupt],
    ) -> Option<ReflexiveAlert> {
        if let Some(ref mut auditor) = self.reflexive {
            for int in interrupts {
                auditor.record_interrupt(int.clone());
            }
            if self.trace_phase {
                auditor.record_phase_shift(crate::adapters::reflexive_audit::PhaseShift::new(
                    arbitrated_word.phase().inner(),
                    arbitrated_word.phase().inner(),
                    "arbitration",
                ));
            }
            return Self::check_reflexive_guard(
                &policy.domain,
                arbitrated_word,
                interrupts,
                &self.safe_fallback,
            );
        }
        None
    }

    /// Check whether a forced True/False decision was made while unresolved
    /// cross-frame conflicts or explanation impulses remain.
    fn check_reflexive_guard(
        domain: &Domain,
        decision: &TritWord,
        interrupts: &[MetaInterrupt],
        safe_fallback: &SafeFallback,
    ) -> Option<ReflexiveAlert> {
        let unresolved_conflicts = interrupts
            .iter()
            .filter(|i| {
                matches!(
                    i.conflict,
                    ConflictType::FrameMismatch | ConflictType::ExplainImpulse
                )
            })
            .count();

        let is_forced = decision.value() == TritValue::True || decision.value() == TritValue::False;

        if unresolved_conflicts > 0 && is_forced {
            let dangerous = safe_fallback.is_dangerous(domain);
            if dangerous {
                return None;
            }
            let alert = ReflexiveAlert {
                reason: format!(
                    "Forced {:?} output with {} unresolved conflict(s)",
                    decision.value(),
                    unresolved_conflicts
                ),
                recommendation: "Reflexive guard suggests returning Hold.".to_string(),
            };
            return Some(alert);
        }

        None
    }
}

impl Default for DecisionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::frame::Frame;
    use crate::core::phase::Phase;

    fn word(frame: Frame, value: TritValue, phase: f64) -> TritWord {
        TritWord::new(value, Phase::new_clamped(phase), frame)
    }

    #[test]
    fn decide_same_frame_commits_true() {
        let mut engine = DecisionEngine::new();
        let trits = vec![
            word(Frame::Science, TritValue::True, 0.9),
            word(Frame::Science, TritValue::True, 0.8),
        ];
        let result = engine.decide(&trits, &Domain::General).unwrap();
        assert_eq!(result.final_word.value(), TritValue::True);
        assert_eq!(result.final_word.frame(), Frame::Science);
        assert!(!result.safe_fallback_triggered);
    }

    #[test]
    fn decide_cross_frame_produces_hold() {
        let mut engine = DecisionEngine::new();
        let trits = vec![
            word(Frame::Science, TritValue::True, 0.9),
            word(Frame::Individual, TritValue::False, 0.8),
        ];
        let result = engine.decide(&trits, &Domain::General).unwrap();
        assert_eq!(result.final_word.value(), TritValue::Hold);
        assert!(!result.interrupts.is_empty());
    }

    #[test]
    fn decide_medical_ethics_preserves_individual() {
        let mut engine = DecisionEngine::new();
        let trits = vec![
            word(Frame::Science, TritValue::True, 0.8),
            word(Frame::Individual, TritValue::False, 0.2),
        ];
        let result = engine.decide(&trits, &Domain::MedicalEthics).unwrap();
        assert_eq!(result.final_word.value(), TritValue::False);
        assert_eq!(result.final_word.frame(), Frame::Individual);
    }

    #[test]
    fn decide_value_judgment_holds() {
        let mut engine = DecisionEngine::new();
        let trits = vec![
            word(Frame::Individual, TritValue::False, 0.3),
            word(Frame::Consensus, TritValue::True, 0.7),
        ];
        let result = engine.decide(&trits, &Domain::ValueJudgment).unwrap();
        assert_eq!(result.final_word.value(), TritValue::Hold);
    }

    #[test]
    fn reflexive_guard_overrides_forced_true_with_frame_mismatch() {
        // Use same-frame True signals so TAND produces True, but inject a
        // FrameMismatch interrupt manually via check_reflexive_guard.
        // The reflexive guard fires when the decision is forced True/False
        // with unresolved FrameMismatch or ExplainImpulse interrupts.
        let frame_interrupt = MetaInterrupt::new(
            ConflictType::FrameMismatch,
            "cross-frame conflict".to_string(),
        );
        let decision = TritWord::tru(Frame::Science);
        let sf = SafeFallback::new();
        let alert = DecisionEngine::check_reflexive_guard(
            &Domain::General,
            &decision,
            &[frame_interrupt],
            &sf,
        );
        assert!(alert.is_some());
    }

    #[test]
    fn reflexive_guard_overrides_forced_true_with_explain_impulse() {
        let explain_interrupt = MetaInterrupt::new(
            ConflictType::ExplainImpulse,
            "explanation impulse detected".to_string(),
        );
        let decision = TritWord::tru(Frame::Science);
        let sf = SafeFallback::new();
        let alert = DecisionEngine::check_reflexive_guard(
            &Domain::General,
            &decision,
            &[explain_interrupt],
            &sf,
        );
        assert!(alert.is_some());
    }

    #[test]
    fn reflexive_guard_does_not_override_in_dangerous_domain() {
        let mut engine = DecisionEngine::new().with_reflexive(ReflexiveAuditor::new());
        let trits = vec![
            word(Frame::Science, TritValue::True, 0.9),
            word(Frame::Individual, TritValue::True, 0.8),
        ];
        let result = engine.decide(&trits, &Domain::Physical).unwrap();
        // In Physical domain with cross-frame True, arbitration may ForceCollapse
        // → SafeFallback forces False. Reflexive guard should not override.
        assert!(result.safe_fallback_triggered || result.reflexive_alert.is_none());
    }

    #[test]
    fn safe_fallback_forces_false_in_physical_with_hold_and_interrupts() {
        let mut engine = DecisionEngine::new();
        let trits = vec![
            word(Frame::Individual, TritValue::True, 0.9),
            word(Frame::Consensus, TritValue::False, 0.2),
        ];
        let result = engine.decide(&trits, &Domain::Physical).unwrap();
        assert_eq!(result.final_word.value(), TritValue::False);
        assert!(result.safe_fallback_triggered);
    }

    #[test]
    fn decision_result_fields_are_populated() {
        let mut engine = DecisionEngine::new();
        let trits = vec![word(Frame::Science, TritValue::True, 0.9)];
        let result = engine.decide(&trits, &Domain::General).unwrap();
        assert_eq!(result.final_word.value(), TritValue::True);
        assert!(!result.safe_fallback_triggered);
        assert!(result.reflexive_alert.is_none());
        assert!(result.interrupts.is_empty());
    }

    #[test]
    fn single_signal_false_passes_through() {
        let mut engine = DecisionEngine::new();
        let trits = vec![word(Frame::Science, TritValue::False, 0.9)];
        let result = engine.decide(&trits, &Domain::General).unwrap();
        assert_eq!(result.final_word.value(), TritValue::False);
        assert_eq!(result.final_word.frame(), Frame::Science);
    }
}
