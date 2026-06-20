# 文档整理与架构设计落地计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 三合一：写架构设计文档 + 更新代码导航 + 建立 SESSION_START 快速定位文件，确保下次开机 30 秒内定位工作进度。

**Architecture:** 三个并行任务——架构文档是本次对话的核心产出；代码导航是维护现有双螺旋知识库；SESSION_START 是新增的"开机导航"文件，解决"文档众多、AI 失忆迷路"的问题。

**Tech Stack:** Markdown 文档，无代码变更。

---

## 任务 1：写架构设计文档

**Files:**
- Create: `docs/superpowers/specs/2026-06-20-aurora-architecture-design.md`

- [ ] **Step 1：创建架构设计文档**

写入以下完整内容到 `docs/superpowers/specs/2026-06-20-aurora-architecture-design.md`：

```markdown
# Aurora 个人版架构设计文档

**版本**: 0.1.0
**日期**: 2026-06-20
**状态**: 已批准 — 技术方案阶段产出
**范围**: M0-M1 个人版（模块化单体），分布式接口预留但本期不实现

---

## 1. 架构风格选择

**推荐方案：模块化单体（Modular Monolith）**

**选择理由**：

1. M0-M1 阶段全部单机运行，无分布式需求。模块化单体编译为单一二进制，零网络依赖、零序列化开销、零服务发现——个人用户场景的最优解。
2. Trit-Core 已验证的 `core → meta → sandbox` 单向分层可直接复用，Aurora 在此基础上增量扩展，不推翻现有架构。
3. 分布式接口通过 trait 抽象预留（`DataSource`、`DecisionPort`），未来 M3/M4 在 trait 后放 gRPC stub 即可，业务逻辑不变。
4. Rust 编译时依赖检查消除循环依赖——不需要运行时治理工具。

**不选其他方案的原因**：

| 方案 | 不选原因 |
|------|----------|
| 微服务 | 个人用户场景不存在独立部署/扩缩容需求。引入微服务 = 引入网络延迟、序列化开销、分布式调试——全是成本，零收益。 |
| 纯单体（无模块边界） | 放弃模块边界会退化为"上帝模块"。Aurora 的五层架构天然是模块边界，必须保持。 |

**未来演进路径**：
```
M0-M1：模块化单体（Tauri 桌面应用，Trit-Core 嵌入）
  → M2：本地服务化（Trit-Core 决策引擎作为独立本地进程，IPC 通信）
  → M3：可选团队服务器（局域网内服务，端到端加密同步）
  → M4：企业私有部署（企业内网完整运行）
```

---

## 2. 系统架构图（C4 Level 2 文字版）

```
┌──────────────────────────────────────────────────────────────────────┐
│                        Aurora Desktop App                             │
│                                                                       │
│  ┌─────────────┐    ┌──────────────────┐    ┌────────────────────┐   │
│  │   Tauri UI   │    │  Presentation    │    │  本地文件系统       │   │
│  │  (React/     │───▶│  Layer           │    │  ~/.aurora/        │   │
│  │   Svelte)    │    │  render::html    │    │  ├── data/         │   │
│  └─────────────┘    │  render::json     │    │  ├── config/       │   │
│         │           └────────┬─────────┘    │  └── audit/        │   │
│         │                    │              └────────────────────┘   │
│         ▼                    ▼                                        │
│  ┌──────────────────────────────────────────────┐                    │
│  │          Application Layer (aurora/)          │                    │
│  │                                               │                    │
│  │  ┌──────────┐  ┌──────────┐  ┌────────────┐  │                    │
│  │  │ pipeline │  │   cli    │  │  feedback   │  │                    │
│  │  │ 模块     │  │  模块    │  │  loop       │  │                    │
│  │  └────┬─────┘  └────┬─────┘  └─────┬──────┘  │                    │
│  │       └──────────────┼──────────────┘         │                    │
│  │                      ▼                        │                    │
│  │  ┌───────────────────────────────────────┐    │                    │
│  │  │         Domain Layer                   │    │                    │
│  │  │  ┌──────────┐  ┌──────────────────┐   │    │                    │
│  │  │  │ decision │  │    wavelet       │   │    │                    │
│  │  │  │ (适配层) │  │  (信号分析引擎)   │   │    │                    │
│  │  │  └────┬─────┘  └────────┬─────────┘   │    │                    │
│  │  └───────┼─────────────────┼─────────────┘    │                    │
│  └──────────┼─────────────────┼──────────────────┘                    │
│             │                 │                                       │
│             ▼                 ▼                                       │
│  ┌──────────────────────────────────────────────┐                    │
│  │        Trit-Core Engine (trit-core/)          │                    │
│  │                                               │                    │
│  │  ┌──────────┐  ┌──────────┐  ┌────────────┐  │                    │
│  │  │  anchor  │  │   meta   │  │  security   │  │                    │
│  │  │ (锚点层) │  │(策略引擎)│  │ (安全门禁)  │  │                    │
│  │  └────┬─────┘  └────┬─────┘  └──────┬─────┘  │                    │
│  │       └──────────────┼───────────────┘         │                    │
│  │                      ▼                        │                    │
│  │  ┌───────────────────────────────────────┐    │                    │
│  │  │           Core Algebra                 │    │                    │
│  │  │  TritValue · Phase · Frame · TritWord │    │                    │
│  │  │  TernaryAlgebra (TAND/TOR/TNOT)       │    │                    │
│  │  └───────────────────────────────────────┘    │                    │
│  └──────────────────────────────────────────────┘                    │
│             │                                                         │
│             ▼                                                         │
│  ┌──────────────────────────────────────────────┐                    │
│  │           Data Layer (本地 SQLite)            │                    │
│  │  ┌──────────┐  ┌──────────┐  ┌────────────┐  │                    │
│  │  │ 通信元数据│  │ 关系标注 │  │  审计日志   │  │                    │
│  │  │ (只读)   │  │ (用户写) │  │ (只追加)   │  │                    │
│  │  └──────────┘  └──────────┘  └────────────┘  │                    │
│  └──────────────────────────────────────────────┘                    │
└──────────────────────────────────────────────────────────────────────┘
```

**各组件职责**：

| 组件 | 职责 |
|------|------|
| Tauri UI | 用户交互界面：关系图谱、冲突面板、注意力提醒、设置。不包含业务逻辑。 |
| Presentation Layer | 将决策结果渲染为 HTML/JSON/图表。纯函数，输入数据→输出视图。 |
| Application Layer | 流程编排：pipeline 串联数据采集→信号分析→决策→反馈。不含领域逻辑。 |
| Domain Layer | 领域逻辑：小波信号分析、Trit-Core 适配（物理量→TritWord 映射）。 |
| Trit-Core Engine | 三元决策引擎：代数运算、策略仲裁、安全门禁、锚点约束。零 UI 依赖。 |
| Data Layer | 本地 SQLite 存储：通信元数据（只读）、用户关系标注（读写）、审计日志（只追加）。 |
| 本地文件系统 | `~/.aurora/` 目录：用户数据、配置、审计日志。用户完全控制。 |

---

## 3. 限界上下文（BC）划分

| BC名称 | 职责（一句话） | 核心实体 | 聚合根 | 对外接口 |
|--------|---------------|----------|--------|----------|
| **SignalAnalysis** | 将通信元数据时间序列转换为频域特征（基频、谐波、相位漂移） | `TimeSeries`, `FrequencyPeak`, `WaveletResult`, `SignalQuality` | `FrequencySpectrum` | `WaveletEngine` trait |
| **RelationshipAnnotation** | 管理用户对人际关系的主动标注（关系类型、Frame 归属、关注级别） | `RelationLabel`, `ContactProfile`, `FrameAnnotation`, `AnnotationHistory` | `ContactProfile` | `AnnotationStore` trait |
| **TernaryDecision** | 将标注后的信号映射为 TritWord，执行三值运算，输出决策结果和冲突解释 | `TritWord`, `Frame`, `MetaInterrupt`, `ArbitrationResult`, `DecisionRecord` | `DecisionSession` | `DecisionPort` trait |
| **AttentionGuidance** | 基于决策结果生成注意力提醒，记录用户响应，计算注意力自主性指数（ASI） | `AttentionReminder`, `UserResponse`, `ShiftTarget`, `ASIMetric` | `AttentionSession` | `AttentionPort` trait |
| **AuditTrail** | 记录所有决策输入/输出/用户操作，提供不可篡改的审计日志 | `AuditEntry`, `DecisionSnapshot`, `OverrideRecord` | `AuditLog` | `AuditPort` trait |
| **Presentation** | 将内部数据渲染为用户可见的视图（关系图谱、冲突面板、节奏报告） | `RadarChart`, `ConflictCard`, `RhythmReport`, `ExportFormat` | `ViewState` | `RenderPort` trait |

**BC 依赖关系（单向，无循环）**：

```
SignalAnalysis ─────┐
                    ├──▶ TernaryDecision ──▶ AttentionGuidance ──▶ Presentation
