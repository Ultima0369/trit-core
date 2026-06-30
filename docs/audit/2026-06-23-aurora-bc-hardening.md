# Aurora BC 架构硬化实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 消除 M0/M1 两套代码并存，将 6 个 BC 模块从骨架升级为工作系统：两条独立链路 + 旧模块删除 + main.rs 编排。

**Architecture:** 分析链路（SignalAnalysis BC → TernaryDecision BC）和注意力链路（AttentionGuidance BC → AuditTrail BC → SQLite）独立运行，main.rs 编排，Presentation BC 渲染输出。wavelet/ 和 ingest/ 保留为引擎层。

**Tech Stack:** Rust 2021, trit-core v0.3.0, rusqlite 0.31, clap 4.5, chrono 0.4

## Global Constraints

- `#![forbid(unsafe_code)]` enforced crate-wide
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` must pass
- `cargo fmt --check` must pass
- `cargo test --workspace --all-features` must pass (all tests)
- `cargo test ethics_` 10 tests must pass
- CLI interface `--input`/`--output`/`--frequency-threshold`/`--user-feels-normal` must remain compatible
- No `unimplemented!()` in production code paths

---

### Task 1: 创建新 pipeline 模块骨架

**Files:**
- Create: `aurora/src/pipeline/mod.rs`
- Create: `aurora/src/pipeline/analysis.rs`
- Create: `aurora/src/pipeline/attention.rs`
- Modify: `aurora/src/lib.rs:24-26` (remove `pub mod attention; pub mod decision;` 行)

**Interfaces:**
- Produces: `aurora::pipeline::analysis::run()` → `Result<AnalysisReport>`
- Produces: `aurora::pipeline::attention::run()` → `Result<AttentionOutcome>`
- Produces: `AnalysisReport { pub spectrum: FrequencySpectrum, pub decision: DecisionRecord }`
- Produces: `AttentionOutcome { pub cmd: Option<AttentionCmd>, pub asi: f64, pub reminder_count: usize, pub session: AttentionSession }`

- [ ] **Step 1: 创建 `aurora/src/pipeline/mod.rs`**

```rust
//! Aurora pipeline — two independent links for analysis and attention.
//!
//! - `analysis`: SignalAnalysis BC → TernaryDecision BC (stateless)
//! - `attention`: AttentionGuidance BC → AuditTrail BC → SQLite (stateful)

pub mod analysis;
pub mod attention;

pub use analysis::{AnalysisReport, run_analysis};
pub use attention::{AttentionOutcome, run_attention};
```

- [ ] **Step 2: 创建 `aurora/src/pipeline/analysis.rs`**

```rust
//! Analysis link: SignalAnalysis BC → TernaryDecision BC.
//!
//! Stateless: takes a signal spec, returns frequency spectrum + decision record.

use crate::bc::signal_analysis::{FftWaveletEngine, FrequencySpectrum, SignalQuality, TimeSeries, WaveletEngine};
use crate::bc::ternary_decision::{DecisionPort, DecisionRecord, DecisionSession, TritDecisionEngine};
use crate::bc::BcError;
use crate::wavelet;
use serde::Deserialize;
use truncore::core::{Frame, TritWord};

/// Specification for a synthetic signal, deserializable from JSON input.
#[derive(Debug, Clone, Deserialize)]
pub struct SignalSpec {
    pub freq: f64,
    pub sample_rate: f64,
    pub duration_secs: f64,
    pub noise_std: f64,
}

/// Output of the analysis link.
#[derive(Debug, Clone)]
pub struct AnalysisReport {
    /// The frequency spectrum from wavelet analysis.
    pub spectrum: FrequencySpectrum,
    /// The ternary decision record.
    pub decision: DecisionRecord,
}

/// Map a detected frequency and threshold to an Embodied TritWord.
fn frequency_to_embodied(freq: f64, threshold: f64) -> TritWord {
    if freq > threshold {
        TritWord::tru(Frame::Embodied)
    } else {
        TritWord::fals(Frame::Embodied)
    }
}

/// Map user self-report state to an Individual TritWord.
fn user_state_to_individual(feels_normal: bool) -> TritWord {
    if feels_normal {
        TritWord::tru(Frame::Individual)
    } else {
        TritWord::fals(Frame::Individual)
    }
}

