//! Attention scheduler adapter — cognitive resource allocation.
//!
//! The attention scheduler monitors signal patterns and cognitive load
//! to suggest attention commands. It detects loop entrainment and
//! escalates consecutive holds to recalibration.

use serde::{Deserialize, Serialize};

use crate::adapters::{
    adapter_lifecycle, AttentionCmd, CognitiveModule, ModuleInput, ModuleOutput, ShiftTarget,
};
use crate::budget::{ComputeBudget, DepthLevel};
use crate::core::frame::Frame;
use crate::core::interrupt::MetaInterrupt;
use crate::core::word::TritWord;
use crate::core::TritValue;
use crate::hook::module_registry::{ModuleId, ModuleState};
use crate::hook::HookContext;

// ── Load profile ────────────────────────────────────────────────────

/// Current distribution of task types being processed.
#[derive(Debug, Clone, Copy, PartialEq, Default, Deserialize, Serialize)]
pub struct LoadProfile {
    /// Fraction of cognitive resources spent on reactive processing.
    pub reactive: f64,
    /// Fraction of cognitive resources spent on deliberative processing.
    pub deliberative: f64,
    /// Fraction of cognitive resources spent on reflective processing.
    pub reflective: f64,
}

impl LoadProfile {
    /// Create a balanced load profile.
    pub fn balanced() -> Self {
        Self {
            reactive: 1.0 / 3.0,
            deliberative: 1.0 / 3.0,
            reflective: 1.0 / 3.0,
        }
    }

    /// Return true if any single load fraction exceeds 0.7.
    pub fn is_overloaded(&self) -> bool {
        self.reactive > 0.7 || self.deliberative > 0.7 || self.reflective > 0.7
    }
}

// ── Bandwidth from depth ────────────────────────────────────────────

/// Map [`DepthLevel`] to a bandwidth value in `[0.0, 1.0]`.
///
/// | DepthLevel | bandwidth |
/// |-----------|-----------|
/// | Minimal   | 0.2       |
/// | Reduced   | 0.4       |
/// | Standard  | 0.6       |
/// | Deep      | 0.8       |
/// | Exhaustive| 1.0       |
pub fn bandwidth_from_depth(depth: DepthLevel) -> f64 {
    match depth {
        DepthLevel::Minimal => 0.2,
        DepthLevel::Reduced => 0.4,
        DepthLevel::Standard => 0.6,
        DepthLevel::Deep => 0.8,
        DepthLevel::Exhaustive => 1.0,
    }
}

// ── Attention scheduler (inner engine) ──────────────────────────────

/// Attention scheduler that monitors signal patterns and cognitive load.
///
/// Bandwidth is dynamically derived from the current [`ComputeBudget`]'s
/// [`DepthLevel`] rather than being a static value. The scheduler also
/// tracks consecutive `HoldCurrent` suggestions and escalates to
/// `Recalibrate` when the threshold is reached.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct AttentionScheduler {
    /// Estimated current cognitive bandwidth in `[0.0, 1.0]`.
    pub bandwidth: f64,
    /// Current load profile across reactive/deliberative/reflective modes.
    pub load_profile: LoadProfile,
    /// Number of consecutive `HoldCurrent` suggestions.
    consecutive_holds: usize,
    /// Threshold for consecutive holds before escalating to Recalibrate.
    hold_escalation_threshold: usize,
    /// Lifecycle state for CognitiveModule.
    #[serde(skip, default)]
    state: ModuleState,
}

impl AttentionScheduler {
    /// Create a scheduler with the given bandwidth and a balanced load profile.
    pub fn new(bandwidth: f64) -> Self {
        Self {
            bandwidth: bandwidth.clamp(0.0, 1.0),
            load_profile: LoadProfile::balanced(),
            consecutive_holds: 0,
            hold_escalation_threshold: 3,
            state: ModuleState::Idle,
        }
    }

    /// Create a scheduler whose bandwidth is derived from a [`DepthLevel`].
    pub fn from_depth(depth: DepthLevel) -> Self {
        Self::new(bandwidth_from_depth(depth))
    }

    /// Create a scheduler whose bandwidth is derived from a [`ComputeBudget`].
    pub fn from_budget(budget: &ComputeBudget) -> Self {
        Self::from_depth(budget.depth_level)
    }

