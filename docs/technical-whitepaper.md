# Trit-Core 技术白皮书（v0.3.0）

**版本**：0.3.0  
**日期**：2026-06-19  
**协议**：MIT License  
**语言**：Rust 2021 Edition  
**状态**：稳定 — 核心代数语义已冻结，沙盒与可观测性接口可能在 0.3.x 继续演进。

> **提醒，而非指令**：本项目所有内容均作为“邀请检验的提醒”，而非“必须服从的指令”。我们鼓励每一位读者进行实践测试、交叉验证与持续探索。完整声明见 [`docs/explanation/insights/EPISTEMIC-HUMILITY.md`](explanation/insights/EPISTEMIC-HUMILITY.md)。

---

## 摘要

Trit-Core 是一个**面向冲突感知 AI 对齐的三值决策引擎**。与二值逻辑或概率平滑方案不同，Trit-Core 在 `True`/`False` 之外引入独立的 `Hold`（悬置）状态，用于显式表达“系统检测到跨域冲突，选择不强制判定”。

核心主张：

> 在人类中心的咨询场景（医疗伦理、价值冲突、工程安全、公共协商）中，**允许悬置的三值协议比二值概率输出更能保留冲突信息、尊重人格主权，并避免误导性的共识坍缩**。

本白皮书同时作为**综合审核索引**，汇总架构设计、形式语义、验证证据、性能数据、安全审计、代码质量评估与已知局限。

---

## 1. 引言与动机

### 1.1 二值逻辑的盲区

当前主流大语言模型对齐方法通常将多源信号压缩为单一概率或偏好方向。这种“强制坍缩”在以下场景会产生系统性偏差：

- **医疗伦理**：患者个体风险 vs 统计证据 —— 二值多数投票会忽略少数关键风险。
- **价值判断**：高薪但无聊的工作 vs 低薪但有创造性的工作 —— 没有算法应该替人回答。
- **工程安全**：公众观感 vs 物理定律 —— 加权平均会稀释安全约束的硬度。
- **公共协商**：不可排序的权利冲突 —— 多数决会制造虚假的“共识”。

### 1.2 三值响应：Hold 不是失败

Trit-Core 的设计哲学是：**当输入来自不可通约的参考系时，正确的输出不是 True 也不是 False，而是 Hold + 可审计的冲突记录**。

Hold 表达的是：

- 系统**理解**了问题；
- 系统**检测到**跨参考系冲突；
- 系统**拒绝**在没有人类授权的情况下强制决策。

这与“不确定性”或“模型能力不足”有本质区别 —— 后者对应 `Unknown`（超出认知范围）。

---

## 2. 核心概念与形式语义

### 2.1 TritValue — 三值逻辑单元

```rust
pub enum TritValue {
    True,    // +1
    Hold,    //  0
    False,   // -1
    Unknown, //  ⊥ — 超出认知范围，不可计算
}
```

| 状态 | 含义 | 系统是否理解输入 | 是否可计算 |
|---|---|---|---|
| True | 肯定 | 是 | 是 |
| Hold | 有意暂停 | 是 | 是 |
| False | 否定 | 是 | 是 |
| Unknown | 超出认知 | 否 | 否 |

`True`/`Hold`/`False` 构成 MVL-3（三值逻辑可计算空间）。`Unknown` 是元层面的安全标记，用于输入门控和降级。

### 2.2 Phase — 连续倾向度

```rust
pub struct Phase(f64);  // 范围 [0.0, 1.0]
```

- `0.0`：完全倾向于 False
- `0.5`：完全中性
- `1.0`：完全倾向于 True

`Phase` 提供离散三值之外的**强度维度**。同帧信号叠加时，Phase 取均值；构造和运算后自动量化，防止浮点漂移。

### 2.3 Frame — 参考系

```rust
pub enum Frame {
    Science,     // 经验/证据驱动
    Individual,  // 个人上下文/个人事实
    Consensus,   // 统计/群体偏好
    Absolute,    // 不可知/不可观测（永远 Hold）
    Meta,        // 冲突仲裁/策略输出（系统内部使用）
    FirstPerson, // 第一人称主观报告
    Embodied,    // 身体/生理状态参考系
    Relational,  // 关系/社会互动参考系
}
```

