# Trit-Core 代码质量审计报告

**审计日期**: 2026-06-17
**审计版本**: v0.1.0 (commit: c8e9155)
**审计方法**: SonarQube 规则映射 + SOLID 原则检查 + 圈复杂度分析 + 人工设计审查

> **历史版本说明**：本报告审计的是 Trit-Core v0.1.0。v0.2.0 已重构多项问题（如 `TritWord` 字段私有、移除网络层、消除关键路径 `.unwrap()`）。当前代码质量状态请参考 `audit_log/08_reflexive_audit.md`。

---

## 1. 评分卡

| 维度 | 得分 | 说明 |
|------|------|------|
| **SRP** 单一职责 | 4/5 | 核心模块职责清晰。`sandbox.rs:main()` 违反 SRP（4 个职责）。 |
| **OCP** 开闭原则 | 3/5 | `meta/mod.rs:arbitrate()` 用 match 硬编码 5 个域，新增域需修改源码。 |
| **命名规范** | 4/5 | 大部分良好。"代数"模块名 `TernaryAlgebra` 准确。`run_pipeline` 略宽泛。 |
| **重复代码 (DRY)** | 3/5 | `sandbox.rs` 与 `integration_test.rs` 中 frame/value 映射逻辑完全重复。`TAND`/`TOR` 跨帧检测块重复。 |
| **错误处理** | 3/5 | 核心库使用 `MetaInterrupt` 模式良好。`bus.rs` 中有 4 处 `unwrap()` 用于测试假设。 |
| **测试覆盖** | 4/5 | 227 测试覆盖核心路径，但 `HarmonicClock`、`FrameRegistry`、`Node.to_trit()`、`Node.enter_hold()` 无直接测试。 |

**综合得分**: **3.5 / 5** — MVP 阶段可接受，M3 前需修复 P2 问题。

---

## 2. 问题清单

### P2-01: `sandbox.rs:main()` 违反单一职责 (SRP)

**位置**: `src/bin/sandbox.rs:108-218` (110 行)
**违反原则**: SRP — 一个函数承担 4 项独立职责
**影响**: 可测试性（无法单独测试管线段）、可维护性（修改任一步骤都需理解整个 main）

**当前代码** (110 行, 圈复杂度 ≈ 15):
```rust
fn main() {
    // 职责1: CLI 参数解析和校验 (L108-146)
    let args: Vec<String> = std::env::args().collect();
    // ...
    let scenario = match serde_json::from_str(&raw) { ... };
    // ...

    // 职责2: 域策略选择 (L148-156)
    let policy = match scenario.domain.as_str() { ... };
    let mut monitor = MetaMonitor::new(policy.clone());

    // 职责3: 信号转换 + TAND 级联 (L158-196)
    let trits: Vec<TritWord> = scenario.signals.iter().map(|s| { ... }).collect();
    // ...
    for next in &trits[1..] { ... }

    // 职责4: 输出构造与序列化 (L199-217)
    let output = SandboxOutput { ... };
    println!("{}", serde_json::to_string_pretty(&output).unwrap_or_else(|e| { ... }));
}
```

