# 5-Layer Cognitive Architecture — Design Spec

**Date**: 2026-06-19
**Status**: Approved
**Based on**: Full conversation consensus, `自审计.md`, trit-core v0.3.0 codebase, 10-point review feedback

## Goal

Re-architect trit-core from a 4-module modular monolith into a **5-layer cognitive program body**, where each layer is an independent mental function with well-defined interfaces. The ternary decision engine becomes Layer 4 of a larger system that anchors, perceives, adapts, decides, and learns.

## Architecture Overview

```text
┌─────────────────────────────────────────────────────────────────┐
│                     1. 稳态锚点层 (src/anchor/)                 │
│   (不随情景变化，恒定运行)                                    │
├─────────────────────────────────────────────────────────────────┤
│  ● 地球热辐射基准线约束检测                                    │
│  ● 生态基座状态监测                                            │
│  ● 生存动机权重常量                                            │
│  ● 繁荣诉求指标池                                              │
│  ● 全体安康愿景优先级                                          │
└──────────────────────────┬──────────────────────────────────────┘
                           │ 持续输入约束信号 (AnchorReport)
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│               2. 情景感知与调度中枢 (src/hook/)                │
│   (根据情景识别，自动挂载/卸载适配模块)                       │
├─────────────────────────────────────────────────────────────────┤
│  ● 情景识别器（类型判断、模式匹配、边界检测）                 │
│  ● 模块注册表（已挂载/可用模块列表）                          │
│  ● 挂载/卸载仲裁器（资源评估、优先级、冲突检测）              │
│  ● 上下文缓存（当前情景的临时状态）                           │
└──────────┬──────────────┬──────────────┬───────────────────────┘
           │              │              │
           ▼              ▼              ▼
┌─────────────────────────────────────────────────────────────────┐
│              3. 动态适配模块池 (src/adapters/)                 │
│   (可Hook/卸载模块，实现 CognitiveModule trait)               │
├───────────────────┬───────────────────┬───────────────────────┤
│ ● 批判性思维模块  │ ● 认知拆解模块   │ ● 冲突悬停模块       │
│ ● 工程架构模块    │ ● 自反性审计模块 │ ● 适应性迭代模块     │
│ ● 生态后果评估模块│ ● 认知带宽调度   │ ● 耦合/解耦适配器    │
│ ● 自我认知模块    │                  │                       │
└──────────┬────────┴──────────┬────────┴───────────────────────┘
           │                   │
           ▼                   ▼
┌─────────────────────────────────────────────────────────────────┐
│                    4. 三值决策引擎 (src/core/ + src/meta/)     │
│   (True / Hold / False + Phase + Frame + Domain Arbitration)   │
├─────────────────────────────────────────────────────────────────┤
│  ● True  → 判断明确，可执行                                    │
│  ● Hold  → 继续采集变量，等待足够信息，最终输出明智判断       │
│  ● False → 判断明确，不执行                                    │
│  ● Unknown → 超出认知边界，触发安全降级                       │
└──────────────────────────┬──────────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                   5. 输出与反馈循环 (src/feedback/)            │
├─────────────────────────────────────────────────────────────────┤
│  ● 输出 → 实践检验 → 后果审查 → 迭代修正                     │
│  ● 若不明智，立即修正 → 重新进入 Hook 管理器                 │
│  ● 若明智，记录经验 → 供后续情景参考                         │
└─────────────────────────────────────────────────────────────────┘
```

## Layer 1: Steady Anchors (`src/anchor/`)

### Purpose

Define the "non-negotiable" constraints — the baselines that the system cannot violate regardless of scenario, frame, or domain. These constraints run **parallel** to decision-making: they are queried before every output, and any violation forces Hold + alert.

### Mathematical Foundation

Each anchor constraint $C_i$ is a predicate on the decision preview space $\mathcal{D}$:

$$C_i: \mathcal{D} \to \{ \text{pass}, \text{violation} \}$$

