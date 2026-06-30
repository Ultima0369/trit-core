# Aurora BC 架构硬化设计

**版本**: 0.1.0  
**日期**: 2026-06-23  
**状态**: 待审批  
**父文档**: `docs/superpowers/specs/2026-06-20-aurora-architecture-design.md`

---

## 一、目标

消除 M0/M1 两套代码并存的问题，将 6 个 BC 模块从"骨架"升级为"工作系统"。

### 1.1 当前问题

```
Pipeline (M0) ──→ decision/adapter.rs (旧)    BC attention_guidance.rs (新，未使用)
              ├─→ decision/conflict.rs (旧)     BC ternary_decision.rs (新，未使用)
              ├─→ attention/mod.rs (旧)         BC signal_analysis.rs (新，包装旧 wavelet)
              └─→ render/html.rs (旧)           BC presentation.rs (新，独立实现)
                                                BC audit_trail.rs (新，未使用)
                                                BC relationship_annotation.rs (新，未使用)

❌ 两套代码并存，Pipeline 只走旧路径，BC + SQLite 从未被调用
```

### 1.2 目标状态

```
main.rs ─┬─→ 分析链路: SignalAnalysis BC → wavelet/ (引擎)
         │              → TernaryDecision BC → trit-core (引擎)
         │
         └─→ 注意力链路: AttentionGuidance BC → AttentionScheduler (引擎)
                         → AuditTrail BC → SQLite (持久化)
         
         结果呈现: Presentation BC → HTML/JSON

✅ 旧 decision/、attention/、render/ 删除
✅ Pipeline 拆为两条独立链路
✅ main.rs 做编排
✅ wavelet/、ingest/ 保留为引擎层
```

---

## 二、架构设计

### 2.1 新模块结构

```
aurora/src/
├── bc/                          # BC 模块（门面层）
│   ├── mod.rs                   # BcError + 公共类型
│   ├── signal_analysis.rs       # WaveletEngine trait + FftWaveletEngine
│   ├── relationship_annotation.rs # AnnotationStore trait + InMemory store
│   ├── ternary_decision.rs      # DecisionPort trait + TritDecisionEngine
│   ├── attention_guidance.rs    # AttentionPort trait + AttentionManager
│   ├── audit_trail.rs           # AuditPort trait + InMemoryAuditLog
│   └── presentation.rs          # RenderPort trait + AuroraRenderer
│
├── wavelet/                     # 引擎层（保留）
│   ├── mod.rs
│   ├── synthetic.rs
│   └── detect.rs
│
├── ingest/                      # 引擎层（保留）
│   ├── mod.rs
│   ├── json_fallback.rs
│   └── mail_abstract.rs
│
├── db/                          # 持久化层
│   ├── mod.rs                   # Database + DbError
│   ├── schema.rs
│   ├── migrate.rs
│   ├── aurora_dir.rs
│   ├── annotation_store.rs      # SqliteAnnotationStore
│   └── audit_log.rs             # SqliteAuditLog
│
├── pipeline/                    # 两条链路（新）
│   ├── mod.rs
│   ├── analysis.rs              # 分析链路: SignalAnalysis → TernaryDecision
│   └── attention.rs             # 注意力链路: AttentionGuidance → AuditTrail
│
├── cli.rs                       # CLI 参数（保留，调整）
├── main.rs                      # 编排入口（重写）
└── lib.rs                       # 公共导出（更新）
```

### 2.2 两条链路

#### 分析链路 (`pipeline/analysis.rs`)

```
输入: SignalSpec { freq, sample_rate, duration, noise }
  │
  ▼
SignalAnalysis BC (FftWaveletEngine)
  │ 调用 wavelet::sine_wave() + wavelet::WaveletEngine::analyze()
  │ 返回 FrequencySpectrum
  ▼
TernaryDecision BC (TritDecisionEngine)
  │ 将 FrequencySpectrum 转为 TritWord[] 
  │ 调用 TernaryAlgebra::t_and_n()
  │ 返回 DecisionRecord { result, interrupts }
  ▼
输出: AnalysisReport { spectrum, decision }
```

#### 注意力链路 (`pipeline/attention.rs`)

```
输入: &[TritWord] (来自分析链路的决策结果)
  │
  ▼
AttentionGuidance BC (AttentionManager)
  │ 调用 AttentionScheduler::suggest_reprioritization()
  │ 返回 Option<AttentionCmd> + 更新 AttentionSession
  ▼
AuditTrail BC (SqliteAuditLog)
  │ 写入 audit_log 表
  │ 记录 decision_snapshot + override_record
  ▼
输出: AttentionOutcome { cmd, asi, reminder_count }
```

### 2.3 main.rs 编排

```rust
fn main() {
    let cli = Cli::parse();
    let db = Database::open(&cli.db_path)?;
    
    // 两条链路独立运行
    let analysis = analysis::run(&cli.signal, &cli.thresholds)?;
    let attention = attention::run(&analysis.decision.signals(), &db)?;
    
    // 呈现
    let view = ViewState::from((analysis, attention));
    let renderer = AuroraRenderer;
    write_output(&cli.output, renderer.render_html(&view))?;
}
```