**重构后代码**:
```rust
// --- 抽取为独立模块: src/sandbox/pipeline.rs ---

/// 将 ScenarioInput 转换为 TritWord 数组。
fn signals_to_trits(scenario: &ScenarioInput) -> Vec<TritWord> {
    scenario.signals.iter().map(|s| {
        let frame = match s.frame.as_str() {
            "Science" => Frame::Science,
            "Individual" => Frame::Individual,
            "Consensus" => Frame::Consensus,
            "Absolute" => Frame::Absolute,
            _ => Frame::Meta,
        };
        let val = match s.value {
            1 => TritValue::True,
            -1 => TritValue::False,
            _ => TritValue::Hold,
        };
        TritWord::new(val, s.phase, frame)
    }).collect()
}

/// 执行 TAND 级联 + MetaInterrupt 记录。
fn run_tand_cascade(
    trits: &[TritWord],
    monitor: &mut MetaMonitor,
) -> (TritWord, Vec<MetaInterrupt>) {
    let mut current = trits[0].clone();
    let mut interrupts: Vec<MetaInterrupt> = vec![];
    for next in &trits[1..] {
        let (result, maybe_int) = TernaryAlgebra::t_and(&current, next);
        if let Some(int) = maybe_int {
            monitor.record(int.clone());
            interrupts.push(int);
        }
        current = result;
    }
    (current, interrupts)
}

/// 应用策略仲裁，选择最终 TritWord。
fn apply_policy(
    policy: &ResolutionPolicy,
    trits: &[TritWord],
    cascade_result: &TritWord,
) -> TritWord {
    let result = policy.arbitrate(trits);
    match &result {
        ArbitrationResult::Commit(w) | ArbitrationResult::Preserve(w) => w.clone(),
        _ => cascade_result.clone(),
    }
}

/// 构建 JSON 输出。
fn build_output(
    scenario: &ScenarioInput,
    final_word: &TritWord,
    interrupts: &[MetaInterrupt],
    policy_result: &ArbitrationResult,
) -> SandboxOutput {
    SandboxOutput {
        scenario_id: sanitize_log_field(&scenario.id),
        final_value: final_word.value.to_i8(),
        final_frame: format!("{}", final_word.frame),
        final_phase: final_word.phase.inner(),
        interrupts: interrupts
            .iter()
            .map(|i| format!("{:?}: {}", i.conflict, sanitize_log_field(&i.reason)))
            .collect(),
        policy_action: format!("{:?}", policy_result),
    }
}

// --- main() 现在只做编排 ---
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 || args[1] != "--scenario" {
        eprintln!("Usage: trit-sandbox --scenario <path.json>");
        std::process::exit(1);
    }

    let path = validate_scenario_path(&args[2]).unwrap_or_else(|e| {
        eprintln!("Security error: {}", e);
        std::process::exit(1);
    });

    let raw = read_scenario_file(&path).unwrap_or_else(|e| {
        eprintln!("{}", e);
        std::process::exit(1);
    });

    let scenario = parse_and_validate_scenario(&raw).unwrap_or_else(|e| {
        eprintln!("{}", e);
        std::process::exit(1);
    });

    let policy = domain_to_policy(&scenario.domain);
    let mut monitor = MetaMonitor::new(policy.clone());

    let trits = signals_to_trits(&scenario);
    let (cascade_result, interrupts) = run_tand_cascade(&trits, &mut monitor);
    let final_word = apply_policy(&policy, &trits, &cascade_result);
    let output = build_output(&scenario, &final_word, &interrupts, &policy.arbitrate(&trits));

    println!("{}", serde_json::to_string_pretty(&output).unwrap_or_else(|e| {
        eprintln!("Failed to serialize output: {}", e);
        std::process::exit(1);
    }));
}
```

**投资回报**: 高 — 每个抽取的函数可独立测试，main() 从 110 行缩减到 ~30 行。

---

### P2-02: `meta/mod.rs:arbitrate()` 违反开闭原则 (OCP)

**位置**: `src/meta/mod.rs:30-62`
**违反原则**: OCP — 新增域需要修改 `arbitrate()` 方法
**影响**: 可扩展性。目前仅 5 个域，但 ADR-003 明确提到未来可能扩展。

**当前代码**:
```rust
pub fn arbitrate(&self, inputs: &[TritWord]) -> ArbitrationResult {
    let result = match self.domain {
        Domain::Physical | Domain::Engineering => {
            if let Some(t) = inputs.iter().find(|t| t.frame == Frame::Science) {
                ArbitrationResult::Commit(t.clone())
            } else {
                ArbitrationResult::ForceCollapse
            }
        }
        Domain::MedicalEthics => {
            if let Some(t) = inputs.iter().find(|t| t.frame == Frame::Individual) {
                ArbitrationResult::Preserve(t.clone())
            } else {
                ArbitrationResult::Negotiate
            }
        }
        Domain::ValueJudgment => ArbitrationResult::Hold,
        Domain::General => {
            let first = &inputs[0];
            if inputs.iter().all(|t| t.frame == first.frame) {
                ArbitrationResult::Commit(first.clone())
            } else {
                ArbitrationResult::Negotiate
            }
        }
    };
    // ...
}
```