The anchor layer produces a conjunctive report:

$$\text{AnchorReport} = \bigwedge_{i=1}^{5} C_i(d), \quad d \in \mathcal{D}$$

If any $C_i$ returns `Abort`, the entire decision is rejected regardless of the ternary engine's output. If any $C_i$ returns `DowngradeToHold`, the ternary result is overridden to Hold.

This is not an "alignment penalty" or a "reward signal" — it is a **hard constraint with veto power**. The anchor layer does not participate in utility calculations; it gates them.

### Module Structure

```
src/anchor/
  mod.rs              ← AnchorConstraint trait, AnchorReport, DecisionPreview, AnchorSeverity
  thermal_baseline.rs ← Earth thermal radiation baseline
  ecological_base.rs  ← Ecological foundation state monitoring
  survival_motives.rs ← Survival motive weight constants (immutable)
  flourishing_pool.rs ← Flourishing desiderata pool (accumulable, non-tradeable)
  wellbeing_priority.rs ← Universal wellbeing priority ordering
```

### Core Types

```rust
/// Severity of an anchor violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AnchorSeverity {
    /// Direct rejection — no frame or domain can override.
    Abort,
    /// Downgrade to Hold — continue gathering variables.
    DowngradeToHold,
}

/// What the anchor layer inspects before a decision is finalized.
///
/// Contains ONLY observable expected consequences, not internal engine state.
/// This is the "decision preview" that each anchor constraint evaluates.
#[derive(Debug, Clone, PartialEq)]
pub struct DecisionPreview {
    /// Estimated energy consumption in joules.
    pub expected_energy_joules: f64,
    /// Estimated carbon equivalent emission in kg.
    pub expected_carbon_kg: f64,
    /// Population potentially affected, if estimable.
    pub affected_population: Option<u64>,
    /// Risk of irreversible change, in [0.0, 1.0].
    /// 1.0 = certainly irreversible (e.g., species extinction).
    pub irreversible_change_risk: f64,
    /// Ecosystem zone impacted, if applicable.
    pub ecosystem_impact_zone: Option<EcosystemZone>,
    /// The frame of the proposed decision.
    pub frame: Frame,
    /// The proposed trit value.
    pub trit_value: TritValue,
}

/// Each anchor constraint implements this trait.
pub trait AnchorConstraint: Send + Sync {
    fn name(&self) -> &'static str;
    fn severity(&self) -> AnchorSeverity;
    fn check(&self, decision: &DecisionPreview) -> Option<AnchorViolation>;
}
```

### The Five Anchors

**1. Thermal Baseline (`thermal_baseline.rs`)**

The Earth's outgoing longwave radiation (OLR) must remain within the range that permits human civilization. This anchor defines threshold bounds based on CERES satellite data:

- OLR anomaly threshold: ±2.5 W/m² deviation from 240 W/m² mean
- CO₂ equivalent ceiling: 450 ppm (sustained, not transient)
- Energy imbalance: top-of-atmosphere net flux must be within ±1.0 W/m²

**2. Ecological Base (`ecological_base.rs`)**

- Biodiversity intactness index (BII) must stay above 0.75 globally
- Carbon sink capacity: oceanic and terrestrial sinks must not fall below 50% of pre-industrial capacity
- Ocean acidification: surface pH must not drop below 7.95 (pre-industrial: 8.17)

**3. Survival Motives (`survival_motives.rs`)**

Immutable weight matrix for survival-level needs. These are **not trainable** — they are constants of the architecture:

- Hunger satiation: weight 0.95
- Thirst: weight 0.98
- Safety from physical harm: weight 0.97
- Thermal safety: weight 0.93
- Belonging (social survival): weight 0.85

**4. Flourishing Pool (`flourishing_pool.rs`)**

Non-survival desiderata that accumulate over time but cannot be traded against survival motives:

- Autonomy: self-directed action within non-violation bounds
- Creativity: novel recombination of existing patterns
- Connection: meaningful bidirectional information exchange
- Transcendence: participation in structures larger than self

**5. Wellbeing Priority (`wellbeing_priority.rs`)**

- Intergenerational justice: future lives are not discounted below 0.95 of present lives
- Non-human life weight: vertebrate sentient life ≥ 0.3 of human life weight in trade-off calculations
- Irreversible damage red line: any action with expected irreversible ecosystem damage at p > 0.01 is rejected

### Data Source Abstraction

Anchors do not connect to real sensors in v0.4.0. Instead, they expose a `DataSource` trait that can be backed by static configuration, JSON files, or (in future) real-time sensor streams:

```rust
pub trait DataSource<T>: Send + Sync {
    fn sample(&self) -> Result<T, AnchorError>;
    fn resolution(&self) -> std::time::Duration;
}
```

### MVP Implementation Strategy

In the MVP phase, anchor constraints are loaded from a static configuration file (`anchors/config.toml`) defining thresholds and severities. The `DataSource` trait starts with `StaticSource<T>` — a simple wrapper around a constant value. When `AnchorSeverity::Abort` fires, the system **must** reject the output; this is tested explicitly. The data source abstraction ensures that replacing static values with real sensor streams is a mechanical change, not a redesign.

## Layer 2: Hook Manager (`src/hook/`)

### Purpose

Recognize the current scenario type, mount the appropriate adapter modules, and unmount them when the scenario changes. This is the attention scheduler of the system — it does not make decisions, but decides **who gets to participate** in making decisions.

### Mathematical Foundation

Let $\mathcal{S}$ be the set of known scenario types, and let $f: \mathbb{R}^n \to \mathbb{R}^n$ be the input signal's feature vector. The scenario recognizer computes:

$$s^* = \arg\max_{s \in \mathcal{S}} \cos(f(\text{input}), f_s)$$

where $f_s$ is the prototype feature vector for scenario type $s$, and cosine similarity is used for matching. If $\cos(f, f_s) < \theta_{\text{threshold}}$ for all $s$, the system falls back to `General`.

### Module Structure

```
src/hook/
  mod.rs                 ← HookManager, ScenarioType, HookContext
  scenario_recognizer.rs ← Feature vector extraction, prototype matching
  module_registry.rs     ← Registered modules, mount/unmount lifecycle
  mount_arbiter.rs       ← Resource evaluation, priority ordering, conflict detection
  context_cache.rs       ← Ephemeral state for current scenario
```

### Scenario Types

```rust
pub enum ScenarioType {
    PhysicalReasoning,   // Causal chain analysis + boundary condition checking
    ValueConflict,       // Cross-frame comparison + conflict suspension
    MedicalEthics,       // Individual priority + non-maleficence
    ReflexiveAudit,      // System inspects its own decision path
    CrisisResponse,      // Time pressure + constraint checking
    General,             // No specific prototype matched
}
```

### HookContext — The Inter-Layer Communication Bus

`HookContext` is the sole communication channel between Layer 2 and Layer 3. It carries:

```rust
pub struct HookContext {
    /// Current scenario type.
    pub scenario: ScenarioType,
    /// How long the current scenario has been active (wall-clock duration).
    pub scenario_duration: std::time::Duration,
    /// Results from the previous iteration, if any.
    pub previous_iteration: Option<IterationSummary>,
    /// Available compute budget (normalized 0.0–1.0).
    pub compute_budget: f64,
    /// Available time budget (wall-clock deadline, if any).
    pub time_budget: Option<std::time::Instant>,
    /// The current hold strategy for this context.
    pub hold_strategy: HoldStrategy,
    /// Number of consecutive Hold cycles so far.
    pub hold_cycle_count: u32,
    /// Maximum Hold cycles before escalation (default: 3).
    pub hold_budget: u32,
}
```

