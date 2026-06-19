# 5-Layer Cognitive Architecture — Design Spec

**Date**: 2026-06-19
**Status**: Draft
**Based on**: Full conversation consensus, `自审计.md`, trit-core v0.3.0 codebase

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

### Mount Arbitration

When a new scenario is recognized, the mount arbiter:

1. Computes the **module request set** $M_{\text{need}}$ for the scenario
2. Checks the **currently mounted set** $M_{\text{current}}$
3. Unmounts modules in $M_{\text{current}} \setminus M_{\text{need}}$ (calling `on_unmount()`)
4. Mounts modules in $M_{\text{need}} \setminus M_{\text{current}}$ (calling `on_mount()`)
5. Resolves conflicts: if two requested modules share a resource bottleneck, the arbiter prioritizes by scenario-criticality

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

## Migration Plan

### Phase 1: Scaffold new directories (no code moved)

1. Create `src/anchor/`, `src/hook/`, `src/adapters/`, `src/feedback/` directories with `mod.rs` files
2. Define all traits and type signatures first
3. Register new modules in `src/lib.rs`

### Phase 2: Implement Layer 1 (anchors)

4. Implement all five anchor constraints
5. Implement `AnchorReport` aggregation logic
6. Unit tests for each anchor's threshold behavior

### Phase 3: Implement Layer 2 (hook manager)

7. Implement scenario recognizer with feature vector matching
8. Implement module registry and mount arbiter
9. Integration tests: scenario recognition → correct module set

### Phase 4: Implement Layer 3 (adapters) — migrate existing modules

10. Migrate `src/attention/` → `src/adapters/bandwidth_scheduler.rs`, implement `CognitiveModule`
11. Migrate `src/knowledge/` → `src/adapters/self_knowledge.rs`, implement `CognitiveModule`
12. Migrate `src/reflexive/` → `src/adapters/reflexive_audit.rs`, implement `CognitiveModule`
13. Implement `critical_thinking.rs`, `cognitive_deconstruction.rs`, `conflict_suspension.rs`
14. Implement `engineering.rs`, `ecological_assessment.rs`, `adaptive_iteration.rs`
15. Implement `coupling_adapter.rs`

### Phase 5: Implement Layer 4 facade

16. Create `DecisionEngine` facade in `src/core/`
17. Integrate anchor pre-check into decision pipeline

### Phase 6: Implement Layer 5 (feedback)

18. Implement practice test and consequence review
19. Implement correction trigger and experience recorder
20. Wire feedback signal back to Hook Manager

### Phase 7: Integration and validation

21. End-to-end scenario tests using the full 5-layer pipeline
22. Update `SandboxPipeline` to use `DecisionEngine` facade
23. Update all documentation

## Immutable Design Principles (from conversation consensus)

1. **Anchor layer does not participate in dynamic adaptation.** Baselines are queried, not computed; they provide constraints, not inputs.
2. **Module mounting is scenario-driven, not preset.** There is no "universal module" — only "what the current scenario needs."
3. **Hold is not a failure state.** It is the active intermediate state of "gathering more variables."
4. **An unwise True/False can be corrected immediately.** No need to wait for the next cycle.
5. **Unmount = release.** Prevent the inertia of a previous scenario's modules from contaminating the next one.
6. **Output must pass through practice testing.** An untested output does not constitute "decision complete."
7. **All decisions must operate within the non-negotiable baseline.** Otherwise, downgrade to Hold and alert.
