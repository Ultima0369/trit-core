# M0 剩余任务实现计划：数据采集 → 注意力调度 → 图谱渲染

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现 M0 剩余 3 个 P0 任务：邮件采集抽象层 + JSON fallback、注意力调度最小闭环、注意力图谱 HTML 渲染。按依赖顺序串联为完整数据管道。

**Architecture:** 三层增量——底层 `ingest/` 提供 `DataSource` trait + JSON fallback 实现，数据流入现有 pipeline；中层 `attention/` 在决策结果上运行 `AttentionScheduler`，记录用户响应，计算 ASI；顶层 `render/html.rs` 从 `AttentionSession` 读取数据渲染雷达图 + 冲突面板。

**Tech Stack:** Rust (edition 2021), trit-core (path dep), serde/serde_json, plotters (already in Cargo.toml), chrono.

---

## 文件结构总览

```
新增:
  aurora/src/ingest/mod.rs           — DataSource trait + IngestManager
  aurora/src/ingest/json_fallback.rs — JSON 文件数据源
  aurora/src/ingest/mail_abstract.rs — 邮件抽象层（M1 占位）
  aurora/src/attention/mod.rs        — AttentionManager + AttentionSession
  aurora/tests/ingest_json.rs        — JSON fallback 测试
  aurora/tests/attention_session.rs  — 注意力会话测试

修改:
  aurora/src/lib.rs                  — 注册 ingest + attention 模块
  aurora/src/pipeline.rs             — 扩展 DecisionReport，串联三阶段
  aurora/src/cli.rs                  — 新增 --data-source 参数
  aurora/src/render/html.rs          — 雷达图 + 冲突面板 + ASI 显示
  aurora/src/render/json.rs          — 新增 attention + ingest 字段
  aurora/src/main.rs                 — 串联新 pipeline 阶段
```

---

### 任务 1：DataSource trait + JSON fallback 数据采集层

**Files:**
- Create: `aurora/src/ingest/mod.rs`
- Create: `aurora/src/ingest/json_fallback.rs`
- Create: `aurora/src/ingest/mail_abstract.rs`
- Create: `aurora/tests/ingest_json.rs`
- Modify: `aurora/src/lib.rs`

- [ ] **Step 1：写 DataSource trait 和 JSON fallback 的失败测试**

创建 `aurora/tests/ingest_json.rs`：

```rust
//! JSON fallback data source tests.

use aurora::ingest::{DataSource, IngestManager, json_fallback::JsonFallbackSource};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct TestContact {
    name: String,
    emails_per_week: f64,
    relation_label: String,
}

#[test]
fn json_fallback_loads_contacts_from_file() {
    // Write a temp JSON file
    let dir = std::env::temp_dir().join("aurora_test_ingest");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("contacts.json");
    std::fs::write(
        &path,
        r#"[
            {"name": "张三", "emails_per_week": 12.0, "relation_label": "colleague"},
            {"name": "李四", "emails_per_week": 3.0, "relation_label": "friend"}
        ]"#,
    )
    .unwrap();

    let source = JsonFallbackSource::new(&path).unwrap();
    let contacts: Vec<TestContact> = source.load().unwrap();

    assert_eq!(contacts.len(), 2);
    assert_eq!(contacts[0].name, "张三");
    assert_eq!(contacts[0].emails_per_week, 12.0);
    assert_eq!(contacts[1].relation_label, "friend");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn json_fallback_returns_error_for_missing_file() {
    let source = JsonFallbackSource::new(std::path::Path::new("/nonexistent/aurora_contacts.json"));
    assert!(source.is_err());
}

#[test]
fn ingest_manager_falls_back_to_json_when_mail_unavailable() {
    let dir = std::env::temp_dir().join("aurora_test_ingest_mgr");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("contacts.json");
    std::fs::write(
        &path,
        r#"[
            {"name": "王五", "emails_per_week": 8.0, "relation_label": "family"}
        ]"#,
    )
    .unwrap();

    let mut manager = IngestManager::with_json_fallback(&path).unwrap();
    let count = manager.contact_count();
    assert_eq!(count, 1);

    std::fs::remove_dir_all(&dir).ok();
}
```

- [ ] **Step 2：运行测试验证失败**

```bash
cargo test --package aurora -- ingest_json
```

Expected: FAIL — `aurora::ingest` module not found.

- [ ] **Step 3：创建 DataSource trait**

创建 `aurora/src/ingest/mod.rs`：

