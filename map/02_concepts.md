# MOC — 核心概念

> **Scope**: Trit-Core 中所有核心类型的定义、语义、设计理由，以及它们在实际代码中的对应位置。
> 
> #trit-core #concepts #types #semantics #frames #trit-value

---

## 三值逻辑单元

| 文件 | 位置 | 语言 | 关键内容 |
|---|---|---|---|
| [[CONCEPTS]] | `docs/explanation/CONCEPTS.md` | 中文 | TritValue 定义、MVL-3 数学基础、运算法则（TAND/TOR/TNOT）。 |
| [[001-ternary-logic]] | `docs/adr/001-ternary-logic.md` | 英文 | ADR：为什么三值逻辑替代二值。 |
| [[003-ternary-over-binary]] | `aurora/05_adr/003-ternary-over-binary.md` | 中文 | Aurora ADR：三值在决策质量上的优势。 |

**代码位置**: `src/core/trit.rs` (TritValue enum), `src/core/algebra.rs` (TernaryAlgebra)

---

## 帧系统（Frame）

| 文件 | 位置 | 语言 | 关键内容 |
|---|---|---|---|
| [[CONCEPTS]] | `docs/explanation/CONCEPTS.md` | 中文 | Frame 定义：Science, Individual, Consensus, Absolute, Meta + 扩展。 |
| [[004-geoeco-frame]] | `aurora/05_adr/004-geoeco-frame.md` | 中文 | 为什么 Frame 从 4 个扩展到 9 个（Aurora 扩展）。 |
| [[FRAME_MODEL_SPEC]] | `aurora/07_specs/FRAME_MODEL_SPEC.md` | 中文 | 帧模型规格：语义、映射、枚举定义。 |

**代码位置**: `src/core/frame.rs` (Frame enum, 9 变体)

### 帧之间的不可通约性

| 文件 | 位置 | 语言 | 关键内容 |
|---|---|---|---|
| [[CONFLICT_CATALOG]] | `docs/explanation/insights/CONFLICT_CATALOG.md` | 中文 | 跨域冲突模式分类：Science vs Individual, Consensus vs Absolute 等。 |
| [[FRAME_EPISTEMOLOGY]] | `aurora/01_insights/FRAME_EPISTEMOLOGY.md` | 中文 | 帧认识论：为什么不同帧不能互相化约。 |

---

## 相位算术（Phase）

| 文件 | 位置 | 语言 | 关键内容 |
|---|---|---|---|
| [[PHASE_ARITHMETIC]] | `aurora/02_math/PHASE_ARITHMETIC.md` | 中文 | 相位定义 [0.0, 1.0]、运算、连续性。 |
| [[002-phase-arithmetic]] | `docs/adr/002-phase-arithmetic.md` | 英文 | ADR：为什么用浮点相位而非离散状态。 |
| [[CONCEPTS]] | `docs/explanation/CONCEPTS.md` | 中文 | Phase 的构造器约束（new 返回 Result）。 |

**代码位置**: `src/core/phase.rs` (Phase struct, 区间不变量由类型系统保证)

---

## 元监控与中断

| 文件 | 位置 | 语言 | 关键内容 |
|---|---|---|---|
| [[CONCEPTS]] | `docs/explanation/CONCEPTS.md` | 中文 | MetaInterrupt 定义：跨域冲突时系统如何表达"我检测到冲突"。 |
| [[META_MONITOR]] | `src/meta/` (代码内) | Rust | `MetaInterrupt` 和 `SafeFallback` 的实现。 |
| [[009-ethics-hardening]] | `aurora/05_adr/009-ethics-hardening.md` | 中文 | 元监控的伦理约束：系统不阻断运算，只通知。 |

**代码位置**: `src/meta/interrupt.rs`, `src/meta/safe_fallback.rs`, `src/meta/arbitration.rs`

---

## 安全回退（SafeFallback）

| 文件 | 位置 | 语言 | 关键内容 |
|---|---|---|---|
| [[CONCEPTS]] | `docs/explanation/CONCEPTS.md` | 中文 | 危险域（Physical, Engineering）强制 False 的设计。 |
| [[009-ethics-hardening]] | `aurora/05_adr/009-ethics-hardening.md` | 中文 | 用户可关闭 SafeFallback（不剥夺）。 |
| [[SECURITY_MODEL]] | `aurora/03_whitepaper/SECURITY_MODEL.md` | 中文 | 安全模型与回退策略。 |

**代码位置**: `src/meta/safe_fallback.rs` (`SafeFallback::disabled()` 允许用户关闭)

---

## 跨链连接

| 概念 | 文档 | 代码 |
|---|---|---|
| TritValue (4 状态) | `docs/explanation/CONCEPTS.md` §1 | `src/core/trit.rs` enum |
| Frame (9 变体) | `aurora/07_specs/FRAME_MODEL_SPEC.md` | `src/core/frame.rs` enum |
| Phase [0.0, 1.0] | `aurora/02_math/PHASE_ARITHMETIC.md` | `src/core/phase.rs` struct |
| TAND/TOR/TNOT | `docs/explanation/CONCEPTS.md` §1.4 | `src/core/algebra.rs` |
| MetaInterrupt | `docs/explanation/CONCEPTS.md` §2 | `src/meta/interrupt.rs` |
| SafeFallback | `aurora/05_adr/009-ethics-hardening.md` | `src/meta/safe_fallback.rs` |
| ResolutionPolicy | `docs/explanation/CONCEPTS.md` §3 | `src/meta/arbitration.rs` |

---

**相关 MOC**: [[01_manifest]] · [[03_adr]] · [[04_math]] · [[06_code]]

#map-of-content #concepts #types #trit-value #frame #phase #meta-interrupt #safe-fallback
