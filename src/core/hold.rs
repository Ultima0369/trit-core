//! Intelligent Hold state for mind-engineering extensions.
//!
//! In Trit-Core, `Hold` is not a failure mode — it is an intentional
//! suspension of judgment when conflicting reference frames are detected.
//! This module models the *finality* and *questionability* of a Hold so
//! that downstream systems can decide whether to wait, ask, or accept.

use serde::{Deserialize, Serialize};

/// Finality classification of a `Hold` result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum HoldFinality {
    /// The Hold is the final answer for this domain; no further input is
    /// expected to resolve the conflict.
    Final,
    /// The Hold is waiting for a follow-up question or clarification.
    AwaitingQuestion,
    /// The Hold could be resolved if additional information arrives within
    /// the configured question window.
    Resolvable,
    /// The Hold budget was exhausted — the system escalated to Layer 1
    /// anchor check. This finality is set by the Hook Manager when
    /// `hold_cycle_count >= hold_budget`.
    Expired,
}

/// State attached to a `Hold` output, describing how it should be treated.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct HoldState {
    /// Whether this Hold is final, awaiting a question, or resolvable.
    pub finality: HoldFinality,
    /// Window (in milliseconds) within which a follow-up question may
    /// transition the Hold to a resolved state. `0` means no automatic
    /// questioning.
    pub question_window_ms: u64,
}

impl HoldState {
    /// Create a final Hold.
    pub fn final_hold() -> Self {
        Self {
            finality: HoldFinality::Final,
            question_window_ms: 0,
        }
    }

    /// Create a Hold that is awaiting a follow-up question.
    pub fn awaiting(question_window_ms: u64) -> Self {
        Self {
            finality: HoldFinality::AwaitingQuestion,
            question_window_ms,
        }
    }

    /// Create a Hold that is resolvable within the given window.
    pub fn resolvable(question_window_ms: u64) -> Self {
        Self {
            finality: HoldFinality::Resolvable,
            question_window_ms,
        }
    }

    /// Returns true if the Hold decision path can be audited back to the
    /// conflicts that produced it.
    pub fn is_auditable(&self) -> bool {
        true
    }

    /// Returns true if this Hold is the final answer (not waiting).
    pub fn is_final(&self) -> bool {
        self.finality == HoldFinality::Final
    }

    /// Returns true if the Hold can be questioned / thawed.
    pub fn can_be_questioned(&self) -> bool {
        matches!(
            self.finality,
            HoldFinality::AwaitingQuestion | HoldFinality::Resolvable
        )
    }
}

impl Default for HoldState {
    fn default() -> Self {
        Self::final_hold()
    }
}

/// Configuration for how Hold states are produced across domains.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct HolderConfig {
    /// Domains for which a Hold is treated as final by default.
    #[serde(default)]
    pub hold_is_final_by_domain: Vec<String>,
    /// Default window (in milliseconds) after which an awaiting Hold is
    /// automatically considered final if no question arrives.
    #[serde(default)]
    pub auto_question_after_ms: u64,
}

impl HolderConfig {
    /// Create a config where Hold is final in every domain.
    pub fn final_everywhere() -> Self {
        Self {
            hold_is_final_by_domain: vec![],
            auto_question_after_ms: 0,
        }
    }

    /// Mark Hold as final for the named domain.
    pub fn with_final_domain(mut self, domain: impl Into<String>) -> Self {
        let name = domain.into();
        if !self.hold_is_final_by_domain.contains(&name) {
            self.hold_is_final_by_domain.push(name);
        }
        self
    }

    /// Returns the HoldState to use for a Hold produced in `domain`.
    pub fn hold_state_for(&self, domain: &str) -> HoldState {
        if self.hold_is_final_by_domain.contains(&domain.to_string()) {
            HoldState::final_hold()
        } else if self.auto_question_after_ms > 0 {
            HoldState::awaiting(self.auto_question_after_ms)
        } else {
            HoldState::final_hold()
        }
    }
}

impl Default for HolderConfig {
    fn default() -> Self {
        Self::final_everywhere()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn final_hold_is_not_questionable() {
        let h = HoldState::final_hold();
        assert!(h.is_auditable());
        assert!(h.is_final());
        assert!(!h.can_be_questioned());
    }

    #[test]
    fn awaiting_hold_is_questionable() {
        let h = HoldState::awaiting(500);
        assert!(!h.is_final());
        assert!(h.can_be_questioned());
        assert_eq!(h.question_window_ms, 500);
    }

    #[test]
    fn holder_config_final_by_domain() {
        let cfg = HolderConfig::default().with_final_domain("ValueJudgment");
        assert!(cfg.hold_state_for("ValueJudgment").is_final());
        assert!(cfg.hold_state_for("General").is_final());
    }

    #[test]
    fn holder_config_awaiting_when_window_set() {
        let cfg = HolderConfig {
            hold_is_final_by_domain: vec![],
            auto_question_after_ms: 1000,
        };
        let h = cfg.hold_state_for("General");
        assert!(!h.is_final());
        assert!(h.can_be_questioned());
    }
}
