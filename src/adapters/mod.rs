//! Dynamic adapter module pool — cognitive modules for Layer 3.
//!
//! Each module implements [`CognitiveModule`] and is mounted/unmounted by
//! the Layer 2 [`HookManager`](crate::hook::HookManager) according to
//! scenario needs. No module is "always on" — even the reflexive auditor
//! runs only when the scenario demands it.
//!
//! # Design rules
//!
//! - Modules do NOT call each other. All cross-module communication goes
//!   through [`HookContext`](crate::hook::HookContext).
//! - Every module output includes a `confidence` score in [0.0, 1.0].
//! - Unmount = release. No background processing after unmount.
//! - The `AdaptiveIteration` module is the only module permitted to suggest
//!   parameter changes — and its changes are bounded (cannot modify anchors
//!   or core algebra).

pub mod adaptive_iteration;
pub mod bandwidth_scheduler;
pub mod cognitive_deconstruction;
pub mod conflict_suspension;
pub mod coupling_adapter;
pub mod critical_thinking;
pub mod ecological_assessment;
pub mod engineering;
pub mod reflexive_audit;
pub mod self_knowledge;

use crate::core::TritValue;
use crate::core::TritWord;
use crate::hook::module_registry::{ModuleId, ModuleState};
use crate::hook::HookContext;
use crate::meta::MetaInterrupt;

// ── Module I/O ──────────────────────────────────────────────────────

/// Input to a cognitive module's [`CognitiveModule::process`] method.
///
/// Carries the signals and interrupts the module should reason about,
/// plus an optional attention command from the bandwidth scheduler.
#[derive(Debug, Clone)]
pub struct ModuleInput {
    /// TritWords the module should reason about.
    pub signals: Vec<TritWord>,
    /// MetaInterrupts from the most recent decision cycle.
    pub interrupts: Vec<MetaInterrupt>,
    /// Optional attention command from the bandwidth scheduler.
    pub attention_cmd: Option<AttentionCmd>,
}

/// Output from a cognitive module's [`CognitiveModule::process`] method.
///
/// Every module must return a recommendation with a confidence score.
/// Low-confidence outputs are flagged by the Hook Manager.
#[derive(Debug, Clone)]
pub struct ModuleOutput {
    /// The module's recommendation.
    pub recommendation: TritValue,
    /// Confidence in [0.0, 1.0].
    pub confidence: f64,
    /// Any interrupts the module wants to raise.
    pub interrupts: Vec<MetaInterrupt>,
    /// Human-readable reasoning trace.
    pub trace: String,
}

impl ModuleOutput {
    /// Create a new module output.
    pub fn new(recommendation: TritValue, confidence: f64, trace: impl Into<String>) -> Self {
        ModuleOutput {
            recommendation,
            confidence: confidence.clamp(0.0, 1.0),
            interrupts: Vec::new(),
            trace: trace.into(),
        }
    }

    /// Add interrupts to the output.
    pub fn with_interrupts(mut self, interrupts: Vec<MetaInterrupt>) -> Self {
        self.interrupts = interrupts;
        self
    }
}

// ── Re-exported types (from migrated modules) ───────────────────────

/// Attention command from the bandwidth scheduler.
///
/// Re-exported from the `bandwidth_scheduler` module; kept here for
/// use in [`ModuleInput`] without circular imports.
#[derive(Debug, Clone, PartialEq)]
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

/// Target of an attention shift.
#[derive(Debug, Clone, PartialEq)]
pub enum ShiftTarget {
    Body,
    Environment,
    ConflictTrace,
    Meta,
    Frame(crate::core::Frame),
    Label(String),
}

// ── CognitiveModule trait ───────────────────────────────────────────

/// Core trait for all Layer 3 adapter modules.
///
/// Every module in the adapter pool implements this trait. Modules are
/// mounted/unmounted by the Hook Manager and communicate exclusively
/// through [`HookContext`].
///
/// # Default implementations
///
/// - [`on_mount`](CognitiveModule::on_mount) and [`on_unmount`](CognitiveModule::on_unmount)
///   default to no-ops — most modules don't need lifecycle hooks.
/// - [`calibrate`](CognitiveModule::calibrate) defaults to returning `0.0` —
///   only modules that participate in the feedback loop override it.
pub trait CognitiveModule: Send + Sync {
    /// Unique module identifier.
    fn id(&self) -> ModuleId;