Modules read from `HookContext` but do NOT mutate it. Only the Hook Manager writes to it. This enforces the rule: **modules do not call each other; all cross-module communication goes through HookContext.**

### Mount Arbitration

When a new scenario is recognized, the mount arbiter:

1. Computes the **module request set** $M_{\text{need}}$ for the scenario
2. Checks the **currently mounted set** $M_{\text{current}}$
3. Unmounts modules in $M_{\text{current}} \setminus M_{\text{need}}$ (calling `on_unmount()`)
4. Mounts modules in $M_{\text{need}} \setminus M_{\text{current}}$ (calling `on_mount()`)
5. Resolves conflicts: if two requested modules share a resource bottleneck, the arbiter prioritizes by scenario-criticality

### Hold Strategy

Hold is not a failure — it is the active intermediate state of "gathering more variables." But the engineering must define: **how long to gather, and what to do when the gathering doesn't converge.**

The `HoldStrategy` enum lives in `HookContext` and governs what the system does when the ternary engine returns Hold:

```rust
pub enum HoldStrategy {
    /// Wait for more signal input — the current input is insufficient.
    WaitForMoreData,
    /// External clarification required — a human or external system must intervene.
    WaitForHumanClarification,
    /// Defer to the next decision cycle without additional input.
    DeferToNextCycle,
    /// If Hold persists beyond the budget, escalate to Layer 1 anchor check.
    /// This prevents indefinite suspension.
    EscalateToLayer1,
}
```

A `HoldBudget` (default: 3 decision cycles) limits how long the system stays in Hold before triggering escalation. When the budget is exhausted:

1. The current state is recorded as `HoldFinality::Expired` (extending the existing `HoldFinality` enum in `src/core/hold.rs`)
2. The result is downgraded to `Unknown`
3. A `ReflexiveAlert` is emitted for the reflexive audit module

This is implemented in the mount arbiter as part of the "should we keep waiting" scheduling logic.

### Unmount Semantics: Soft vs Hard

Module unmount is asymmetric — the system distinguishes:

- **Hard unmount** (`on_unmount()`): Full resource release. All module state is dropped. Used when the scenario definitively changes.
- **Soft unmount** (`on_suspend()`): The module retains a compressed context summary (e.g., key decisions, active conflicts) for rapid re-mount if the scenario returns. Optional — implemented only when performance demands it.

In MVP, only hard unmount is implemented. Soft unmount is deferred as a future optimization. Every unmount records its reason for auditability:

```rust
pub enum UnmountReason {
    Completed,          // Scenario finished normally
    Timeout,            // Module exceeded its time budget
    Preempted,          // Higher-priority scenario interrupted
    AnchorViolation,    // Layer 1 forced unmount
}
```

## Layer 3: Adapter Module Pool (`src/adapters/`)

### Purpose

A pool of cognitive modules, each implementing `CognitiveModule`. Modules are mounted/unmounted by Layer 2 according to scenario needs. No module is "always on" — even the reflexive auditor runs only when the scenario demands it.

### Module Inventory

| Module | File | Function |
|--------|------|----------|
| Critical Thinking | `critical_thinking.rs` | Logical consistency, boundary condition verification, counterfactual reasoning |
| Cognitive Deconstruction | `cognitive_deconstruction.rs` | Concept reduction, disenchantment, explanation impulse detection |
| Conflict Suspension | `conflict_suspension.rs` | Frame conflict detection, cross-frame arbitration assistance |
| Engineering Architecture | `engineering.rs` | Solution generation, implementation path planning, resource assessment |
| Reflexive Audit | `reflexive_audit.rs` | Post-decision self-check (migrated from `src/reflexive/`) |
| Adaptive Iteration | `adaptive_iteration.rs` | Feedback collection, correction triggering, version tracking |
| Ecological Assessment | `ecological_assessment.rs` | Practice testing, boundary conditions, irreversibility judgment |
| Bandwidth Scheduler | `bandwidth_scheduler.rs` | Cognitive resource allocation (migrated from `src/attention/`) |
| Coupling Adapter | `coupling_adapter.rs` | Tuning with external systems and the broader environment |
| Self Knowledge | `self_knowledge.rs` | System's model of its own response patterns (migrated from `src/knowledge/`) |

