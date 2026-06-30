//! Hook Manager — scenario perception and module scheduling (Layer 2).
//!
//! The Hook Manager is the "perceptual foundation" of the 5-layer cognitive
//! architecture. It recognizes the current scenario type, mounts/unmounts
//! adapter modules, and provides the [`HookContext`] communication bus
//! through which modules read scenario state.
//!
//! # Design rules
//!
//! - Modules READ from `HookContext` but do NOT mutate it.
//! - Only the Hook Manager writes to `HookContext`.
//! - All cross-module communication goes through `HookContext`.
//! - Unmount = release; no background processing after unmount.

pub mod module_registry;
pub mod mount_arbiter;
pub mod scenario_recognizer;

use std::time::{Duration, Instant};

use crate::anchor::AnchorReport;
use crate::core::hold::HoldFinality;
use crate::meta::ArbitrationResult;

// ── Scenario types ─────────────────────────────────────────────────

/// Known scenario types the recognizer can identify.
///
/// Each type maps to a default set of modules that should be mounted
/// when the scenario activates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScenarioType {
    /// Causal chain analysis + boundary condition checking.
    PhysicalReasoning,
    /// Cross-frame comparison + conflict suspension.
    ValueConflict,
    /// Individual priority + non-maleficence.
    MedicalEthics,
    /// System inspects its own decision path.
    ReflexiveAudit,
    /// Time pressure + constraint checking.
    CrisisResponse,
    /// No specific prototype matched — fallback.
    General,
}

impl ScenarioType {
    /// Human-readable label for this scenario type.
    pub fn as_str(&self) -> &'static str {
        match self {
            ScenarioType::PhysicalReasoning => "physical_reasoning",
            ScenarioType::ValueConflict => "value_conflict",
            ScenarioType::MedicalEthics => "medical_ethics",
            ScenarioType::ReflexiveAudit => "reflexive_audit",
            ScenarioType::CrisisResponse => "crisis_response",
            ScenarioType::General => "general",
        }
    }
}

impl std::fmt::Display for ScenarioType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ── Hold strategy ──────────────────────────────────────────────────

/// Governs what the system does when the ternary engine returns Hold.
///
/// Hold is not a failure — it is the active intermediate state of
/// "gathering more variables." This enum defines how long to gather
/// and what to do when gathering doesn't converge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HoldStrategy {
    /// Wait for more signal input — the current input is insufficient.
    #[default]
    WaitForMoreData,
    /// External clarification required — a human or external system
    /// must intervene.
    WaitForHumanClarification,
    /// Defer to the next decision cycle without additional input.
    DeferToNextCycle,
    /// If Hold persists beyond the budget, escalate to Layer 1 anchor check.
    /// This prevents indefinite suspension.
    EscalateToLayer1,
}

impl HoldStrategy {
    /// Returns true if this strategy involves waiting (not immediate escalation).
    pub fn is_waiting(&self) -> bool {
        matches!(
            self,
            HoldStrategy::WaitForMoreData
                | HoldStrategy::WaitForHumanClarification
                | HoldStrategy::DeferToNextCycle
        )
    }

    /// Returns true if this strategy triggers escalation.
    pub fn escalates(&self) -> bool {
        matches!(self, HoldStrategy::EscalateToLayer1)
    }
}

// ── Unmount reason ─────────────────────────────────────────────────

/// Why a module was unmounted — recorded for auditability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnmountReason {
    /// Scenario finished normally.
    Completed,
    /// Module exceeded its time budget.
    Timeout,
    /// Higher-priority scenario interrupted.
    Preempted,
    /// Layer 1 forced unmount (anchor violation).
    AnchorViolation,
}

impl std::fmt::Display for UnmountReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnmountReason::Completed => write!(f, "completed"),
            UnmountReason::Timeout => write!(f, "timeout"),
            UnmountReason::Preempted => write!(f, "preempted"),
            UnmountReason::AnchorViolation => write!(f, "anchor_violation"),
        }
    }
}

// ── Iteration summary ──────────────────────────────────────────────