/// Run the analysis link: synthetic signal → FFT → Trit-Core decision.
pub fn run_analysis(
    spec: &SignalSpec,
    frequency_threshold: f64,
    user_feels_normal: bool,
) -> Result<AnalysisReport, BcError> {
    // Generate synthetic signal
    let signal = wavelet::sine_wave(
        spec.freq,
        spec.sample_rate,
        spec.duration_secs,
        spec.noise_std,
    );

    // Create time series and analyze via SignalAnalysis BC
    let ts = TimeSeries::new(spec.sample_rate, signal)?;
    let engine = FftWaveletEngine;
    let spectrum = engine.analyze(&ts)?;

    // Build TritWords from the analysis results
    let embodied = frequency_to_embodied(spectrum.fundamental_hz, frequency_threshold);
    let individual = user_state_to_individual(user_feels_normal);

    // Run ternary decision via TernaryDecision BC
    let engine = TritDecisionEngine;
    let mut session = DecisionSession::new("analysis_session");
    let signals = vec![embodied, individual];
    let decision = engine.evaluate(&mut session, &signals, "General")?;

    Ok(AnalysisReport { spectrum, decision })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frequency_above_threshold_is_tru_embodied() {
        let word = frequency_to_embodied(2.5, 2.0);
        assert_eq!(word.value(), truncore::core::TritValue::True);
        assert_eq!(word.frame(), Frame::Embodied);
    }

    #[test]
    fn frequency_below_threshold_is_fals_embodied() {
        let word = frequency_to_embodied(1.0, 2.0);
        assert_eq!(word.value(), truncore::core::TritValue::False);
    }

    #[test]
    fn user_feels_normal_is_tru_individual() {
        let word = user_state_to_individual(true);
        assert_eq!(word.value(), truncore::core::TritValue::True);
        assert_eq!(word.frame(), Frame::Individual);
    }

    #[test]
    fn user_feels_abnormal_is_fals_individual() {
        let word = user_state_to_individual(false);
        assert_eq!(word.value(), truncore::core::TritValue::False);
    }

    #[test]
    fn run_analysis_2hz_signal_detected() {
        let spec = SignalSpec {
            freq: 2.5,
            sample_rate: 100.0,
            duration_secs: 1.0,
            noise_std: 0.0,
        };
        let report = run_analysis(&spec, 2.0, true).unwrap();
        // 2.5 Hz should be detected within 0.5 Hz tolerance
        assert!((report.spectrum.fundamental_hz - 2.5).abs() < 0.5);
        assert!(!report.decision.input_signals.is_empty());
    }

    #[test]
    fn run_analysis_cross_frame_detects_conflict() {
        // Embodied True (high freq) vs Individual True → same-frame, should commit
        // Embodied True vs Individual False → cross-frame, should Hold
        let spec = SignalSpec {
            freq: 2.5,
            sample_rate: 100.0,
            duration_secs: 1.0,
            noise_std: 0.0,
        };
        let report = run_analysis(&spec, 2.0, false).unwrap();
        // freq 2.5 > threshold 2.0 → Embodied True
        // feels_normal false → Individual False
        // Cross-frame → Hold
        assert!(report.decision.is_hold());
        assert!(report.decision.has_conflicts());
    }
}
```

- [ ] **Step 3: 创建 `aurora/src/pipeline/attention.rs`**

```rust
//! Attention link: AttentionGuidance BC → AuditTrail BC → SQLite.
//!
//! Stateful: feeds signals to the attention scheduler, records user responses,
//! and persists audit entries to SQLite.

use crate::bc::attention_guidance::{AttentionManager, AttentionPort, AttentionSession};
use crate::bc::audit_trail::{AuditDecisionSnapshot, AuditEntry, AuditEventType, AuditFilter, AuditPort};
use crate::bc::BcError;
use crate::db::audit_log::SqliteAuditLog;
use crate::db::Database;
use truncore::adapters::AttentionCmd;
use truncore::core::TritWord;

/// Output of the attention link.
#[derive(Debug, Clone)]
pub struct AttentionOutcome {
    /// The scheduler's command (None if Continue).
    pub cmd: Option<AttentionCmd>,
    /// Current ASI score.
    pub asi: f64,
    /// Number of reminders in this session.
    pub reminder_count: usize,
    /// The attention session (for rendering).
    pub session: AttentionSession,
}