RelationshipAnnotation ─┘        │                                    │
                                 │                                    │
                                 ▼                                    ▼
                            AuditTrail ◀──────────────────────────────┘
                              (所有 BC 都写入 AuditTrail，但 AuditTrail 不依赖任何 BC)
```

**依赖方向验证**：
- `SignalAnalysis → TernaryDecision`：信号分析结果作为决策输入（单向）
- `RelationshipAnnotation → TernaryDecision`：用户标注作为决策输入（单向）
- `TernaryDecision → AttentionGuidance`：决策结果触发注意力提醒（单向）
- `AttentionGuidance → Presentation`：提醒和 ASI 数据驱动视图（单向）
- `所有 BC → AuditTrail`：各 BC 写入审计日志，但 AuditTrail 不反向依赖任何 BC（单向写入）

✅ 无循环依赖。每个 BC 的依赖数 ≤ 2。

---

## 4. 接口契约（核心 API 示例）

### 4.1 决策引擎核心接口

```
POST /api/v1/decisions/evaluate
```

**请求格式**：
```json
{
  "session_id": "uuid-v4",
  "domain": "Relational",
  "signals": [
    {
      "source": "communication_frequency",
      "frame": "Embodied",
      "value": 1,
      "phase": 0.78,
      "confidence": 0.92,
      "metadata": {
        "contact_id": "uuid-v4",
        "window_days": 30,
        "base_frequency_hz": 2.4,
        "trend": "increasing"
      }
    },
    {
      "source": "user_annotation",
      "frame": "Individual",
      "value": -1,
      "phase": 0.65,
      "confidence": 1.0,
      "metadata": {
        "contact_id": "uuid-v4",
        "relation_label": "colleague",
        "user_mood": "drained"
      }
    }
  ],
  "context": {
    "timezone": "Asia/Shanghai",
    "previous_session_id": null
  }
}
```

**响应格式**：
```json
{
  "session_id": "uuid-v4",
  "final_value": "Hold",
  "final_value_code": 0,
  "final_frame": "Meta",
  "final_phase": 0.50,
  "policy_action": "Negotiate",
  "interrupts": [
    {
      "conflict": "FrameMismatch",
      "reason": "Embodied signal (communication frequency increasing) conflicts with Individual signal (user reports feeling drained by this contact)",
      "severity": "P1"
    }
  ],
  "attention_suggestion": {
    "action": "ShiftTo",
    "target": "ConflictTrace",
    "rationale": "跨 Frame 冲突检测到关系节奏与个人感受不一致，建议审视此关系"
  },
  "hold_explanation": {
    "summary": "你的沟通频率显示与 [contact_name] 的联系在增加，但你标注与此人相处感到消耗。这两个信号指向不同方向——系统不替你判断哪个更'真实'，而是提醒你注意这个冲突。",
    "conflict_structure": "Embodied(↑) vs Individual(↓)",
    "user_action": "你可以选择：调整与此人的沟通边界 / 重新审视自己的感受 / 暂时不做任何改变"
  },
  "audit_entry_id": "uuid-v4",
  "created_at": "2026-06-20T15:30:00+08:00"
}
```

**状态码与错误模型**：

| 状态码 | 含义 | 错误模型 |
|--------|------|----------|
| 200 | 决策完成 | 正常响应体 |
| 400 | 输入验证失败 | `{ "error": { "code": "INVALID_SIGNAL", "message": "signal[1].phase out of range [0.0, 1.0]", "details": { "index": 1, "field": "phase", "value": 1.5 }, "trace_id": "uuid-v4" } }` |
| 401 | 未授权（未来 M3 团队版） | `{ "error": { "code": "UNAUTHORIZED", "message": "Valid session required", "trace_id": "uuid-v4" } }` |
| 422 | 不可处理的冲突 | `{ "error": { "code": "UNRESOLVABLE_CONFLICT", "message": "All frames are Absolute — no resolution possible", "details": { "frames": ["Absolute", "Absolute"] }, "trace_id": "uuid-v4" } }` |
| 500 | 内部错误 | `{ "error": { "code": "INTERNAL", "message": "Unexpected error in decision engine", "trace_id": "uuid-v4" } }` |

**版本策略**：URL 路径版本（`/api/v1/...`）。v1 是当前版本。v2 引入时，v1 保留至少 12 个月并标记 deprecated。

### 4.2 关系标注接口

```
GET    /api/v1/contacts                    # 列出所有联系人
POST   /api/v1/contacts                    # 创建联系人标注
PUT    /api/v1/contacts/{contact_id}       # 更新标注
DELETE /api/v1/contacts/{contact_id}       # 删除标注（软删除，审计日志保留）
GET    /api/v1/contacts/{contact_id}/history  # 标注变更历史
```

**POST /api/v1/contacts 请求示例**：
```json
{
  "name": "张三",
  "relation_label": "colleague",
  "frames": [
    { "frame": "Individual", "annotation": "相处后感到消耗", "phase": 0.35 },
    { "frame": "Relational", "annotation": "工作依赖关系，无法完全断开", "phase": 0.70 }
  ],
  "attention_level": "monitor",
  "notes": "项目结束后重新评估"
}
```

---

## 5. 数据一致性方案

| 场景 | 一致性要求 | 方案 | 补偿机制 |
|------|-----------|------|----------|
| 用户标注写入 | 强一致（ACID） | 单机 SQLite 事务 | 不涉及分布式，SQLite WAL 模式保证原子性 |
| 决策结果 → 审计日志 | 强一致（ACID） | 同一事务内写入决策记录 + 审计条目 | 不涉及分布式 |
| 通信元数据导入 | 最终一致 | 批量导入 + 幂等键（source + timestamp 唯一约束） | 重复导入自动跳过，失败可重试 |
| 注意力提醒 → 用户响应 | 最终一致 | 提醒发出后，用户响应可延迟（无超时强制） | 用户始终可以事后查看和响应 |
| 未来 M3 团队同步 | 最终一致（BASE） | 本地消息表 + 端到端加密同步 | 冲突时以用户本地数据为准（用户主权优先） |

**本期不涉及分布式事务**（M0-M1 全部单机 SQLite）。上表第 5 行仅作为未来预留标注。

---

## 6. 非功能性需求（NFR）

| 指标 | 目标值 | 度量方式 | 备注 |
|------|--------|----------|------|
| 单次决策延迟 | P99 < 100ms | `SandboxDiagnostics.elapsed_us` | 当前合成数据基准：~50μs（不含 I/O） |
| 日数据分析吞吐 | < 1 秒（365 天数据） | `cargo bench` 基准测试 | 已在 CTO_ROADMAP 中定义 |
| 应用冷启动 | < 3 秒 | Tauri 启动计时 | M1 验收标准 |
| 内存占用（空闲） | < 200MB | OS 资源监控 | 待确认——取决于 Tauri WebView 开销 |
| 内存占用（分析中） | < 500MB | OS 资源监控 | 已在 CTO_ROADMAP 中定义 |
| 可用性 | 离线可用 100%（无云端依赖） | 断网测试 | 本地优先架构天然满足 |
| 数据保留 | 用户完全控制：可导出（JSON/SQLite）、可删除、无锁定期 | 功能测试 | CHARTER 要求 |
| 数据存储上限 | 默认保留 5 年元数据，用户可调整 | 配置项 | 待确认 |
| 审计日志保留 | 永久（除非用户主动删除） | 功能测试 | 只追加，不自动清理 |
| 伦理门禁 | 10 个测试全部通过，禁止跳过 | `cargo test ethics_` | CI 强制 |

---

## 7. 风险评估

| 风险 | 概率 | 影响 | 缓解方案 |
|------|------|------|----------|
| 用户不理解 Hold 的含义，认为系统"坏了" | 高 | P1 | UI 层明确解释冲突结构（`hold_explanation` 字段）；文档强调"Hold 不是失败" |
| 关系标注数据稀疏（用户懒得标注） | 高 | P1 | 提供智能默认值（基于通信频率自动建议 Frame）；标注不是必填项；无标注时仍可做纯元数据分析 |
| 小波分析对稀疏数据（低频联系人）精度不足 | 中 | P2 | 对数据点 < 30 的联系人降级为简单统计（均值/方差），不强行做频域分析 |
| Tauri WebView 在 Linux 上的兼容性问题 | 中 | P2 | 优先支持 macOS/Windows；Linux 用 CLI 模式作为 fallback |
| 用户数据隐私泄露（本地文件被其他应用读取） | 低 | P0 | SQLite 加密（SQLCipher）；`~/.aurora/` 目录权限 0700；审计日志记录所有数据访问 |
| 铁律 5（最小权限）：模块间出现隐式耦合 | 低 | P1 | 所有 BC 间通信通过 trait 接口；CI 中检查 `cargo modules` 依赖图；禁止 `pub(crate)` 跨 BC 访问 |
| 铁律 6（契约优先）：API 变更破坏前端 | 中 | P1 | URL 路径版本化；JSON Schema 验证；契约测试（consumer-driven contract） |

---

## 8. 铁律合规检查

| 铁律 | 合规状态 | 说明 |
|------|----------|------|
| 铁律 1（可复现） | ✅ | CHANGELOG.md 已存在，每个 PR 有变更说明 |
| 铁律 2（测试先行） | ✅ | 核心路径有测试覆盖；伦理门禁测试不可跳过 |
| 铁律 3（无感知回归） | ✅ | API 版本化（v1/v2）；性能基准防止劣化 > 5% |
| 铁律 4（可观测性） | ✅ | 结构化 JSON 日志（tracing-subscriber）；SandboxDiagnostics 提供每阶段耗时 |
| 铁律 5（最小权限） | ✅ | BC 单向依赖，无循环；接口通过 trait 暴露，内部实现不可见 |
| 铁律 6（契约优先） | ✅ | 本文档先定义接口契约，再考虑实现 |
| 铁律 7（零技术债沉积） | ✅ | 不增加新债；`cargo clippy -D warnings` 在 CI 中强制 |

---

## 9. 术语精确定义

- **限界上下文（BC）**：领域模型的一个有效边界，BC 内部术语一致，BC 之间通过 trait 接口通信
- **聚合根（AR）**：一组相关对象的入口，外部只能通过 AR 访问聚合内部
- **Saga**：分布式事务的补偿模式——每个步骤有对应的补偿操作（本期不涉及）
- **C4 模型**：架构描述的 4 层视图（Context→Container→Component→Code）
- **CAP 定理**：一致性（Consistency）、可用性（Availability）、分区容错（Partition tolerance）三者只能同时满足两个
- **模块化单体**：编译为单一二进制，但内部按 BC 划分模块，模块间通过 trait 通信，可未来拆分为独立服务

---

*本架构设计文档为 Aurora 个人版（M0-M1）技术方案阶段产出。所有技术决策以 CHARTER.md 为最高判据。分布式接口预留但本期不实现。不是指教，是提醒。*
```

