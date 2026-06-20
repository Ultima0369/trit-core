//! Trit-Core — a ternary decision engine for conflict-aware AI alignment.
//!
//! This crate provides a multi-valued logic (MVL) computation framework where
//! each decision unit (trit) carries three computable states: `True`, `Hold`,
//! and `False`, plus an out-of-distribution `Unknown` state. Unlike binary
//! logic which forces a determination, Trit-Core introduces a `Hold` state
//! that represents intentional suspension of judgment when conflicting
//! decision domains are detected.
//!
//! # Version
//!
//! Current version: **0.3.0**.
//!
//! # Safety
//!
//! `#![forbid(unsafe_code)]` is enforced crate-wide. Warnings are denied in
//! CI via `RUSTFLAGS="-D warnings"` rather than in the library crate, to
//! avoid breaking downstream builds when new Rust versions introduce new
//! warnings.
//!
//! # Modules
//!
//! - [`anchor`] — steady-state constraints with veto power (Layer 1).
//! - [`core`] — ternary algebra and data types: `TritValue`, `Phase`, `Frame`,
//!   `TritWord`, `TernaryAlgebra`.
//! - [`meta`] — policy engine: `Domain`, `ResolutionPolicy`, `ArbitrationResult`,
//!   `MetaInterrupt`, `SafeFallback`, custom rules.
//! - [`sandbox`] — scenario I/O, validation, pipeline, and expected-behavior
//!   verification.
//! - [`adapter`] — cognitive module pool (Layer 3): scenario-driven module mounting.
//! - [`budget`] — hardware-aware compute budget and depth-level gating.
//! - [`calibration`] — decision history recording for feedback-driven learning.
//! - [`clock`] — phase oscillator and time-scale management.
//!
//! # Documentation
//!
//! See [`docs/INDEX.md`](https://github.com/trit-core/trit-core/blob/main/docs/INDEX.md)
//! for the full documentation map.
//!
//! # Quick Example
//!
//! ```rust
//! use trit_core::core::{Frame, TernaryAlgebra, TritValue, TritWord};
//!
//! let science = TritWord::tru(Frame::Science);
//! let individual = TritWord::fals(Frame::Individual);
//!
//! let (result, interrupt) = TernaryAlgebra::t_and(&science, &individual);
//!
//! assert_eq!(result.value(), TritValue::Hold);
//! assert!(interrupt.is_some());
//! ```

#![forbid(unsafe_code)]

/// Assert that two `f64` values are equal within `f64::EPSILON`.
///
/// Replaces the 43-instance pattern `assert!((actual - expected).abs() < f64::EPSILON)`
/// with a single readable macro. Accepts an optional custom message.
///
/// # Examples
///
/// ```ignore
/// assert_float_eq!(result.phase(), 0.55);
/// assert_float_eq!(clock.omega, 10.0, "physical clock should have ω=10.0");
/// ```
#[macro_export]
macro_rules! assert_float_eq {
    ($actual:expr, $expected:expr) => {
        assert!(
            ($actual - $expected).abs() < f64::EPSILON,
            "assertion failed: `(left == right)`\n  left: `{}`\n right: `{}`",
            $actual,
            $expected
        )
    };
    ($actual:expr, $expected:expr, $($arg:tt)+) => {
        assert!(
            ($actual - $expected).abs() < f64::EPSILON,
            $($arg)+
        )
    };
}

pub mod adapters;
pub mod anchor;
pub mod baseline;
pub mod budget;
pub mod calibration;
pub mod clock;
pub mod core;
pub mod feedback;
pub mod hook;
pub mod meta;
pub mod sandbox;
pub mod tracing_init;

pub use adapters::{
    adaptive_iteration::AdaptiveIteration,
    bandwidth_scheduler::{bandwidth_from_depth, AttentionScheduler, LoadProfile},
    cognitive_deconstruction::CognitiveDeconstruction,
    conflict_suspension::ConflictSuspension,
    coupling_adapter::CouplingAdapter,
    critical_thinking::CriticalThinking,
    ecological_assessment::EcologicalAssessment,
    engineering::EngineeringArchitecture,
    reflexive_audit::{
        AttentionEvent, AuditReport, PhaseShift, ReflexiveAlert, ReflexiveAuditModule,
        ReflexiveAuditor,
    },
    self_knowledge::{
        CalibrationEvent, ReceiverEstimate, ResponsePattern, SelfKnowledge, SelfKnowledgeModule,
        TriggerSignature,
    },
    AttentionCmd, CognitiveModule, FeedbackSignal, ModuleInput, ModuleOutput, ShiftTarget,
};
pub use anchor::{
    check_all as check_all_anchors, AnchorConstraint, AnchorError, AnchorReport, AnchorSeverity,
    AnchorViolation, DataSource, DecisionPreview, EcosystemZone, StaticSource,
};
pub use core::{
    algebra::TernaryAlgebra,
    decision_engine::{DecisionEngine, DecisionResult},
    frame::{Frame, FrameError, FrameRegistry},
    hold::{HoldFinality, HoldState, HolderConfig},
    phase::{Commitment, Phase, PhaseError},
    sensor::{
        BodyState, CogState, EnvSnapshot, EnvironmentalContext, SensorSignal, TemporalScale,
        TextInput,
    },
    value::TritValue,
    word::{Trit, TritWord, WordError},
};
pub use feedback::{
    proxy_env::{ProxyEnvironment, StaticRuleModel},
    ConsequencePrediction, CorrectionHint, CorrectionSeverity, FeedbackLoop, PracticeTestResult,
};
pub use hook::{
    context_cache::ContextCache,
    module_registry::{
        ModuleEntry, ModuleId, ModuleRegistry, ModuleState, RegistryAction, RegistryEvent,
    },
    mount_arbiter::{MountArbiter, Resource, ResourceCost},
    scenario_recognizer::{recognize, recognize_with_score},
    HoldStrategy, HookContext, HookManager, IterationSummary, ScenarioType, UnmountReason,
};
pub use meta::{
    ArbitrationResult, ConflictType, CustomRule, Domain, DomainParseError, FallbackBehavior,
    JsonRuleLoader, MetaInterrupt, MetaMonitor, PolicyError, ResolutionPolicy, RuleError,
    RuleLoader, SafeFallback,
};
pub use sandbox::{
    sanitize_log_field, validate_scenario, ErrorCategory, SandboxDiagnostics, SandboxError,
    SandboxOutput, SandboxPipeline, ScenarioInput, ScenarioValidator, SignalInput, MAX_JSON_SIZE,
    MAX_SIGNALS, MAX_STRING_LEN,
};