    /// Create a scheduler from a cognitive-state estimate.
    pub fn from_cognitive_state(attention_bandwidth: f64, cognitive_load: f64) -> Self {
        let bandwidth = (attention_bandwidth - cognitive_load).clamp(0.0, 1.0);
        Self::new(bandwidth)
    }

    /// Update the load profile.
    pub fn with_load_profile(mut self, profile: LoadProfile) -> Self {
        self.load_profile = profile;
        self
    }

    /// Set the consecutive hold escalation threshold.
    pub fn with_hold_escalation_threshold(mut self, threshold: usize) -> Self {
        self.hold_escalation_threshold = threshold;
        self
    }

    /// Update bandwidth from the current compute budget.
    pub fn update_bandwidth(&mut self, budget: &ComputeBudget) {
        self.bandwidth = bandwidth_from_depth(budget.depth_level);
    }

    /// Detect loop entrainment: a repetitive alternation among a small set
    /// of frames in the recent signal window.
    pub fn detect_loop_entrainment(&self, recent_signals: &[TritWord]) -> bool {
        if recent_signals.len() < 4 {
            return false;
        }
        let window = &recent_signals[recent_signals.len().saturating_sub(6)..];
        let mut counts: std::collections::HashMap<Frame, usize> = std::collections::HashMap::new();
        for word in window {
            *counts.entry(word.frame()).or_insert(0) += 1;
        }
        if counts.len() < 3 && window.len() >= 6 {
            return true;
        }
        counts.values().any(|&c| c > 4)
    }

    /// Suggest a reprioritization based on bandwidth, load, and optional
    /// recent signal history.
    pub fn suggest_reprioritization(&mut self, recent_signals: &[TritWord]) -> AttentionCmd {
        if self.bandwidth <= 0.2 {
            return self.track_hold(AttentionCmd::HoldCurrent);
        }

        if self.detect_loop_entrainment(recent_signals) {
            self.consecutive_holds = 0;
            return AttentionCmd::ShiftTo(ShiftTarget::ConflictTrace);
        }

        if self.bandwidth < 0.2 {
            return self.track_hold(AttentionCmd::HoldCurrent);
        }

        if self.load_profile.is_overloaded() {
            self.consecutive_holds = 0;
            return AttentionCmd::Recalibrate;
        }

        if let Some(last) = recent_signals.last() {
            if last.frame() == Frame::Embodied && self.bandwidth < 0.5 {
                self.consecutive_holds = 0;
                return AttentionCmd::ShiftTo(ShiftTarget::Body);
            }
        }

        self.consecutive_holds = 0;
        AttentionCmd::Continue
    }

    /// Suggest a reprioritization with an explicit compute budget.
    pub fn suggest_with_budget(
        &mut self,
        budget: &ComputeBudget,
        recent_signals: &[TritWord],
    ) -> AttentionCmd {
        self.update_bandwidth(budget);
        self.suggest_reprioritization(recent_signals)
    }

    fn track_hold(&mut self, cmd: AttentionCmd) -> AttentionCmd {
        if cmd == AttentionCmd::HoldCurrent {
            self.consecutive_holds += 1;
            if self.consecutive_holds >= self.hold_escalation_threshold {
                self.consecutive_holds = 0;
                return AttentionCmd::Recalibrate;
            }
        } else {
            self.consecutive_holds = 0;
        }
        cmd
    }

    /// Returns the number of consecutive HoldCurrent suggestions seen.
    pub fn consecutive_holds(&self) -> usize {
        self.consecutive_holds
    }
}

impl Default for AttentionScheduler {
    fn default() -> Self {
        Self::new(0.5)
    }
}

impl CognitiveModule for AttentionScheduler {
    adapter_lifecycle!();

    fn id(&self) -> ModuleId {
        ModuleId::AttentionScheduler
    }