- [ ] **Step 2：验证文件写入成功**

```bash
wc -l docs/superpowers/specs/2026-06-20-aurora-architecture-design.md
```

Expected: 文件存在，行数 > 200。

- [ ] **Step 3：Commit**

```bash
git add docs/superpowers/specs/2026-06-20-aurora-architecture-design.md
git commit -m "docs: Aurora 个人版架构设计文档（M0-M1 模块化单体）"
```

---

## 任务 2：更新代码导航 map/06_code.md

**Files:**
- Modify: `map/06_code.md`

- [ ] **Step 1：更新 map/06_code.md 反映当前代码实际状态**

当前 `map/06_code.md` 引用了不存在的文件（如 `src/core/trit.rs`、`src/meta/arbitration.rs`、`src/meta/security_mode.rs`），且缺少 v0.3.0 新增的模块（`adapters/`、`anchor/`、`budget/`、`calibration/`、`feedback/`、`hook/`、`security/`、`clock.rs`、`src/core/decision_engine.rs`、`src/core/hold.rs`、`src/core/sensor.rs`、`src/meta/frame_mask.rs`、`src/meta/rules.rs`）。

用以下完整内容替换 `map/06_code.md`：

```markdown
# MOC — 代码链导航

> **Scope**: 从源码文件出发，指向对应的知识文档。这是"链 A → 链 B"的反向连接。
>
> **最后更新**: 2026-06-20（v0.3.0 同步）
>
> #trit-core #code #source #implementation #cross-chain

---

## 核心代数（`src/core/`）

| 源码文件 | 职责 | 对应文档 |
|---|---|---|
| `src/core/value.rs` | `TritValue` enum（4 状态：True/Hold/False/Unknown） | [[CONCEPTS]] §1, [[001-ternary-logic]], [[003-ternary-over-binary]] |
| `src/core/frame.rs` | `Frame` enum（5 变体：Science/Individual/Consensus/Absolute/Meta）+ `FrameRegistry` | [[CONCEPTS]] §2, [[004-geoeco-frame]] |
| `src/core/phase.rs` | `Phase` struct（[0.0, 1.0]）+ `Commitment` enum | [[CONCEPTS]] §3, [[PHASE_ARITHMETIC]], [[002-phase-arithmetic]] |
| `src/core/algebra.rs` | `TernaryAlgebra`（TAND/TOR/TNOT + 热路径 + `t_and_n` 批量） | [[CONCEPTS]] §1.4, [[PHASE_ARITHMETIC]] |
| `src/core/word.rs` | `TritWord`（值 + 帧 + 相位，字段私有，构造器强制不变量） | [[CONCEPTS]] §4, [[api]] |
| `src/core/hold.rs` | `HoldState`, `HoldFinality`, `HolderConfig` | [[CONCEPTS]] §5, [[003-domain-conflict]] |
| `src/core/sensor.rs` | `SensorSignal`, `BodyState`, `CogState`, `EnvSnapshot`, `EnvironmentalContext`, `TemporalScale`, `TextInput` | [[WAVELET_ANALYSIS]], [[PIPELINE_DESIGN]] |
| `src/core/decision_engine.rs` | `DecisionEngine` facade：TAND → 仲裁 → 反射审计 → SafeFallback | [[ARCHITECTURE]], [[PIPELINE_DESIGN]] |
| `src/core/mod.rs` | 核心模块导出 | [[MODULES]], [[ARCHITECTURE]] |

---

## 元监控（`src/meta/`）

| 源码文件 | 职责 | 对应文档 |
|---|---|---|
| `src/meta/interrupt.rs` | `MetaInterrupt`, `ConflictType`（FrameMismatch/OutOfScope/PhaseDrift/PolicyViolation/ExplainImpulse）, `MetaMonitor`, `PolicyViolation` | [[CONCEPTS]] §2, [[003-domain-conflict]] |
| `src/meta/domain.rs` | `Domain` enum, `ResolutionPolicy::arbitrate()`, `ArbitrationResult`, `PolicyError`, `DomainParseError` | [[CONCEPTS]] §3, [[003-domain-conflict]], [[CONFLICT_CATALOG]] |
| `src/meta/rules.rs` | `CustomRule`, `RuleLoader` trait, `JsonRuleLoader`, `FallbackBehavior` enum, `RuleError` | [[CUSTOM_RULE]], [[CONFIGURATION]] |
| `src/meta/safe_fallback.rs` | `SafeFallback`（IEC 61508 风格安全覆盖，可关闭） | [[CONCEPTS]] §5, [[009-ethics-hardening]], [[SECURITY_MODEL]] |
| `src/meta/frame_mask.rs` | O(1) 位掩码帧存在性检查（内部模块） | [[ARCHITECTURE]] |
| `src/meta/mod.rs` | 元模块导出 | [[MODULES]], [[ARCHITECTURE]] |

---

## 适配器层（`src/adapters/`）— v0.3.0 新增

| 源码文件 | 职责 | 对应文档 |
|---|---|---|
| `src/adapters/mod.rs` | `CognitiveModule` trait + `ModuleInput`/`ModuleOutput`/`FeedbackSignal`/`AttentionCmd`/`ShiftTarget` | [[COGNITIVE_ARCHITECTURE_LAYERS]], [[CTO_ROADMAP]] |
| `src/adapters/reflexive_audit.rs` | `ReflexiveAuditor`, `ReflexiveAlert`, `AuditReport`, `PhaseShift`, `AttentionEvent` | [[SECURITY_MODEL]], [[009-ethics-hardening]] |
| `src/adapters/self_knowledge.rs` | `SelfKnowledge`, `CalibrationEvent`, `ReceiverEstimate`, `ResponsePattern`, `TriggerSignature` | [[COGNITIVE_ARCHITECTURE_LAYERS]] |
| `src/adapters/bandwidth_scheduler.rs` | `AttentionScheduler`, `LoadProfile`, `bandwidth_from_depth()` | [[ATTENTION_CAPITALISM]], [[CTO_ROADMAP]] |
| `src/adapters/cognitive_deconstruction.rs` | `CognitiveDeconstruction` | [[COGNITIVE_ARCHITECTURE_LAYERS]] |
| `src/adapters/conflict_suspension.rs` | `ConflictSuspension` | [[CONFLICT_CATALOG]] |
| `src/adapters/coupling_adapter.rs` | `CouplingAdapter` | [[COGNITIVE_ARCHITECTURE_LAYERS]] |
| `src/adapters/critical_thinking.rs` | `CriticalThinking` | [[COGNITIVE_ARCHITECTURE_LAYERS]] |
| `src/adapters/ecological_assessment.rs` | `EcologicalAssessment` | [[ENVIRONMENTAL_SHOCK]], [[004-geoeco-frame]] |
| `src/adapters/engineering.rs` | `EngineeringArchitecture` | [[COGNITIVE_ARCHITECTURE_LAYERS]] |
| `src/adapters/adaptive_iteration.rs` | `AdaptiveIteration` | [[COGNITIVE_ARCHITECTURE_LAYERS]] |

---

## 锚点层（`src/anchor/`）— v0.3.0 新增

| 源码文件 | 职责 | 对应文档 |
|---|---|---|
| `src/anchor/mod.rs` | `AnchorConstraint` trait, `AnchorReport`, `AnchorViolation`, `AnchorSeverity`, `DecisionPreview`, `DataSource`, `StaticSource`, `EcosystemZone`, `check_all()` | [[CHARTER]], [[FIRST_PRINCIPLES]] |
| `src/anchor/thermal_baseline.rs` | 热基线锚点 | [[CHARTER]] |
| `src/anchor/survival_motives.rs` | 生存动机锚点 | [[CHARTER]] |
| `src/anchor/flourishing_pool.rs` | 繁荣池锚点 | [[CHARTER]] |
| `src/anchor/ecological_base.rs` | 生态基础锚点 | [[ENVIRONMENTAL_SHOCK]], [[004-geoeco-frame]] |
| `src/anchor/wellbeing_priority.rs` | 福祉优先级锚点 | [[CHARTER]] |

---

## 反馈层（`src/feedback/`）— v0.3.0 新增

| 源码文件 | 职责 | 对应文档 |
|---|---|---|
| `src/feedback/mod.rs` | `FeedbackLoop` facade, `ConsequencePrediction`, `CorrectionHint`, `CorrectionSeverity`, `PracticeTestResult` | [[CTO_ROADMAP]] §五层架构-反馈层 |
| `src/feedback/practice_test.rs` | 练习测试比较器（加权偏差公式） | [[CTO_ROADMAP]] |
| `src/feedback/proxy_env.rs` | `ProxyEnvironment` trait + `StaticRuleModel` MVP | [[CTO_ROADMAP]] |
| `src/feedback/consequence_review.rs` | `ConsequenceReview` | [[CTO_ROADMAP]] |
| `src/feedback/correction.rs` | `CorrectionTrigger` | [[CTO_ROADMAP]] |
| `src/feedback/experience_recorder.rs` | `ExperienceRecorder` | [[CTO_ROADMAP]] |

---

## Hook 管理层（`src/hook/`）— v0.3.0 新增

| 源码文件 | 职责 | 对应文档 |
|---|---|---|
| `src/hook/mod.rs` | `HookManager`, `HookContext`, `HoldStrategy`, `ScenarioType`, `IterationSummary`, `UnmountReason` | [[COGNITIVE_ARCHITECTURE_LAYERS]], [[CTO_ROADMAP]] |
| `src/hook/context_cache.rs` | `ContextCache` | [[COGNITIVE_ARCHITECTURE_LAYERS]] |
| `src/hook/module_registry.rs` | `ModuleRegistry`, `ModuleEntry`, `ModuleId`, `ModuleState`, `RegistryAction`, `RegistryEvent` | [[COGNITIVE_ARCHITECTURE_LAYERS]] |
| `src/hook/mount_arbiter.rs` | `MountArbiter`, `Resource`, `ResourceCost` | [[COGNITIVE_ARCHITECTURE_LAYERS]] |
| `src/hook/scenario_recognizer.rs` | `recognize()`, `recognize_with_score()` | [[COGNITIVE_ARCHITECTURE_LAYERS]] |

---

## 安全层（`src/security/`）— v0.3.0 新增

| 源码文件 | 职责 | 对应文档 |
|---|---|---|
| `src/security/mod.rs` | `SecurityMode` enum（Service/Refusal/Awareness/Transparency） | [[009-ethics-hardening]], [[SECURITY_MODEL]], [[CHARTER]] |

---

## 预算与时钟（`src/budget/`, `src/clock/`, `src/calibration/`）— v0.3.0 新增

| 源码文件 | 职责 | 对应文档 |
|---|---|---|
| `src/budget/mod.rs` | `ComputeBudget`：硬件感知计算预算 + 深度级别门控 | [[CTO_ROADMAP]], [[ARCHITECTURE]] |
| `src/clock.rs` | `HarmonicClock`：相位振荡器（physical ω=10.0 / deliberative ω=0.5） | [[PHASE_ARITHMETIC]], [[002-phase-arithmetic]] |
| `src/calibration/mod.rs` | `CalibrationLog`, `CalibrationEntry`：决策历史记录 | [[CTO_ROADMAP]] |

---

## 沙盒层（`src/sandbox/`）

| 源码文件 | 职责 | 对应文档 |
|---|---|---|
| `src/sandbox/input.rs` | `ScenarioInput`, `SignalInput` | [[PIPELINE_DESIGN]], [[QUICKSTART]], [[CLI_REFERENCE]] |
| `src/sandbox/output.rs` | `SandboxOutput` | [[PIPELINE_DESIGN]], [[api]] |
| `src/sandbox/validate.rs` | 输入验证与净化（`MAX_JSON_SIZE`, `MAX_SIGNALS`, `MAX_STRING_LEN`） | [[PIPELINE_DESIGN]], [[TESTING_STRATEGY]] |
| `src/sandbox/pipeline.rs` | 14 阶段主管道：验证→TAND→仲裁→反射→SafeFallback→预算→注意力→时钟→锚点→输出→校准→反馈 | [[PIPELINE_DESIGN]], [[ARCHITECTURE]] |
| `src/sandbox/diagnostic.rs` | `SandboxDiagnostics`：每阶段耗时、中断计数、帧分布、SafeFallback 激活 | [[ARCHITECTURE]] |
| `src/sandbox/error.rs` | `SandboxError` + `ErrorCategory` + 帮助文本 | [[api]], [[CLI_REFERENCE]] |
| `src/sandbox/validator.rs` | `ScenarioValidator`：预期行为验证（hold/commit_true/commit_false/negotiate） | [[TESTING_STRATEGY]], [[validation-report]] |
| `src/sandbox/mod.rs` | 沙盒模块导出 | [[MODULES]], [[ARCHITECTURE]] |

---

## 二进制入口（`src/bin/`）

| 源码文件 | 职责 | 对应文档 |
|---|---|---|
| `src/bin/sandbox.rs` | `trit-sandbox` CLI（thin wrapper over SandboxPipeline） | [[CLI_REFERENCE]], [[QUICKSTART]], [[CONTRIBUTING]] |
| `src/bin/dhat_profile.rs` | 堆内存分析（dhat feature-gated） | [[BENCHMARK]], [[DEPLOYMENT_GUIDE]] |
| `src/bin/adversarial_audit.rs` | 对抗性审计工具 | [[SECURITY_MODEL]], [[security-audit]] |

---

## 根目录文件

| 源码文件 | 职责 | 对应文档 |
|---|---|---|
| `src/lib.rs` | 公共 API 导出（v0.3.0 全部模块） | [[api]], [[MODULES]] |
| `src/tracing_init.rs` | tracing-subscriber 初始化（JSON/文本格式） | [[ARCHITECTURE]] |
| `Cargo.toml` | workspace：trit-core + aurora | [[CONTRIBUTING]], [[DEPLOYMENT_GUIDE]] |
| `Cargo.lock` | 锁定依赖版本 | [[CONTRIBUTING]] |
| `deny.toml` | 依赖审计策略 | [[SECURITY_MODEL]], [[CONTRIBUTING]] |
| `tarpaulin.toml` | 代码覆盖率配置 | [[TESTING_STRATEGY]] |

---

## Aurora 专用代码（M0 已实现）

| 源码位置 | 职责 | 对应文档 |
|---|---|---|
| `aurora/src/main.rs` | CLI 入口 | [[CLI_REFERENCE]], [[QUICKSTART]], [[CTO_ROADMAP]] |
| `aurora/src/lib.rs` | 库入口：wavelet / decision / cli / pipeline / render | [[CTO_ROADMAP]] |
| `aurora/src/cli.rs` | 命令行参数（clap derive） | [[CLI_REFERENCE]], [[PIPELINE_DESIGN]] |
| `aurora/src/pipeline.rs` | 端到端管道：合成信号 → 小波 → 三值 → 报告 | [[PIPELINE_DESIGN]], [[CTO_ROADMAP]] |
| `aurora/src/wavelet/synthetic.rs` | 合成正弦波生成器（可控噪声） | [[WAVELET_ANALYSIS]], [[WAVELET_ENGINE_SPEC]] |
| `aurora/src/wavelet/detect.rs` | 基频检测（FFT 基，M1 引入 CWT） | [[WAVELET_ANALYSIS]], [[WAVELET_ENGINE_SPEC]], [[002-wavelet-over-fft]] |
| `aurora/src/wavelet/mod.rs` | 小波模块导出 | [[WAVELET_ANALYSIS]] |
| `aurora/src/decision/adapter.rs` | 基频 → TritWord（Embodied/Individual 映射） | [[TRIT_CORE_INTEGRATION_SPEC]] |
| `aurora/src/decision/conflict.rs` | 跨域冲突检测 → Hold + MetaInterrupt | [[CONFLICT_CATALOG]], [[SECURITY_MODEL]] |
| `aurora/src/decision/mod.rs` | 决策模块导出 | [[TRIT_CORE_INTEGRATION_SPEC]] |
| `aurora/src/render/json.rs` | JSON 决策报告 | [[UI_SPEC]], [[api]] |
| `aurora/src/render/html.rs` | HTML 报告渲染（plotters 图表） | [[UI_SPEC]] |
| `aurora/src/render/mod.rs` | 渲染模块导出 | [[UI_SPEC]] |
| `aurora/tests/smoke.rs` | 冒烟测试 | [[TESTING_STRATEGY]] |
| `aurora/tests/wavelet_detect.rs` | 小波检测单元测试 | [[WAVELET_ENGINE_SPEC]] |
| `aurora/tests/decision_conflict.rs` | 决策冲突单元测试 | [[TRIT_CORE_INTEGRATION_SPEC]] |
| `aurora/tests/cli_end_to_end.rs` | CLI 端到端测试 | [[PIPELINE_DESIGN]] |
| `aurora/tests/ethics_gates.rs` | 10 个伦理门禁测试（不可跳过） | [[009-ethics-hardening]], [[TECH_REVIEW_CHECKLIST]] |
| `aurora/benches/aurora_bench.rs` | Aurora 性能基准 | [[BENCHMARK]] |

---

## Aurora 预留代码（M1-M4，尚未实现）

| 源码位置 | 职责 | 对应文档 |
|---|---|---|
| `src-tauri/` | Tauri GUI 框架（M1） | [[006-tauri-over-electron]], [[UI_SPEC]], [[CTO_ROADMAP]] §M1 |
| `aurora/src/db/` | SQLite 数据层（M1） | [[DATA_MODEL]], [[007-sqlite-over-postgres]] |
| `aurora/src/attention/` | 注意力调度引擎（M0 剩余） | [[ATTENTION_CAPITALISM]], [[CTO_ROADMAP]] |
| `aurora/src/ingest/` | 邮件/日历/通知采集（M0 剩余） | [[DATA_INGESTION_SPEC]], [[CTO_ROADMAP]] §M0 |
| `aurora/src/security/` | SecurityMode 落地与审计日志（M1） | [[SECURITY_MODEL]], [[009-ethics-hardening]] |

---

## 使用这个导航

### 场景：修改代码后同步文档

1. 确定修改的源码文件
2. 在本 MOC 中找到该文件的对应文档列表
3. 打开每个文档，检查是否需要更新
4. 如果修改涉及架构决策，检查是否需要新增 ADR

### 场景：新开发者理解代码意图

1. 从本 MOC 找到目标源码文件
2. 阅读对应文档，理解设计意图
3. 如果文档不足，追溯 ADR 和哲学文档

---

**相关 MOC**: [[01_manifest]] · [[02_concepts]] · [[03_adr]] · [[05_engineering]]

#map-of-content #code #source #implementation #cross-chain #navigation
```

