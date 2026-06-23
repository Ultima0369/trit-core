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

use truncore::adapters::AttentionCmd;
use truncore::core::TritWord;
use truncore::meta::MetaInterrupt;

/// Legacy decision report type — backward compatibility with render module.
///
/// This will be removed when the render module is migrated to use
/// the new `AnalysisReport` + `AttentionOutcome` types (Task 3).
#[derive(Debug, Clone)]
pub struct DecisionReport {
    pub input_freq: f64,
    pub detected_freq: f64,
    pub embodied: TritWord,
    pub individual: TritWord,
    pub result: TritWord,
    pub interrupt: Option<MetaInterrupt>,
    pub attention_cmd: Option<AttentionCmd>,
    pub asi: f64,
    pub reminder_count: usize,
}
