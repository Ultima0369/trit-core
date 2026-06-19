//! Hook manager layer: scenario perception and module scheduling.
//!
//! The hook manager recognizes the current scenario type from input signals,
//! mounts the appropriate cognitive modules, and unmounts them when the
//! scenario changes. It is the "attention scheduler" of the system.

pub mod scenario_recognizer;

use std::time::{Duration, Instant};

pub use scenario_recognizer::{FeatureVector, ScenarioRecognizer, ScenarioType};

/// Strategy for handling a Hold result from the ternary engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HoldStrategy {
    /// Wait for more signal input — the current input is insufficient.
    #[default]
    WaitForMoreData,
    /// External clarification required — human or external system must intervene.
    WaitForHumanClarification,
    /// Defer to the next decision cycle without additional input.
    DeferToNextCycle,
    /// If Hold persists beyond the budget, escalate to Layer 1 anchor check.
    EscalateToLayer1,
}

/// Reason a module was unmounted (for audit trail).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnmountReason {
    Completed,
    Timeout,
    Preempted,
    AnchorViolation,
}

/// Summary of one completed iteration.
#[derive(Debug, Clone, PartialEq)]
pub struct IterationSummary {
    pub scenario: ScenarioType,
    pub modules_mounted: Vec<String>,
    pub decision_value: String,
    pub hold_cycles: u32,
}

/// The sole communication bus between Layer 2 and Layer 3.
///
/// Modules read from this context but do NOT mutate it.
/// Only the Hook Manager writes to it.
#[derive(Debug, Clone, PartialEq)]
pub struct HookContext {
    /// Current scenario type.
    pub scenario: ScenarioType,
    /// How long the current scenario has been active.
    pub scenario_duration: Duration,
    /// Results from the previous iteration, if any.
    pub previous_iteration: Option<Box<IterationSummary>>,
    /// Available compute budget (normalized 0.0-1.0).
    pub compute_budget: f64,
    /// Available time budget (wall-clock deadline, if any).
    pub time_budget: Option<Instant>,
    /// The current hold strategy.
    pub hold_strategy: HoldStrategy,
    /// Number of consecutive Hold cycles so far.
    pub hold_cycle_count: u32,
    /// Maximum Hold cycles before escalation (default: 3).
    pub hold_budget: u32,
}

impl HookContext {
    pub fn new(scenario: ScenarioType) -> Self {
        HookContext {
            scenario,
            scenario_duration: Duration::ZERO,
            previous_iteration: None,
            compute_budget: 1.0,
            time_budget: None,
            hold_strategy: HoldStrategy::default(),
            hold_cycle_count: 0,
            hold_budget: 3,
        }
    }

    /// Increment hold cycle count. Returns true if budget is exceeded.
    pub fn increment_hold(&mut self) -> bool {
        self.hold_cycle_count += 1;
        if self.hold_cycle_count >= self.hold_budget {
            self.hold_strategy = HoldStrategy::EscalateToLayer1;
            true
        } else {
            false
        }
    }

    /// Reset hold tracking after a non-Hold decision.
    pub fn reset_hold(&mut self) {
        self.hold_cycle_count = 0;
        self.hold_strategy = HoldStrategy::default();
    }

    /// Update elapsed time since scenario start.
    pub fn tick(&mut self, elapsed: Duration) {
        self.scenario_duration = elapsed;
    }
}

impl Default for HookContext {
    fn default() -> Self {
        HookContext::new(ScenarioType::General)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hold_budget_escalates_after_three_cycles() {
        let mut ctx = HookContext::new(ScenarioType::General);
        assert!(!ctx.increment_hold());
        assert!(!ctx.increment_hold());
        assert!(ctx.increment_hold());
        assert_eq!(ctx.hold_strategy, HoldStrategy::EscalateToLayer1);
    }

    #[test]
    fn reset_hold_clears_escalation() {
        let mut ctx = HookContext::new(ScenarioType::General);
        ctx.increment_hold();
        ctx.increment_hold();
        ctx.increment_hold();
        assert!(ctx.hold_cycle_count >= ctx.hold_budget);
        ctx.reset_hold();
        assert_eq!(ctx.hold_cycle_count, 0);
        assert_eq!(ctx.hold_strategy, HoldStrategy::default());
    }

    #[test]
    fn hook_context_default_values() {
        let ctx = HookContext::default();
        assert_eq!(ctx.scenario, ScenarioType::General);
        assert_eq!(ctx.hold_budget, 3);
        assert_eq!(ctx.compute_budget, 1.0);
        assert!(ctx.time_budget.is_none());
    }
}