**重构后代码** (策略模式):
```rust
/// 域仲裁策略 trait — 新增域只需实现此 trait，无需修改现有代码。
trait ArbitrationStrategy: std::fmt::Debug {
    fn resolve(&self, inputs: &[TritWord]) -> ArbitrationResult;
    fn domain(&self) -> Domain;
}

/// 硬科学域：科学优先，允许强制坍缩。
#[derive(Debug)]
struct PhysicalEngineeringStrategy;
impl ArbitrationStrategy for PhysicalEngineeringStrategy {
    fn resolve(&self, inputs: &[TritWord]) -> ArbitrationResult {
        inputs.iter()
            .find(|t| t.frame == Frame::Science)
            .map(|t| ArbitrationResult::Commit(t.clone()))
            .unwrap_or(ArbitrationResult::ForceCollapse)
    }
    fn domain(&self) -> Domain { Domain::Physical } // 两个域共享同一策略实例
}

/// 医学伦理：个体优先，永不强制。
#[derive(Debug)]
struct MedicalEthicsStrategy;
impl ArbitrationStrategy for MedicalEthicsStrategy {
    fn resolve(&self, inputs: &[TritWord]) -> ArbitrationResult {
        inputs.iter()
            .find(|t| t.frame == Frame::Individual)
            .map(|t| ArbitrationResult::Preserve(t.clone()))
            .unwrap_or(ArbitrationResult::Negotiate)
    }
    fn domain(&self) -> Domain { Domain::MedicalEthics }
}

/// 价值判断：始终悬置。
#[derive(Debug)]
struct ValueJudgmentStrategy;
impl ArbitrationStrategy for ValueJudgmentStrategy {
    fn resolve(&self, _inputs: &[TritWord]) -> ArbitrationResult {
        ArbitrationResult::Hold
    }
    fn domain(&self) -> Domain { Domain::ValueJudgment }
}

/// 通用：同帧提交，异帧协商。
#[derive(Debug)]
struct GeneralStrategy;
impl ArbitrationStrategy for GeneralStrategy {
    fn resolve(&self, inputs: &[TritWord]) -> ArbitrationResult {
        let first = &inputs[0];
        if inputs.iter().all(|t| t.frame == first.frame) {
            ArbitrationResult::Commit(first.clone())
        } else {
            ArbitrationResult::Negotiate
        }
    }
    fn domain(&self) -> Domain { Domain::General }
}

// ResolutionPolicy 现在委托给策略：
impl ResolutionPolicy {
    pub fn new(domain: Domain) -> Self {
        let strategy: Box<dyn ArbitrationStrategy> = match domain {
            Domain::Physical | Domain::Engineering => Box::new(PhysicalEngineeringStrategy),
            Domain::MedicalEthics => Box::new(MedicalEthicsStrategy),
            Domain::ValueJudgment => Box::new(ValueJudgmentStrategy),
            Domain::General => Box::new(GeneralStrategy),
        };
        Self { domain, strategy }
    }

    pub fn arbitrate(&self, inputs: &[TritWord]) -> ArbitrationResult {
        info!(domain = ?self.domain, "arbitration started");
        let result = self.strategy.resolve(inputs);
        info!(?result, "arbitration completed");
        result
    }
}
```

**投资回报**: 中等 — 为未来的域扩展做好准备，与 ADR-003 中"未来可能需要 DSL 扩展"的描述一致。当前仅 5 个域，不紧急。

---

### P2-03: frame/value 映射逻辑重复 (DRY)

**位置**: 
- `src/bin/sandbox.rs:162-168` (frame 映射)
- `src/bin/sandbox.rs:169-173` (value 映射)
- `tests/integration_test.rs:87-97` (完全相同的逻辑)
- `src/net/node.rs:122` (部分重复)

**违反原则**: DRY — 相同字符串→枚举映射出现 3 次以上
**影响**: 可维护性。新增 Frame 变体需要修改 3+ 处。

**当前代码** (sandbox.rs — 在两处出现):
```rust
let frame = match s.frame.as_str() {
    "Science" => Frame::Science,
    "Individual" => Frame::Individual,
    "Consensus" => Frame::Consensus,
    "Absolute" => Frame::Absolute,
    _ => Frame::Meta,
};
let val = match s.value {
    1 => TritValue::True,
    -1 => TritValue::False,
    _ => TritValue::Hold,
};
```

**重构后代码** (在 `src/frame/mod.rs` 和 `src/trit/value.rs` 中实现 `FromStr`):
```rust
// src/frame/mod.rs
impl std::str::FromStr for Frame {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Science" => Ok(Frame::Science),
            "Individual" => Ok(Frame::Individual),
            "Consensus" => Ok(Frame::Consensus),
            "Absolute" => Ok(Frame::Absolute),
            "Meta" => Ok(Frame::Meta),
            unknown => Err(format!("Unknown frame: '{}'", unknown)),
        }
    }
}

// src/trit/value.rs
impl std::str::FromStr for TritValue {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "1" | "true" | "True" => Ok(TritValue::True),
            "-1" | "false" | "False" => Ok(TritValue::False),
            "0" | "hold" | "Hold" => Ok(TritValue::Hold),
            unknown => Err(format!("Unknown TritValue: '{}'", unknown)),
        }
    }
}

impl From<i8> for TritValue {
    fn from(v: i8) -> Self {
        match v {
            1 => TritValue::True,
            -1 => TritValue::False,
            _ => TritValue::Hold,
        }
    }
}

// 调用方简化为：
let frame: Frame = s.frame.parse().unwrap_or(Frame::Meta);
let val = TritValue::from(s.value);
```

