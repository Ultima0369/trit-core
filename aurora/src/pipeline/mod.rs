//! Aurora pipeline — two independent processing links.
//!
//! # Analysis link
//! SignalAnalysis BC → TernaryDecision BC
//! - Generates synthetic signal via wavelet engine
//! - Detects fundamental frequency via FFT
//! - Maps frequency and user state to TritWords
//! - Evaluates ternary decision
//!
//! # Attention link
//! AttentionGuidance BC → AuditTrail BC → SQLite
//! - Runs attention scheduling cycle
//! - Persists audit entries
//! - Computes ASI metric

pub mod analysis;
pub mod attention;

pub use analysis::{run_analysis, AnalysisReport, SignalSpec};
pub use attention::{run_attention, run_attention_in_memory, AttentionOutcome};
