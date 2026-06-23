//! Aurora: a local-first cognitive sovereignty tool built on Trit-Core.
//!
//! This crate is currently at the M0 proof-of-concept stage. The immediate
//! goal is an end-to-end Rust CLI that takes synthetic communication-frequency
//! data, extracts a base frequency via wavelet analysis, feeds it into
//! Trit-Core for a ternary decision (Embodied vs Individual), and renders the
//! result as static HTML.
//!
//! `#![forbid(unsafe_code)]` is enforced crate-wide per CHARTER engineering discipline.

#![forbid(unsafe_code)]

/// Returns the current Aurora crate version.
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Wavelet analysis and synthetic signal generation.
pub mod wavelet;

/// Data ingestion layer (M0: JSON fallback, M1: mail).
pub mod ingest;

/// Command-line argument definitions.
pub mod cli;

/// End-to-end pipeline orchestration (two independent links).
pub mod pipeline;

/// Bounded Context modules (M1 architecture).
///
/// Six independent BCs with trait-defined boundaries:
/// SignalAnalysis, RelationshipAnnotation, TernaryDecision,
/// AttentionGuidance, AuditTrail, Presentation.
pub mod bc;

/// SQLite data layer (M1).
///
/// Local SQLite database at ~/.aurora/data/aurora.db.
/// Schema: contacts, frame_annotations, annotation_history,
/// audit_log, communication_events.
pub mod db;
