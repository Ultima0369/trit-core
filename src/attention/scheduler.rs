//! Attention scheduling: engineering the "stop and turn" of mind.
//!
//! The scheduler observes recent signal patterns and suggests when the
//! system should shift focus, hold current processing, or recalibrate its
//! internal weights. It is a lightweight, heuristic layer intended to
//! interrupt loop entrainment and ruminative cascades.
//!
//! ## Adaptive scheduling (v0.3.0)
//!
//! Bandwidth is now a function of [`DepthLevel`](crate::budget::DepthLevel)
//! rather than a static value. High system load → low bandwidth → fewer
//! attention shifts. Idle → high bandwidth → more active attention management.
//!
//! Consecutive `HoldCurrent` suggestions are tracked: after N consecutive
//! holds (default 3), the scheduler escalates to `Recalibrate`.

use serde::{Deserialize, Serialize};

use crate::budget::{ComputeBudget, DepthLevel};
use crate::core::frame::Frame;
use crate::core::word::TritWord;

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

/// Target of an attention shift.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum ShiftTarget {
    /// Shift attention to body-state signals.
    Body,
    /// Shift attention to environmental signals.
    Environment,
    /// Shift attention to the current conflict trace.
    ConflictTrace,
    /// Shift attention to the meta/decision layer.
    Meta,
    /// Shift to a named frame.
    Frame(Frame),
    /// Shift to a named custom target.
    Label(String),
}

/// Command produced by the attention scheduler.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum AttentionCmd {
    /// Shift attention to the given target.
    ShiftTo(ShiftTarget),
    /// Pause current processing.
    HoldCurrent,
    /// Recalibrate internal weights.
    Recalibrate,
    /// No change needed.
    Continue,
}

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

/// Attention scheduler that monitors signal patterns and cognitive load.
///
/// Bandwidth is dynamically derived from the current [`ComputeBudget`]'s
/// [`DepthLevel`] rather than being a static value. The scheduler also
/// tracks consecutive `HoldCurrent` suggestions and escalates to
/// `Recalibrate` when the threshold is reached.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct AttentionScheduler {
    /// Estimated current cognitive bandwidth in `[0.0, 1.0]`.
    /// Derived from the current [`DepthLevel`] via [`bandwidth_from_depth`].
    pub bandwidth: f64,
    /// Current load profile across reactive/deliberative/reflective modes.
    pub load_profile: LoadProfile,
    /// Number of consecutive `HoldCurrent` suggestions.
    /// Reset to 0 on any non-HoldCurrent command.
    consecutive_holds: usize,
    /// Threshold for consecutive holds before escalating to Recalibrate.
    hold_escalation_threshold: usize,
}

impl AttentionScheduler {
    /// Create a scheduler with the given bandwidth and a balanced load profile.
    pub fn new(bandwidth: f64) -> Self {
        Self {
            bandwidth: bandwidth.clamp(0.0, 1.0),
            load_profile: LoadProfile::balanced(),
            consecutive_holds: 0,
            hold_escalation_threshold: 3,
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
    ///
    /// Returns true if fewer than 3 distinct frames appear in at least 6
    /// recent signals, or if the same frame appears more than 4 times.
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
    ///
    /// When depth is Minimal (bandwidth <= 0.2), always returns
    /// `Continue` — there is no time for attention shifts.
    ///
    /// Consecutive `HoldCurrent` suggestions are tracked: after
    /// `hold_escalation_threshold` consecutive holds, escalates to
    /// `Recalibrate`.
    pub fn suggest_reprioritization(&mut self, recent_signals: &[TritWord]) -> AttentionCmd {
        // Depth gating: at Minimal depth, hold current processing.
        // The pipeline should gate whether to call the scheduler at all
        // (via depth_level.has_extensions()), but if called at Minimal
        // depth, the safest command is HoldCurrent.
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

        // If the most recent signal is from an embodied frame and bandwidth
        // is moderate, suggest returning attention to the body.
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
    ///
    /// This is the preferred entry point for pipeline integration.
    /// It updates bandwidth from the budget before evaluating.
    pub fn suggest_with_budget(
        &mut self,
        budget: &ComputeBudget,
        recent_signals: &[TritWord],
    ) -> AttentionCmd {
        self.update_bandwidth(budget);
        self.suggest_reprioritization(recent_signals)
    }

    /// Track a HoldCurrent command. If consecutive holds exceed the
    /// escalation threshold, escalate to Recalibrate.
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
        // Minimal depth → HoldCurrent (not Continue).
        // The pipeline gates attention at the stage level.
        let mut scheduler = AttentionScheduler::from_depth(DepthLevel::Minimal);
        let signals = vec![word(Frame::Science, TritValue::True)];
        let cmd = scheduler.suggest_reprioritization(&signals);
        assert_eq!(cmd, AttentionCmd::HoldCurrent);
    }

    #[test]
    fn depth_reduced_still_runs_attention() {
        // Reduced (0.4) is above the Minimal gate (0.2), so attention runs.
        let mut scheduler = AttentionScheduler::from_depth(DepthLevel::Reduced);
        // With bandwidth 0.4 and an Embodied signal, should suggest Body shift
        let signals = vec![word(Frame::Embodied, TritValue::True)];
        let cmd = scheduler.suggest_reprioritization(&signals);
        assert_eq!(cmd, AttentionCmd::ShiftTo(ShiftTarget::Body));
    }

    // ── legacy tests (adapted for &mut self) ──────────────────────

    #[test]
    fn low_bandwidth_suggests_hold() {
        let mut scheduler = AttentionScheduler::new(0.1);
        let cmd = scheduler.suggest_reprioritization(&[]);
        // bandwidth 0.1 <= 0.2 → depth gate → HoldCurrent
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
        // bandwidth 0.19 <= 0.2 → depth gate → HoldCurrent.
        // After 3 consecutive HoldCurrent, escalates to Recalibrate.
        let mut scheduler = AttentionScheduler::new(0.19);
        assert_eq!(
            scheduler.suggest_reprioritization(&[]),
            AttentionCmd::HoldCurrent
        );
        assert_eq!(
            scheduler.suggest_reprioritization(&[]),
            AttentionCmd::HoldCurrent
        );
        // 3rd call: escalation threshold reached → Recalibrate
        assert_eq!(
            scheduler.suggest_reprioritization(&[]),
            AttentionCmd::Recalibrate
        );
        assert_eq!(scheduler.consecutive_holds(), 0);
    }

    #[test]
    fn hold_counter_resets_on_non_hold() {
        let mut scheduler = AttentionScheduler::new(0.19);
        // bandwidth 0.19 <= 0.2 → depth gate returns HoldCurrent
        // Two holds
        assert_eq!(
            scheduler.suggest_reprioritization(&[]),
            AttentionCmd::HoldCurrent
        );
        assert_eq!(
            scheduler.suggest_reprioritization(&[]),
            AttentionCmd::HoldCurrent
        );
        assert_eq!(scheduler.consecutive_holds(), 2);
        // Then a non-hold (loop entrainment) resets
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
}