```rust
//! Data ingestion layer — abstract over mail, JSON, and future sources.
//!
//! The [`DataSource`] trait is the single abstraction boundary between
//! data acquisition and the decision pipeline. Every data source
//! (Apple Mail, JSON fallback, calendar, etc.) implements this trait.
//!
//! [`IngestManager`] selects the best available source at startup:
//! mail first, JSON fallback second. This is the "mail采集抽象层"
//! required by M0.

use serde::de::DeserializeOwned;
use std::path::PathBuf;

pub mod json_fallback;
pub mod mail_abstract;

/// Unified error type for data ingestion.
#[derive(Debug, thiserror::Error)]
pub enum IngestError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("mail source unavailable: {0}")]
    MailUnavailable(String),
    #[error("no data source available")]
    NoSourceAvailable,
}

/// Abstract data source — the single trait boundary for all ingestion.
///
/// Implementations: `JsonFallbackSource` (M0), `MailSource` (M1).
pub trait DataSource {
    /// Human-readable name of this source (e.g. "json_fallback", "apple_mail").
    fn name(&self) -> &str;

    /// Whether this source is currently available.
    fn is_available(&self) -> bool;

    /// Load contacts as a deserializable type.
    /// The type parameter allows callers to specify their own contact schema.
    fn load<T: DeserializeOwned>(&self) -> Result<Vec<T>, IngestError>;

    /// Return the number of contacts available.
    fn contact_count(&self) -> usize;
}

/// Manages data source selection with fallback logic.
///
/// Priority order:
/// 1. Mail source (if available on this platform)
/// 2. JSON fallback (always available if file exists)
pub struct IngestManager {
    source: Box<dyn DataSource>,
}

impl IngestManager {
    /// Create an IngestManager with JSON fallback only (M0 default).
    pub fn with_json_fallback(path: &std::path::Path) -> Result<Self, IngestError> {
        let source = json_fallback::JsonFallbackSource::new(path)?;
        Ok(Self {
            source: Box::new(source),
        })
    }

    /// Return a reference to the active data source.
    pub fn source(&self) -> &dyn DataSource {
        self.source.as_ref()
    }

    /// Load contacts from the active source.
    pub fn load<T: DeserializeOwned>(&self) -> Result<Vec<T>, IngestError> {
        self.source.load()
    }

    /// Number of contacts in the active source.
    pub fn contact_count(&self) -> usize {
        self.source.contact_count()
    }
}
```

- [ ] **Step 4：创建 JSON fallback 实现**

创建 `aurora/src/ingest/json_fallback.rs`：

```rust
//! JSON fallback data source — reads contacts from a local JSON file.
//!
//! This is the M0 default data source. Users manually export their
//! communication metadata as JSON and point Aurora at the file.
//! Format: a JSON array of contact objects, each with at minimum
//! `name`, `emails_per_week`, and `relation_label` fields.

use super::{DataSource, IngestError};
use serde::de::DeserializeOwned;
use std::fs;
use std::path::{Path, PathBuf};

/// JSON file-based data source.
///
/// Reads a JSON array of contact objects from a local file.
/// The file is read once at construction time and held in memory.
/// For M0, this is acceptable; M1 will add incremental updates.
pub struct JsonFallbackSource {
    path: PathBuf,
    raw_json: String,
    contact_count: usize,
}

impl JsonFallbackSource {
    /// Create a new JSON fallback source from a file path.
    ///
    /// Returns `IngestError::Io` if the file cannot be read.
    /// Returns `IngestError::Json` if the file is not valid JSON.
    pub fn new(path: &Path) -> Result<Self, IngestError> {
        let raw_json = fs::read_to_string(path)?;
        // Validate it's a JSON array by parsing to serde_json::Value
        let parsed: serde_json::Value = serde_json::from_str(&raw_json)?;
        let contact_count = parsed
            .as_array()
            .map(|a| a.len())
            .unwrap_or(0);
        Ok(Self {
            path: path.to_path_buf(),
            raw_json,
            contact_count,
        })
    }
}

impl DataSource for JsonFallbackSource {
    fn name(&self) -> &str {
        "json_fallback"
    }

    fn is_available(&self) -> bool {
        true // JSON file was validated at construction time
    }

    fn load<T: DeserializeOwned>(&self) -> Result<Vec<T>, IngestError> {
        let contacts: Vec<T> = serde_json::from_str(&self.raw_json)?;
        Ok(contacts)
    }

    fn contact_count(&self) -> usize {
        self.contact_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestContact {
        name: String,
        emails_per_week: f64,
    }

    #[test]
    fn loads_valid_json_array() {
        let dir = std::env::temp_dir().join("aurora_test_json_fb");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.json");
        std::fs::write(&path, r#"[{"name":"Test","emails_per_week":5.0}]"#).unwrap();

        let source = JsonFallbackSource::new(&path).unwrap();
        assert_eq!(source.name(), "json_fallback");
        assert!(source.is_available());
        assert_eq!(source.contact_count(), 1);

        let contacts: Vec<TestContact> = source.load().unwrap();
        assert_eq!(contacts[0].name, "Test");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rejects_invalid_json() {
        let dir = std::env::temp_dir().join("aurora_test_json_fb_invalid");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bad.json");
        std::fs::write(&path, "not json").unwrap();

        let result = JsonFallbackSource::new(&path);
        assert!(result.is_err());

        std::fs::remove_dir_all(&dir).ok();
    }
}
```

- [ ] **Step 5：创建邮件抽象层占位（M1 预留）**

创建 `aurora/src/ingest/mail_abstract.rs`：

