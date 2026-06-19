//! Sandbox layer: scenario input validation, pipeline execution, and output validation.

pub mod diagnostic;
pub mod error;
pub mod input;
pub mod output;
pub mod pipeline;
pub mod validate;
pub mod validator;

pub use diagnostic::SandboxDiagnostics;
pub use error::{ErrorCategory, SandboxError};
pub use input::{ScenarioInput, SignalInput};
pub use output::SandboxOutput;
pub use pipeline::SandboxPipeline;
pub use validate::{
    sanitize_log_field, validate_domain, validate_scenario, validate_signal, MAX_JSON_SIZE,
    MAX_SIGNALS, MAX_STRING_LEN,
};
pub use validator::ScenarioValidator;
