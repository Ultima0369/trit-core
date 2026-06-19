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
//! - [`hook`] — scenario perception and module scheduling (Layer 2).
//! - [`adapters`] — dynamic cognitive module pool (Layer 3).
//! - [`core`] — ternary algebra and data types: `TritValue`, `Phase`, `Frame`,
//!   `TritWord`, `TernaryAlgebra`.
//! - [`meta`] — policy engine: `Domain`, `ResolutionPolicy`, `ArbitrationResult`,
//!   `MetaInterrupt`, `SafeFallback`, custom rules.
//! - [`feedback`] — output testing and corrective feedback loop (Layer 5).
//! - [`sandbox`] — scenario I/O, validation, pipeline, and expected-behavior
//!   verification.
//! - [`clock`] — phase oscillator and time-scale management.
//! - [`baseline`] — binary baseline comparator for validation.
//! - [`tracing_init`] — structured logging initialization.
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

pub mod adapters;
pub mod anchor;
pub mod attention;
pub mod baseline;
pub mod budget;
pub mod calibration;
pub mod clock;
pub mod core;
pub mod feedback;
pub mod hook;
pub mod knowledge;
pub mod meta;
pub mod reflexive;
pub mod sandbox;
pub mod tracing_init;

pub use anchor::{
    check_all as check_all_anchors, AnchorConstraint, AnchorError, AnchorReport, AnchorSeverity,
    AnchorViolation, DataSource, DecisionPreview, EcosystemZone, StaticSource,
};
pub use core::{
    algebra::TernaryAlgebra,
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