```rust
//! Mail data source abstraction — M1 reserved, M0 stub.
//!
//! This module defines the interface for mail client integration
//! (Apple Mail, Outlook, Thunderbird). In M0, all methods return
//! `IngestError::MailUnavailable`. M1 will add real implementations.

use super::{DataSource, IngestError};
use serde::de::DeserializeOwned;

/// Mail data source — not yet implemented in M0.
///
/// M1 will add platform-specific implementations behind this facade.
/// For now, `is_available()` always returns `false`.
pub struct MailSource {
    platform: String,
}

impl MailSource {
    /// Create a mail source for the current platform.
    /// Always returns unavailable in M0.
    pub fn new() -> Self {
        Self {
            platform: std::env::consts::OS.to_string(),
        }
    }
}

impl Default for MailSource {
    fn default() -> Self {
        Self::new()
    }
}

impl DataSource for MailSource {
    fn name(&self) -> &str {
        "mail"
    }

    fn is_available(&self) -> bool {
        false // M0: mail not yet implemented
    }

    fn load<T: DeserializeOwned>(&self) -> Result<Vec<T>, IngestError> {
        Err(IngestError::MailUnavailable(
            "Mail source not implemented in M0. Use JSON fallback.".into(),
        ))
    }

    fn contact_count(&self) -> usize {
        0
    }
}
```

- [ ] **Step 6：注册 ingest 模块**

修改 `aurora/src/lib.rs`，在 `pub mod wavelet;` 行之后添加：

```rust
/// Data ingestion layer (M0: JSON fallback, M1: mail).
pub mod ingest;
```

- [ ] **Step 7：运行测试验证通过**

```bash
cargo test --package aurora -- ingest_json
```

Expected: 3 tests PASS.

- [ ] **Step 8：运行全部测试确保无回归**

```bash
cargo test --workspace --all-features
```

Expected: ALL tests PASS.

- [ ] **Step 9：Commit**

```bash
git add aurora/src/ingest/ aurora/tests/ingest_json.rs aurora/src/lib.rs
git commit -m "feat(aurora): DataSource trait + JSON fallback 数据采集层（M0 邮件采集抽象层）"
```

---

### 任务 2：注意力调度最小闭环

**Files:**
- Create: `aurora/src/attention/mod.rs`
- Create: `aurora/tests/attention_session.rs`
- Modify: `aurora/src/lib.rs`
- Modify: `aurora/src/pipeline.rs`
- Modify: `aurora/src/cli.rs`
- Modify: `aurora/src/main.rs`

- [ ] **Step 1：写注意力会话的失败测试**

创建 `aurora/tests/attention_session.rs`：

```rust
//! Attention session tests — ASI tracking and user response recording.

use aurora::attention::{AttentionSession, UserResponse};
use truncore::core::{Frame, TritWord};

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
    let word = TritWord::tru(Frame::Embodied);

    // Record a reminder + user active shift
    session.record_reminder("ShiftTo", "ConflictTrace", "跨 Frame 冲突");
    session.record_user_response(UserResponse::ShiftedTo("ConflictTrace".into()));

    assert_eq!(session.reminder_count(), 1);
    assert_eq!(session.user_active_shift_count(), 1);
    // ASI = (1 active shift) / (1 reminder) * 1.0 = 1.0
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

    // Reminder 1: user shifts
    session.record_reminder("ShiftTo", "ConflictTrace", "冲突1");
    session.record_user_response(UserResponse::ShiftedTo("ConflictTrace".into()));

    // Reminder 2: user ignores
    session.record_reminder("ShiftTo", "Body", "冲突2");
    session.record_user_response(UserResponse::Ignored);

    // Reminder 3: user overrides hold
    session.record_reminder("HoldCurrent", "Meta", "冲突3");
    session.record_user_response(UserResponse::OverrodeHold {
        chosen_frame: "Individual".into(),
    });

    assert_eq!(session.reminder_count(), 3);
    assert_eq!(session.user_active_shift_count(), 2);
    // ASI = 2/3 * 1.0 = 0.666...
    assert!(session.asi() > 0.5 && session.asi() < 0.7);
}
```

- [ ] **Step 2：运行测试验证失败**

```bash
cargo test --package aurora -- attention_session
```

Expected: FAIL — `aurora::attention` module not found.

- [ ] **Step 3：创建 AttentionSession + AttentionManager**

创建 `aurora/src/attention/mod.rs`：

