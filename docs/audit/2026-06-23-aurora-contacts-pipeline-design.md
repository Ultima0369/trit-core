# Aurora Contacts 数据接入 Pipeline 设计

**版本**: 0.1.0  
**日期**: 2026-06-23  
**状态**: 已审批  
**父文档**: `docs/superpowers/specs/2026-06-23-aurora-bc-hardening-design.md`

---

## 一、目标

将 `IngestManager` 加载的 contacts 数据通过 `RelationshipAnnotation` BC 映射为 TritWord，送入分析链路的 `TritDecisionEngine::evaluate()`，并将参与决策的 contacts 完整信息写入 `AuditTrail`。

### 1.1 当前问题

```
main.rs 加载 contacts → 打印计数 → 丢弃 ❌
RelationshipAnnotation BC → 完整实现 → 从未被 pipeline 调用 ❌
SqliteAnnotationStore → 完整 CRUD + 测试 → 从未被 pipeline 调用 ❌
ViewState.radar → 字段存在 → 从未被填充 ❌
```

### 1.2 目标状态

```
main.rs
  │
  ├── IngestManager::load() → Vec<ContactProfile>
  │
  ├── contacts_to_tritwords(contacts) → Vec<TritWord>
  │       每个 contact 的 frame_annotation → TritWord(frame, phase)
  │
  ├── analysis::run_analysis(spec, threshold, feels_normal, &contact_signals)
  │       embodied + individual + [contact_tritwords...]
  │       → TritDecisionEngine::evaluate(session, &all_signals, domain)
  │
  └── attention::run_attention(&decision.input_signals, db, &contacts)
          ↓
        audit entry 附带 contact_participation snapshot
```

---

## 二、架构设计

### 2.1 数据流

```
JSON contacts file (--data-source)
  │
  ▼
IngestManager::with_json_fallback(path)
  │ load::<ContactProfile>()
  ▼
Vec<ContactProfile>
  │
  ├──────────────────────────────────────────┐
  │                                          │
  ▼                                          ▼
contacts_to_tritwords()                 build_snapshot(contacts)
  │ 每个 annotation → TritWord              │ 每个 contact → ContactAuditRecord
  │ phase 无效 → 跳过 + warning             │
  ▼                                          ▼
&[TritWord] (contact_signals)           AuditDecisionSnapshot
  │                                          │
  ▼                                          │
run_analysis(spec, threshold,           run_attention(signals, db)
  feels_normal, &contact_signals)         │
  │                                       ▼
  ▼                                    SqliteAuditLog::record(entry)
TritDecisionEngine::evaluate(             │ contact_participation → JSON
  &[embodied, individual,                │
     contact1, contact2, ...],             ▼
  "General")                             audit_log 表
  │
  ▼
AnalysisReport { spectrum, decision, contact_count }
```

### 2.2 关键设计决策

1. **全部 contacts 参与决策** — 不做过滤，让 `t_and_n` 批量处理。用户通过控制 `--data-source` 文件内容来控制参与范围。
2. **直接使用 contact 的 phase** — `TritWord::new(value, Phase::new(contact.phase), frame)`，保留用户精细标定。
3. **完整 audit 记录** — 每个 contact 的 frame、phase、relation_label、trit_value 都写入 snapshot。
4. **无效 phase 跳过** — phase 不在 [0,1] 范围的 annotation 被跳过，通过 `eprintln!` 输出 warning。

---

## 三、具体变更

### 3.1 `pipeline/analysis.rs` — 扩展 run_analysis

**新增函数** `contacts_to_tritwords`：

```rust
use crate::bc::relationship_annotation::ContactProfile;
use truncore::core::{Frame, Phase, TritValue, TritWord};

/// Convert loaded contacts to TritWords for decision input.
///
/// Each contact's frame annotations are mapped to TritWords.
/// Annotations with invalid phase values are skipped with a warning.
pub fn contacts_to_tritwords(contacts: &[ContactProfile]) -> Vec<TritWord> {
    let mut words = Vec::new();
    for contact in contacts {
        for ann in &contact.frames {
            match Phase::new(ann.phase) {
                Ok(phase) => {
                    let value = if phase.inner() >= 0.5 {
                        TritValue::True
                    } else {
                        TritValue::False
                    };
                    let frame = match Frame::from_str(&ann.frame) {
                        Ok(f) => f,
                        Err(_) => {
                            eprintln!(
                                "warning: unknown frame '{}' for contact {}, skipping",
                                ann.frame, contact.name
                            );
                            continue;
                        }
                    };
                    words.push(TritWord::new(value, phase, frame));
                }
                Err(_) => {
                    eprintln!(
                        "warning: invalid phase {} for contact {}, skipping",
                        ann.phase, contact.name
                    );
                }
            }
        }
    }
    words
}
```

**扩展 `run_analysis`** — 新增参数 `contact_signals: &[TritWord]`：