- [ ] **Step 2：验证更新后的文件结构**

```bash
grep -c "v0.3.0" map/06_code.md
```

Expected: 至少 5 处提及 v0.3.0。

- [ ] **Step 3：Commit**

```bash
git add map/06_code.md
git commit -m "docs: 同步 map/06_code.md 至 v0.3.0 代码实际状态（新增 adapters/anchor/feedback/hook/security/budget/calibration 模块）"
```

---

## 任务 3：建立 SESSION_START.md 开机导航文件

**Files:**
- Create: `SESSION_START.md`

- [ ] **Step 1：创建 SESSION_START.md**

写入以下内容到项目根目录 `SESSION_START.md`：

```markdown
# ⚡ SESSION_START — 开机导航

> **目的**：每次新会话开始时，AI 或人类协作者读此文件，30 秒内定位工作进度。
> **维护规则**：每次完成一个阶段或做出重大决策后，更新"当前进度"和"上次决策"两节。
> **版本**：1.0.0
> **最后更新**：2026-06-20

---

## 1. 项目是什么（一句话）

**Trit-Core**：三元决策引擎（Rust 库），用 True/Hold/False 替代二元逻辑。
**Aurora**：基于 Trit-Core 的注意力主权训练系统（桌面应用），帮助用户梳理人际关系、提升决策质量。

---

## 2. 当前进度

| 维度 | 状态 |
|------|------|
| **Trit-Core 版本** | v0.3.0 — 单机决策引擎，5 层架构完整（Core→Meta→Hook→Adapter→Feedback） |
| **Aurora 阶段** | M0 概念验证 — 合成数据→小波→三值→CLI/HTML 输出已完成；伦理门禁 10 个测试通过 |
| **架构设计** | 已完成 — 见 `docs/superpowers/specs/2026-06-20-aurora-architecture-design.md` |
| **M0 剩余工作** | 邮件采集抽象层 + JSON fallback / 注意力调度最小闭环 / 注意力图谱 HTML 渲染 |
| **M1 入口** | Tauri 桌面应用（见 `aurora/06_roadmap/CTO_ROADMAP.md` §M1） |

---

## 3. 上次决策（最近 3 个）

| 日期 | 决策 | 文档 |
|------|------|------|
| 2026-06-20 | Aurora 架构风格：模块化单体（M0-M1），分布式接口 trait 预留，本期不实现 | `docs/superpowers/specs/2026-06-20-aurora-architecture-design.md` |
| 2026-06-20 | 产品定位确认：B 层（关系标注层）— 基于物理量客观分析 + 用户手动标注关系类型，不替用户决策 | 本次对话 |
| 2026-06-20 | 技术方案由 AI 全权负责，认知科学/哲学/初心由创始人兜底 | 本次对话 |

---

## 4. 文档导航（按阅读顺序）

### 新协作者（第一次加入）

1. **本文件**（你在读的）— 30 秒了解进度
2. `aurora/MASTER_PLAN.md` — 唯一执行入口
3. `aurora/06_roadmap/CTO_ROADMAP.md` — CTO 级战略规划
4. `aurora/00_manifest/CHARTER.md` — 不可谈判的 4 条底线
5. `map/00_START_HERE.md` — 双螺旋知识库入口

### AI 协作者（每次新会话）

1. **本文件** — 看"当前进度"和"上次决策"
2. `CLAUDE.md` — 项目技术约束（构建命令、架构、设计规则）
3. `map/06_code.md` — 代码→文档的交叉引用
4. `docs/superpowers/specs/2026-06-20-aurora-architecture-design.md` — 最新架构设计

### 按主题深入

| 你想了解... | 入口 |
|------------|------|
| 为什么用三值逻辑 | `docs/adr/001-ternary-logic.md` |
| Frame 系统怎么工作 | `docs/explanation/CONCEPTS.md` §2 |
| 安全模型 | `aurora/03_whitepaper/SECURITY_MODEL.md` |
| 伦理约束 | `aurora/00_manifest/CHARTER.md` |
| 认知架构 5 层 | `aurora/00_manifest/COGNITIVE_ARCHITECTURE_LAYERS.md` |
| 所有架构决策 | `map/03_adr.md` |
| API 契约 | `docs/reference/api.md` |

---

## 5. 关键文件清单（防迷路）

```
项目根目录/
├── SESSION_START.md          ← ⚡ 你在这里
├── CLAUDE.md                 ← AI 协作者必读（技术约束）
├── README.md                 ← 项目主页
├── CHANGELOG.md              ← 版本变更记录
├── SECURITY.md               ← 安全策略
│
├── src/                      ← Trit-Core 源码（Rust 库）
│   ├── core/                 ← 三值代数核心
│   ├── meta/                 ← 策略引擎
│   ├── adapters/             ← 认知模块池（10 个模块）
│   ├── anchor/               ← 锚点约束层
│   ├── feedback/             ← 反馈循环层
│   ├── hook/                 ← Hook 管理层
│   ├── security/             ← 安全门禁
│   ├── sandbox/              ← 场景管道
│   ├── budget/               ← 计算预算
│   ├── calibration/          ← 校准日志
│   └── clock.rs              ← 相位振荡器
│
├── aurora/                   ← Aurora 应用（Rust 二进制）
│   ├── MASTER_PLAN.md        ← 唯一执行入口
│   ├── 00_manifest/          ← 宪章、原则、认知架构
│   ├── 01_insights/          ← 认知科学洞察
│   ├── 02_math/              ← 数学模型
│   ├── 03_whitepaper/        ← 技术白皮书
│   ├── 04_engineering/       ← 工程实现
│   ├── 05_adr/               ← 架构决策记录（9 个）
│   ├── 06_roadmap/           ← 路线图 + CTO_ROADMAP
│   ├── 07_specs/             ← 详细规格
│   ├── 08_reports/           ← 报告模板
│   └── src/                  ← Aurora 源码
│
├── docs/                     ← Trit-Core 技术文档
│   ├── adr/                  ← 架构决策记录（4 个）
│   ├── explanation/          ← 架构、概念、哲学
│   ├── how-to/               ← CLI 参考、配置、贡献
│   ├── reference/            ← API、模块、基准
│   ├── reports/              ← 验证与审计报告
│   ├── tutorials/            ← 快速上手
│   └── superpowers/specs/    ← 架构设计文档
│
├── map/                      ← Obsidian 风格知识库 MOC
│   ├── 00_START_HERE.md      ← 双螺旋入口
│   ├── 01_manifest.md        ← 宣言 MOC
│   ├── 02_concepts.md        ← 概念 MOC
│   ├── 03_adr.md             ← ADR MOC
│   ├── 04_math.md            ← 数学 MOC
│   ├── 05_engineering.md     ← 工程 MOC
│   ├── 06_code.md            ← 代码导航 MOC
│   ├── 07_insights.md        ← 洞察 MOC
│   └── 99_tag_index.md       ← 标签索引
│
├── 圆桌会议.md               ← 哲学对话记录
├── 开悟.md                   ← 长篇哲学论述
├── 审计2023.6.19.md          ← 审计记录
└── 自审计.md                 ← 自我审计
```

---

## 6. 快速命令

```bash
# 构建
cargo build --release