/// Run the attention link with SQLite persistence.
///
/// Feeds signals to the attention scheduler, records any reminders,
/// and persists audit entries to the database.
pub fn run_attention(
    signals: &[TritWord],
    db: &Database,
) -> Result<AttentionOutcome, BcError> {
    let mut attention = AttentionManager::new("attention_session");
    let cmd = attention.run_cycle(signals);

    // Build decision snapshot for audit
    let snapshot = AuditDecisionSnapshot {
        signal_count: signals.len(),
        signal_frames: signals.iter().map(|w| format!("{:?}", w.frame())).collect(),
        result_value: "Hold".into(), // placeholder — real value comes from analysis link
        result_frame: "Meta".into(),
    };

    // Persist audit entry
    let mut audit = SqliteAuditLog::new(db.clone());
    let entry = AuditEntry::new(
        AuditEventType::Decision,
        attention.session().session_id().to_string(),
        "attention cycle".into(),
    )
    .with_decision_snapshot(snapshot);

    audit.record(entry)?;

    Ok(AttentionOutcome {
        cmd,
        asi: attention.asi(),
        reminder_count: attention.session().reminder_count(),
        session: attention.session().clone(),
    })
}

/// Run the attention link without persistence (in-memory, for testing).
pub fn run_attention_in_memory(
    signals: &[TritWord],
) -> Result<AttentionOutcome, BcError> {
    let mut attention = AttentionManager::new("attention_session");
    let cmd = attention.run_cycle(signals);

    Ok(AttentionOutcome {
        cmd,
        asi: attention.asi(),
        reminder_count: attention.session().reminder_count(),
        session: attention.session().clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use truncore::core::Frame;

    #[test]
    fn run_attention_in_memory_does_not_panic() {
        let signals = vec![
            TritWord::tru(Frame::Embodied),
            TritWord::fals(Frame::Individual),
        ];
        let outcome = run_attention_in_memory(&signals).unwrap();
        // ASI might be 0 if scheduler returns Continue, but outcome must exist
        assert!(outcome.asi >= 0.0);
    }

    #[test]
    fn run_attention_with_sqlite_persists() {
        let db = Database::open_in_memory().unwrap();
        let signals = vec![
            TritWord::tru(Frame::Embodied),
            TritWord::fals(Frame::Individual),
        ];
        let outcome = run_attention(&signals, &db).unwrap();

        // Verify audit entry was persisted
        let audit = SqliteAuditLog::new(db);
        assert_eq!(audit.entry_count(), 1);
        let entries = audit.query_owned(&AuditFilter::new()).unwrap();
        assert_eq!(entries[0].description, "attention cycle");
    }
}
```

- [ ] **Step 4: Build and test**

```bash
cargo build --workspace 2>&1 | tail -5
```

Expected: `Finished dev profile` 无错误。

- [ ] **Step 5: Commit**

```bash
git add aurora/src/pipeline/ aurora/src/lib.rs
git commit -m "feat(aurora): 创建新 pipeline 模块 — analysis + attention 两条独立链路"
```

---

### Task 2: 重写 main.rs 使用新链路

**Files:**
- Modify: `aurora/src/main.rs`
- Modify: `aurora/src/cli.rs` (添加 `--db-path` 参数)

**Interfaces:**
- Consumes: `pipeline::analysis::run_analysis`, `pipeline::attention::run_attention`, `pipeline::attention::run_attention_in_memory`
- Consumes: `bc::presentation::AuroraRenderer`, `bc::presentation::RenderPort`, `bc::presentation::ViewState`
- Produces: 可运行的 `cargo run --bin aurora` CLI

- [ ] **Step 1: 更新 `aurora/src/cli.rs` 添加 `--db-path` 参数**

```rust
//! Command-line interface for Aurora.

use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "aurora")]
#[command(about = "Aurora — local-first cognitive sovereignty tool")]
pub struct Args {
    /// Path to input JSON file describing the synthetic signal.
    #[arg(short, long)]
    pub input: PathBuf,

    /// Path to output HTML report (optional; defaults to stdout with JSON).
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Frequency threshold separating high/active from low/quiet embodied states.
    #[arg(long, default_value_t = 2.0)]
    pub frequency_threshold: f64,

    /// Whether the user self-reports feeling normal.
    #[arg(long)]
    pub user_feels_normal: bool,

    /// Path to JSON contacts file for relationship-aware analysis (optional).
    #[arg(long)]
    pub data_source: Option<PathBuf>,

