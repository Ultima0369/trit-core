# MODULES — 模块参考

`src/` 下每个子模块的职责、关键函数和设计约束摘要（v0.3.0）。

---

## `core/` — 核心三值代数

### 文件

| 文件 | 职责 |
|---|---|
| `mod.rs` | 公共重导出 |
| `value.rs` | `TritValue` 枚举 + LUT 实现 |
| `phase.rs` | `Phase` 连续倾向度 + 量化 + 严格构造 |
| `frame.rs` | `Frame` 枚举（13 变体）+ `FrameError` |
| `word.rs` | `TritWord` 定义 + 构造器 + 不变量强制 |
| `algebra.rs` | `TernaryAlgebra`: TAND/TOR/TNOT/THOLD/TSENSE |
| `sensor.rs` | 多模态传感器信号 `SensorSignal` + `EnvironmentalContext` |
| `hold.rs` | `HoldState` + `HolderConfig` |

### 关键函数

| 函数 | 签名 | 说明 |
|---|---|---|
| `TritValue::negate` | `fn negate(self) -> TritValue` | LUT 驱动的分支无关取反 |
| `TritValue::to_i8` | `fn to_i8(self) -> i8` | LUT 驱动的数值转换 |
| `TritValue::from_i8_strict` | `fn from_i8_strict(i8) -> Result<TritValue, _>` | 严格转换，拒绝越界 |
| `Phase::new` | `fn new(f64) -> Result<Phase, PhaseError>` | 严格构造；无效值返回 `Err` |
| `Phase::new_clamped` | `fn new_clamped(f64) -> Phase` | 显式静默归一化，带 `tracing::warn` |
| `Phase::neutral` | `const fn neutral() -> Phase` | 常量 0.5 phase |
| `Phase::full_true` | `const fn full_true() -> Phase` | 常量 1.0 phase |
| `Phase::full_false` | `const fn full_false() -> Phase` | 常量 0.0 phase |
| `Phase::mean` | `fn mean(Phase, Phase) -> Phase` | 算术均值 + 自动量化 |
| `Phase::complement` | `fn complement(self) -> Phase` | `1.0 - p` + 自动量化 |
| `Phase::quantize` | `fn quantize(self, epsilon: f64) -> Phase` | 锚点吸附：0.5→0.0→1.0 |
| `TritWord::try_new` | `fn try_new(TritValue, f64, &str) -> Result<Self, WordError>` | 一站式构造 |
| `TritWord::absolute` | `fn absolute() -> Self` | 强制 `Hold` + 中性相位 + `Frame::Absolute` |
| `TernaryAlgebra::precheck_same_frame` | `fn precheck_same_frame(&TritWord, &TritWord) -> bool` | O(1) Frame 检查 |
| `TernaryAlgebra::t_and` | `fn t_and(&TritWord, &TritWord) -> (TritWord, Option<MetaInterrupt>)` | 完整路径 TAND |
| `TernaryAlgebra::t_and_hot` | `fn t_and_hot(&TritWord, &TritWord) -> TritWord` | 热路径 TAND（要求同帧，否则 panic） |
| `TernaryAlgebra::t_or` | `fn t_or(&TritWord, &TritWord) -> (TritWord, Option<MetaInterrupt>)` | 完整路径 TOR |
| `TernaryAlgebra::t_or_hot` | `fn t_or_hot(&TritWord, &TritWord) -> TritWord` | 热路径 TOR（要求同帧，否则 panic） |
| `TernaryAlgebra::t_not` | `fn t_not(&TritWord) -> TritWord` | 相位翻转否定 |
| `TernaryAlgebra::t_hold` | `fn t_hold(&TritWord) -> TritWord` | 强制 Hold |
| `TernaryAlgebra::t_sense` | `fn t_sense(f64, Frame) -> Result<TritWord, PhaseError>` | 从原始传感器数据创建 Hold（无效 phase 返回 Err） |
| `TernaryAlgebra::t_sense_clamped` | `fn t_sense_clamped(f64, Frame) -> TritWord` | 从原始传感器数据创建 Hold（无效 phase 自动钳制） |

