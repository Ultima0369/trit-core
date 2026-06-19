//! Attention scheduling: engineering the "stop and turn" of mind.
//!
//! The scheduler observes recent signal patterns and suggests when the
//! system should shift focus, hold current processing, or recalibrate its
//! internal weights. It is a lightweight, heuristic layer intended to
//! interrupt loop entrainment and ruminative cascades.

use serde::{Deserialize, Serialize};

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

/// Attention scheduler that monitors signal patterns and cognitive load.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct AttentionScheduler {
    /// Estimated current cognitive bandwidth in `[0.0, 1.0]`.
    pub bandwidth: f64,
    /// Current load profile across reactive/deliberative/reflective modes.
    pub load_profile: LoadProfile,
}

impl AttentionScheduler {
    /// Create a scheduler with the given bandwidth and a balanced load profile.
    pub fn new(bandwidth: f64) -> Self {
        Self {
            bandwidth: bandwidth.clamp(0.0, 1.0),
            load_profile: LoadProfile::balanced(),
        }
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
    pub fn suggest_reprioritization(&self, recent_signals: &[TritWord]) -> AttentionCmd {
        if self.detect_loop_entrainment(recent_signals) {
            return AttentionCmd::ShiftTo(ShiftTarget::ConflictTrace);
        }

        if self.bandwidth < 0.2 {
            return AttentionCmd::HoldCurrent;
        }

        if self.load_profile.is_overloaded() {
            return AttentionCmd::Recalibrate;
        }

        // If the most recent signal is from an embodied frame and bandwidth
        // is moderate, suggest returning attention to the body.
        if let Some(last) = recent_signals.last() {
            if last.frame() == Frame::Embodied && self.bandwidth < 0.5 {
                return AttentionCmd::ShiftTo(ShiftTarget::Body);
            }
        }

        AttentionCmd::Continue
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

    #[test]
    fn low_bandwidth_suggests_hold() {
        let scheduler = AttentionScheduler::new(0.1);
        let cmd = scheduler.suggest_reprioritization(&[]);
        assert_eq!(cmd, AttentionCmd::HoldCurrent);
    }

    #[test]
    fn loop_entrainment_detected() {
        let scheduler = AttentionScheduler::new(0.6);
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
        let scheduler = AttentionScheduler::new(0.6).with_load_profile(LoadProfile {
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
        let scheduler = AttentionScheduler::new(0.4);
        let signals = vec![word(Frame::Embodied, TritValue::True)];
        assert!(matches!(
            scheduler.suggest_reprioritization(&signals),
            AttentionCmd::ShiftTo(ShiftTarget::Body)
        ));
    }
}