```rust
pub fn run_analysis(
    spec: &SignalSpec,
    frequency_threshold: f64,
    user_feels_normal: bool,
    contact_signals: &[TritWord],
) -> Result<AnalysisReport, BcError> {
    // ... existing signal generation + FFT ...

    // Merge all signals
    let mut all_signals = vec![embodied, individual];
    all_signals.extend_from_slice(contact_signals);

    let decision = decision_engine.evaluate(&mut session, &all_signals, "General")?;

    Ok(AnalysisReport {
        spectrum,
        decision,
        contact_count: contact_signals.len(),
    })
}
```

**扩展 `AnalysisReport`** — 新增字段：

```rust
pub struct AnalysisReport {
    pub spectrum: FrequencySpectrum,
    pub decision: DecisionRecord,
    /// Number of contact-derived TritWords that participated in the decision.
    pub contact_count: usize,
}
```

### 3.2 `bc/audit_trail.rs` — 扩展 AuditDecisionSnapshot

**新增类型**：

```rust
/// A record of a single contact's participation in a decision.
#[derive(Debug, Clone)]
pub struct ContactAuditRecord {
    pub contact_id: String,
    pub contact_name: String,
    pub relation_label: String,
    pub frame: String,
    pub phase: f64,
    pub trit_value: String,
}
```

**扩展 `AuditDecisionSnapshot`** — 新增字段：

```rust
pub struct AuditDecisionSnapshot {
    pub signal_count: usize,
    pub signal_frames: Vec<String>,
    pub result_value: String,
    pub result_frame: String,
    /// Per-contact participation records (None if no contacts participated).
    pub contact_participation: Option<Vec<ContactAuditRecord>>,
}
```

### 3.3 `pipeline/attention.rs` — 扩展 build_snapshot

**扩展 `build_snapshot`** — 新增参数 `contacts: &[ContactProfile]`：

```rust
fn build_snapshot(signals: &[TritWord], contacts: &[ContactProfile]) -> AuditDecisionSnapshot {
    let contact_participation = if contacts.is_empty() {
        None
    } else {
        Some(
            contacts
                .iter()
                .flat_map(|c| {
                    c.frames.iter().map(|ann| {
                        let value = if ann.phase >= 0.5 { "True" } else { "False" };
                        ContactAuditRecord {
                            contact_id: c.id.clone(),
                            contact_name: c.name.clone(),
                            relation_label: c.relation_label.as_str().to_string(),
                            frame: ann.frame.clone(),
                            phase: ann.phase,
                            trit_value: value.to_string(),
                        }
                    })
                })
                .collect(),
        )
    };

    AuditDecisionSnapshot {
        signal_count: signals.len(),
        signal_frames: signals.iter().map(|s| s.frame().to_string()).collect(),
        result_value: "pending".into(),
        result_frame: "Meta".into(),
        contact_participation,
    }
}
```

**扩展 `run_attention`** — 新增参数 `contacts: &[ContactProfile]`：

```rust
pub fn run_attention(
    signals: &[TritWord],
    db: Database,
    contacts: &[ContactProfile],
) -> Result<AttentionOutcome, BcError> {
    let mut attention = AttentionManager::new("attention_session");
    let cmd = attention.run_cycle(signals);

    let snapshot = build_snapshot(signals, contacts);
    // ... rest unchanged ...
}
```

### 3.4 `main.rs` — 编排变更

```rust
fn main() -> Result<()> {
    let args = Args::parse();
    let db = /* ... */;

    // Load contacts
    let contacts: Vec<ContactProfile> = if let Some(ref path) = args.data_source {
        let manager = aurora::ingest::IngestManager::with_json_fallback(path)?;
        eprintln!(
            "Loaded {} contacts from {}",
            manager.contact_count(),
            manager.source_name()
        );
        manager.load::<ContactProfile>()?
    } else {
        Vec::new()
    };

    // Convert contacts to TritWords
    let contact_signals = analysis::contacts_to_tritwords(&contacts);

    // Read and parse input
    let spec: analysis::SignalSpec = /* ... */;

    // ── Link 1: Analysis (with contacts) ─────────────────────────
    let analysis_report = analysis::run_analysis(
        &spec,
        args.frequency_threshold,
        args.user_feels_normal,
        &contact_signals,
    )
    .map_err(|e| anyhow::anyhow!("analysis link failed: {e}"))?;

    // ── Link 2: Attention (with contacts in audit) ───────────────
    let attention_outcome =
        attention::run_attention(&analysis_report.decision.input_signals, db, &contacts)
            .map_err(|e| anyhow::anyhow!("attention link failed: {e}"))?;

    // ── Presentation ────────────────────────────────────────────
    // ... existing rendering code ...
}
```

### 3.5 `db/audit_log.rs` — 扩展 SQLite 持久化

