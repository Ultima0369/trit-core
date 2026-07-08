//! Aurora: a local-first cognitive sovereignty tool built on Trit-Core.
//!
//! This crate is currently at the **M2 stage**: end-to-end Rust CLI and Tauri
//! desktop shell with wavelet analysis, ternary decision pipeline, attention
//! scheduling with SQLite persistence, LLM perception chain, and HTML/JSON
//! presentation rendering. See [`MASTER_PLAN.md`] for the full roadmap.
//!
//! `#![deny(unsafe_code)]` is enforced crate-wide per CHARTER engineering discipline.
//! The `config::dpapi` module is the only exception — it uses `#[allow(unsafe_code)]`
//! for Windows DPAPI FFI calls (all isolated to two functions).

#![deny(unsafe_code)]

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

/// Application facade — shared entry point for CLI and Tauri.
pub mod app;

/// External perception layer (M2).
///
/// Unified abstraction for cloud LLMs, local models, and FFT analysis.
/// Provides the `ExternalPercept` trait and `PerceptChain` degradation
/// orchestrator.
pub mod percept;

/// Encrypted configuration storage (M2).
///
/// Windows DPAPI-backed API key and provider settings store.
/// Config file at %APPDATA%\aurora\config.enc.
pub mod config;

// Re-export trit-core's anchor layer so the desktop shell (src-tauri) and
// other aurora consumers can build anchor snapshots for display without a
// direct trit-core dependency.
pub use trit_core::anchor;

// Re-export key types for Tauri commands and external consumers.
pub use percept::{RetrospectiveDoc, SspScenario};
pub use pipeline::analysis::TrajectorySummary;
