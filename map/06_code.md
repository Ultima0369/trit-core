# MOC — 代码链导航

> **Scope**: 从源码文件出发，指向对应的知识文档。这是"链 A → 链 B"的反向连接。
>
> #trit-core #code #source #implementation #cross-chain

---

## 核心代数（`src/core/`）

| 源码文件 | 职责 | 对应文档 |
|---|---|---|
| `src/core/trit.rs` | `TritValue` enum（4 状态） | [[CONCEPTS]] §1, [[001-ternary-logic]], [[003-ternary-over-binary]] |
| `src/core/frame.rs` | `Frame` enum（9 变体） | [[CONCEPTS]] §2, [[004-geoeco-frame]], [[FRAME_MODEL_SPEC]] |
| `src/core/phase.rs` | `Phase` struct（[0.0, 1.0]） | [[CONCEPTS]] §3, [[PHASE_ARITHMETIC]], [[002-phase-arithmetic]] |
| `src/core/algebra.rs` | `TernaryAlgebra`（TAND/TOR/TNOT） | [[CONCEPTS]] §1.4, [[PHASE_ARITHMETIC]] |
| `src/core/word.rs` | `TritWord`（值 + 帧 + 相位） | [[CONCEPTS]] §4, [[api]] |
| `src/core/mod.rs` | 核心模块导出 | [[MODULES]], [[ARCHITECTURE]] |

---

## 元监控（`src/meta/`）

| 源码文件 | 职责 | 对应文档 |
|---|---|---|
| `src/meta/interrupt.rs` | `MetaInterrupt` 结构 | [[CONCEPTS]] §2, [[003-domain-conflict]] |
| `src/meta/arbitration.rs` | `ResolutionPolicy` 与仲裁逻辑 | [[CONCEPTS]] §3, [[003-domain-conflict]], [[CONFLICT_CATALOG]] |
| `src/meta/safe_fallback.rs` | `SafeFallback`（可关闭） | [[CONCEPTS]] §5, [[009-ethics-hardening]], [[SECURITY_MODEL]] |
| `src/meta/security_mode.rs` | `SecurityMode` enum | [[009-ethics-hardening]], [[SECURITY_MODEL]] |
| `src/meta/mod.rs` | 元模块导出 | [[MODULES]], [[ARCHITECTURE]] |

---

## 沙盒层（`src/sandbox/`）

| 源码文件 | 职责 | 对应文档 |
|---|---|---|
| `src/sandbox/input.rs` | `ScenarioInput`, `SignalInput` | [[PIPELINE_DESIGN]], [[QUICKSTART]], [[CLI_REFERENCE]] |
| `src/sandbox/output.rs` | `SandboxOutput` | [[PIPELINE_DESIGN]], [[api]] |
| `src/sandbox/validate.rs` | 输入验证与净化 | [[PIPELINE_DESIGN]], [[TESTING_STRATEGY]] |
| `src/sandbox/pipeline.rs` | 主管道：t_and_n → arbitrate → SafeFallback | [[PIPELINE_DESIGN]], [[SYSTEM_DESIGN]] |
| `src/sandbox/diagnostic.rs` | 运行时遥测 | [[ARCHITECTURE]], [[SYSTEM_DESIGN]] |
| `src/sandbox/error.rs` | `SandboxError` | [[api]], [[CLI_REFERENCE]] |
| `src/sandbox/validator.rs` | 预期行为验证 | [[TESTING_STRATEGY]], [[validation-report]] |
| `src/sandbox/mod.rs` | 沙盒模块导出 | [[MODULES]], [[ARCHITECTURE]] |

---

## 二进制入口（`src/bin/`）

| 源码文件 | 职责 | 对应文档 |
|---|---|---|
| `src/bin/sandbox.rs` | `trit-sandbox` CLI | [[CLI_REFERENCE]], [[QUICKSTART]], [[CONTRIBUTING]] |
| `src/bin/dhat_profile.rs` | 堆内存分析 | [[BENCHMARK]], [[DEPLOYMENT_GUIDE]] |

---

## 根目录文件

| 源码文件 | 职责 | 对应文档 |
|---|---|---|
| `src/lib.rs` | 公共 API 导出 | [[api]], [[MODULES]] |
| `Cargo.toml` | 依赖与构建配置 | [[CONTRIBUTING]], [[DEPLOYMENT_GUIDE]] |
| `Cargo.lock` | 锁定依赖版本 | [[CONTRIBUTING]] |
| `deny.toml` | 依赖审计策略 | [[SECURITY_MODEL]], [[CONTRIBUTING]] |

---

## 测试与基准（`tests/`, `benches/`）

| 源码位置 | 职责 | 对应文档 |
|---|---|---|
| `tests/` | 单元测试与集成测试 | [[TESTING_STRATEGY]], [[CONTRIBUTING]] |
| `benches/` | Criterion 性能基准 | [[BENCHMARK]], [[performance-validation]] |
| `fuzz/` | 模糊测试输入 | [[TESTING_STRATEGY]] |

---

## Aurora 专用代码（预留）

| 源码位置 | 职责 | 对应文档 |
|---|---|---|
| `src-tauri/` | Tauri GUI 框架 | [[006-tauri-over-electron]], [[UI_SPEC]] |
| `src/wavelet/` | 小波分析引擎 | [[WAVELET_ANALYSIS]], [[WAVELET_ENGINE_SPEC]], [[002-wavelet-over-fft]] |
| `src/db/` | SQLite 数据层 | [[DATA_MODEL]], [[007-sqlite-over-postgres]] |
| `src/attention/` | 注意力引擎（预留） | [[ATTENTION_DYNAMICS]], [[ATTENTION_CAPITALISM]] |

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