```rust
//! Attention guidance layer — minimal closed loop for M0.
//!
//! [`AttentionSession`] tracks reminders, user responses, and computes
//! the Attention Sovereignty Index (ASI). This is the "注意力调度最小闭环"
//! required by M0.
//!
//! [`AttentionManager`] wraps the existing Trit-Core [`AttentionScheduler`]
//! and feeds its output into the session tracker.

use chrono::Utc;
use truncore::adapters::bandwidth_scheduler::AttentionScheduler;
use truncore::adapters::AttentionCmd;
use truncore::core::TritWord;

/// A user's response to an attention reminder.
#[derive(Debug, Clone, PartialEq)]
pub enum UserResponse {
    /// User actively shifted attention to the suggested target.
    ShiftedTo(String),
    /// User overrode a Hold by choosing a specific frame.
    OverrodeHold { chosen_frame: String },
    /// User saw the reminder but took no action.
    Ignored,
    /// User explicitly dismissed the reminder.
    Dismissed,
}

/// A single attention reminder event.
#[derive(Debug, Clone)]
pub struct ReminderRecord {
    pub timestamp: chrono::DateTime<Utc>,
    pub action: String,
    pub target: String,
    pub rationale: String,
    pub user_response: Option<UserResponse>,
}

/// Tracks attention reminders and user responses across a session.
///
/// Computes the Attention Sovereignty Index (ASI):
///
/// ```text
/// ASI = (user_active_shift_count) / max(reminder_count, 1) * phase_recovery_coefficient
/// ```
///
/// In M0, `phase_recovery_coefficient` defaults to 1.0 (not yet measured).
/// M1 will integrate HarmonicClock phase data.
#[derive(Debug, Clone)]
pub struct AttentionSession {
    session_id: String,
    reminders: Vec<ReminderRecord>,
    user_active_shift_count: usize,
    phase_recovery_coefficient: f64,
}

impl AttentionSession {
    /// Create a new attention session.
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            reminders: Vec::new(),
            user_active_shift_count: 0,
            phase_recovery_coefficient: 1.0, // M0 default
        }
    }

    /// Record a system-generated attention reminder.
    pub fn record_reminder(
        &mut self,
        action: impl Into<String>,
        target: impl Into<String>,
        rationale: impl Into<String>,
    ) {
        self.reminders.push(ReminderRecord {
            timestamp: Utc::now(),
            action: action.into(),
            target: target.into(),
            rationale: rationale.into(),
            user_response: None,
        });
    }

    /// Record the user's response to the most recent reminder.
    pub fn record_user_response(&mut self, response: UserResponse) {
        if let Some(last) = self.reminders.last_mut() {
            if matches!(response, UserResponse::ShiftedTo(_) | UserResponse::OverrodeHold { .. }) {
                self.user_active_shift_count += 1;
            }
            last.user_response = Some(response);
        }
    }

    /// Attention Sovereignty Index in [0.0, 1.0].
    ///
    /// Higher = user is more actively managing their own attention.
    pub fn asi(&self) -> f64 {
        let denominator = self.reminders.len().max(1) as f64;
        (self.user_active_shift_count as f64) / denominator * self.phase_recovery_coefficient
    }

    /// Total number of reminders issued.
    pub fn reminder_count(&self) -> usize {
        self.reminders.len()
    }

    /// Number of times the user actively shifted or overrode.
    pub fn user_active_shift_count(&self) -> usize {
        self.user_active_shift_count
    }

    /// All reminder records (for rendering).
    pub fn reminders(&self) -> &[ReminderRecord] {
        &self.reminders
    }

    /// Session identifier.
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}

/// Manages the attention scheduling loop for a session.
///
/// Wraps Trit-Core's [`AttentionScheduler`] and feeds its commands
/// into an [`AttentionSession`] for tracking.
pub struct AttentionManager {
    scheduler: AttentionScheduler,
    session: AttentionSession,
}

impl AttentionManager {
    /// Create a new attention manager.
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            scheduler: AttentionScheduler::default(),
            session: AttentionSession::new(session_id),
        }
    }

    /// Run one cycle: feed signals to the scheduler, record any reminder.
    ///
    /// Returns the scheduler's command if a non-Continue action was suggested.
    pub fn run_cycle(&mut self, signals: &[TritWord]) -> Option<AttentionCmd> {
        let cmd = self.scheduler.suggest_reprioritization(signals);

        match &cmd {
            AttentionCmd::Continue => None,
            AttentionCmd::HoldCurrent => {
                self.session.record_reminder("HoldCurrent", "Meta", "带宽不足，建议暂停当前处理");
                Some(cmd)
            }
            AttentionCmd::ShiftTo(target) => {
                let target_str = format!("{:?}", target);
                self.session.record_reminder(
                    "ShiftTo",
                    &target_str,
                    "注意力调度建议切换焦点",
                );
                Some(cmd)
            }
            AttentionCmd::Recalibrate => {
                self.session
                    .record_reminder("Recalibrate", "Meta", "连续 Hold 超过阈值，建议重新校准");
                Some(cmd)
            }
        }
    }

    /// Record the user's response to the last reminder.
    pub fn respond(&mut self, response: UserResponse) {
        self.session.record_user_response(response);
    }

    /// Access the underlying session (for rendering).
    pub fn session(&self) -> &AttentionSession {
        &self.session
    }

    /// Mutable access to the session.
    pub fn session_mut(&mut self) -> &mut AttentionSession {
        &mut self.session
    }
}
```

- [ ] **Step 4：注册 attention 模块**

修改 `aurora/src/lib.rs`，在 `pub mod ingest;` 之后添加：

```rust
/// Attention guidance layer (M0: minimal closed loop with ASI).
pub mod attention;
```

- [ ] **Step 5：扩展 DecisionReport 包含 attention 数据**

修改 `aurora/src/pipeline.rs`，在 `DecisionReport` 结构体中添加 `attention_cmd` 和 `asi` 字段：

```rust
use crate::attention::AttentionManager;
use truncore::adapters::AttentionCmd;

