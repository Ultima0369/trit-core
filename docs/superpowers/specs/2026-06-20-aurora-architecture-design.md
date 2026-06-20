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
