//! Sandbox layer: scenario input validation, pipeline execution, and output validation.
//!
//! # Orchestrator modules
//!
//! - [`decision_engine`] — ternary decision cycle (TAND → arbitration → guard → SafeFallback)
//! - [`pipeline`] — full scenario pipeline (validation → decision → feedback)
//!
//! These are intentional orchestrators (Façade pattern): they depend on all layers
//! below them but own no domain logic themselves.

pub mod decision_engine;
pub mod diagnostic;
pub mod error;
pub mod input;
pub mod io_loader;
pub mod output;
pub mod pipeline;
pub mod validate;

pub use decision_engine::{DecisionEngine, DecisionResult};
pub use diagnostic::SandboxDiagnostics;
pub use error::{ErrorCategory, SandboxError};
pub use input::{ScenarioInput, SignalInput};
pub use output::SandboxOutput;
pub use pipeline::SandboxPipeline;
pub use validate::{
    sanitize_log_field, validate_domain, validate_scenario, validate_signal, ScenarioValidator,
    MAX_JSON_SIZE, MAX_SIGNALS, MAX_STRING_LEN,
};