/// Structured report produced by the end-to-end pipeline.
#[derive(Debug, Clone)]
pub struct DecisionReport {
    pub input_freq: f64,
    pub detected_freq: f64,
    pub embodied: TritWord,
    pub individual: TritWord,
    pub result: TritWord,
    pub interrupt: Option<MetaInterrupt>,
    /// Attention scheduler command (None if Continue).
    pub attention_cmd: Option<AttentionCmd>,
    /// Current ASI score.
    pub asi: f64,
    /// Number of reminders in this session.
    pub reminder_count: usize,
}
```

在 `run_pipeline` 函数末尾（`detect_conflict` 调用之后，`Ok(DecisionReport { ... })` 之前），添加 attention 逻辑。由于 `run_pipeline` 当前是纯函数（无状态），需要改为接受 `&mut AttentionManager` 参数：

修改 `run_pipeline` 函数签名和实现：

```rust
/// Run the full M0 pipeline on a synthetic signal specification.
///
/// Now includes attention scheduling: after the decision, the attention
/// manager runs one cycle and records any reminders.
pub fn run_pipeline(
    spec: &SignalSpec,
    frequency_threshold: f64,
    user_feels_normal: bool,
    attention: &mut AttentionManager,
) -> Result<DecisionReport, Box<dyn std::error::Error>> {
    let signal = sine_wave(
        spec.freq,
        spec.sample_rate,
        spec.duration_secs,
        spec.noise_std,
    );
    let engine = WaveletEngine::new(spec.sample_rate);
    let wavelet_result = engine.analyze(&signal)?;

    let embodied = embodied_from_frequency(wavelet_result.fundamental_freq, frequency_threshold);
    let individual = individual_from_user_state(user_feels_normal);
    let (result, interrupt) = detect_conflict(&embodied, &individual);

    // Run attention scheduler on the decision signals
    let attention_cmd = attention.run_cycle(&[embodied, individual, result]);

    Ok(DecisionReport {
        input_freq: spec.freq,
        detected_freq: wavelet_result.fundamental_freq,
        embodied,
        individual,
        result,
        interrupt,
        attention_cmd,
        asi: attention.session().asi(),
        reminder_count: attention.session().reminder_count(),
    })
}
```

- [ ] **Step 6：更新 CLI 参数**

修改 `aurora/src/cli.rs`，在 `Args` 结构体中添加 `data_source` 参数：

```rust
/// Path to JSON contacts file for relationship-aware analysis (optional).
#[arg(long)]
pub data_source: Option<PathBuf>,
```

- [ ] **Step 7：更新 main.rs 串联新 pipeline**

修改 `aurora/src/main.rs`，替换 `main` 函数为：

```rust
use anyhow::{Context, Result};
use aurora::attention::AttentionManager;
use aurora::cli::Args;
use aurora::pipeline::{run_pipeline, SignalSpec};
use aurora::render::{html, json};
use clap::Parser;
use std::fs;

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize attention manager for this session (M0: one session per run)
    let mut attention = AttentionManager::new("aurora_session");

    // Load data source if provided (M0: JSON fallback; M1: mail)
    if let Some(ref path) = args.data_source {
        let manager = aurora::ingest::IngestManager::with_json_fallback(path)?;
        eprintln!(
            "Loaded {} contacts from {}",
            manager.contact_count(),
            manager.source().name()
        );
    }

    let input_text = fs::read_to_string(&args.input)
        .with_context(|| format!("failed to read input file {:?}", args.input))?;
    let spec: SignalSpec = serde_json::from_str(&input_text)
        .with_context(|| "failed to parse input JSON as SignalSpec")?;

    let report = run_pipeline(&spec, args.frequency_threshold, args.user_feels_normal, &mut attention)
        .map_err(|e| anyhow::anyhow!("pipeline failed: {e}"))?;

    match args.output {
        Some(path) => {
            let html = html::render(&report, attention.session());
            fs::write(&path, html)
                .with_context(|| format!("failed to write HTML report to {:?}", path))?;
            println!("Report written to {}", path.display());
        }
        None => {
            let json =
                json::to_string(&report).map_err(|e| anyhow::anyhow!("JSON render failed: {e}"))?;
            println!("{}", json);
        }
    }

    Ok(())
}
```

- [ ] **Step 8：运行注意力测试**

```bash
cargo test --package aurora -- attention_session
```

Expected: 5 tests PASS.

- [ ] **Step 9：运行全部测试确保无回归**

```bash
cargo test --workspace --all-features
```

Expected: ALL tests PASS.

- [ ] **Step 10：Commit**

```bash
git add aurora/src/attention/ aurora/tests/attention_session.rs aurora/src/lib.rs aurora/src/pipeline.rs aurora/src/cli.rs aurora/src/main.rs
git commit -m "feat(aurora): 注意力调度最小闭环 — AttentionSession + ASI 追踪"
```

---

### 任务 3：注意力图谱 HTML 渲染

**Files:**
- Modify: `aurora/src/render/html.rs`
- Modify: `aurora/src/render/json.rs`
- Modify: `aurora/src/main.rs`

- [ ] **Step 1：写 HTML 渲染的失败测试**

创建 `aurora/tests/render_attention.rs`：

```rust
//! Attention-aware HTML rendering tests.

