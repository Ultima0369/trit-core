# MOC — 代码链导航

> **Scope**: 从源码文件出发，指向对应的知识文档。这是"链 A → 链 B"的反向连接。
>
> **最后更新**: 2026-06-22（M0 完成同步 — ingest/attention 模块从预留移至已实现）
>
> #trit-core #code #source #implementation #cross-chain

---

## 核心代数（`src/core/`）

| 源码文件 | 职责 | 对应文档 |
|---|---|---|
| `src/core/value.rs` | `TritValue` enum（4 状态：True/Hold/False/Unknown） | [[CONCEPTS]] §1, [[001-ternary-logic]], [[003-ternary-over-binary]] |
| `src/core/frame.rs` | `Frame` enum（13 变体：Science/Individual/Consensus/Absolute/Meta/FirstPerson/Embodied/Relational/GeoEco/Developmental/Role/Environmental/Instrumental） | [[CONCEPTS]] §2, [[004-geoeco-frame]], [[005-instrumental-frame]] |
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
| `aurora/src/lib.rs` | 库入口：pipeline / percept / cli / bc | [[CTO_ROADMAP]] |
| `aurora/src/cli.rs` | 命令行参数（clap derive） | [[CLI_REFERENCE]], [[PIPELINE_DESIGN]] |
| `aurora/src/pipeline/analysis.rs` | 分析链路：SignalSpec → FFT → TritWord → TernaryDecision | [[PIPELINE_DESIGN]], [[CTO_ROADMAP]] |
| `aurora/src/pipeline/attention.rs` | 注意力链路：TritWord[] → AttentionScheduler → AuditTrail → SQLite | [[PIPELINE_DESIGN]] |
| `aurora/src/percept/chain.rs` | PerceptChain：LLM → 结构化降级（感知信号分解） | [[LLM_PERCEPTION_LAYER]] |
| `aurora/src/percept/prism.rs` | PrismEngine：RawSignal → TritWord 分解 | [[LLM_PERCEPTION_LAYER]] |
| `aurora/src/bc/` | 6 个限界上下文（M1 BC 架构） | [[ARCHITECTURE]] |
| `aurora/src/db/` | SQLite 数据层（rusqlite） | [[DATA_MODEL]], [[007-sqlite-over-postgres]] |
| `aurora/src/wavelet/` | 合成信号生成 + FFT 基频检测 | [[WAVELET_ANALYSIS]], [[WAVELET_ENGINE_SPEC]] |
| `aurora/src/ingest/` | JSON fallback 数据源 | [[DATA_INGESTION_SPEC]] |

---

## Aurora 预留代码（M1-M4，进行中）

| 源码位置 | 职责 | 对应文档 |
|---|---|---|
| `src-tauri/` | Tauri GUI 框架（M1，桌面打包已验证） | [[006-tauri-over-electron]], [[UI_SPEC]], [[CTO_ROADMAP]] §M1 |
| `dataforge/` | 互联网数据采集 crate（5 数据源 + L2 缓存，M2 完成） | [[CTO_ROADMAP]] §M2 |

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