`SqliteAuditLog::record()` 中扩展 `snapshot_json` 序列化，新增 `contact_participation` 数组：

```json
{
  "signal_count": 5,
  "signal_frames": ["Embodied", "Individual", "Embodied", "Individual", "Science"],
  "result_value": "Hold",
  "result_frame": "Meta",
  "contact_participation": [
    {
      "contact_id": "c1",
      "contact_name": "张三",
      "relation_label": "colleague",
      "frame": "Embodied",
      "phase": 0.8,
      "trit_value": "True"
    }
  ]
}
```

---

## 四、ContactProfile 序列化支持

`ContactProfile` 包含内部字段（`history: Vec<AnnotationChange>`）不适合直接反序列化。
使用独立的 `ContactInput` 类型作为 JSON 加载的中间表示，再通过 `From` trait 转换为 `ContactProfile`。

### 4.1 `ContactInput` 类型（新增在 `bc/relationship_annotation.rs`）

```rust
/// JSON-deserializable contact input format.
#[derive(Debug, Clone, Deserialize)]
pub struct ContactInput {
    pub id: String,
    pub name: String,
    pub relation_label: String,
    #[serde(default)]
    pub annotations: Vec<FrameAnnotationInput>,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FrameAnnotationInput {
    pub frame: String,
    #[serde(default)]
    pub annotation: String,
    pub phase: f64,
}

impl From<ContactInput> for ContactProfile {
    fn from(input: ContactInput) -> Self {
        let relation_label = match input.relation_label.as_str() {
            "colleague" => RelationLabel::Colleague,
            "friend" => RelationLabel::Friend,
            "family" => RelationLabel::Family,
            "partner" => RelationLabel::Partner,
            "self" => RelationLabel::Self_,
            other => RelationLabel::Other(other.to_string()),
        };
        let mut profile = ContactProfile::new(input.id, input.name, relation_label);
        profile.notes = input.notes;
        for ann in input.annotations {
            if let Ok(fa) = FrameAnnotation::new(ann.frame, ann.annotation, ann.phase) {
                profile.annotate_frame(fa);
            }
        }
        profile
    }
}
```

### 4.2 JSON 格式

```json
[
  {
    "id": "c1",
    "name": "张三",
    "relation_label": "colleague",
    "annotations": [
      {"frame": "Embodied", "annotation": "高频沟通，身体在场感强", "phase": 0.8}
    ]
  },
  {
    "id": "c2",
    "name": "李四",
    "relation_label": "friend",
    "annotations": [
      {"frame": "Individual", "annotation": "低频但深度", "phase": 0.3}
    ]
  }
]
```

### 4.3 main.rs 加载流程

```rust
let contacts: Vec<ContactProfile> = if let Some(ref path) = args.data_source {
    let manager = IngestManager::with_json_fallback(path)?;
    let inputs: Vec<ContactInput> = manager.load()?;
    inputs.into_iter().map(ContactProfile::from).collect()
} else {
    Vec::new()
};
```

---

## 五、不变

- 不修改任何 BC trait 签名
- `RelationshipAnnotation` BC 的 `AnnotationStore` trait 保持不变
- `InMemoryAnnotationStore` 和 `SqliteAnnotationStore` 保持不变
- 现有 114 个测试全部保持通过
- `#![forbid(unsafe_code)]` 不变

---

## 六、测试策略

### 6.1 单元测试（`pipeline/analysis.rs` `#[cfg(test)]`）

- `contacts_to_tritwords_empty` — 空 contacts 返回空 Vec
- `contacts_to_tritwords_maps_frame_and_phase` — 正确映射 frame/phase → TritWord
- `contacts_to_tritwords_skips_invalid_phase` — phase > 1.0 被跳过
- `contacts_to_tritwords_skips_unknown_frame` — 未知 frame 被跳过
- `run_analysis_with_contacts` — contacts 参与决策，contact_count 正确
- `run_analysis_without_contacts` — 空 contacts 不崩溃，contact_count = 0

### 6.2 单元测试（`pipeline/attention.rs` `#[cfg(test)]`）

- `build_snapshot_with_contacts` — contact_participation 包含正确记录
- `build_snapshot_without_contacts` — contact_participation 为 None

### 6.3 集成测试

- `contacts_end_to_end` — 创建 temp contacts.json → CLI `--data-source` → HTML 输出包含 contact 信息

---

## 七、验收标准

1. `cargo build --workspace` 通过
2. `cargo test --workspace --all-features` 全部通过
3. `cargo test ethics_` 10 项全部通过
4. `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过
5. `cargo fmt --check` 通过
6. `cargo run --bin aurora -- --input synthetic_2hz.json --data-source contacts.json --output report.html` 可运行，HTML 含 contact 参与信息
7. audit_log 表 `snapshot_json` 字段包含 `contact_participation` 数组