use aurora::attention::{AttentionManager, UserResponse};
use aurora::pipeline::{DecisionReport, SignalSpec};
use truncore::core::{Frame, TritWord};
use truncore::meta::MetaInterrupt;
use truncore::meta::ConflictType;

#[test]
fn html_report_includes_asi_section() {
    let mut attention = AttentionManager::new("test_render");
    attention.run_cycle(&[TritWord::tru(Frame::Embodied), TritWord::fals(Frame::Individual)]);
    attention.respond(UserResponse::ShiftedTo("ConflictTrace".into()));

    let report = DecisionReport {
        input_freq: 2.5,
        detected_freq: 2.5,
        embodied: TritWord::tru(Frame::Embodied),
        individual: TritWord::fals(Frame::Individual),
        result: TritWord::hold(Frame::Meta),
        interrupt: Some(MetaInterrupt::new(
            ConflictType::FrameMismatch,
            "Embodied vs Individual conflict",
        )),
        attention_cmd: None,
        asi: attention.session().asi(),
        reminder_count: attention.session().reminder_count(),
    };

    let html = aurora::render::html::render(&report, attention.session());
    assert!(html.contains("Attention Sovereignty Index"));
    assert!(html.contains("ASI"));
    assert!(html.contains("reminder"));
}

#[test]
fn html_report_includes_conflict_panel_when_interrupt_present() {
    let mut attention = AttentionManager::new("test_render");
    let report = DecisionReport {
        input_freq: 2.5,
        detected_freq: 2.5,
        embodied: TritWord::tru(Frame::Embodied),
        individual: TritWord::fals(Frame::Individual),
        result: TritWord::hold(Frame::Meta),
        interrupt: Some(MetaInterrupt::new(
            ConflictType::FrameMismatch,
            "Embodied vs Individual conflict",
        )),
        attention_cmd: None,
        asi: 0.0,
        reminder_count: 0,
    };

    let html = aurora::render::html::render(&report, attention.session());
    assert!(html.contains("Conflict Panel"));
    assert!(html.contains("FrameMismatch"));
}

#[test]
fn html_report_shows_no_conflict_when_interrupt_absent() {
    let mut attention = AttentionManager::new("test_render");
    let report = DecisionReport {
        input_freq: 2.5,
        detected_freq: 2.5,
        embodied: TritWord::tru(Frame::Individual),
        individual: TritWord::tru(Frame::Individual),
        result: TritWord::tru(Frame::Individual),
        interrupt: None,
        attention_cmd: None,
        asi: 0.0,
        reminder_count: 0,
    };

    let html = aurora::render::html::render(&report, attention.session());
    assert!(html.contains("No conflict detected"));
}
```

- [ ] **Step 2：运行测试验证失败**

```bash
cargo test --package aurora -- render_attention
```

Expected: FAIL — `render::html::render` function signature mismatch (now takes 2 args).

- [ ] **Step 3：重写 HTML 渲染函数**

修改 `aurora/src/render/html.rs`，替换 `pub fn render(report: &DecisionReport) -> String` 为接受 `AttentionSession` 的新版本：

```rust
//! HTML rendering for decision reports — M0 attention-aware version.
//!
//! Renders: decision table, conflict panel, ASI gauge, reminder history.

use crate::attention::AttentionSession;
use crate::pipeline::DecisionReport;
use truncore::core::TritValue;