核心规则：

- **同 Frame**：正常三元逻辑运算，Phase 取均值（热路径）。
- **跨 Frame**：任何跨 Frame 操作返回 `Hold` + `MetaInterrupt`（冷路径），确保冲突不被悄悄抹平。
- **第一人称优先**：当 `FirstPerson` 与 `Science` 冲突时，默认 Preserve `FirstPerson`，避免科学框架覆盖主观事实。

### 2.4 Domain — 仲裁域

```rust
pub enum Domain {
    Physical,        // 硬科学约束
    Engineering,     // 应用约束
    MedicalEthics,   // 软约束
    ValueJudgment,   // 不可通约
    General,         // 默认协商
    Custom(String),  // 外部规则
}
```

| Domain | 优先 Frame | 可强制坍缩 | 逻辑 |
|---|---|---|---|
| Physical | Science | 是 | 物理定律不谈判 |
| Engineering | Science | 是 | 安全系数不妥协 |
| MedicalEthics | Individual | 否 | 患者自主权是安全默认 |
| ValueJudgment | 无 | 否 | 不可通约的价值 —— 永远 Hold |
| General | 首个（同帧时）| 否 | 同帧提交，跨帧协商 |
| Custom | 由规则定义 | 由规则定义 | 外部 JSON 规则文件 |

### 2.5 TritWord — 计算原子

```rust
pub struct TritWord {
    value: TritValue,  // 私有
    phase: Phase,      // 私有
    frame: Frame,      // 私有
}
```

一个 `TritWord` 携带了做出可审计决策所需的全部信息：是什么、有多确定、在什么参考系中。字段私有，不变量由构造器保证；`TritWord` 和 `Frame` 均为 `Copy`，热路径零堆分配。

### 2.6 MetaInterrupt — 冲突审计记录

```rust
pub struct MetaInterrupt {
    pub conflict: ConflictType,       // FrameMismatch / OutOfScope / PhaseDrift / PolicyViolation
    pub reason: String,
    pub timestamp: DateTime<Utc>,
}
```

每一次跨域冲突、安全降级、策略违反都产生一条 `MetaInterrupt`，确保“为什么系统说 Hold”可被完整追溯。

---

## 3. 三值代数

### 3.1 基本运算

**否定（TNOT）**：

| 输入 | True | Hold | False | Unknown |
|---|---|---|---|---|
| 输出 | False | Hold | True | Unknown |

**谐波与（TAND）**：

| TAND | True | Hold | False | Unknown |
|---|---|---|---|---|
| True | True | Hold | False | Unknown |
| Hold | Hold | Hold | False | Unknown |
| False | False | False | False | Unknown |
| Unknown | Unknown | Unknown | Unknown | Unknown |

**谐波或（TOR）**：

| TOR | True | Hold | False | Unknown |
|---|---|---|---|---|
| True | True | True | True | True |
| Hold | True | Hold | Hold | Unknown |
| False | True | Hold | False | Unknown |
| Unknown | True | Unknown | Unknown | Unknown |

关键观察：

- False 在 TAND 中湮灭一切（安全保守）。
- True 在 TOR 中主导一切。
- Unknown 在 TAND 中传染；在 TOR 中被 True 覆盖。

### 3.2 热路径与冷路径

- **热路径**：`t_and_hot` / `t_or_hot`，要求同 Frame，跳过检查，零分配，约 ~4 ns。
- **冷路径**：`t_and` / `t_or`，跨 Frame 时分配 `MetaInterrupt`，约 ~100 ns。
- **批量级联**：`t_and_n` 对 3+ 信号做等权 Phase 平均，消除左折叠偏差。

---

## 4. 架构

### 4.1 模块分层