# 全部测试
cargo test --workspace --all-features

# 伦理门禁（不可跳过）
cargo test ethics_

# 代码质量
cargo fmt --check && cargo clippy --workspace --all-targets --all-features -- -D warnings

# 运行 Aurora
cargo run --bin aurora -- --input synthetic_2hz.json --output report.html

# 运行 Trit-Core 沙盒
cargo run --bin trit-sandbox -- --scenario scenarios/medical_conflict_01.json
```

---

*此文件为项目开机导航。每次重大决策或阶段完成后更新"当前进度"和"上次决策"。不是指教，是提醒。*
```

- [ ] **Step 2：验证文件写入成功**

```bash
wc -l SESSION_START.md
```

Expected: 文件存在，行数 > 150。

- [ ] **Step 3：Commit**

```bash
git add SESSION_START.md
git commit -m "docs: 新增 SESSION_START.md 开机导航文件（30 秒定位工作进度）"
```

---

## 任务 4：更新 CLAUDE.md 引用 SESSION_START.md

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1：在 CLAUDE.md 顶部添加 SESSION_START 引用**

在 `CLAUDE.md` 的 `# CLAUDE.md` 标题下方、`## Project Overview` 上方，插入以下段落：

```markdown

> **⚡ 新会话启动**：先读 `SESSION_START.md`（30 秒了解当前进度和上次决策），再回到本文件看技术约束。
```