/// Render a full decision report as HTML, including attention data.
pub fn render(report: &DecisionReport, session: &AttentionSession) -> String {
    let conflict_section = render_conflict_panel(report);
    let attention_section = render_attention_section(report, session);
    let reminder_rows = render_reminder_rows(session);

    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <title>Aurora Report — {scenario_id}</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
               margin: 2rem; background: #0d1117; color: #c9d1d9; }}
        h1 {{ color: #58a6ff; }}
        h2 {{ color: #7ee787; margin-top: 2rem; border-bottom: 1px solid #30363d; padding-bottom: 0.5rem; }}
        table {{ border-collapse: collapse; margin-top: 1rem; width: 100%; max-width: 800px; }}
        th, td {{ border: 1px solid #30363d; padding: 0.6rem 1rem; text-align: left; }}
        th {{ background: #161b22; color: #8b949e; }}
        .hold {{ color: #d2991d; font-weight: bold; }}
        .true {{ color: #3fb950; font-weight: bold; }}
        .false {{ color: #f85149; font-weight: bold; }}
        .conflict-panel {{ background: #161b22; border: 1px solid #d2991d; border-radius: 6px;
                           padding: 1rem; margin-top: 1rem; max-width: 800px; }}
        .conflict-panel h3 {{ color: #d2991d; margin-top: 0; }}
        .no-conflict {{ background: #161b22; border: 1px solid #3fb950; border-radius: 6px;
                        padding: 1rem; margin-top: 1rem; max-width: 800px; }}
        .asi-gauge {{ display: flex; align-items: center; gap: 1rem; margin-top: 1rem; }}
        .asi-bar {{ flex: 1; height: 24px; background: #21262d; border-radius: 12px; overflow: hidden; }}
        .asi-fill {{ height: 100%; border-radius: 12px; transition: width 0.5s; }}
        .asi-value {{ font-size: 1.5rem; font-weight: bold; min-width: 4rem; text-align: right; }}
        .reminder {{ padding: 0.5rem; border-left: 3px solid #30363d; margin: 0.5rem 0; }}
        .reminder.shifted {{ border-left-color: #3fb950; }}
        .reminder.ignored {{ border-left-color: #f85149; }}
        .reminder.pending {{ border-left-color: #d2991d; }}
        footer {{ margin-top: 3rem; color: #484f58; font-size: 0.85rem; }}
    </style>
</head>
<body>
    <h1>Aurora Decision Report</h1>
    <table>
        <tr><th>Field</th><th>Value</th></tr>
        <tr><td>Input frequency</td><td>{input_freq:.3} Hz</td></tr>
        <tr><td>Detected frequency</td><td>{detected_freq:.3} Hz</td></tr>
        <tr><td>Embodied signal</td><td><span class="{embodied_class}">{embodied:?}</span> ({embodied_frame})</td></tr>
        <tr><td>Individual report</td><td><span class="{individual_class}">{individual:?}</span> ({individual_frame})</td></tr>
        <tr><td>Decision</td><td><span class="{result_class}">{result:?}</span> ({result_frame})</td></tr>
        <tr><td>ASI</td><td>{asi:.2}</td></tr>
        <tr><td>Reminders</td><td>{reminder_count}</td></tr>
    </table>

    {conflict_section}

    {attention_section}

    <h2>Reminder History</h2>
    <table>
        <tr><th>Time</th><th>Action</th><th>Target</th><th>Response</th></tr>
        {reminder_rows}
    </table>

    <footer>
        <p>Generated by Aurora v0.1.0 — M0 proof-of-concept. 不是指教，是提醒。</p>
    </footer>
</body>
</html>"#,
        scenario_id = "aurora_session",
        input_freq = report.input_freq,
        detected_freq = report.detected_freq,
        embodied_class = css_class(report.embodied.value()),
        embodied = report.embodied.value(),
        embodied_frame = report.embodied.frame(),
        individual_class = css_class(report.individual.value()),
        individual = report.individual.value(),
        individual_frame = report.individual.frame(),
        result_class = css_class(report.result.value()),
        result = report.result.value(),
        result_frame = report.result.frame(),
        asi = report.asi,
        reminder_count = report.reminder_count,
        conflict_section = conflict_section,
        attention_section = attention_section,
        reminder_rows = reminder_rows,
    )
}

fn css_class(value: TritValue) -> &'static str {
    match value {
        TritValue::True => "true",
        TritValue::False => "false",
        TritValue::Hold => "hold",
        TritValue::Unknown => "false",
    }
}

fn render_conflict_panel(report: &DecisionReport) -> String {
    match &report.interrupt {
        Some(interrupt) => format!(
            r#"<div class="conflict-panel">
    <h3>⚡ Conflict Panel — 冲突面板</h3>
    <p><strong>Conflict type:</strong> {:?}</p>
    <p><strong>Reason:</strong> {}</p>
    <p><strong>Structure:</strong> {} vs {}</p>
    <p style="color: #8b949e; font-style: italic;">
        💡 系统不替你判断哪个更"真实"。这是你的注意力被两个方向拉扯的信号。
    </p>
</div>"#,
            interrupt.conflict,
            interrupt.reason,
            report.embodied.frame(),
            report.individual.frame(),
        ),
        None => r#"<div class="no-conflict">
    <p>✅ No conflict detected — signals are aligned.</p>
</div>"#
        .to_string(),
    }
}

fn render_attention_section(report: &DecisionReport, session: &AttentionSession) -> String {
    let asi = session.asi();
    let asi_pct = (asi * 100.0) as u32;
    let bar_color = if asi > 0.6 {
        "#3fb950"
    } else if asi > 0.3 {
        "#d2991d"
    } else {
        "#f85149"
    };

    let cmd_text = match &report.attention_cmd {
        Some(cmd) => format!("{:?}", cmd),
        None => "Continue (no action needed)".into(),
    };

    format!(
        r#"<h2>Attention Sovereignty Index (ASI)</h2>
<div class="asi-gauge">
    <div class="asi-bar">
        <div class="asi-fill" style="width:{asi_pct}%; background:{bar_color};"></div>
    </div>
    <div class="asi-value" style="color:{bar_color};">{asi:.2}</div>
</div>
<p style="color: #8b949e;">
    ASI = 用户主动调度次数 / 系统提醒次数。越高 = 你越自主。
</p>
<p><strong>Last scheduler command:</strong> {cmd_text}</p>
<p><strong>Active shifts:</strong> {active} / <strong>Reminders:</strong> {total}</p>"#,
        asi_pct = asi_pct,
        bar_color = bar_color,
        asi = asi,
        cmd_text = cmd_text,
        active = session.user_active_shift_count(),
        total = session.reminder_count(),
    )
}

fn render_reminder_rows(session: &AttentionSession) -> String {
    session
        .reminders()
        .iter()
        .map(|r| {
            let (response_text, row_class) = match &r.user_response {
                Some(crate::attention::UserResponse::ShiftedTo(t)) => {
                    (format!("Shifted → {}", t), "shifted")
                }
                Some(crate::attention::UserResponse::OverrodeHold { chosen_frame }) => {
                    (format!("Overrode Hold → {}", chosen_frame), "shifted")
                }
                Some(crate::attention::UserResponse::Ignored) => ("Ignored".into(), "ignored"),
                Some(crate::attention::UserResponse::Dismissed) => ("Dismissed".into(), "ignored"),
                None => ("Pending".into(), "pending"),
            };
            format!(
                r#"<tr class="reminder {row_class}">
    <td>{time}</td><td>{action}</td><td>{target}</td><td>{response}</td>
</tr>"#,
                time = r.timestamp.format("%H:%M:%S"),
                action = r.action,
                target = r.target,
                response = response_text,
                row_class = row_class,
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}
```

- [ ] **Step 4：更新 JSON 渲染以包含新字段**

修改 `aurora/src/render/json.rs`，在 `JsonReport` 中添加 `asi` 和 `reminder_count` 字段：

```rust
#[derive(Debug, Serialize)]
pub struct JsonReport {
    pub input_frequency_hz: f64,
    pub detected_frequency_hz: f64,
    pub embodied_value: String,
    pub embodied_frame: String,
    pub individual_value: String,
    pub individual_frame: String,
    pub result_value: String,
    pub result_frame: String,
    pub conflict_detected: bool,
    pub conflict_type: Option<String>,
    pub conflict_reason: Option<String>,
    pub asi: f64,
    pub reminder_count: usize,
}

impl From<&DecisionReport> for JsonReport {
    fn from(r: &DecisionReport) -> Self {
        Self {
            input_frequency_hz: r.input_freq,
            detected_frequency_hz: r.detected_freq,
            embodied_value: format!("{:?}", r.embodied.value()),
            embodied_frame: r.embodied.frame().to_string(),
            individual_value: format!("{:?}", r.individual.value()),
            individual_frame: r.individual.frame().to_string(),
            result_value: format!("{:?}", r.result.value()),
            result_frame: r.result.frame().to_string(),
            conflict_detected: r.interrupt.is_some(),
            conflict_type: r.interrupt.as_ref().map(|i| format!("{:?}", i.conflict)),
            conflict_reason: r.interrupt.as_ref().map(|i| i.reason.clone()),
            asi: r.asi,
            reminder_count: r.reminder_count,
        }
    }
}
```

- [ ] **Step 5：运行渲染测试**

```bash
cargo test --package aurora -- render_attention
```

Expected: 3 tests PASS.

- [ ] **Step 6：运行全部测试确保无回归**

```bash
cargo test --workspace --all-features
```

Expected: ALL tests PASS. `cargo test ethics_` must also pass.

- [ ] **Step 7：Commit**

```bash
git add aurora/src/render/html.rs aurora/src/render/json.rs aurora/src/main.rs aurora/tests/render_attention.rs
git commit -m "feat(aurora): 注意力图谱 HTML 渲染 — ASI 仪表 + 冲突面板 + 提醒历史"
```

---

### 任务 4：端到端集成验证

**Files:**
- Modify: `aurora/tests/cli_end_to_end.rs`

- [ ] **Step 1：更新端到端测试验证新输出**

修改 `aurora/tests/cli_end_to_end.rs` 的 `cli_generates_html_report` 测试，添加对新 HTML 内容的断言：

```rust
#[test]
fn cli_generates_html_report_with_asi() {
    let output = std::env::temp_dir().join("aurora_test_report_v2.html");

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
    assert!(html.contains("Detected frequency"));
    // New M0 assertions:
    assert!(html.contains("Attention Sovereignty Index"), "HTML should contain ASI section");
    assert!(html.contains("Conflict Panel"), "HTML should contain conflict panel");
    assert!(html.contains("Reminder History"), "HTML should contain reminder history");
    std::fs::remove_file(&output).ok();
}
```

- [ ] **Step 2：运行端到端测试**

```bash
cargo test --package aurora -- cli_end_to_end
```

Expected: 2 tests PASS.

- [ ] **Step 3：运行完整测试套件**

```bash
cargo test --workspace --all-features
cargo test ethics_
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Expected: ALL pass.

- [ ] **Step 4：Commit**

```bash
git add aurora/tests/cli_end_to_end.rs
git commit -m "test(aurora): 更新端到端测试验证 ASI + 冲突面板 + 提醒历史"
```

---

## 完成检查

- [ ] `cargo test --workspace --all-features` 全部通过
- [ ] `cargo test ethics_` 10 项全部通过
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` 无警告
- [ ] `cargo fmt --check` 无差异
- [ ] JSON fallback 可加载联系人文件
- [ ] AttentionSession 正确计算 ASI
- [ ] HTML 渲染包含 ASI 仪表、冲突面板、提醒历史
- [ ] 端到端 CLI 测试验证新输出