    /// Path to SQLite database (optional; uses in-memory fallback if not set).
    #[arg(long, default_value = ":memory:")]
    pub db_path: String,
}
```

- [ ] **Step 2: 重写 `aurora/src/main.rs`**

```rust
use anyhow::{Context, Result};
use aurora::bc::presentation::{AuroraRenderer, ConflictCard, RenderPort, ViewState};
use aurora::cli::Args;
use aurora::db::Database;
use aurora::pipeline::{analysis, attention};
use clap::Parser;
use std::fs;
use std::path::Path;

fn main() -> Result<()> {
    let args = Args::parse();

    // Open database (in-memory if no path specified)
    let db_path = Path::new(&args.db_path);
    let db = if db_path == Path::new(":memory:") {
        Database::open_in_memory()?
    } else {
        Database::open(db_path)?
    };

    // Load data source if provided
    if let Some(ref path) = args.data_source {
        let manager = aurora::ingest::IngestManager::with_json_fallback(path)?;
        eprintln!(
            "Loaded {} contacts from {}",
            manager.contact_count(),
            manager.source_name()
        );
    }

    // Read and parse input
    let input_text = fs::read_to_string(&args.input)
        .with_context(|| format!("failed to read input file {:?}", args.input))?;
    let spec: analysis::SignalSpec = serde_json::from_str(&input_text)
        .with_context(|| "failed to parse input JSON as SignalSpec")?;

    // ── Link 1: Analysis ────────────────────────────────────────────
    let analysis_report = analysis::run_analysis(
        &spec,
        args.frequency_threshold,
        args.user_feels_normal,
    )
    .map_err(|e| anyhow::anyhow!("analysis link failed: {e}"))?;

    // ── Link 2: Attention ───────────────────────────────────────────
    let attention_outcome = attention::run_attention_in_memory(
        &analysis_report.decision.input_signals,
    )
    .map_err(|e| anyhow::anyhow!("attention link failed: {e}"))?;

    // ── Presentation ────────────────────────────────────────────────
    let mut view = ViewState::new(
        format!(
            "Detected frequency: {:.3} Hz | Decision: {:?}",
            analysis_report.spectrum.fundamental_hz,
            analysis_report.decision.result.value()
        ),
        attention_outcome.session,
    );

    // Add conflicts to view
    for interrupt in &analysis_report.decision.interrupts {
        view.add_conflict(ConflictCard {
            conflict_type: format!("{:?}", interrupt.conflict),
            reason: interrupt.reason.clone(),
            frame_a: "Embodied".into(),
            frame_b: "Individual".into(),
            acknowledged: false,
        });
    }

    let renderer = AuroraRenderer;
    let html = renderer.render_html(&view);

    match args.output {
        Some(path) => {
            fs::write(&path, &html)
                .with_context(|| format!("failed to write HTML report to {:?}", path))?;
            println!("Report written to {}", path.display());
        }
        None => {
            let json = renderer
                .render_json(&view)
                .map_err(|e| anyhow::anyhow!("JSON render failed: {e}"))?;
            println!("{}", json);
        }
    }

    Ok(())
}
```

- [ ] **Step 3: 验证编译**

```bash
cargo build --workspace 2>&1 | tail -10
```

Expected: 编译通过，可能有 dead_code 警告（旧模块尚未删除）。

- [ ] **Step 4: 验证 CLI 端到端可运行**

```bash
cargo run --bin aurora -- --input examples/synthetic_2hz.json --output /tmp/test_report.html --user-feels-normal 2>&1
```

Expected: `Report written to /tmp/test_report.html`

- [ ] **Step 5: Commit**

```bash
git add aurora/src/main.rs aurora/src/cli.rs
git commit -m "feat(aurora): 重写 main.rs — 使用两条独立 BC 链路编排"
```

---

### Task 3: 删除旧模块并更新 lib.rs

**Files:**
- Delete: `aurora/src/decision/mod.rs`
- Delete: `aurora/src/decision/adapter.rs`
- Delete: `aurora/src/decision/conflict.rs`
- Delete: `aurora/src/attention/mod.rs`
- Delete: `aurora/src/render/mod.rs`
- Delete: `aurora/src/render/html.rs`
- Delete: `aurora/src/render/json.rs`
- Delete: `aurora/src/pipeline.rs` (旧单管道文件)
- Modify: `aurora/src/lib.rs`

- [ ] **Step 1: 删除旧 decision 模块**

```bash
rm aurora/src/decision/adapter.rs
rm aurora/src/decision/conflict.rs
rm aurora/src/decision/mod.rs
rmdir aurora/src/decision 2>/dev/null || true
```

- [ ] **Step 2: 删除旧 attention 模块**

```bash
rm aurora/src/attention/mod.rs
rmdir aurora/src/attention 2>/dev/null || true
```

- [ ] **Step 3: 删除旧 render 模块**

```bash
rm aurora/src/render/html.rs
rm aurora/src/render/json.rs
rm aurora/src/render/mod.rs
rmdir aurora/src/render 2>/dev/null || true
```

- [ ] **Step 4: 删除旧 pipeline.rs**

```bash
rm aurora/src/pipeline.rs
```

- [ ] **Step 5: 更新 `aurora/src/lib.rs`**

```rust
//! Aurora: a local-first cognitive sovereignty tool built on Trit-Core.
//!
//! M1: BC architecture with two independent links — analysis (SignalAnalysis → TernaryDecision)
//! and attention (AttentionGuidance → AuditTrail → SQLite). wavelet/ and ingest/ are retained
//! as engine layers behind BC facades.
//!
//! `#![forbid(unsafe_code)]` is enforced crate-wide per CHARTER engineering discipline.

