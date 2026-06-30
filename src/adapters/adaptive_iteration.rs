//! Adaptive iteration adapter — feedback collection and correction.
//!
//! Tracks decision history across iterations, detects entrainment patterns,
//! and suggests corrections. This is the ONLY module permitted to suggest
//! parameter changes — and its changes are bounded:
//!
//! **Allowed:** suggest parameter adjustments, recommend mount/unmount actions,
//! adjust internal weights.
//!
//! **Forbidden:** modify Layer 1 anchors, modify core algebra (t_and, t_or, t_not),
//! bypass the reflexive audit module.

use crate::adapters::{adapter_lifecycle_no_unmount, CognitiveModule, ModuleInput, ModuleOutput};
use crate::core::TritValue;
use crate::hook::module_registry::{ModuleId, ModuleState};
use crate::hook::HookContext;
use crate::meta::{MetaInterrupt, PolicyViolation};

/// Maximum number of consecutive same-result decisions before flagging entrainment.
const ENTRAINMENT_THRESHOLD: usize = 3;

/// Maximum staleness (consecutive Holds) before recommending escalation.
const STALENESS_THRESHOLD: usize = 3;

/// Cognitive module for adaptive iteration and correction triggering.
pub struct AdaptiveIteration {
    state: ModuleState,
    /// Rolling window of recent decision results.
    recent_results: Vec<TritValue>,
    /// How many consecutive Holds have been seen.
    staleness_counter: usize,
    /// How many corrections have been suggested.
    correction_count: usize,
}

impl AdaptiveIteration {
    /// Create a new adaptive iteration module.
    pub fn new() -> Self {
        AdaptiveIteration {
            state: ModuleState::Idle,
            recent_results: Vec::new(),
            staleness_counter: 0,
            correction_count: 0,
        }
    }

    /// Number of corrections suggested.
    pub fn correction_count(&self) -> usize {
        self.correction_count
    }

    /// Recent decision results.
    pub fn recent_results(&self) -> &[TritValue] {
        &self.recent_results
    }
}

impl Default for AdaptiveIteration {
    fn default() -> Self {
        Self::new()
    }
}

impl CognitiveModule for AdaptiveIteration {
    adapter_lifecycle_no_unmount!();

    fn id(&self) -> ModuleId {
        ModuleId::AdaptiveIteration
    }