    /// Human-readable module name.
    fn name(&self) -> &'static str;

    /// Process input signals and return a recommendation.
    ///
    /// Modules read from `ctx` but do NOT mutate it.
    fn process(&mut self, input: &ModuleInput, ctx: &HookContext) -> ModuleOutput;

    /// Called when the module is mounted by the Hook Manager.
    ///
    /// Use this to allocate resources or initialize state.
    fn on_mount(&mut self) {}

    /// Called when the module is unmounted by the Hook Manager.
    ///
    /// The module must persist any state it needs and release
    /// computational resources. No background processing after this call.
    fn on_unmount(&mut self) {}

    /// Current lifecycle state of the module.
    fn state(&self) -> ModuleState;

    /// Calibrate internal weights from a feedback signal.
    ///
    /// Returns the magnitude of the adjustment in [0.0, 1.0].
    /// Default: no-op returning 0.0.
    ///
    /// This is called by the Layer 5 feedback loop. Until Layer 5 is
    /// implemented, this method is never invoked.
    fn calibrate(&mut self, _feedback: &FeedbackSignal) -> f64 {
        0.0
    }
}

// ── Feedback signal (from Layer 5) ─────────────────────────────────

/// Feedback signal from Layer 5's practice testing.
///
/// Re-exported from [`crate::feedback::FeedbackSignal`]. This replaces
/// the v0.3.0 placeholder with the real Layer 5 type.
pub use crate::feedback::FeedbackSignal;

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal test module that implements CognitiveModule.
    struct TestModule {
        mounted: bool,
    }

    impl CognitiveModule for TestModule {
        fn id(&self) -> ModuleId {
            ModuleId::SelfKnowledge
        }

        fn name(&self) -> &'static str {
            "test_module"
        }

        fn process(&mut self, _input: &ModuleInput, _ctx: &HookContext) -> ModuleOutput {
            ModuleOutput::new(TritValue::True, 0.9, "test pass-through")
        }

        fn state(&self) -> ModuleState {
            if self.mounted {
                ModuleState::Idle
            } else {
                ModuleState::Completed
            }
        }

        fn on_mount(&mut self) {
            self.mounted = true;
        }

        fn on_unmount(&mut self) {
            self.mounted = false;
        }
    }

    #[test]
    fn module_output_confidence_clamped() {
        let out = ModuleOutput::new(TritValue::True, 1.5, "overconfident");
        assert_float_eq!(out.confidence, 1.0);

        let out = ModuleOutput::new(TritValue::False, -0.3, "underconfident");
        assert_float_eq!(out.confidence, 0.0);
    }

    #[test]
    fn module_output_with_interrupts() {
        let interrupt = MetaInterrupt::new(
            crate::meta::ConflictType::FrameMismatch,
            "test conflict".to_string(),
        );
        let out = ModuleOutput::new(TritValue::Hold, 0.5, "conflict detected")
            .with_interrupts(vec![interrupt]);
        assert_eq!(out.interrupts.len(), 1);
    }

    #[test]
    fn cognitive_module_lifecycle() {
        let mut m = TestModule { mounted: false };
        assert_eq!(m.state(), ModuleState::Completed);

        m.on_mount();
        assert_eq!(m.state(), ModuleState::Idle);

        m.on_unmount();
        assert_eq!(m.state(), ModuleState::Completed);
    }

    #[test]
    fn cognitive_module_process() {
        let mut m = TestModule { mounted: true };
        let input = ModuleInput {
            signals: vec![],
            interrupts: vec![],
            attention_cmd: None,
        };
        let ctx = HookContext::default();
        let out = m.process(&input, &ctx);
        assert_eq!(out.recommendation, TritValue::True);
        assert_float_eq!(out.confidence, 0.9);
    }

    #[test]
    fn default_calibrate_returns_zero() {
        let mut m = TestModule { mounted: false };
        let fb = FeedbackSignal {
            test_result: crate::feedback::PracticeTestResult::Matched { confidence: 0.9 },
            source_decision_id: "test".into(),
            deviation_delta: 0.0,
            recommended_scenario: None,
            anchor_violations: vec![],
        };
        assert_float_eq!(m.calibrate(&fb), 0.0);
    }

    #[test]
    fn module_output_trace_is_set() {
        let out = ModuleOutput::new(TritValue::Hold, 0.5, "insufficient evidence");
        assert_eq!(out.trace, "insufficient evidence");
    }
}