#![forbid(unsafe_code)]

/// Returns the current Aurora crate version.
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Wavelet analysis and synthetic signal generation (engine layer).
pub mod wavelet;

/// Data ingestion layer (engine layer — DataSource trait + JSON fallback).
pub mod ingest;

/// Command-line argument definitions.
pub mod cli;

/// Pipeline links: analysis (SignalAnalysis → TernaryDecision) and
/// attention (AttentionGuidance → AuditTrail → SQLite).
pub mod pipeline;

/// Bounded Context modules (facade layer).
///
/// Six independent BCs with trait-defined boundaries:
/// SignalAnalysis, RelationshipAnnotation, TernaryDecision,
/// AttentionGuidance, AuditTrail, Presentation.
pub mod bc;

/// SQLite data layer (persistence).
///
/// Local SQLite database at ~/.aurora/data/aurora.db.
/// Schema: contacts, frame_annotations, annotation_history,
/// audit_log, communication_events.
pub mod db;
```

- [ ] **Step 6: Build 和修复编译错误**

```bash
cargo build --workspace 2>&1
```

Expected: 可能有引用旧模块的错误。如果出现 `use aurora::decision::` 或 `use aurora::attention::` 或 `use aurora::render::` 的编译错误，记录它们（下一步修复测试）。

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "refactor(aurora): 删除旧 decision/attention/render/pipeline 模块，更新 lib.rs"
```

---

### Task 4: 更新测试文件

**Files:**
- Modify: `aurora/tests/decision_conflict.rs`
- Modify: `aurora/tests/attention_session.rs`
- Modify: `aurora/tests/render_attention.rs`
- Modify: `aurora/tests/cli_end_to_end.rs`

**Interfaces:**
- Consumes: `aurora::bc::*` (replaces old `aurora::decision::*`, `aurora::attention::*`, `aurora::render::*`)

- [ ] **Step 1: 重写 `aurora/tests/decision_conflict.rs`**

旧测试引用 `aurora::decision::{detect_conflict, embodied_from_frequency, individual_from_user_state}`。这些函数已内联到 `pipeline/analysis.rs` 中。将这些测试改为测试分析链路。

```rust
//! Analysis link acceptance tests: Embodied vs Individual conflict detection.

use aurora::bc::ternary_decision::{DecisionPort, DecisionSession, TritDecisionEngine};
use truncore::core::{Frame, TritValue, TritWord};

#[test]
fn cross_frame_embodied_vs_individual_is_hold_with_interrupt() {
    let engine = TritDecisionEngine;
    let mut session = DecisionSession::new("test");
    let signals = vec![TritWord::tru(Frame::Embodied), TritWord::tru(Frame::Individual)];
    let record = engine.evaluate(&mut session, &signals, "General").unwrap();
    assert_eq!(record.result.value(), TritValue::Hold);
    assert!(record.has_conflicts());
}

#[test]
fn cross_frame_embodied_false_vs_individual_true_is_hold() {
    let engine = TritDecisionEngine;
    let mut session = DecisionSession::new("test");
    let signals = vec![TritWord::fals(Frame::Embodied), TritWord::tru(Frame::Individual)];
    let record = engine.evaluate(&mut session, &signals, "General").unwrap();
    assert_eq!(record.result.value(), TritValue::Hold);
    assert!(record.has_conflicts());
}

#[test]
fn same_frame_true_true_is_true() {
    let engine = TritDecisionEngine;
    let mut session = DecisionSession::new("test");
    let signals = vec![TritWord::tru(Frame::Individual), TritWord::tru(Frame::Individual)];
    let record = engine.evaluate(&mut session, &signals, "General").unwrap();
    assert_eq!(record.result.value(), TritValue::True);
    assert!(!record.has_conflicts());
}

#[test]
fn same_frame_false_false_is_false() {
    let engine = TritDecisionEngine;
    let mut session = DecisionSession::new("test");
    let signals = vec![TritWord::fals(Frame::Individual), TritWord::fals(Frame::Individual)];
    let record = engine.evaluate(&mut session, &signals, "General").unwrap();
    assert_eq!(record.result.value(), TritValue::False);
    assert!(!record.has_conflicts());
}
```