**投资回报**: 高 — 单点修改，3+ 处受益，减少 30 行重复代码。

---

### P3-01: TAND/TOR 跨帧检测代码重复

**位置**: `src/trit/algebra.rs:19-27` 与 `src/trit/algebra.rs:45-53`
**违反原则**: DRY — 近完全相同的 8 行代码块出现两次
**影响**: 可维护性。修改跨帧行为需要同步两处。

**重构后代码**:
```rust
impl TernaryAlgebra {
    /// 通用跨帧冲突处理 — TAND 和 TOR 共用。
    fn cross_frame_conflict(op_name: &str, a: &TritWord, b: &TritWord) -> (TritWord, Option<MetaInterrupt>) {
        let hold = TritWord::new(TritValue::Hold, 0.5, Frame::Meta);
        let interrupt = MetaInterrupt::new(
            ConflictType::FrameMismatch,
            format!("{} conflict: {} vs {}", op_name, a.frame, b.frame),
        );
        warn!(reason = %interrupt.reason, "cross-frame conflict detected");
        (hold, Some(interrupt))
    }

    pub fn t_and(a: &TritWord, b: &TritWord) -> (TritWord, Option<MetaInterrupt>) {
        // ...
        if a.frame != b.frame {
            return Self::cross_frame_conflict("TAND", a, b);
        }
        // ...
    }

    pub fn t_or(a: &TritWord, b: &TritWord) -> (TritWord, Option<MetaInterrupt>) {
        // ...
        if a.frame != b.frame {
            return Self::cross_frame_conflict("TOR", a, b);
        }
        // ...
    }
}
```

**投资回报**: 低 — 减少 8 行重复，但两处逻辑完全对称，未来可能差异化。

---

### P3-02: 测试名不遵循 `should_xxx_when_yyy` 模式

**位置**: `tests/integration_test.rs` 全部 18 个测试、`src/baseline/mod.rs` 5 个测试、`src/net/` 11 个测试
**违反原则**: 命名规范 — 测试名应描述行为+条件，而非仅描述预期输出

**示例 — 当前**:
```rust
#[test]
fn tand_same_frame_true_true() { }
fn scenario_medical_conflict_holds() { }
fn binary_tie_defaults_false() { }
fn pll_corrects_toward_peer() { }
```

**建议命名**:
```rust
#[test]
fn should_return_true_when_tand_on_two_science_trues() { }
fn should_preserve_individual_when_medical_ethics_cross_frame_conflict() { }
fn should_default_to_false_when_binary_majority_tie() { }
fn should_correct_phase_toward_peer_when_error_exceeds_deadband() { }
```

**投资回报**: 中等 — 提高测试可读性，但需批量重命名 34 个测试，建议在 M3 前分批完成。

---

## 3. 复杂度报告

### 圈复杂度最高的 5 个函数

| 排名 | 函数 | 位置 | 圈复杂度 | 问题 |
|------|------|------|----------|------|
| 1 | `main()` | `src/bin/sandbox.rs:108` | **15** | 110 行，4 职责，7 个条件分支 |
| 2 | `arbitrate()` | `src/meta/mod.rs:30` | **9** | 5 个域分支 + 2 个嵌套 if-let |
| 3 | `run_pipeline()` | `tests/integration_test.rs:83` | **6** | 测试辅助函数可用 builder |
| 4 | `validate_scenario()` | `src/bin/sandbox.rs:56` | **6** | 4 个 if 守卫 + 1 个 match + 1 个 for |
| 5 | `handle_resonate_req()` | `src/net/bus.rs:56` | **5** | 3 个 match（frame 比较 + 相位差 + 推荐） |

### 简化方案

**`main()` → 圈复杂度 15 → 目标 5**:
采用 P2-01 的重构方案，拆分为 4 个独立函数。每个新函数的圈复杂度 ≤ 3。

**`arbitrate()` → 圈复杂度 9 → 目标 3**:
采用 P2-02 的策略模式将每个域分支独立为策略实现，每个策略的圈复杂度 ≤ 2。