---

## 三、删除清单

### 3.1 要删除的文件

| 文件 | 原因 | BC 替代 |
|------|------|---------|
| `aurora/src/decision/mod.rs` | 旧决策层 | `bc/ternary_decision.rs` |
| `aurora/src/decision/adapter.rs` | embodied_from_frequency / individual_from_user_state | 内联到分析链路 |
| `aurora/src/decision/conflict.rs` | detect_conflict | TritDecisionEngine::evaluate() |
| `aurora/src/attention/mod.rs` | 旧注意力模块 | `bc/attention_guidance.rs` |
| `aurora/src/render/mod.rs` | 旧渲染模块 | `bc/presentation.rs` |
| `aurora/src/render/html.rs` | 旧 HTML 渲染 | AuroraRenderer::render_html() |
| `aurora/src/render/json.rs` | 旧 JSON 渲染 | AuroraRenderer::render_json() |
| `aurora/src/pipeline.rs` | 旧单管道 | `pipeline/analysis.rs` + `pipeline/attention.rs` |

### 3.2 要保留的文件

| 文件 | 原因 |
|------|------|
| `aurora/src/wavelet/` | FFT 引擎，BC 门面调用 |
| `aurora/src/ingest/` | DataSource trait，未来多点接入 |
| `aurora/src/cli.rs` | CLI 参数，调整后保留 |
| `aurora/src/bc/` | BC 模块，本次硬化的主体 |
| `aurora/src/db/` | SQLite 持久化 |
| `aurora/src/lib.rs` | 公共导出（更新） |

---

## 四、关键设计决策

### 4.1 分析链路的信号映射

`embodied_from_frequency()` 和 `individual_from_user_state()` 当前在 `decision/adapter.rs` 中。
它们是将"频率 / 用户状态 → TritWord"的映射逻辑。删除后，这个映射内联到分析链路中：

```rust
// pipeline/analysis.rs
fn frequency_to_tritword(freq: f64, threshold: f64) -> TritWord {
    if freq > threshold {
        TritWord::tru(Frame::Embodied)
    } else {
        TritWord::fals(Frame::Embodied)
    }
}

fn user_state_to_tritword(feels_normal: bool) -> TritWord {
    if feels_normal {
        TritWord::tru(Frame::Individual)
    } else {
        TritWord::fals(Frame::Individual)
    }
}
```

### 4.2 AttentionGuidance 与 DB 的关系

`AttentionManager` 是纯内存结构（`AttentionSession` + `AttentionScheduler`）。`AuditTrail` BC 负责持久化。它们的连接点在注意力链路中：

```rust
// pipeline/attention.rs
pub fn run(signals: &[TritWord], db: &Database) -> Result<AttentionOutcome> {
    let mut attention = AttentionManager::new(uuid());
    let cmd = attention.run_cycle(signals);
    
    // 持久化到 SQLite
    let mut audit = SqliteAuditLog::new(db.clone());
    let entry = AuditEntry::new(Decision, session_id, "attention cycle")
        .with_decision_snapshot(snapshot);
    audit.record(entry)?;
    
    Ok(AttentionOutcome { cmd, asi: attention.asi(), ... })
}
```

### 4.3 InMemory vs SQLite 的选择

- **分析链路**：完全无状态，不需要持久化。使用 BC 的默认引擎即可。
- **注意力链路**：需要持久化（审计日志）。`main.rs` 通过 CLI 参数决定使用 InMemory 还是 SQLite：
  - `--db-path ~/.aurora/data/aurora.db` → SQLite
  - 无 `--db-path` → InMemory（测试/演示用）

### 4.4 测试策略

- BC 模块现有 47+ 测试全部保留
- 新增两条链路的集成测试（`pipeline/analysis.rs` + `pipeline/attention.rs` 的 `#[cfg(test)]`）
- 端到端测试更新：从旧 `run_pipeline()` 迁移到新的两条链路调用
- 删除的旧模块测试迁移到新位置（如有价值）

---

## 五、风险与缓解

| 风险 | 缓解 |
|------|------|
| 旧测试引用被删模块导致编译失败 | 先更新所有引用再删除，逐文件操作 |
| `cargo test ethics_` 依赖旧 Pipeline | 更新伦理测试指向新链路 |
| CLI 参数变化破坏现有脚本 | `cli.rs` 增量调整，保持 `--input`/`--output` 接口 |
| BC 模块内联测试与实际行为不符 | 每条链路写完后立即运行全量测试 |

---

## 六、验收标准

1. `cargo build --workspace` 通过
2. `cargo test --workspace --all-features` 全部通过（含更新后的端到端测试）
3. `cargo test ethics_` 10 项全部通过
4. `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过
5. `cargo fmt --check` 通过
6. `grep -r "decision::" aurora/src/` 返回空（旧模块引用全部消除）
7. `grep -r "attention::" aurora/src/` 仅命中 `bc::attention_guidance`
8. `cargo run --bin aurora -- --input synthetic_2hz.json --output report.html` 可运行并输出有效 HTML