/// Lightweight summary of a completed decision iteration.
///
/// Stored in [`HookContext`] so modules can reference the previous
/// iteration's result without retaining full pipeline state.
#[derive(Debug, Clone, PartialEq)]
pub struct IterationSummary {
    /// The arbitration result from the last iteration.
    pub arbitration: ArbitrationResult,
    /// How many interrupts were detected.
    pub interrupt_count: usize,
    /// The anchor report (if any violations).
    pub anchor_report: Option<AnchorReport>,
    /// Wall-clock time of the iteration.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

// ── Hook context ───────────────────────────────────────────────────

/// The sole communication channel between Layer 2 and Layer 3.
///
/// Modules read from `HookContext` but do NOT mutate it. Only the
/// Hook Manager writes to it. This enforces the rule: **modules do not
/// call each other; all cross-module communication goes through
/// HookContext.**
#[derive(Debug, Clone, PartialEq)]
pub struct HookContext {
    /// Current scenario type.
    pub scenario: ScenarioType,
    /// How long the current scenario has been active.
    pub scenario_duration: Duration,
    /// Results from the previous iteration, if any.
    pub previous_iteration: Option<IterationSummary>,
    /// Available compute budget (normalized 0.0–1.0).
    pub compute_budget: f64,
    /// Available time budget (wall-clock deadline, if any).
    pub time_budget: Option<Instant>,
    /// The current hold strategy for this context.
    pub hold_strategy: HoldStrategy,
    /// Number of consecutive Hold cycles so far.
    pub hold_cycle_count: u32,
    /// Maximum Hold cycles before escalation (default: 3).
    pub hold_budget: u32,
}

impl HookContext {
    /// Create a new HookContext for the given scenario type.
    pub fn new(scenario: ScenarioType) -> Self {
        HookContext {
            scenario,
            scenario_duration: Duration::ZERO,
            previous_iteration: None,
            compute_budget: 0.5,
            time_budget: None,
            hold_strategy: HoldStrategy::default(),
            hold_cycle_count: 0,
            hold_budget: 3,
        }
    }

    /// Returns true if the Hold budget is exhausted.
    pub fn hold_budget_exhausted(&self) -> bool {
        self.hold_cycle_count >= self.hold_budget
    }

    /// Increment the hold cycle counter.
    pub fn increment_hold_cycle(&mut self) {
        self.hold_cycle_count = self.hold_cycle_count.saturating_add(1);
    }

    /// Reset the hold cycle counter (e.g., on non-Hold result).
    pub fn reset_hold_cycle(&mut self) {
        self.hold_cycle_count = 0;
    }

    /// Escalate the hold strategy when the budget is exhausted.
    ///
    /// Returns the new `HoldFinality::Expired` if escalation occurred.
    pub fn escalate_if_exhausted(&mut self) -> Option<HoldFinality> {
        if self.hold_budget_exhausted() && !self.hold_strategy.escalates() {
            self.hold_strategy = HoldStrategy::EscalateToLayer1;
            Some(HoldFinality::Expired)
        } else {
            None
        }
    }

    /// Set the result of the previous iteration.
    pub fn record_iteration(&mut self, summary: IterationSummary) {
        self.previous_iteration = Some(summary);
    }

    /// Update compute budget from external sampling.
    pub fn update_budget(&mut self, budget: f64) {
        self.compute_budget = budget.clamp(0.0, 1.0);
    }

    /// Set a time budget (deadline).
    pub fn with_time_budget(mut self, deadline: Instant) -> Self {
        self.time_budget = Some(deadline);
        self
    }

    /// Returns true if the time budget is exceeded.
    pub fn time_budget_exceeded(&self) -> bool {
        self.time_budget
            .map(|d| Instant::now() >= d)
            .unwrap_or(false)
    }
}

impl Default for HookContext {
    fn default() -> Self {
        Self::new(ScenarioType::General)
    }
}

// ── Hook manager ───────────────────────────────────────────────────

/// Central orchestrator for Layer 2.
///
/// Owns the [`HookContext`], delegates scenario recognition to the
/// recognizer, and coordinates mount/unmount through the module
/// registry and mount arbiter.
#[derive(Debug)]
pub struct HookManager {
    /// Current hook context (the inter-layer communication bus).
    pub ctx: HookContext,
    /// When the current scenario was activated.
    scenario_start: Instant,
}

impl HookManager {
    /// Create a new HookManager with default context.
    pub fn new() -> Self {
        HookManager {
            ctx: HookContext::default(),
            scenario_start: Instant::now(),
        }
    }

    /// Create a HookManager for a specific scenario type.
    pub fn for_scenario(scenario: ScenarioType) -> Self {
        HookManager {
            ctx: HookContext::new(scenario),
            scenario_start: Instant::now(),
        }
    }

    /// Create a HookManager with a specific compute budget.
    pub fn with_budget(mut self, budget: f64) -> Self {
        self.ctx.update_budget(budget);
        self
    }

    /// Create a HookManager with a specific hold budget.
    pub fn with_hold_budget(mut self, budget: u32) -> Self {
        self.ctx.hold_budget = budget;
        self
    }

    /// Update the scenario duration from the internal clock.
    pub fn tick(&mut self) {
        self.ctx.scenario_duration = self.scenario_start.elapsed();
    }

    /// Record a decision iteration result.
    pub fn record(&mut self, summary: IterationSummary) {
        // Update hold cycle tracking.
        match &summary.arbitration {
            ArbitrationResult::Hold => {
                self.ctx.increment_hold_cycle();
            }
            _ => {
                self.ctx.reset_hold_cycle();
            }
        }
        self.ctx.record_iteration(summary);
    }