```
┌─────────────────────────────────────────────────────────────┐
│  应用层                                                       │
│  - src/bin/sandbox.rs      (trit-sandbox CLI)                │
│  - src/bin/dhat_profile.rs (堆分析二进制)                     │
├─────────────────────────────────────────────────────────────┤
│  沙盒层 (src/sandbox/)                                        │
│  - input.rs / output.rs    JSON I/O                          │
│  - validate.rs             输入校验与清理                    │
│  - pipeline.rs             TAND → 仲裁 → Reflexive → SafeFallback │
│  - diagnostic.rs           运行时遥测                        │
│  - error.rs / validator.rs 统一错误与期望行为校验            │
├─────────────────────────────────────────────────────────────┤
│  元策略层 (src/meta/)                                         │
│  - domain.rs               Domain / ResolutionPolicy         │
│  - interrupt.rs            MetaInterrupt / MetaMonitor       │
│  - rules.rs                CustomRule / RuleLoader           │
│  - safe_fallback.rs        IEC 61508 风格安全降级            │
├─────────────────────────────────────────────────────────────┤
│  心智工程层（opt-in）                                         │
│  - src/reflexive/          ReflexiveAuditor / AuditReport    │
│  - src/attention/          AttentionScheduler / AttentionCmd │
│  - src/knowledge/          SelfKnowledge / ReceiverEstimate  │
├─────────────────────────────────────────────────────────────┤
│  核心代数层 (src/core/)                                       │
│  - value.rs                TritValue                         │
│  - phase.rs                Phase / Commitment                │
│  - frame.rs                Frame / FrameRegistry             │
│  - word.rs                 TritWord（不变量集中）            │
│  - algebra.rs              TernaryAlgebra                    │
│  - sensor.rs               SensorSignal / EnvironmentalContext│
│  - hold.rs                 HoldState / HolderConfig          │
├─────────────────────────────────────────────────────────────┤
│  支持层                                                       │
│  - src/clock.rs            相位振荡器                        │
│  - src/baseline/           二元基线对比器                    │
│  - src/tracing_init.rs     结构化日志初始化                  │
└─────────────────────────────────────────────────────────────┘
```

### 4.2 数据流

```
scenarios/*.json
       │
       ▼
ScenarioInput ──validate──► SandboxPipeline::run()
                               │
                               ▼
                         SignalInput[] ──► TritWord[]
                               │
                               ▼
                         t_and_n batch TAND
                               │
                               ▼
                         MetaInterrupt[] + current TritWord
                               │
                               ▼
                         ResolutionPolicy::arbitrate()
                               │
                               ▼
                    ReflexiveAuditor (optional)
                               │
                               ▼
                         SafeFallback::guard()
                               │
                               ▼
                    AttentionScheduler / SelfKnowledge (optional)
                               │
                               ▼
                         SandboxOutput ──► stdout (JSON)
                               │
                               ▼
                         SandboxDiagnostics ──► stderr (--diagnostic)
```

### 4.3 关键不变量

- `Phase` 永远有限且落在 `[0.0, 1.0]`；非法值通过 `Phase::new` 返回 `Err` 显式拒绝。
- `TritWord` 字段私有；`Frame::Absolute` 强制搭配 `Hold` + 中性 Phase。
- 跨 Frame 运算不产生真假结论。
- `#![forbid(unsafe_code)]` 全 crate 强制。

---

## 5. 策略引擎与安全降级

### 5.1 仲裁流程

`ResolutionPolicy::arbitrate` 根据 `Domain` 选择冲突解决策略：

1. **Physical / Engineering**：若存在 `Science` 帧则 Commit；否则 ForceCollapse（由 SafeFallback 接管）。
2. **MedicalEthics**：若存在 `Individual` 帧则 Preserve；否则 Negotiate。
3. **ValueJudgment**：永远 Hold。
4. **General**：同帧 Commit 首个；异帧 Negotiate。
5. **Custom(name)**：若加载了 `CustomRule` 则按规则执行；否则 Negotiate。

### 5.2 SafeFallback

遵循 IEC 61508 / ISO 26262 原则：**在危险域中，“不能决定”必须默认为“不做”**。

当 Domain 为危险域（`Physical`、`Engineering` 以及注册的 `Custom` 危险域如 chemistry、genetics、nuclear、pharmaceutical、structural），且结果为 `Hold` 或 `Unknown`、存在中断时，`SafeFallback::guard` 强制将结果改为 `False`。

这意味着：化工厂安全系统检测到跨域冲突时，不会“保持中立”，而是明确阻止操作，把决定权交还人类。

### 5.3 心智工程扩展（Mind-Engineering Extension）