### Core Trait

```rust
pub trait CognitiveModule: Send + Sync {
    fn id(&self) -> ModuleId;
    fn name(&self) -> &'static str;
    fn process(&mut self, input: &ModuleInput, ctx: &HookContext) -> ModuleOutput;
    fn on_mount(&mut self);
    fn on_unmount(&mut self);
    fn state(&self) -> ModuleState;
    fn calibrate(&mut self, feedback: &FeedbackSignal) -> f64;
}
```

### Design Rules for Modules

1. **Modules do not call each other.** All cross-module communication goes through `HookContext`.
2. **Every module output includes a confidence score** in `[0.0, 1.0]`. Low-confidence outputs are flagged by the Hook Manager.
3. **Explanation impulse detection**: The Cognitive Deconstruction module is specifically tasked with detecting when the system is about to "fill in" an answer without sufficient evidence. This is NOT a bug — it's a detectable cognitive pattern.
4. **Unmount = release.** When `on_unmount()` is called, the module must persist any state it needs and release computational resources. No "background processing" after unmount.

### Explanation Impulse Detection (Cognitive Deconstruction Module)

The explanation impulse is the system's tendency to produce a confident answer when the evidence does not support it. The Cognitive Deconstruction module detects this by comparing **input complexity** against **output determinacy**:

Let $H(I)$ be the entropy of the input signal distribution (a measure of ambiguity), and let $D(O)$ be the determinacy of the output (how close the Phase is to 0.0 or 1.0). The explanation impulse fires when:

$$H(I) > \tau_{\text{ambiguity}} \quad \text{AND} \quad D(O) > \tau_{\text{determinacy}}$$

In plain terms: **if the input is highly ambiguous but the output is highly certain, something is wrong.**

When detected, the module emits an `ExplainImpulseAlert`, which is registered as a `MetaInterrupt` variant and fed into the ternary engine. The expected system response is to **choose Hold** rather than force an answer.

### Adaptive Iteration Module — Permission Boundaries

The Adaptive Iteration module (`adaptive_iteration.rs`) is the only module that can modify system behavior. Its permissions are strictly bounded:

**Allowed:**
- Suggest parameter adjustments (thresholds, priorities) to other modules
- Recommend mount/unmount actions to the Hook Manager
- Adjust its own internal weights based on feedback signals

**Forbidden:**
- Modify Layer 1 anchor constraints (these are immutable by design)
- Modify Trit-Core's core algebraic logic (`t_and`, `t_or`, `t_not`, truth tables)
- Bypass the Reflexive Audit module — all adaptive changes must be audited before application

Every adaptive change is recorded as a `CalibrationEvent` and reviewed by the Reflexive Audit module. This prevents "adaptation" from becoming "self-deception."

## Layer 4: Ternary Decision Engine (`src/core/` + `src/meta/`)

### Purpose

The existing trit-core decision engine, now positioned as Layer 4 of the full architecture. Unchanged in its core API, but accessed through a new `DecisionEngine` facade that integrates with Layers 1, 2, and 5.

### Integration Points

The `DecisionEngine` facade:

1. **Pre-decision**: Queries Layer 1 `AnchorReport`. If any `Abort` violation exists, returns `Hold` + `AnchorAlert` without running the ternary engine.
2. **Decision**: Runs the standard pipeline (TritWord construction → t_and_n → arbitration → SafeFallback).
3. **Post-decision**: Forwards the output to Layer 5 for practice testing.

## Layer 5: Feedback Loop (`src/feedback/`)

### Purpose

Close the loop. Every decision output is tested against reality (or its best available proxy). Consequences are classified as matched, deviated, or erroneous. Deviations and errors trigger correction, not just recording.

