//! Bounded Context (BC) modules for Aurora M1.
//!
//! Each sub-module is an independent bounded context as defined in
//! `docs/superpowers/specs/2026-06-20-aurora-architecture-design.md` §3.
//!
//! BC dependency graph (单向，无循环):
//!
//! ```text
//! SignalAnalysis ─────┐
//!                     ├──▶ TernaryDecision ──▶ AttentionGuidance ──▶ Presentation
//! RelationshipAnnotation ─┘        │                                    │
//!                                  │                                    │
//!                                  ▼                                    ▼
//!                             AuditTrail ◀──────────────────────────────┘
//! ```
//!
//! # Design rules
//!
//! 1. Each BC exposes exactly ONE public trait (its "port").
//! 2. Each BC has exactly ONE aggregate root — external code goes through it.
//! 3. Core entities have private fields with invariant-enforcing constructors.
//! 4. BCs are independent crates in spirit; they import each other only through traits.

pub mod attention_guidance;
pub mod audit_trail;
pub mod presentation;
pub mod relationship_annotation;
pub mod signal_analysis;
pub mod ternary_decision;

use std::fmt;

/// Unified error type shared across all BC modules.
///
/// Each BC may define its own error variants through the `Domain` variant.
/// This keeps the error type centralized while allowing BC-specific failures.
#[derive(Debug)]
pub enum BcError {
    /// The requested entity was not found.
    NotFound { entity: String, id: String },
    /// Input validation failed.
    InvalidInput { field: String, reason: String },
    /// The operation is not allowed in the current state.
    InvalidState { current: String, required: String },
    /// A domain-specific error from a BC.
    Domain { bc: String, message: String },
    /// Wraps an I/O error.
    Io(std::io::Error),
}

impl fmt::Display for BcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BcError::NotFound { entity, id } => write!(f, "{entity} not found: {id}"),
            BcError::InvalidInput { field, reason } => {
                write!(f, "invalid input for {field}: {reason}")
            }
            BcError::InvalidState { current, required } => {
                write!(f, "invalid state: current={current}, required={required}")
            }
            BcError::Domain { bc, message } => write!(f, "[{bc}] {message}"),
            BcError::Io(e) => write!(f, "I/O error: {e}"),
        }
    }
}

impl BcError {
    /// Machine-readable error kind for frontend discrimination.
    pub fn kind(&self) -> &'static str {
        match self {
            BcError::NotFound { .. } => "not_found",
            BcError::InvalidInput { .. } => "invalid_input",
            BcError::InvalidState { .. } => "invalid_state",
            BcError::Domain { .. } => "domain",
            BcError::Io(_) => "io",
        }
    }
}

impl std::error::Error for BcError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BcError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for BcError {
    fn from(e: std::io::Error) -> Self {
        BcError::Io(e)
    }
}