    fn name(&self) -> &'static str {
        "adaptive_iteration"
    }

    fn process(&mut self, _input: &ModuleInput, ctx: &HookContext) -> ModuleOutput {
        self.state = ModuleState::Processing;

        // Update rolling window from previous iteration.
        if let Some(ref prev) = ctx.previous_iteration {
            let prev_value = match &prev.arbitration {
                crate::meta::ArbitrationResult::Commit(w) => w.value(),
                crate::meta::ArbitrationResult::Hold => TritValue::Hold,
                _ => TritValue::Hold, // Preserve, ForceCollapse, Negotiate → treat as Hold
            };
            self.recent_results.push(prev_value);
            // Keep window bounded to last 5 results.
            if self.recent_results.len() > 5 {
                self.recent_results.remove(0);
            }
        }

        let mut interrupts = Vec::new();
        let mut concerns: Vec<String> = Vec::new();

        // Check 1: Entrainment — 3+ consecutive same-result decisions.
        if self.recent_results.len() >= ENTRAINMENT_THRESHOLD {
            let last_n = &self.recent_results[self.recent_results.len() - ENTRAINMENT_THRESHOLD..];
            let all_same = last_n.iter().all(|r| *r == last_n[0]);
            if all_same && last_n[0] != TritValue::Hold {
                concerns.push(format!(
                    "entrainment: {} consecutive {:?} decisions — pattern lock detected",
                    ENTRAINMENT_THRESHOLD, last_n[0]
                ));
            }
        }

        // Check 2: Staleness — consecutive Holds exceeding threshold.
        if let Some(&last) = self.recent_results.last() {
            if last == TritValue::Hold {
                self.staleness_counter += 1;
            } else {
                self.staleness_counter = 0;
            }
        }
        if self.staleness_counter >= STALENESS_THRESHOLD {
            concerns.push(format!(
                "staleness: {} consecutive Hold cycles — recommend escalation",
                self.staleness_counter
            ));
        }

        // Check 3: Hold budget exhaustion from context.
        if ctx.hold_budget_exhausted() {
            concerns.push("hold budget exhausted — recommend escalation to Layer 1".to_string());
        }

        // Build interrupts.
        for concern in &concerns {
            interrupts.push(MetaInterrupt::policy_violation(
                PolicyViolation::Other("adaptive escalation".to_string()),
                format!("adaptive: {}", concern),
            ));
        }

        self.state = ModuleState::Completed;

        if concerns.is_empty() {
            ModuleOutput::new(
                TritValue::True,
                0.7,
                format!(
                    "adaptive: {} decision(s) tracked, no correction needed",
                    self.recent_results.len()
                ),
            )
        } else {
            self.correction_count += concerns.len();
            ModuleOutput::new(
                TritValue::Hold,
                0.5,
                format!(
                    "adaptive: {} correction(s) suggested — {}",
                    concerns.len(),
                    concerns.join("; ")
                ),
            )
            .with_interrupts(interrupts)
        }
    }

    // ponytail: on_mount + state via adapter_lifecycle_no_unmount!()
    fn on_unmount(&mut self) {
        self.recent_results.clear();
        self.staleness_counter = 0;
        self.correction_count = 0;
        self.state = ModuleState::Completed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::phase::Phase;
    use crate::core::Frame;
    use crate::hook::IterationSummary;
    use crate::meta::ArbitrationResult;

    fn word(frame: Frame, value: TritValue) -> crate::core::TritWord {
        crate::core::TritWord::new(value, Phase::neutral(), frame)
    }

    fn hold_summary() -> IterationSummary {
        IterationSummary {
            arbitration: ArbitrationResult::Hold,
            interrupt_count: 1,
            anchor_report: None,
            timestamp: chrono::Utc::now(),
        }
    }

    fn commit_summary(value: TritValue) -> IterationSummary {
        IterationSummary {
            arbitration: ArbitrationResult::Commit(word(Frame::Science, value)),
            interrupt_count: 0,
            anchor_report: None,
            timestamp: chrono::Utc::now(),
        }
    }

    #[test]
    fn no_history_no_correction() {
        let mut m = AdaptiveIteration::new();
        let input = ModuleInput {
            signals: vec![],
            interrupts: vec![],
            attention_cmd: None,
        };
        let out = m.process(&input, &HookContext::default());
        assert_eq!(out.recommendation, TritValue::True);
    }

    #[test]
    fn detects_entrainment() {
        let mut m = AdaptiveIteration::new();
        let input = ModuleInput {
            signals: vec![],
            interrupts: vec![],
            attention_cmd: None,
        };

        // Feed 3 consecutive True results.
        let mut ctx = HookContext::default();
        ctx.record_iteration(commit_summary(TritValue::True));
        m.process(&input, &ctx);
        ctx.record_iteration(commit_summary(TritValue::True));
        m.process(&input, &ctx);
        ctx.record_iteration(commit_summary(TritValue::True));
        let out = m.process(&input, &ctx);

        assert_eq!(out.recommendation, TritValue::Hold);
        assert!(out.trace.contains("entrainment"));
    }

    #[test]
    fn detects_staleness() {
        let mut m = AdaptiveIteration::new();
        let input = ModuleInput {
            signals: vec![],
            interrupts: vec![],
            attention_cmd: None,
        };

        // Feed 3 consecutive Holds.
        let mut ctx = HookContext::default();
        for _ in 0..3 {
            ctx.record_iteration(hold_summary());
            m.process(&input, &ctx);
        }
        let out = m.process(&input, &ctx);

        assert_eq!(out.recommendation, TritValue::Hold);
        assert!(out.trace.contains("staleness"));
    }

    #[test]
    fn staleness_resets_on_non_hold() {
        let mut m = AdaptiveIteration::new();
        let input = ModuleInput {
            signals: vec![],
            interrupts: vec![],
            attention_cmd: None,
        };

        let mut ctx = HookContext::default();
        ctx.record_iteration(hold_summary());
        m.process(&input, &ctx);
        ctx.record_iteration(hold_summary());
        m.process(&input, &ctx);
        // Now a non-Hold result.
        ctx.record_iteration(commit_summary(TritValue::True));
        let out = m.process(&input, &ctx);

        assert_eq!(out.recommendation, TritValue::True);
        assert_eq!(m.staleness_counter, 0);
    }

    #[test]
    fn clear_on_unmount() {
        let mut m = AdaptiveIteration::new();
        let input = ModuleInput {
            signals: vec![],
            interrupts: vec![],
            attention_cmd: None,
        };
        let mut ctx = HookContext::default();
        ctx.record_iteration(commit_summary(TritValue::True));
        m.process(&input, &ctx);

        assert!(!m.recent_results().is_empty());
        m.on_unmount();
        assert!(m.recent_results().is_empty());
        assert_eq!(m.correction_count(), 0);
    }

    #[test]
    fn module_id_and_name() {
        let m = AdaptiveIteration::new();
        assert_eq!(m.id(), ModuleId::AdaptiveIteration);
        assert_eq!(m.name(), "adaptive_iteration");
    }
}