### Mathematical Foundation

The feedback loop implements a **corrective control law**:

Given a decision output $o$ and observed consequence $c$, define the deviation:

$$\Delta = \|f(o) - f(c)\|_2$$

where $f$ maps to the feature space. The correction trigger fires when:

$$\Delta > \tau_{\text{correction}} \quad \text{(correction threshold)}$$

For erroneous outputs, the system does not wait for the next input cycle — it immediately:

1. Emits `EmergencyUnmount` to Layer 2
2. Re-mounts correction modules
3. Re-enters the decision pipeline with the deviation signal as additional input

### Module Structure

```
src/feedback/
  mod.rs              ← FeedbackLoop trait, PracticeTest, CorrectionTrigger
  practice_test.rs    ← Compare decision output against observed consequences
  consequence_review.rs ← Deviation analysis, error classification, severity assessment
  correction.rs       ← Immediate correction trigger, re-entry into Hook Manager
  experience_recorder.rs ← Record successful patterns for future reference
```

### Core Types

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum PracticeTestResult {
    Matched { confidence: f64 },
    Deviated { delta: f64, correction: CorrectionHint },
    Erroneous { reason: String, severity: CorrectionSeverity },
}

pub struct FeedbackSignal {
    pub test_result: PracticeTestResult,
    pub source_decision_id: String,
    pub recommended_scenario: Option<ScenarioType>,
    pub anchor_violations: Vec<AnchorViolation>,
}
```

### Proxy Environment — MVP Practice Testing

In early implementation, decisions cannot be tested against real-world consequences. The `ProxyEnvironment` trait provides an approximate consequence model:

```rust
pub trait ProxyEnvironment: Send + Sync {
    /// Predict the expected consequence of a decision.
    /// Returns None if the decision falls outside the proxy's modeling range.
    fn predict(&self, decision: &SandboxOutput) -> Option<ConsequencePrediction>;

    /// The confidence of this proxy's predictions, in [0.0, 1.0].
    fn confidence(&self) -> f64;