### 设计约束

- TAND/TOR/TNOT 真值表不可变，保证结果可复现。
- `TritWord` 字段私有；`Frame::Absolute` 必须搭配 `Hold` + 中性相位。
- 热路径使用 `assert!` 验证 Frame 一致（release 也 panic，避免静默错误结果）。

---

## `meta/` — 策略引擎与仲裁

### 文件

| 文件 | 职责 |
|---|---|
| `mod.rs` | 模块声明 + 重导出 |
| `domain.rs` | `Domain` 枚举 + `ResolutionPolicy` + `ArbitrationResult` + `PolicyError` |
| `frame_mask.rs` | O(1) u16 位掩码帧检测（当前用 13 位，上限 16） |
| `interrupt.rs` | `MetaInterrupt` + `ConflictType` + `MetaMonitor` |
| `rules.rs` | `CustomRule` + `RuleLoader` 特质 + `JsonRuleLoader` + `RuleError` |
| `safe_fallback.rs` | `SafeFallback`: IEC 61508 安全降级 |

### 关键函数

| 函数 | 签名 | 说明 |
|---|---|---|
| `FrameMask::from_inputs` | `fn from_inputs(&[TritWord]) -> FrameMask` | 单次遍历 O(n) 构建位掩码 |
| `FrameMask::has` | `fn has(&Frame) -> bool` | O(1) 位检测 |
| `ResolutionPolicy::arbitrate` | `fn arbitrate(&[TritWord]) -> Result<ArbitrationResult, PolicyError>` | 域仲裁核心 |
| `ResolutionPolicy::with_custom_rule` | `fn with_custom_rule(self, rule) -> Self` | 附加 `Domain::Custom` 规则 |
| `MetaMonitor::inspect` | `fn inspect(&TritWord) -> Option<MetaInterrupt>` | Absolute 帧不变性检查 |
| `MetaMonitor::inspect_all` | `fn inspect_all(&[TritWord]) -> Vec<MetaInterrupt>` | 批量不变性检查 |
| `SafeFallback::is_dangerous` | `fn is_dangerous(&Domain) -> bool` | 危险域判定 |
| `SafeFallback::guard` | `fn guard(&Domain, &TritWord, interrupt_count) -> (TritWord, Option<MetaInterrupt>)` | 安全降级主入口 |
| `JsonRuleLoader::load` | `fn load(path) -> Result<CustomRule, RuleError>` | 从文件加载自定义规则 |
| `JsonRuleLoader::apply` | `fn apply(&CustomRule, &[TritWord]) -> ArbitrationResult` | 应用自定义规则 |

### 设计约束

- `FrameMask` 基于 `u16`，当前使用 13 位（对应 13 个 `Frame` 变体），上限 16。
- `ResolutionPolicy::arbitrate` 不 panic；缺失输入或规则错误返回 `PolicyError`。
- `Domain::Custom` 优先使用已附加的 `CustomRule`；无规则时返回 `Negotiate`。
- SafeFallback 的预置危险域列表在 `SafeFallback::new()` 中硬编码。

---

## `sandbox/` — 场景管道

### 文件

| 文件 | 职责 |
|---|---|
| `mod.rs` | 公共重导出 |
| `input.rs` | `ScenarioInput` / `SignalInput` |
| `output.rs` | `SandboxOutput` |
| `validate.rs` | 输入校验与日志清理 |
| `pipeline.rs` | `SandboxPipeline`: TAND 级联 → 仲裁 → SafeFallback |
| `validator.rs` | `ScenarioValidator`: 校验 `expected_behavior` |
| `error.rs` | `SandboxError` |

### 关键函数

| 函数 | 签名 | 说明 |
|---|---|---|
| `SandboxPipeline::run` | `fn run(&ScenarioInput) -> Result<SandboxOutput, SandboxError>` | 端到端场景执行 |
| `ScenarioValidator::validate` | `fn validate(&SandboxOutput, &str) -> Result<(), SandboxError>` | 校验期望行为 |
| `validate_scenario` | `fn validate_scenario(&ScenarioInput) -> Result<(), SandboxError>` | 场景级校验 |
| `validate_signal` | `fn validate_signal(usize, &SignalInput) -> Result<(), SandboxError>` | 信号级校验 |
| `sanitize_log_field` | `fn sanitize_log_field(&str) -> String` | 控制字符替换与截断 |