v0.3.0 引入一组可选的“心智工程”模块，用于模拟注意力调控、自我知识与自反审计。它们默认不启用，通过 `SandboxPipeline` 的 builder 方法或 CLI 标志打开：

- **`src/reflexive/`**：`ReflexiveAuditor` 记录冲突历史与相位漂移，对强制 True/False 输出进行自动审计。当未解决的跨帧冲突仍存在时，guard 将结果覆写为 `Hold`，实现算法化的“停下来，转回去”。
- **`src/attention/`**：`AttentionScheduler` 根据信号负载、带宽与身体状态给出 `AttentionCmd`（`HoldCurrent` / `ZoomIn` / `WidenScope` / `Recalibrate` / `BodyShift`）。
- **`src/knowledge/`**：`SelfKnowledge` 维护自身响应模式与触发签名，通过 `infer_receiver_state` 推断潜在接收者的认知状态。
- **`src/core/sensor.rs`**：`SensorSignal` 与 `EnvironmentalContext` 为多模态输入（身体状态、环境快照、认知负荷）提供可扩展容器。
- **`src/core/hold.rs`**：`HoldState` 区分 `Awaiting`（等待更多信息）与 `Final`（保持即为最终答案）。

这些扩展不影响核心代数的稳定性；CLI 通过 `--reflexive`、`--self-knowledge`、`--trace-phase`、`--hold-final` 暴露相关能力。

---

## 6. 沙盒管道与可观测性

### 6.1 管道阶段

`SandboxPipeline::run_with_diagnostics` 分为 12 个阶段：

1. `validate`：场景级与信号级校验。
2. `build_policy`：解析 Domain 并构建 `ResolutionPolicy`。
3. `build_trits`：将 `SignalInput` 转换为 `TritWord`。
4. `registry_check`：可选 Frame 白名单校验。
5. `t_and_n`：批量 TAND 级联。
6. `arbitrate`：域策略仲裁。
7. `reflexive_guard`：可选自反审计 guard。
8. `safe_fallback`：危险域安全降级。
9. `attention`：可选注意力调度。
10. `self_knowledge`：可选自我知识推断。
11. `phase_trace`：可选相位追踪记录。
12. `build_output`：构造 JSON 输出。

### 6.2 输入校验

- 最大 JSON 尺寸：64 KB
- 最大信号数：100
- 最大字符串长度：1 KB
- Phase 必须有限且在 `[0.0, 1.0]`
- Frame 必须可解析为已知变体
- 路径遍历尝试被分类为 `ErrorCategory::Security`

### 6.3 可观测性

- `--trace` / `--diagnostic` CLI 标志输出每阶段耗时、帧分布、中断类型。
- `SandboxError::report()` 提供分类错误与可操作建议。
- 所有关键路径均通过 `tracing` 记录 `info` / `warn` / `error` 事件。

---

## 7. 验证与证据

### 7.1 测试矩阵

| 类型 | 数量 | 说明 |
|---|---|---|
| 单元测试 | 253 | 核心代数、Phase、Frame、TritWord、策略、SafeFallback、心智工程模块 |
| 集成测试 | 20 | 端到端场景、元策略、三值不变量 |
| 属性测试 | 19 | proptest：代数定律、Phase 有界性、SafeFallback |
| 场景校验 | 2 | 全部 40 个 JSON 场景匹配 expected_behavior |
| CLI 测试 | 19 | 路径遍历、未知参数、dry-run、validate-only、中文场景、新 CLI 标志 |
| 不变量测试 | 9 | 核心不变量 |
| 错误路径测试 | 16 | 非法 domain/frame/phase/value |
| Doc 测试 | 1 | lib.rs 示例 |
| **总计** | **~340** | **全部通过** |

### 7.2 场景套件

- **20 个英文场景** + **20 个中文翻译场景** = 40 个 JSON 文件。
- 覆盖 5 个 Domain：Physical、Engineering、MedicalEthics、ValueJudgment、General。
- 每个场景均声明 `expected_behavior` 并通过自动化校验（空 expected_behavior 仅用于测试 reflexive 标志）。

### 7.3 二元基线对比结果