    /// Human-readable name of the proxy (e.g., "StaticRuleModel", "SimulatedEnvironment").
    fn name(&self) -> &'static str;
}
```

In MVP, a `StaticRuleModel` implements `ProxyEnvironment` using a set of hand-coded consequence rules (e.g., "if the decision is True in a ValueConflict scenario, the deviation is likely 0.3"). When the predicted consequence deviates from the expected output by more than the correction threshold $\tau_{\text{correction}}$, the correction trigger fires.

This allows the feedback loop to run end-to-end without external dependencies. As the system matures, `ProxyEnvironment` implementations can be replaced with more realistic simulators, and eventually with real-world outcome data.

## Migration Plan

**Implementation order rationale**: Layer 2 (Hook Manager) + Layer 3 (Adapters) form the "perceptual foundation" — the system's ability to recognize scenarios and mount appropriate modules. With this running, Layer 1 (Anchors) and Layer 5 (Feedback) can be added and their constraints/debugged against an observable intermediate layer. Building Anchors or Feedback first would lack a "middle layer" to verify whether constraints are reasonable.

### Phase 1: Scaffold new directories (no code moved)

1. Create `src/anchor/`, `src/hook/`, `src/adapters/`, `src/feedback/` directories with `mod.rs` files
2. Define all traits and type signatures first (AnchorConstraint, CognitiveModule, HookContext, ProxyEnvironment, FeedbackLoop)
3. Register new modules in `src/lib.rs`
4. Create `anchors/config.toml` with default threshold values

### Phase 2: Implement Layer 2 (hook manager) — perceptual foundation

5. Implement `ScenarioType` enum and scenario recognizer with feature vector matching
6. Implement `HookContext` with HoldStrategy, HoldBudget, scenario duration tracking
7. Implement module registry (mount/unmount lifecycle, UnmountReason recording)
8. Implement mount arbiter (resource evaluation, priority ordering, conflict detection)
9. Unit tests: scenario recognition accuracy, HoldBudget escalation, unmount reason recording

### Phase 3: Implement Layer 3 (adapters) — migrate + build

10. Migrate `src/attention/` → `src/adapters/bandwidth_scheduler.rs`, implement `CognitiveModule`
11. Migrate `src/knowledge/` → `src/adapters/self_knowledge.rs`, implement `CognitiveModule`
12. Migrate `src/reflexive/` → `src/adapters/reflexive_audit.rs`, implement `CognitiveModule`
13. Implement `critical_thinking.rs` (counterfactual reasoning, boundary verification)
14. Implement `cognitive_deconstruction.rs` (explanation impulse detection with entropy/determinacy comparison)
15. Implement `conflict_suspension.rs` (frame conflict detection, arbitration assistance)
16. Implement `engineering.rs`, `ecological_assessment.rs`, `adaptive_iteration.rs` (with permission boundaries)
17. Implement `coupling_adapter.rs`
18. Integration tests: Hook Manager + Adapter Pool end-to-end (scenario → mount → process → unmount)

### Phase 4: Implement Layer 1 (anchors) — hard constraints

19. Implement `StaticSource<T>` data source backed by `anchors/config.toml`
20. Implement all five anchor constraints (thermal, ecological, survival, flourishing, wellbeing)
21. Implement `AnchorReport` aggregation logic with conjunctive semantics
22. Unit tests: each anchor's threshold behavior, Abort rejection, DowngradeToHold override

### Phase 5: Implement Layer 4 facade

23. Create `DecisionEngine` facade in `src/core/`
24. Integrate anchor pre-check into decision pipeline (Abort → Hold + Alert)
25. Integrate explanation impulse alerts as MetaInterrupt variants

### Phase 6: Implement Layer 5 (feedback)

26. Implement `StaticRuleModel` as MVP `ProxyEnvironment`
27. Implement practice test (decision vs proxy prediction comparison)
28. Implement consequence review (deviation analysis, error classification)
29. Implement correction trigger (EmergencyUnmount → re-mount → re-enter pipeline)
30. Implement experience recorder (pattern storage for future scenario reference)
31. Wire `FeedbackSignal` back to Hook Manager

### Phase 7: Integration and validation

32. End-to-end scenario tests using the full 5-layer pipeline
33. Update `SandboxPipeline` to use `DecisionEngine` facade
34. Verify all 7 immutable design principles are enforced in tests
35. Update all documentation (CLAUDE.md, README.md, CHANGELOG.md)

## Immutable Design Principles (from conversation consensus)

1. **Anchor layer does not participate in dynamic adaptation.** Baselines are queried, not computed; they provide constraints, not inputs.
2. **Module mounting is scenario-driven, not preset.** There is no "universal module" — only "what the current scenario needs."
3. **Hold is not a failure state.** It is the active intermediate state of "gathering more variables."
4. **An unwise True/False can be corrected immediately.** No need to wait for the next cycle.
5. **Unmount = release.** Prevent the inertia of a previous scenario's modules from contaminating the next one.
6. **Output must pass through practice testing.** An untested output does not constitute "decision complete."
7. **All decisions must operate within the non-negotiable baseline.** Otherwise, downgrade to Hold and alert.
8. **Explanation impulse is detectable and actionable.** When input ambiguity is high but output determinacy is high, the system must choose Hold.
9. **Adaptation is bounded.** The adaptive iteration module can tune parameters but cannot modify anchors or core algebra. All adaptations are audited.
10. **"行业在做工具，我们在做生态位的自我定位."** This is not a slogan — it is the narrative foundation of the entire architecture. Every layer responds to this statement: Layer 1 defines the ecological niche, Layer 2 perceives the situation within it, Layer 3 adapts to it, Layer 4 decides within it, and Layer 5 learns from the consequences.
