use crate::core::word::WordError;
use thiserror::Error;

/// Errors that can occur during sandbox execution.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum SandboxError {
    #[error("invalid scenario: {0}")]
    InvalidScenario(String),
    #[error("invalid domain '{0}'")]
    InvalidDomain(String),
    #[error("invalid frame in signal {index}: {reason}")]
    InvalidFrame { index: usize, reason: String },
    #[error("invalid value in signal {index}: {reason}")]
    InvalidValue { index: usize, reason: String },
    #[error("invalid phase in signal {index}: {reason}")]
    InvalidPhase { index: usize, reason: String },
    #[error("scenario invariant violated: {0}")]
    Invariant(String),
    #[error("word construction failed: {0}")]
    Word(#[from] WordError),
    #[error("expected behavior mismatch: expected '{expected}', got '{got}'")]
    ExpectedBehaviorMismatch { expected: String, got: String },
    #[error("I/O error: {0}")]
    Io(String),
}

/// Error category for observability and user guidance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Input data problem (scenario JSON, signal values, etc.).
    Input,
    /// Security / permission problem.
    Security,
    /// Internal logic / invariant violation.
    Internal,
    /// Expected behavior mismatch.
    Validation,
    /// I/O or environment problem.
    Io,
}

impl SandboxError {
    /// Classify the error for metrics and dashboards.
    pub fn category(&self) -> ErrorCategory {
        match self {
            SandboxError::InvalidScenario(_)
            | SandboxError::InvalidDomain(_)
            | SandboxError::InvalidFrame { .. }
            | SandboxError::InvalidValue { .. }
            | SandboxError::InvalidPhase { .. } => ErrorCategory::Input,
            SandboxError::Invariant(_) | SandboxError::Word(_) => ErrorCategory::Internal,
            SandboxError::ExpectedBehaviorMismatch { .. } => ErrorCategory::Validation,
            // Security classification is determined by the message prefix set in
            // the CLI path-traversal guard. Keep this check last so other Io
            // variants do not accidentally match.
            SandboxError::Io(msg) if msg.starts_with("Security error:") => ErrorCategory::Security,
            SandboxError::Io(_) => ErrorCategory::Io,
        }
    }

    /// Category name for structured logging.
    pub fn category_name(&self) -> &'static str {
        match self.category() {
            ErrorCategory::Input => "input",
            ErrorCategory::Security => "security",
            ErrorCategory::Internal => "internal",
            ErrorCategory::Validation => "validation",
            ErrorCategory::Io => "io",
        }
    }

    /// Return a human-readable help message suggesting how to fix the error.
    pub fn help(&self) -> String {
        match self {
            SandboxError::InvalidScenario(msg) => format!(
                "Check the scenario JSON structure. Common issues: missing fields, wrong types, or malformed data. Details: {}",
                msg
            ),
            SandboxError::InvalidDomain(msg) => format!(
                "Use a known domain: Physical, Engineering, MedicalEthics, ValueJudgment, General, or Custom(name). Got: {}",
                msg
            ),
            SandboxError::InvalidFrame { index, reason } => format!(
                "Signal at index {} has an invalid frame. Allowed input frames: Science, Individual, Consensus, Absolute. (Meta is reserved for system-internal use.) {}",
                index, reason
            ),
            SandboxError::InvalidValue { index, reason } => format!(
                "Signal at index {} has an invalid value. Allowed values: 1 (True), 0 (Hold), -1 (False). {}",
                index, reason
            ),
            SandboxError::InvalidPhase { index, reason } => format!(
                "Signal at index {} has an invalid phase. Phase must be a finite f64 in [0.0, 1.0]. {}",
                index, reason
            ),
            SandboxError::Invariant(msg) => format!(
                "A runtime invariant was violated. This is likely a bug. Details: {}",
                msg
            ),
            SandboxError::Word(e) => format!(
                "Failed to construct a TritWord. Check that frame/value/phase combinations are valid. Details: {}",
                e
            ),
            SandboxError::ExpectedBehaviorMismatch { expected, got } => format!(
                "The scenario's expected_behavior ('{}') did not match the actual output ('{}'). Update the scenario expectation or inspect the decision pipeline.",
                expected, got
            ),
            SandboxError::Io(msg) => format!(
                "An I/O error occurred. Check file paths, permissions, and disk space. Details: {}",
                msg
            ),
        }
    }

    /// Return a structured error report suitable for logging or CLI output.
    pub fn report(&self) -> String {
        format!(
            "[{}] {}\n\nHelp: {}",
            self.category_name(),
            self,
            self.help()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_categories_are_exhaustive() {
        let errors = vec![
            SandboxError::InvalidScenario("x".into()),
            SandboxError::InvalidDomain("x".into()),
            SandboxError::InvalidFrame {
                index: 0,
                reason: "x".into(),
            },
            SandboxError::InvalidValue {
                index: 0,
                reason: "x".into(),
            },
            SandboxError::InvalidPhase {
                index: 0,
                reason: "x".into(),
            },
            SandboxError::Invariant("x".into()),
            SandboxError::ExpectedBehaviorMismatch {
                expected: "x".into(),
                got: "y".into(),
            },
            SandboxError::Io("x".into()),
            SandboxError::Io("Security error: path traversal denied".into()),
        ];
        for err in errors {
            assert!(!err.category_name().is_empty());
            assert!(!err.help().is_empty());
            assert!(err.report().contains(err.category_name()));
        }
    }
}