| 指标 | 结果 |
|---|---|
| 二值与三值输出一致 | 8 / 19（42%） |
| 二值覆盖/平滑真实冲突 | 11 / 19（58%） |
| 二值无法表达 Hold | 19 / 19（100%） |

关键发现：

- 100% ValueJudgment 案例：二值无法表达“算法不应决定”。
- 100% MedicalEthics 案例：二值忽略患者特定上下文。
- 即使 Physical/Engineering 输出一致，Trit-Core 仍通过 `MetaInterrupt` 记录冲突路径。

完整数据见 [`docs/reports/validation-report.md`](reports/validation-report.md)。

---

## 8. 性能验证

### 8.1 微基准

| 操作 | 延迟 |
|---|---|
| `t_and_hot`（同帧） | ~4.1 ns |
| `precheck_same_frame` | ~0.75 ns |
| TAND 跨帧 | ~104 ns |
| 10 元素热路径级联 | ~3.5 ns/op |
| 10 元素跨帧级联 | ~101 ns/op |

### 8.2 端到端吞吐量

| 路径 | 吞吐量 |
|---|---|
| MedicalEthics 管道 | ~602K signals/s（约 2.1M ops/s） |
| Physical 管道 | ~558K signals/s（约 1.95M ops/s） |

相对 10,000 TPS 目标：

- 端到端 signals/s：**55–60×**
- 端到端 ops/s（估算）：**195–210×**

### 8.3 dhat 堆分析

- 热路径（TAND/TOR/TNOT）确认**零堆分配**。
- 冷路径唯一分配来源为 `MetaInterrupt::new()` 的 `String` 原因文本。
- 端到端管道的主要堆分配来自 serde JSON 序列化/反序列化。

完整数据见 [`docs/reports/performance-validation.md`](reports/performance-validation.md)。

---

## 9. 安全

### 9.1 威胁模型

Trit-Core v0.3.0 是**单机构决策库**，无网络监听、无持久化存储、无多用户认证。主要攻击面为：

- 恶意构造的场景 JSON 输入（路径遍历、反序列化炸弹、非法 phase）。
- 日志注入（通过 `description` / `id` 字段）。

### 9.2 已实施的缓解措施

| 风险 | 缓解 |
|---|---|
| 路径遍历（CWE-22） | 输入路径规范化并限制在 `scenarios/` 目录；越界路径返回 `Security` 错误。 |
| 不可信反序列化（CWE-502） | JSON 大小 ≤ 64 KB；信号数 ≤ 100；字符串长度 ≤ 1 KB；Phase/Frame/Domain 严格校验。 |
| 断言崩溃（CWE-617） | `Phase::new` 返回 `Result`；`Phase::new_clamped` 仅用于显式静默归一化。 |
| 日志注入（CWE-117） | `sanitize_log_field` 替换控制字符并截断。 |
| 空输入索引越界 | `validate_scenario` 要求至少一个信号；`ResolutionPolicy::arbitrate` 拒绝空输入。 |
| 内存安全 | `#![forbid(unsafe_code)]` 编译时强制。 |
| 依赖安全 | `cargo-audit` 通过，无已知 CVE。 |

### 9.3 审计结论

- v0.1.0 安全审计发现 2 个 P1、3 个 P2、2 个 P3 问题；v0.2.0/v0.3.0 已全部修复或随网络层移除而消除。
- 当前 CLI 输入路径已实施白名单与大小限制。

完整审计见 [`docs/reports/security-audit.md`](reports/security-audit.md) 与 [`audit_log/08_reflexive_audit.md`](../audit_log/08_reflexive_audit.md)。

---

## 10. 代码质量

### 10.1 设计原则

- **SRP**：核心模块职责清晰；`sandbox.rs` main 函数已拆分为库代码 `SandboxPipeline`。
- **OCP**：`Domain::Custom` 支持外部 JSON 规则扩展，无需修改核心源码。
- **DRY**：Frame / TritValue 字符串映射通过 `FromStr` 统一；跨帧冲突检测已共享。
- **显式错误处理**：核心 API 返回 `Result`；CLI 使用分类错误报告。

### 10.2 质量门禁

- `cargo fmt -- --check`：通过
- `cargo clippy --all-targets --all-features -- -D warnings`：通过
- `cargo test --all-features -- --test-threads=2`：313 测试通过
- 公共 API 快照门：`cargo public-api -ss --all-features` 与 `api/public-api.txt` 比对