    /// Record a Hold result and check for budget exhaustion.
    ///
    /// Returns `Some(HoldFinality::Expired)` if escalation occurred.
    pub fn record_hold(&mut self, summary: IterationSummary) -> Option<HoldFinality> {
        self.ctx.increment_hold_cycle();
        self.ctx.record_iteration(summary);
        self.ctx.escalate_if_exhausted()
    }
}

impl Default for HookManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── ScenarioType ────────────────────────────────────────────

    #[test]
    fn scenario_type_display() {
        assert_eq!(ScenarioType::General.to_string(), "general");
        assert_eq!(ScenarioType::MedicalEthics.to_string(), "medical_ethics");
    }

    #[test]
    fn scenario_type_all_variants_have_labels() {
        for st in &[
            ScenarioType::PhysicalReasoning,
            ScenarioType::ValueConflict,
            ScenarioType::MedicalEthics,
            ScenarioType::ReflexiveAudit,
            ScenarioType::CrisisResponse,
            ScenarioType::General,
        ] {
            assert!(!st.as_str().is_empty());
        }
    }

    // ── HoldStrategy ────────────────────────────────────────────

    #[test]
    fn hold_strategy_is_waiting() {
        assert!(HoldStrategy::WaitForMoreData.is_waiting());
        assert!(HoldStrategy::WaitForHumanClarification.is_waiting());
        assert!(HoldStrategy::DeferToNextCycle.is_waiting());
        assert!(!HoldStrategy::EscalateToLayer1.is_waiting());
        assert!(HoldStrategy::EscalateToLayer1.escalates());
    }

    #[test]
    fn hold_strategy_default_is_waiting() {
        assert!(HoldStrategy::default().is_waiting());
    }

    // ── UnmountReason ───────────────────────────────────────────

    #[test]
    fn unmount_reason_display() {
        assert_eq!(UnmountReason::Completed.to_string(), "completed");
        assert_eq!(UnmountReason::Timeout.to_string(), "timeout");
        assert_eq!(UnmountReason::Preempted.to_string(), "preempted");
        assert_eq!(
            UnmountReason::AnchorViolation.to_string(),
            "anchor_violation"
        );
    }

    // ── HookContext ─────────────────────────────────────────────

    #[test]
    fn hook_context_starts_with_zero_cycles() {
        let ctx = HookContext::new(ScenarioType::General);
        assert_eq!(ctx.hold_cycle_count, 0);
        assert_eq!(ctx.hold_budget, 3);
        assert!(!ctx.hold_budget_exhausted());
    }

    #[test]
    fn hold_budget_exhausted_after_max_cycles() {
        let mut ctx = HookContext::new(ScenarioType::General);
        ctx.increment_hold_cycle();
        ctx.increment_hold_cycle();
        ctx.increment_hold_cycle();
        assert!(ctx.hold_budget_exhausted());
    }

    #[test]
    fn reset_hold_cycle_clears_counter() {
        let mut ctx = HookContext::new(ScenarioType::General);
        ctx.increment_hold_cycle();
        ctx.increment_hold_cycle();
        ctx.reset_hold_cycle();
        assert_eq!(ctx.hold_cycle_count, 0);
        assert!(!ctx.hold_budget_exhausted());
    }

    #[test]
    fn escalate_if_exhausted_triggers() {
        let mut ctx = HookContext::new(ScenarioType::General);
        ctx.hold_cycle_count = 3;
        let result = ctx.escalate_if_exhausted();
        assert_eq!(result, Some(HoldFinality::Expired));
        assert_eq!(ctx.hold_strategy, HoldStrategy::EscalateToLayer1);
    }

    #[test]
    fn escalate_does_not_double_trigger() {
        let mut ctx = HookContext::new(ScenarioType::General);
        ctx.hold_cycle_count = 3;
        ctx.hold_strategy = HoldStrategy::EscalateToLayer1;
        let result = ctx.escalate_if_exhausted();
        assert_eq!(result, None); // already escalating
    }

    // ── HookManager ─────────────────────────────────────────────

    #[test]
    fn hook_manager_default_is_general() {
        let hm = HookManager::new();
        assert_eq!(hm.ctx.scenario, ScenarioType::General);
    }

    #[test]
    fn hook_manager_for_scenario() {
        let hm = HookManager::for_scenario(ScenarioType::MedicalEthics);
        assert_eq!(hm.ctx.scenario, ScenarioType::MedicalEthics);
    }

    #[test]
    fn hook_manager_tick_updates_duration() {
        let mut hm = HookManager::new();
        assert_eq!(hm.ctx.scenario_duration, Duration::ZERO);
        hm.tick();
        assert!(hm.ctx.scenario_duration > Duration::ZERO);
    }
}