**`validate_scenario()` → 圈复杂度 6 → 目标 3**:
抽取信号验证为独立函数 `validate_signal()`，圈复杂度从 6 降到 3+2。

---

## 4. 坏味道识别

| 坏味道 | 位置 | 严重度 | 说明 |
|--------|------|--------|------|
| **神秘命名** | `run_pipeline()` | 低 | 名称为 What（运行管线），应为 Why（聚合信号并应用策略） |
| **过大的函数** | `sandbox.rs:main()` | **高** | 110 行，远超 50 行上限 |
| **过大的文件** | `src/net/bus.rs` | 中 | 356 行，混合同步逻辑+测试 |
| **霰弹式修改** | Frame 字符串映射 | 中 | 新增 Frame 需改动 sandbox.rs、integration_test.rs、net/node.rs |
| **发散式变化** | `main()` 函数 | 高 | 修改 CLI 参数格式、输出格式、管线逻辑都触发同一个函数改动 |
| **数据泥团** | `NegotiatePayload` 的 participants/frames/phases | 中 | 三个 Vec 应当是一个 `Vec<Participant>` 结构体 |
| **原始迷恋** | `String` 表示 reference frame | 中 | 协议消息中用 `interference: String` 而非常量/枚举 |

---

## 5. 重构路线图

按 ROI 排序：

| 优先级 | 重构任务 | 工作量 | 收益 | 风险 |
|--------|----------|--------|------|------|
| 🔴 P2 | 拆分 `main()` (SRP) | 45 min | 可测试性 ↑↑、圈复杂度 15→5 | 低 — 纯函数抽取 |
| 🔴 P2 | 提取 Frame/TritValue 的 FromStr/i8 映射 (DRY) | 20 min | 3 处重复→1 处 | 低 — 标准 trait 实现 |
| 🟡 P3 | 测试重命名 should_when 模式 | 30 min | 可读性 ↑ | 无 — 纯重命名 |
| 🟡 P3 | TAND/TOR 共享冲突检测函数 (DRY) | 10 min | 减少 8 行重复 | 低 |
| 🟢 P3 | 策略模式重构 arbitrate() (OCP) | 1 hr | 扩展性 ↑ | 中 — 引入 trait 对象，微小的性能开销 |
| 🟢 长期 | `MessagePayload` 枚举中的 String 字段 → 强类型枚举 | 1 hr | 类型安全 ↑ | 中 — 需更新 serde 序列化 |
| 🟢 长期 | `NegotiatePayload` 数据泥团 → `Vec<Participant>` | 30 min | 接口清晰 ↑ | 中 — 破坏协议 JSON 格式 |

### 安全重构顺序

1. **先加测试** — 重构前确保 34 个测试全绿，重构后再次验证
2. **DRY 重构** — FromStr/i8 → main() 拆分 → TAND/TOR 共享
3. **命名重构** — 测试重命名（IDE 自动重构）
4. **OCP 重构** — 策略模式（有性能影响，在 M3 阶段做）

---

## 6. 代码亮点

以下设计值得肯定，不应改动：

- ✅ **`#![forbid(unsafe_code)]`** — 编译时安全边界
- ✅ **`TritWord` 不可变模式** — 构造后通过 `Phase::new()` 校验，内部一致性保证良好
- ✅ **`MetaInterrupt` 审计日志** — 类型安全的审计追溯，含时间戳和原因
- ✅ **`BinaryBaseline` 单元结构体** — 无状态设计，适合基准测试
- ✅ **`PllController` 参数化** — `with_params()` 支持调优，默认参数合理
- ✅ **`Message` 工厂方法** — 6 个构造函数统一创建风格，参数类型明确
- ✅ **`HarmonicClock` 预设工厂** — `physical()` / `deliberative()` 表达意图清晰

---

## 7. 结论

Trit-Core v0.1.0 作为 MVP，代码质量**中等偏上**（3.5/5）。核心模块（trit/frame/meta）架构清晰，`unsafe` 和 unsafe 代码为零。主要问题是 `main()` 函数过长和 frame/value 映射逻辑分散。

**建议行动计划**:
1. M3 前执行 P2 级重构（main 拆分 + FromStr 提取），工作量约 1 小时
2. M3 期间执行测试重命名
3. M4 阶段评估策略模式重构的必要性

---

*审计方: 代码规范与质量工程师 · 方法论: SOLID + DRY + 圈复杂度分析 + 设计味道检测*