具体操作：在 `CLAUDE.md` 第 1 行 `# CLAUDE.md` 和第 3 行 `## Project Overview` 之间插入上述内容。

- [ ] **Step 2：验证**

```bash
head -6 CLAUDE.md
```

Expected: 第 2-3 行显示 SESSION_START 引用。

- [ ] **Step 3：Commit**

```bash
git add CLAUDE.md
git commit -m "docs: CLAUDE.md 添加 SESSION_START.md 引用（新会话启动入口）"
```

---

## 任务 5：更新 map/00_START_HERE.md 添加架构设计文档引用

**Files:**
- Modify: `map/00_START_HERE.md`

- [ ] **Step 1：在快速入口表中添加架构设计文档行**

在 `map/00_START_HERE.md` 的快速入口表中，在"产品战略"行之后添加：

```markdown
| Aurora 架构设计（M0-M1 技术方案） | [docs/superpowers/specs/2026-06-20-aurora-architecture-design.md](../docs/superpowers/specs/2026-06-20-aurora-architecture-design.md) |
```

- [ ] **Step 2：更新最后更新日期**

将 `map/00_START_HERE.md` 底部的 `**最后更新**: 2026-06-20` 保持不变（今天就是 6-20）。

- [ ] **Step 3：Commit**

```bash
git add map/00_START_HERE.md
git commit -m "docs: map/00_START_HERE.md 添加架构设计文档引用"
```

---

## 完成检查

- [ ] `docs/superpowers/specs/2026-06-20-aurora-architecture-design.md` 存在且内容完整
- [ ] `map/06_code.md` 反映 v0.3.0 实际代码结构
- [ ] `SESSION_START.md` 存在且包含进度、决策、导航、文件清单
- [ ] `CLAUDE.md` 引用 SESSION_START.md
- [ ] `map/00_START_HERE.md` 引用架构设计文档
- [ ] 所有 commit 已提交