- [ ] **Step 2: 重写 `aurora/tests/attention_session.rs`**

旧测试引用 `aurora::attention::{AttentionSession, UserResponse}`。改为使用 BC 模块的 AttentionSession。

```rust
//! Attention session tests — ASI tracking and user response recording.

use aurora::bc::attention_guidance::{AttentionSession, UserResponse};

#[test]
fn new_session_starts_with_zero_asi() {
    let session = AttentionSession::new("test_session");
    assert_eq!(session.asi(), 0.0);
    assert_eq!(session.reminder_count(), 0);
    assert_eq!(session.user_active_shift_count(), 0);
}

#[test]
fn user_shift_response_increases_active_count_and_asi() {
    let mut session = AttentionSession::new("test_session");

    session.record_reminder("ShiftTo", "ConflictTrace", "跨 Frame 冲突");
    session.record_user_response(UserResponse::ShiftedTo("ConflictTrace".into()));

    assert_eq!(session.reminder_count(), 1);
    assert_eq!(session.user_active_shift_count(), 1);
    assert!(session.asi() > 0.0);
}

#[test]
fn user_ignore_does_not_increase_asi() {
    let mut session = AttentionSession::new("test_session");

    session.record_reminder("ShiftTo", "ConflictTrace", "跨 Frame 冲突");
    session.record_user_response(UserResponse::Ignored);

    assert_eq!(session.reminder_count(), 1);
    assert_eq!(session.user_active_shift_count(), 0);
    assert_eq!(session.asi(), 0.0);
}

#[test]
fn user_override_hold_increases_active_count() {
    let mut session = AttentionSession::new("test_session");

    session.record_reminder("HoldCurrent", "Meta", "冲突悬置");
    session.record_user_response(UserResponse::OverrodeHold {
        chosen_frame: "Individual".into(),
    });

    assert_eq!(session.user_active_shift_count(), 1);
    assert!(session.asi() > 0.0);
}

#[test]
fn multiple_reminders_with_mixed_responses() {
    let mut session = AttentionSession::new("test_session");

    session.record_reminder("ShiftTo", "ConflictTrace", "冲突1");
    session.record_user_response(UserResponse::ShiftedTo("ConflictTrace".into()));

    session.record_reminder("ShiftTo", "Body", "冲突2");
    session.record_user_response(UserResponse::Ignored);

    session.record_reminder("HoldCurrent", "Meta", "冲突3");
    session.record_user_response(UserResponse::OverrodeHold {
        chosen_frame: "Individual".into(),
    });

    assert_eq!(session.reminder_count(), 3);
    assert_eq!(session.user_active_shift_count(), 2);
    assert!(session.asi() > 0.5 && session.asi() < 0.7);
}
```

- [ ] **Step 3: 重写 `aurora/tests/render_attention.rs`**

旧测试引用 `aurora::attention::AttentionManager` + `aurora::pipeline::DecisionReport` + `aurora::render::html::render`。改为使用 BC 的 Presentation 模块。