### 10.3 历史审计

v0.1.0 代码质量审计综合得分 3.5/5；v0.2.0/v0.3.0 已重构 `TritWord` 私有字段、移除网络层、消除关键路径 `unwrap()`、拆分 `main()` 职责。

完整报告见 [`docs/reports/code-quality-audit.md`](reports/code-quality-audit.md)。

---

## 11. 已知局限与未来工作

### 11.1 局限

- 场景样本小，未经过统计学意义上的充分验证。
- 未进行人类被试研究，“真实用户满意度”仍待验证。
- `Unknown` 状态的传播策略目前偏保守（TAND 中传染）。
- 中文文档多为 v0.1.x 历史版本，尚未完全同步到 v0.3.0 API。

### 11.2 未来方向

- 属性测试覆盖完整代数定律（分配律、结合律）。
- `no_std` 核心 profile。
- 人类被试实验：三值 vs 二值输出的感知真实性。
- 分布式协议作为独立 crate 重新引入，带加密身份与形式化 wire 规范。
- 更多领域规则 DSL 与可视化审计工具。

---

## 12. 综合审核索引

| 主题 | 当前 v0.3.0 文档 | 历史审计/归档 |
|---|---|---|
| 快速开始 | [`docs/tutorials/QUICKSTART.md`](tutorials/QUICKSTART.md) | — |
| 核心概念 | [`docs/explanation/CONCEPTS.md`](explanation/CONCEPTS.md) | — |
| 架构设计 | [`docs/explanation/ARCHITECTURE.md`](explanation/ARCHITECTURE.md) | [`docs/archive/technical-whitepaper.md`](archive/technical-whitepaper.md) |
| API 契约 | [`docs/reference/api.md`](reference/api.md) | [`docs/zh/reference/api.zh.md`](zh/reference/api.zh.md) |
| 模块参考 | [`docs/reference/MODULES.md`](reference/MODULES.md) | — |
| M2/M3 验证 | [`docs/reports/validation-report.md`](reports/validation-report.md) | — |
| 性能验证 | [`docs/reports/performance-validation.md`](reports/performance-validation.md) | — |
| 安全审计 | 本白皮书 §9 | [`docs/reports/security-audit.md`](reports/security-audit.md) |
| 代码质量审计 | 本白皮书 §10 | [`docs/reports/code-quality-audit.md`](reports/code-quality-audit.md) |
| CTO 审计 | [`docs/reports/cto-audit-report.md`](reports/cto-audit-report.md) | [`docs/reports/deep-audit-cto-2026-06-18.md`](reports/deep-audit-cto-2026-06-18.md) |
| 自反性审计 | [`audit_log/08_reflexive_audit.md`](../audit_log/08_reflexive_audit.md) | [`audit_log/00_summary.md`](../audit_log/00_summary.md) |
| 评审者指引 | [`docs/how-to/REVIEWER_GUIDE.md`](how-to/REVIEWER_GUIDE.md) | — |
| 变更记录 | [`CHANGELOG.md`](../CHANGELOG.md) | — |

---

## 13. 结论

Trit-Core v0.3.0 是一个**类型安全、零 unsafe、经过系统验证的三值决策引擎**。它通过独立的 `Hold` 状态、参考系感知的代数运算、域特定的仲裁策略和 IEC 61508 风格的安全降级，在二值系统会强制坍缩的场景中保留了关键冲突信息。

当前证据表明：

- 核心代数和管道语义正确（约 340 测试通过）。
- 性能远超 10,000 TPS 目标（55–210×）。
- v0.1.0 发现的 P1/P2 安全与质量问题已在后续版本中修复。
- 心智工程扩展以 opt-in 方式集成，不破坏既有语义。
- 中文档系统已按 Diátaxis 框架重组，便于按读者目的查找。

Trit-Core 当前最适合作为**研究原型、对齐实验平台和安全关键决策系统的推理组件**。生产部署前仍需更大规模的场景验证、人类被试研究和领域专家评审。

---

*本白皮书随 v0.3.0 发布，作为项目技术总览与综合审核索引。*