### 设计约束

- 未知 frame 不再静默 fallback 到 `Meta`，而是返回 `SandboxError`。
- `expected_behavior` 支持 `"hold"`、`"commit_true"`、`"commit_false"`、`"negotiate"`。

---

## `reflexive/` — 自反审计

### 文件

| 文件 | 职责 |
|---|---|
| `mod.rs` | 公共重导出 |
| `auditor.rs` | `ReflexiveAuditor` + `AuditReport` + `ReflexiveAlert` |

### 关键函数

| 函数 | 说明 |
|---|---|
| `ReflexiveAuditor::record_interrupt` | 记录冲突历史 |
| `ReflexiveAuditor::auto_post_audit` | 对输出做自动审计 |
| `ReflexiveAuditor::reflexive_posture` | 返回建议姿态 |

### 设计约束

- 自反 guard 仅覆写非危险域的强制 True/False；`SafeFallback` 触发的安全降级优先。

---

## `adapters/bandwidth_scheduler.rs` — 注意力调度

### 文件

| 文件 | 职责 |
|---|---|
| `bandwidth_scheduler.rs` | `BandwidthScheduler` + `AttentionCmd` + `SuggestOutput` |

### 关键函数

| 函数 | 说明 |
|---|---|
| `BandwidthScheduler::suggest_with_budget` | 根据输入与资源预算给出注意力命令 |
| `BandwidthScheduler::new` | 以默认注意力带宽构造 |

---

## `adapters/self_knowledge.rs` — 自我知识

### 文件

| 文件 | 职责 |
|---|---|
| `self_knowledge.rs` | `SelfKnowledge` + `ReceiverEstimate` + `ResponsePatternCache` |

### 关键函数

| 函数 | 说明 |
|---|---|
| `SelfKnowledge::with_human_defaults` | 使用人类默认响应模式初始化 |
| `SelfKnowledge::lookup_pattern` | 从输入推断接收者状态 |
| `SelfKnowledge::calibrate` | 反馈驱动校准（Layer 5 调用） |

---

## `clock/` — 相位振荡器

### 文件

| 文件 | 职责 |
|---|---|
| `clock.rs` | `HarmonicClock` 正弦振荡器 |

### 关键函数

| 函数 | 说明 |
|---|---|
| `HarmonicClock::tick(dt)` | 推进时间 dt，返回是否上升过零 |
| `HarmonicClock::phase_now()` | 返回 `sin(ω·t + φ₀)` |
| `HarmonicClock::physical()` | 快速时钟（ω=10.0） |
| `HarmonicClock::deliberative()` | 慢速时钟（ω=0.5） |

---

## `baseline/` — 二元基线对比

### 文件

| 文件 | 职责 |
|---|---|
| `mod.rs` | `BinaryBaseline` 二元多数投票 |

### 关键函数

| 函数 | 说明 |
|---|---|
| `BinaryBaseline::evaluate(&[TritWord]) -> BinaryResult` | 二元多数投票（tie → False） |
| `BinaryBaseline::compare(&TritWord, &BinaryResult) -> BinaryResult` | 二元 vs 三元对比 |
| `BinaryBaseline::has_hidden_conflict(&[TritWord]) -> bool` | 检测二进制会忽略的跨帧冲突 |

---

## 二进制入口

| 文件 | 说明 |
|---|---|
| `src/bin/sandbox.rs` | `trit-sandbox` CLI |
| `src/bin/dhat_profile.rs` | `dhat-profile` 堆分析二进制（需 `dhat-profile` feature） |

---

## 历史说明

v0.1.x 中的 `src/net/` 分布式协议层与 `trit-node` 二进制已在 v0.2.0 移除，计划作为独立 crate 重新引入。