    fn name(&self) -> &'static str {
        "attention_scheduler"
    }

    fn process(&mut self, input: &ModuleInput, ctx: &HookContext) -> ModuleOutput {
        self.state = ModuleState::Processing;

        // Derive a ComputeBudget from HookContext for bandwidth calculation.
        let depth_level = if ctx.compute_budget < 0.2 {
            DepthLevel::Minimal
        } else if ctx.compute_budget < 0.4 {
            DepthLevel::Reduced
        } else if ctx.compute_budget < 0.7 {
            DepthLevel::Standard
        } else if ctx.compute_budget < 0.9 {
            DepthLevel::Deep
        } else {
            DepthLevel::Exhaustive
        };
        let budget = ComputeBudget::new(depth_level, 0.5, 0.5, 4);
        let cmd = self.suggest_with_budget(&budget, &input.signals);

        self.state = ModuleState::Completed;

        match cmd {
            AttentionCmd::Continue => {
                ModuleOutput::new(TritValue::True, 0.8, "attention: no shift needed")
            }
            AttentionCmd::HoldCurrent => {
                ModuleOutput::new(TritValue::Hold, 0.6, "attention: holding current focus")
            }
            AttentionCmd::ShiftTo(ref target) => {
                let trace = format!("attention: shift to {:?}", target);
                ModuleOutput::new(TritValue::Hold, 0.5, trace)
            }
            AttentionCmd::Recalibrate => {
                let interrupt = MetaInterrupt::policy_violation(
                    crate::core::interrupt::PolicyViolation::Other(
                        "attention recalibrate".to_string(),
                    ),
                    "attention scheduler escalated to recalibrate after consecutive holds"
                        .to_string(),
                );
                ModuleOutput::new(TritValue::Hold, 0.4, "attention: recalibrate triggered")
                    .with_interrupts(vec![interrupt])
            }
        }
    }

    // ponytail: lifecycle generated by adapter_lifecycle!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::phase::Phase;
    use crate::core::value::TritValue;

    fn word(frame: Frame, value: TritValue) -> TritWord {
        TritWord::new(value, Phase::neutral(), frame)
    }

    // ── bandwidth_from_depth ──────────────────────────────────────

    #[test]
    fn bandwidth_from_depth_maps_correctly() {
        assert_float_eq!(bandwidth_from_depth(DepthLevel::Minimal), 0.2);
        assert_float_eq!(bandwidth_from_depth(DepthLevel::Reduced), 0.4);
        assert_float_eq!(bandwidth_from_depth(DepthLevel::Standard), 0.6);
        assert_float_eq!(bandwidth_from_depth(DepthLevel::Deep), 0.8);
        assert_float_eq!(bandwidth_from_depth(DepthLevel::Exhaustive), 1.0);
    }

    #[test]
    fn from_depth_uses_correct_bandwidth() {
        let scheduler = AttentionScheduler::from_depth(DepthLevel::Deep);
        assert_float_eq!(scheduler.bandwidth, 0.8);
    }

    #[test]
    fn from_budget_uses_correct_bandwidth() {
        let budget = ComputeBudget::new(DepthLevel::Reduced, 0.6, 0.5, 4);
        let scheduler = AttentionScheduler::from_budget(&budget);
        assert_float_eq!(scheduler.bandwidth, 0.4);
    }

    // ── depth gating ──────────────────────────────────────────────

    #[test]
    fn depth_below_standard_always_continues() {
        let mut scheduler = AttentionScheduler::from_depth(DepthLevel::Minimal);
        let signals = vec![word(Frame::Science, TritValue::True)];
        let cmd = scheduler.suggest_reprioritization(&signals);
        assert_eq!(cmd, AttentionCmd::HoldCurrent);
    }

    #[test]
    fn depth_reduced_still_runs_attention() {
        let mut scheduler = AttentionScheduler::from_depth(DepthLevel::Reduced);
        let signals = vec![word(Frame::Embodied, TritValue::True)];
        let cmd = scheduler.suggest_reprioritization(&signals);
        assert_eq!(cmd, AttentionCmd::ShiftTo(ShiftTarget::Body));
    }

    // ── legacy tests ──────────────────────────────────────────────

    #[test]
    fn low_bandwidth_suggests_hold() {
        let mut scheduler = AttentionScheduler::new(0.1);
        let cmd = scheduler.suggest_reprioritization(&[]);
        assert_eq!(cmd, AttentionCmd::HoldCurrent);
    }

    #[test]
    fn loop_entrainment_detected() {
        let mut scheduler = AttentionScheduler::new(0.6);
        let signals: Vec<_> = (0..6)
            .map(|i| {
                if i % 2 == 0 {
                    word(Frame::Science, TritValue::True)
                } else {
                    word(Frame::Individual, TritValue::False)
                }
            })
            .collect();
        assert!(scheduler.detect_loop_entrainment(&signals));
        assert!(matches!(
            scheduler.suggest_reprioritization(&signals),
            AttentionCmd::ShiftTo(ShiftTarget::ConflictTrace)
        ));
    }

    #[test]
    fn overloaded_load_suggests_recalibrate() {
        let mut scheduler = AttentionScheduler::new(0.6).with_load_profile(LoadProfile {
            reactive: 0.8,
            deliberative: 0.1,
            reflective: 0.1,
        });
        assert!(matches!(
            scheduler.suggest_reprioritization(&[]),
            AttentionCmd::Recalibrate
        ));
    }

    #[test]
    fn embodied_signal_suggests_body_shift_when_low_bandwidth() {
        let mut scheduler = AttentionScheduler::new(0.4);
        let signals = vec![word(Frame::Embodied, TritValue::True)];
        assert!(matches!(
            scheduler.suggest_reprioritization(&signals),
            AttentionCmd::ShiftTo(ShiftTarget::Body)
        ));
    }

    // ── consecutive hold escalation ───────────────────────────────

    #[test]
    fn consecutive_holds_escalates_to_recalibrate() {
        let mut scheduler = AttentionScheduler::new(0.19);
        assert_eq!(
            scheduler.suggest_reprioritization(&[]),
            AttentionCmd::HoldCurrent
        );
        assert_eq!(
            scheduler.suggest_reprioritization(&[]),
            AttentionCmd::HoldCurrent
        );
        assert_eq!(
            scheduler.suggest_reprioritization(&[]),
            AttentionCmd::Recalibrate
        );
        assert_eq!(scheduler.consecutive_holds(), 0);
    }

    #[test]
    fn hold_counter_resets_on_non_hold() {
        let mut scheduler = AttentionScheduler::new(0.19);
        assert_eq!(
            scheduler.suggest_reprioritization(&[]),
            AttentionCmd::HoldCurrent
        );
        assert_eq!(
            scheduler.suggest_reprioritization(&[]),
            AttentionCmd::HoldCurrent
        );
        assert_eq!(scheduler.consecutive_holds(), 2);
        let signals: Vec<_> = (0..6)
            .map(|i| {
                if i % 2 == 0 {
                    word(Frame::Science, TritValue::True)
                } else {
                    word(Frame::Individual, TritValue::False)
                }
            })
            .collect();
        scheduler.suggest_reprioritization(&signals);
        assert_eq!(scheduler.consecutive_holds(), 0);
    }

    #[test]
    fn suggest_with_budget_updates_bandwidth() {
        let mut scheduler = AttentionScheduler::new(0.5);
        let budget = ComputeBudget::new(DepthLevel::Deep, 0.1, 0.2, 8);
        let _ = scheduler.suggest_with_budget(&budget, &[]);
        assert_float_eq!(scheduler.bandwidth, 0.8);
    }

    #[test]
    fn update_bandwidth_syncs_from_budget() {
        let mut scheduler = AttentionScheduler::new(0.5);
        let budget = ComputeBudget::new(DepthLevel::Exhaustive, 0.0, 0.0, 16);
        scheduler.update_bandwidth(&budget);
        assert_float_eq!(scheduler.bandwidth, 1.0);
    }

    // ── CognitiveModule tests ─────────────────────────────────────

    #[test]
    fn attention_scheduler_module_id() {
        let module = AttentionScheduler::default();
        assert_eq!(module.id(), ModuleId::AttentionScheduler);
        assert_eq!(module.name(), "attention_scheduler");
    }

    #[test]
    fn attention_scheduler_process_continue() {
        let mut module = AttentionScheduler::new(0.8); // high bandwidth, balanced load
        let input = ModuleInput {
            signals: vec![],
            interrupts: vec![],
            attention_cmd: None,
        };
        let ctx = HookContext::default();
        let out = module.process(&input, &ctx);
        assert_eq!(out.recommendation, TritValue::True);
        assert!(out.confidence > 0.7);
    }

    #[test]
    fn attention_scheduler_process_hold_on_low_bandwidth() {
        let mut module = AttentionScheduler::new(0.1); // very low bandwidth
        let input = ModuleInput {
            signals: vec![],
            interrupts: vec![],
            attention_cmd: None,
        };
        let mut ctx = HookContext::default();
        ctx.update_budget(0.1); // force minimal depth → hold
        let out = module.process(&input, &ctx);
        assert_eq!(out.recommendation, TritValue::Hold);
    }

    #[test]
    fn attention_scheduler_lifecycle() {
        let mut module = AttentionScheduler::default();
        assert_eq!(module.state(), ModuleState::Idle);

        module.on_unmount();
        assert_eq!(module.state(), ModuleState::Completed);

        module.on_mount();
        assert_eq!(module.state(), ModuleState::Idle);
    }
}