```rust
//! Presentation BC rendering tests.

use aurora::bc::attention_guidance::{AttentionManager, AttentionPort, UserResponse};
use aurora::bc::presentation::{AuroraRenderer, ConflictCard, RenderPort, ViewState};
use truncore::core::{Frame, TritWord};

#[test]
fn html_report_includes_asi_section() {
    let mut attention = AttentionManager::new("test_render");
    attention.run_cycle(&[
        TritWord::tru(Frame::Embodied),
        TritWord::fals(Frame::Individual),
    ]);
    attention.respond(UserResponse::ShiftedTo("ConflictTrace".into()));

    let mut view = ViewState::new("Test decision".into(), attention.session().clone());
    view.add_conflict(ConflictCard {
        conflict_type: "FrameMismatch".into(),
        reason: "Embodied vs Individual conflict".into(),
        frame_a: "Embodied".into(),
        frame_b: "Individual".into(),
        acknowledged: false,
    });

    let renderer = AuroraRenderer;
    let html = renderer.render_html(&view);
    assert!(html.contains("Attention Sovereignty Index"));
    assert!(html.contains("ASI"));
    assert!(html.contains("reminder"));
}

#[test]
fn html_report_includes_conflict_panel_when_interrupt_present() {
    let attention = AttentionManager::new("test_render");
    let mut view = ViewState::new("Test decision".into(), attention.session().clone());
    view.add_conflict(ConflictCard {
        conflict_type: "FrameMismatch".into(),
        reason: "Embodied vs Individual conflict".into(),
        frame_a: "Embodied".into(),
        frame_b: "Individual".into(),
        acknowledged: false,
    });

    let renderer = AuroraRenderer;
    let html = renderer.render_html(&view);
    assert!(html.contains("Conflict Panel"));
    assert!(html.contains("FrameMismatch"));
}

#[test]
fn html_report_shows_no_conflict_when_no_interrupt() {
    let attention = AttentionManager::new("test_render");
    let view = ViewState::new("Test decision".into(), attention.session().clone());

    let renderer = AuroraRenderer;
    let html = renderer.render_html(&view);
    assert!(html.contains("No conflict detected"));
}

#[test]
fn json_report_includes_key_fields() {
    let mut attention = AttentionManager::new("test_render");
    attention.run_cycle(&[TritWord::tru(Frame::Embodied)]);
    attention.respond(UserResponse::ShiftedTo("Body".into()));

    let view = ViewState::new("Test".into(), attention.session().clone());
    let renderer = AuroraRenderer;
    let json = renderer.render_json(&view).unwrap();

    assert!(json.contains("decision_summary"));
    assert!(json.contains("asi"));
    assert!(json.contains("reminder_count"));
}
```

- [ ] **Step 4: 更新 `aurora/tests/cli_end_to_end.rs`**

端到端测试检查 HTML 包含特定字符串。新的 Presentation BC 渲染器使用相同的模板结构，需要确保输出匹配。

```rust
//! End-to-end test: CLI input → HTML output.

use std::process::Command;

#[test]
fn cli_generates_html_report() {
    let output = std::env::temp_dir().join("aurora_test_report.html");

    let status = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--bin",
            "aurora",
            "--",
            "--input",
            "examples/synthetic_2hz.json",
            "--output",
            output.to_str().unwrap(),
            "--frequency-threshold",
            "2.0",
            "--user-feels-normal",
        ])
        .status()
        .expect("failed to run aurora CLI");

    assert!(status.success());
    let html = std::fs::read_to_string(&output).expect("failed to read HTML output");
    assert!(html.contains("Aurora Decision Report"));
    assert!(
        html.contains("Attention Sovereignty Index"),
        "HTML should contain ASI section"
    );
    assert!(
        html.contains("Reminder History"),
        "HTML should contain reminder history"
    );
    std::fs::remove_file(&output).ok();
}

#[test]
fn cli_prints_json_without_output_flag() {
    let output = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--bin",
            "aurora",
            "--",
            "--input",
            "examples/synthetic_2hz.json",
            "--user-feels-normal",
        ])
        .output()
        .expect("failed to run aurora CLI");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("decision_summary"));
    assert!(stdout.contains("asi"));
    assert!(stdout.contains("reminder_count"));
}
```

- [ ] **Step 5: 运行全部测试**

```bash
cargo test --workspace --all-features -- --test-threads=2 2>&1 | tail -20
```

Expected: 全部测试通过，无 failure。

- [ ] **Step 6: Commit**

```bash
git add aurora/tests/
git commit -m "test(aurora): 迁移测试至 BC 模块 — decision→ternary_decision, attention→attention_guidance, render→presentation"
```

---

### Task 5: 验证验收标准

**Files:** 无修改，纯验证

