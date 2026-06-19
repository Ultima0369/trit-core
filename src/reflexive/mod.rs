//! Reflexive audit layer: slow half-beat self-check after decisions.
//!
//! This module provides [`ReflexiveAuditor`], which inspects the conflict
//! trace, attention trace, and phase history of a decision to detect
//! forced collapses and explanation impulses.

pub mod auditor;

pub use auditor::{AttentionEvent, AuditReport, PhaseShift, ReflexiveAlert, ReflexiveAuditor};