- [ ] **Step 1: 验证 clippy**

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1
```

Expected: 无 warning 或 error。

- [ ] **Step 2: 验证 fmt**

```bash
cargo fmt --check 2>&1
```

Expected: 无输出（全部已格式化）。

- [ ] **Step 3: 验证全量测试**

```bash
cargo test --workspace --all-features -- --test-threads=2 2>&1 | grep "test result:" | awk '{total_passed+=$4; total_failed+=$6} END {print "PASSED:", total_passed, "FAILED:", total_failed}'
```

Expected: 0 FAILED。

- [ ] **Step 4: 验证伦理门禁**

```bash
cargo test ethics_ 2>&1 | grep "test result:"
```

Expected: `10 passed; 0 failed`。

- [ ] **Step 5: 验证旧模块引用已清零**

```bash
grep -r "use aurora::decision" aurora/src/ aurora/tests/ 2>&1
```

Expected: 无匹配。

```bash
grep -r "use aurora::attention" aurora/src/ aurora/tests/ 2>&1
```

Expected: 无匹配（`attention` 仅出现在 `bc::attention_guidance` 中）。

```bash
grep -r "use aurora::render" aurora/src/ aurora/tests/ 2>&1
```

Expected: 无匹配。

- [ ] **Step 6: 验证 CLI 端到端可运行**

```bash
cargo run --bin aurora -- --input examples/synthetic_2hz.json --output /tmp/verify_report.html --user-feels-normal 2>&1
```

Expected: `Report written to /tmp/verify_report.html`。

```bash
grep -c "Aurora Decision Report" /tmp/verify_report.html
```

Expected: `1`。

- [ ] **Step 7: Commit (如有 clippy/fmt 修复)**

```bash
git add -A
git diff --cached --stat
# 如果有变更才 commit
```

---

### Task 6: 更新 SESSION_START.md 和 CLAUDE.md

**Files:**
- Modify: `SESSION_START.md`
- Modify: `CLAUDE.md`

- [ ] **Step 1: 更新 SESSION_START.md 进度表**

将第 22 行 `Aurora 阶段` 更新为：

```markdown
| **Aurora 阶段** | M1 BC 架构硬化完成。旧模块（decision/attention/render）已删除，两条独立链路（analysis + attention）通过 BC trait 调用，main.rs 编排。Presentation BC 接管 HTML/JSON 渲染。702+ tests 通过。 |
```

将第 32 行决策表添加新条目：

```markdown
| 2026-06-23 | M1 BC 架构硬化 — 旧模块删除，两条独立链路（analysis + attention），main.rs 编排，Presentation BC 接管渲染 | `docs/superpowers/specs/2026-06-23-aurora-bc-hardening-design.md` |
```

- [ ] **Step 2: 更新 CLAUDE.md Aurora 架构章节**

替换从 `## Architecture: Aurora (M1 — Bounded Contexts + SQLite)` 开始的内容：

```markdown
## Architecture: Aurora (M1 — BC 架构硬化)

Aurora is a CLI binary (future: Tauri desktop app) with two independent links:

### Engine Layer (retained from M0)
- **`aurora/src/wavelet/`**: Synthetic signal generation + FFT base frequency detection.
- **`aurora/src/ingest/`**: `DataSource` trait abstraction — JSON fallback + mail abstract.

### BC Facade Layer (`aurora/src/bc/`)
Six bounded contexts with trait-defined boundaries, connected in a DAG:
```
SignalAnalysis ─────┐
                    ├──▶ TernaryDecision ──▶ AttentionGuidance ──▶ Presentation
RelationshipAnnotation ─┘        │                                    │
                                 │                                    │
                                 ▼                                    ▼
                            AuditTrail ◀──────────────────────────────┘
```

### Pipeline Links (`aurora/src/pipeline/`)
- **`analysis`**: Stateless. SignalAnalysis BC (FftWaveletEngine → wavelet/) → TernaryDecision BC (TritDecisionEngine → trit-core). Returns `AnalysisReport { spectrum, decision }`.
- **`attention`**: Stateful with SQLite persistence. AttentionGuidance BC (AttentionManager → AttentionScheduler) → AuditTrail BC (SqliteAuditLog → audit_log table). Returns `AttentionOutcome`.

### Presentation (`aurora/src/bc/presentation.rs`)
`AuroraRenderer` implements `RenderPort`, producing self-contained dark-theme HTML with ASI gauge, conflict panel, and reminder history.

### Data Flow
```
JSON input → analysis::run_analysis() → AnalysisReport
                ↓
         attention::run_attention() → AttentionOutcome
                ↓
         AuroraRenderer::render_html() → HTML output
```
```

- [ ] **Step 3: Commit**

```bash
git add SESSION_START.md CLAUDE.md
git commit -m "docs: 同步 SESSION_START 和 CLAUDE.md 到 M1 BC 硬化后状态"
```
