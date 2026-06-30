# Trit-Core 技术白皮书

**版本**：0.1.0  
**日期**：2026-06-17  
**状态**：MVP（最小可行产品）  
**代码规模**：~6,500 行 Rust | 227 个测试 | 17 个集成场景  
**许可证**：MIT  

> **历史版本说明**：本文档描述的是 Trit-Core v0.1.x 架构。网络层（`src/net/`、`ResonanceBus`、分布式节点协议）已在 v0.2.0 移除。当前 API 与模块结构请参考 `docs/reference/api.md`、`docs/reference/MODULES.md` 与 `docs/explanation/ARCHITECTURE.md`。

---

## 摘要

Trit-Core 是一个**三值决策引擎**（Ternary Decision Engine），为冲突感知型 AI 对齐提供运行时基础设施。其核心假设是：三元逻辑系统——通过引入"刻意悬置判断"（Hold）这一第三状态——在处理跨领域、跨价值体系决策冲突时，比传统二元 RLHF（基于人类反馈的强化学习）系统产生更真实的对齐结果。

本白皮书涵盖 Trit-Core 的完整架构设计、核心代数原理、工程优化策略及安全性机制，可作为其他项目在"多值逻辑决策系统"方向上的技术参考。

---

## 目录

1. [技术动机](#1-技术动机)
2. [核心概念](#2-核心概念)
3. [架构设计](#3-架构设计)
4. [模块详解](#4-模块详解)
5. [数据流与管道](#5-数据流与管道)
6. [安全机制](#6-安全机制)
7. [性能工程](#7-性能工程)
8. [测试策略](#8-测试策略)
9. [工程规范](#9-工程规范)
10. [已知局限](#10-已知局限)
11. [适用场景参考](#11-适用场景参考)

---

## 1. 技术动机

### 1.1 二元对齐的局限

当前 AI 对齐系统普遍采用二元逻辑（True/False），在以下场景中表现出结构性缺陷：

1. **跨领域冲突**：科学事实（Science Frame）与个人价值判断（Individual Frame）冲突时，二元系统强制收敛到"多数票"或"奖励模型平均值"，实际上抹除了冲突本身
2. **价值不可通约性**：某些决策领域（如生命伦理学）中，不同伦理框架产生的结果不可比较——二元聚合无法表达"无法比较"
3. **安全关键领域**：在物理/工程安全领域，当系统无法确认决策正确性时，乐观地输出 True（"可以这样做"）可能导致灾难性后果

### 1.2 三元方案

Trit-Core 引入第三逻辑状态 **Hold**（悬置判断），语义定义为：

> 系统检测到冲突，但刻意选择不裁决——保留冲突记录，交由上层策略或人类审查。

Hold 不是"不知道"（那是 Unknown），而是"已知冲突，暂不裁决"——这是一个**主动的、可审计的**决策。

---

## 2. 核心概念

### 2.1 TritValue：四状态体系

| 状态 | 符号 | 数值 | 语义 |
|------|------|------|------|
| `True` | +1 | 1 | 肯定裁决 |
| `Hold` | 0 | 0 | 刻意悬置判断（MVL-3 核心） |
| `False` | -1 | -1 | 否定裁决 |
| `Unknown` | ⊥ | 0 (算术) | 分布外/不可知输入 |

**Hold vs Unknown 的区别**：
- `Hold`：系统**可以**裁决但**选择**不裁决——冲突被检测到并保留
- `Unknown`：系统**无法**裁决——输入本身不在可计算范围内

### 2.2 Phase：连续确信度

`Phase` 是 `[0.0, 1.0]` 范围的连续值，提供决策的渐变确信维度：

- `0.5`：完全中性
- `> 0.5`：倾向于 True 的方向性
- `< 0.5`：倾向于 False 的方向性
- `0.0` / `1.0`：完全确信的端点

Phase 与 TritValue 的关系：TritValue 是离散状态，Phase 是该状态的确信强度。例如 `TritWord { value: True, phase: 0.9 }` 表示"系统倾向于 True，且确信度很高"。

### 2.3 Frame：决策域

每个 TritWord 属于一个 Frame（决策上下文域）：

| Frame | 含义 | 示例 |
|-------|------|------|
| `Science` | 经验/证据基础 | 物理测量、实验数据 |
| `Individual` | 个人事实/价值 | 患者意愿、用户偏好 |
| `Consensus` | 统计/群体偏好 | 多数意见、规范标准 |
| `Absolute` | 不可观测/不可知 | 终极实在性问题 |
| `Meta` | 冲突裁决/策略输出 | 系统自身决策 |

同 Frame 内操作走**热路径**（无冲突开销），跨 Frame 操作自动触发 **MetaInterrupt**（冲突检测事件）。

### 2.4 Domain：应用领域策略

Domain 定义了仲裁策略——同一组信号在不同 Domain 中产生不同结果：

| Domain | 策略 | 典型场景 |
|--------|------|----------|
| `Physical` | Science 优先，强制收敛 | 起重机负荷计算 |
| `Engineering` | Science 优先，强制收敛 + SafeFallback | 桥梁安全评估 |
| `MedicalEthics` | Individual 优先，不强制 | 患者知情同意 |
| `ValueJudgment` | 永远 Hold | 职业/价值观选择 |
| `General` | 单帧提交，多帧协商 | 通用决策 |
| `Custom(name)` | 外部规则加载 | 化学、基因等专业域 |

---

## 3. 架构设计

### 3.1 分层架构

```
┌──────────────────────────────────────┐
│          bin/sandbox.rs              │  ← CLI 场景运行器
│          bin/node.rs                 │  ← 分布式节点（存根）
├──────────────────────────────────────┤
│  src/meta/    策略引擎                │  ← Domain, ResolutionPolicy,
│               安全降级                │     SafeFallback, MetaMonitor
├──────────────────────────────────────┤
│  src/net/     分布式协议（存根）       │  ← Node, PLL, Bus, Message
├──────────────────────────────────────┤
│  src/trit/    核心三值代数（冻结）     │  ← TritValue, Phase, TritWord,
│                                         TernaryAlgebra (HTA)
├──────────────────────────────────────┤
│  src/frame/   决策域定义               │  ← Frame 枚举 + 注册
│  src/clock/   相位振荡器               │  ← HarmonicClock
│  src/baseline/ 二元基线对比            │  ← BinaryBaseline
└──────────────────────────────────────┘
```

### 3.2 模块依赖图

```
bin/sandbox ──→ meta ──→ trit ──→ frame
    │            │         │
    └────────────┴─────────┘
                 │
            net/ ──→ trit ──→ frame
                       │
                  clock/
```

`trit/` 模块是整个系统的唯一基础层，所有上层模块依赖它，它不依赖任何上层模块。

---

## 4. 模块详解

### 4.1 `trit/value.rs` — TritValue 实现

```rust
pub enum TritValue { True, Hold, False, Unknown }
```

关键实现：

- **分支消除**：`disc()` 方法返回 0-3 的内部分类值，被 LLVM 优化为单寄存器加载
- **LUT 查找表**：
  - `NEGATE_LUT: [False, Hold, True, Unknown]` — 取反操作 O(1)，无分支
  - `TO_I8_LUT: [1, 0, -1, 0]` — 数值转换 O(1)，无分支
- **`From<i8>` 不产生 Unknown**：Unknown 只能显式构造，防止污染
- **`is_computable()`**：仅 Unknown 返回 false

### 4.2 `trit/phase.rs` — Phase 实现

```rust
pub struct Phase(f64);  // 内部值，公开 API 保证 [0.0, 1.0]
```

关键机制：

**相位量化（quantize）**：
```rust
pub fn quantize(self, epsilon: f64) -> Phase {
    // 优先级：0.5 → 0.0 → 1.0
    // 将 epsilon 范围内的相位吸附到锚点值
}
```

这解决了长级联（cascade）中的浮点累积误差问题。例如，100 次级联后 0.5000000001 与 0.4999999999 在语义上都是"中性"，但原始浮点比较会产生差异。

**自动量化**：`mean()` 和 `complement()` 内部自动调用 `quantize(1e-6)`。

**NaN/Inf 防护**：`Phase::new()` 将非法值 clamp 到 0.5，并通过 tracing 发出警告。

### 4.3 `trit/algebra.rs` — TernaryAlgebra (HTA)

谐波三值代数（Harmonic Ternary Algebra）是所有计算的引擎。

**热/冷路径分离**：
```
precheck_same_frame() → true  → t_and_hot() / t_or_hot()  ← 热路径，零分配
                      → false → t_and() / t_or()          ← 冷路径，生成 MetaInterrupt
```

**真值表（TAND，含 Unknown）**：

| TAND | True | Hold | False | Unknown |
|------|------|------|-------|---------|
| True | True | Hold | False | Unknown |
| Hold | Hold | Hold | False | Unknown |
| False | False | False | False | Unknown |
| Unknown | Unknown | Unknown | Unknown | Unknown |

**真值表（TOR，含 Unknown）**：

| TOR | True | Hold | False | Unknown |
|-----|------|------|-------|---------|
| True | True | True | True | True |
| Hold | True | Hold | Hold | Hold |
| False | True | Hold | False | Hold |
| Unknown | True | Hold | Hold | Unknown |

Unknown 传播语义：TAND 中 Unknown 无条件传播（无法确认的安全风险应当传播），TOR 中 True 支配 Unknown（已知为真的信号不因未知信号而变弱）。

**TNOT**：值取反（通过 NEGATE_LUT），相位互补（1.0 - phase）。

**跨帧冲突处理**：当 `a.frame != b.frame` 时，返回 `(Hold, Some(MetaInterrupt))`——不强制收敛，而是标记冲突并通过 tracing 门控日志记录。

### 4.4 `meta/mod.rs` — 策略引擎

#### FrameMask

```rust
struct FrameMask(u8);
// Science=bit0, Individual=bit1, Consensus=bit2, Absolute=bit3, Meta=bit4
```

使用 u8 位掩码实现 O(1) 帧存在性检查。`from_inputs()` 构建时若所有位已设置（0b11111）则提前退出。

#### ResolutionPolicy::arbitrate()

仲裁逻辑按 Domain 分类：

| Domain | Science 存在 | Individual 存在 | 其他情况 |
|--------|-------------|----------------|---------|
| Physical/Engineering | Commit(Science) | — | ForceCollapse |
| MedicalEthics | — | Preserve(Individual) | Negotiate |
| ValueJudgment | — | — | Hold（永远） |
| General | — | — | 单帧→Commit, 多帧→Negotiate |
| Custom | — | — | Negotiate（默认） |

#### SafeFallback

```rust
pub fn guard(&self, domain: &Domain, result: &TritWord, interrupt_count: usize)
    -> (TritWord, Option<MetaInterrupt>)
```

逻辑：
1. 如果 domain 不是危险域 → 直接通过
2. 如果 result 为 Hold 或 Unknown，且中断计数 > 0 → 强制 False
3. 否则 → 通过

**设计原理（IEC 61508）**：在物理/工程安全领域，"无法确认安全"必须默认为"不安全"——系统宁可拒绝行动，也不可乐观确认。

内置危险域：`Physical`, `Engineering` 始终危险。默认注册：chemistry, genetics, structural, nuclear, pharmaceutical。

#### RuleLoader 特质

```rust
pub trait RuleLoader {
    type Error: std::fmt::Display;
    fn load<P: AsRef<Path>>(path: P) -> Result<CustomRule, Self::Error>;
    fn load_json(json: &str) -> Result<CustomRule, Self::Error>;
    fn apply(rule: &CustomRule, inputs: &[TritWord]) -> ArbitrationResult;
}
```

支持从外部 JSON 文件加载自定义仲裁规则，优先级帧 + fallback 行为的声明式配置。

### 4.5 `clock.rs` — HarmonicClock

正弦振荡器，用于时域管理：

- `physical()`：ω=10.0 快速采样（物理/工程域）
- `deliberative()`：ω=0.5 慢速采样（伦理/价值域）

`tick(dt)` 检测上升过零点，用于同步信号触发。

### 4.6 `net/` — 分布式协议（存根）

为未来 M2+ 分布式部署预定义的协议骨架：

- **Node**：状态机 Sovereign→Coupling→Coupled→Hold
- **PllController**：软件锁相环，比例校正 + 死区（0.05）+ 最大步长限制（0.1）
- **ResonanceBus**：内存中的消息总线，VecDeque 环形缓冲（MAX 10,000 条）
- **Message**：6 种操作码（RESONATE_REQ/ACK, DECOUPLE_REQ/ACK, NEGOTIATE, HEARTBEAT）

`negotiate()` 使用单次遍历（phase_sum 累积 + cross_frame 检测），避免多次遍历。

### 4.7 `baseline/mod.rs` — 二元基线对比

`BinaryBaseline` 实现了简单的多数投票逻辑（不含 Hold 状态），用于证明二元系统在处理跨帧冲突时的结构性缺陷：

```rust
// Binary majority: 1 True, 1 False, 0 Hold → False (tie defaults to False)
// Trit-Core: Science(True) vs Individual(False) → Hold + MetaInterrupt
```

### 4.8 `bin/sandbox.rs` — CLI 场景运行器

完整管道：
```
JSON 场景 → 路径验证(CWE-22防护) → 大小检查(64KB) → 字段验证 →
TritWord[] → TAND 级联 → ResolutionPolicy::arbitrate() →
SafeFallback::guard() → SandboxOutput JSON
```

安全措施：
- 路径遍历防护（canonicalize + starts_with）
- JSON 大小限制 64KB
- 信号数量限制 100
- 字符串长度限制 1024
- 日志字段消毒（控制字符替换为 U+FFFD）

---

## 5. 数据流与管道

### 5.1 完整决策管道

```
┌──────────────┐
│ JSON Scenario │  (输入：id, domain, signals[], expected_behavior)
└──────┬───────┘
       ▼
┌──────────────┐
│ 验证层        │  CWE-22 路径遍历 | 64KB 限制 | 100 信号限制 | 帧/值/相位验证
└──────┬───────┘
       ▼
┌──────────────┐
│ 信号→TritWord │  SignalInput.frame→Frame, value→TritValue, phase→Phase
└──────┬───────┘
       ▼
┌──────────────┐
│ TAND 级联     │  从左到右折叠：t_and(current, next[i])
│               │  同帧→热路径 | 跨帧→Hold + MetaInterrupt
└──────┬───────┘
       ▼
┌──────────────┐
│ FrameMask     │  O(1) 帧存在性检查
└──────┬───────┘
       ▼
┌──────────────┐
│ Policy        │  Domain→arbitrate()→Commit|Preserve|Hold|Negotiate|ForceCollapse
│ Arbitration   │
└──────┬───────┘
       ▼
┌──────────────┐
│ SafeFallback  │  Physical/Engineering + Hold/Unknown + interrupts → False
│ .guard()      │
└──────┬───────┘
       ▼
┌──────────────┐
│ SandboxOutput │  JSON { final_value, final_value_code, final_frame, interrupts, ... }
└──────────────┘
```

### 5.2 热/冷路径示意

```
调用方
  │
  ├─ precheck_same_frame(a, b) == true
  │    │
  │    └─ t_and_hot() ─── 3-5 ns ─── TritWord (无中断)
  │         • 帧枚举比较 ×1
  │         • 值匹配 ×1
  │         • Phase::mean() + quantize()
  │         • TritWord 构造
  │
  └─ precheck_same_frame(a, b) == false
       │
       └─ t_and() ─── ~95 ns ─── (TritWord, Option<MetaInterrupt>)
            • tracing span 进入
            • 帧比较（再次）
            • MetaInterrupt::with_frames() 构造
            • tracing::enabled! 门控日志
            • tracing span 退出
```

---

## 6. 安全机制

### 6.1 SafeFallback

最核心的安全机制。设计依据 IEC 61508（功能安全）原则：

> 在安全关键系统中，无法确认"安全"的状态，必须默认为"不安全"。

触发条件（全部满足）：
1. 域为 Physical, Engineering, 或已注册的 Custom 危险域
2. 决策结果为 Hold 或 Unknown
3. 存在至少 1 个中断事件（证明有实质冲突）

触发效果：
- `final_value` 强制设为 False
- 产生 `OutOfScope` 类型 MetaInterrupt，包含域名称和中断计数
- 通过 tracing::warn 记录

### 6.2 MetaMonitor 不变量

```rust
// Absolute 帧必须保持 Hold
pub fn inspect(&self, word: &TritWord) -> Option<MetaInterrupt> {
    if word.frame == Frame::Absolute && word.value != TritValue::Hold {
        // 产生 PolicyViolation 中断
    }
}
```

"绝对不可知"的事物不能被判定为 True 或 False，这是逻辑一致性不变量。

### 6.3 Unknown 传播

TAND 中 Unknown 无条件传播——系统将"不可知"信号视为污染源，阻止任何肯定裁决。

### 6.4 输入消毒

`sanitize_log_field()`：将控制字符替换为 Unicode 替换字符（U+FFFD），截断至 256 字符。防止日志注入。

---

## 7. 性能工程

### 7.1 关键优化

| 优化项 | 技术 | 效果 |
|--------|------|------|
| 分支消除 | LUT 替换 match（negate, to_i8） | 消除分支预测失败 |
| 热/冷路径分离 | precheck_same_frame → hot/cold dispatch | 热路径 ~3ns，跳过了 MetaMonitor + tracing |
| FrameMask | u8 位掩码替代 Vec<Frame> 遍历 | O(1) 帧检查 |
| 单次遍历 negotiate | phase_sum + cross_frame 合并 | 减少 2/3 遍历 |
| VecDeque 环形缓冲 | 替换 Vec::remove(0) | push/pop O(1) vs O(n) |
| MetaInterrupt 预分配 | String::with_capacity(48) 替代 format!() | 避免堆重分配 |
| tracing 门控 | tracing::enabled! 守卫冷路径日志 | 冷路径延迟降低 71% |

### 7.2 基准测试覆盖

5 个 Criterion 基准组：
- **core_ops**：tand_cross_frame, tand_same_frame, tor_cross_frame, tor_same_frame, tnot
- **hot_path**：t_and_hot, t_or_hot, precheck_same_frame
- **cascades**：cascade_10, cascade_10_hot, cascade_100_hot（测量相位漂移）
- **cross_domain**：100 对跨域 TAND，hot vs cold 对比
- **phase_precision**：quantize 四种路径（near_neutral, near_zero, near_one, noop）

---

## 8. 测试策略

### 8.1 测试金字塔

| 层级 | 数量 | 位置 |
|------|------|------|
| 单元测试 | 96 | 各模块内联 `#[cfg(test)] mod tests` |
| 集成测试 | 18 | `tests/integration_test.rs`：trit(4) + meta(2) + scenario(12) |
| 场景文件 | 17 | `scenarios/*.json`：通过 `trit-sandbox` CLI 验证 |
| 文档测试 | 0 |（当前无 doc-test） |

### 8.2 场景覆盖矩阵

| Domain | Total | Hold | Commit False | Commit True | Other |
|--------|-------|------|-------------|-------------|-------|
| Physical | 2 | 0 | 2 | 0 | — |
| Engineering | 2 | 0 | 2 | 0 | — |
| MedicalEthics | 3 | 0 | 2 | 1 | — |
| ValueJudgment | 3 | 3 | 0 | 0 | — |
| General | 2 | 0 | 0 | 2 | — |

### 8.3 测试覆盖的关键路径

- `TritValue` 所有 4 个状态 + 所有 4 个 NEGATE_LUT 条目 + From<i8> 全范围
- `Phase` quantize 4 路径 + mean/complement 自动量化 + NaN/Inf 防御
- `TernaryAlgebra` TAND/TOR 真值表（含 Unknown）+ 热/冷路径 + precheck
- `ResolutionPolicy` 全部 6 种 Domain + Custom rule + fallback 全路径
- `SafeFallback` guard 全部触发条件 + Physical/Engineering 始终危险 + disabled 模式
- `Node` 完整状态机 + PLL correction clamp + decouple 恢复
- `Bus` 单帧/跨帧 resonance + decouple + 3 节点 negotiate

---

## 9. 工程规范

### 9.1 代码质量保障

```bash
cargo fmt -- --check    # 强制统一格式
cargo clippy -- -D warnings  # Clippy 警告即错误
#![forbid(unsafe_code)] # 全项目零 unsafe
#![deny(warnings)]      # 编译器警告即错误
```

### 9.2 构建配置

- Edition: 2021
- Release: `opt-level = 3, lto = true`（链接时优化）
- Bench: `opt-level = 3`
- 依赖最小化：6 个直接依赖（serde, serde_json, thiserror, chrono, uuid, tracing/tracing-subscriber）

### 9.3 可观测性

- `tracing-subscriber` + JSON/文本双模式
- 环境变量控制：`TRIT_LOG`（过滤级别），`TRIT_LOG_JSON`（格式切换）
- 关键操作标注 `#[tracing::instrument]`：自动生成 span 时长、输入参数、结果值
- JSON 模式包含：timestamp, level, target, filename, line_number, threadId, span 层级

### 9.4 声明与重导出

`lib.rs` 中集中声明所有公开 API，确保外部使用者仅需 `use trit_core::*`（或按需导入），不暴露内部模块结构。

---

## 10. 已知局限

| 局限 | 严重度 | 说明 |
|------|--------|------|
| 无形式化验证 | 高 | 核心真值表和仲裁逻辑仅靠测试覆盖，未经 Coq/Lean 验证——对于声称"AI 对齐"的系统，这是显著缺口 |
| 分布式协议为存根 | 高 | `net/` 模块定义了协议消息和节点模型，但无网络传输层——"分布式"完全在内存中模拟 |
| Phase 累积误差分析未完成 | 中 | quantize() 缓解但未根除，长级联（>10^6）的理论误差边界未计算（见 ADR-002） |
| Unknown 仲裁层覆盖不完整 | 中 | `ResolutionPolicy::arbitrate()` 不显式处理 Unknown，依赖 SafeFallback 兜底 |
| 无 Schema 验证层 | 低 | 场景 JSON 无 JSON Schema 定义，对 phase 值仅依赖 `Phase::new()` 的运行时 clamp |
| 性能上限未验证 | 低 | 目标 10,000 TPS（每秒三值操作）未在外部分布式环境中测试 |

---

## 11. 适用场景参考

### 11.1 适合使用三值决策系统的场景

- **安全关键的多信号融合**：物理/工程安全，医疗决策——当"无法确认安全即不安全"是正确原则时
- **跨价值体系冲突**：个人自主权 vs 集体利益 vs 科学证据——需要保留冲突而非强制解决
- **冲突审计要求**：需要证明"系统检测到了冲突且未忽略"的合规场景
- **多领域 AGI 对齐实验**：作为不同于 RLHF 平均化的替代对齐范式

### 11.2 不适合的场景

- **简单二值决策**：门禁开关、是/否审批——二元逻辑更高效
- **低延迟实时控制**：微秒级闭环控制——Trit-Core 的仲裁管道为微秒级，不适用于纳秒级场景
- **无需冲突感知的统计聚合**：推荐系统、评分融合——加权平均可能更合适

### 11.3 技术可迁移部分

以下工程模式可直接用于其他 Rust 项目：

1. **LUT 分支消除**：`const NEGATE_LUT: [T; N]` 替代 `match` for 小枚举
2. **位掩码加速**：`struct FrameMask(u8)` 模式——用于任何有限集合的成员检查
3. **热/冷路径分离**：`precheck_*() + *_hot()` 模式——用于有条件跳过昂贵操作的场景
4. **相位置化**：`quantize(epsilon)` 模式——用于任何涉及浮点累积的判定逻辑
5. **SafeFallback 模式**：`guard(domain, result, interrupt_count)` 模式——用于安全关键系统的多层验证

---

## 附录 A：依赖关系

```
trit-core v0.1.0
├── serde 1.0 (derive)
├── serde_json 1.0
├── thiserror 1.0
├── chrono 0.4 (serde)
├── uuid 1.0 (v4, serde)
├── tracing 0.1
├── tracing-subscriber 0.3 (json, env-filter)
│
[dev-dependencies]
├── assert_fs 1.0
├── predicates 3.0
└── criterion 0.5 (html_reports)
```

## 附录 B：模块文件清单

```
src/
├── lib.rs                  # 模块声明与公开 API 重导出
├── trit/
│   ├── mod.rs              # TritWord 定义
│   ├── value.rs            # TritValue 枚举 + LUT
│   ├── phase.rs            # Phase(f64) + quantize
│   └── algebra.rs          # TernaryAlgebra (HTA) + 热路径
├── frame/
│   └── mod.rs              # Frame 枚举 + FrameRegistry
├── meta/
│   └── mod.rs              # Domain, FrameMask, ResolutionPolicy,
│                           # MetaMonitor, RuleLoader, SafeFallback
├── clock.rs                # HarmonicClock 振荡器
├── sandbox.rs              # ScenarioInput / SandboxOutput DTO
├── baseline/
│   └── mod.rs              # BinaryBaseline 对比
├── net/
│   ├── mod.rs
│   ├── message.rs           # 协议消息定义
│   ├── node.rs              # Node 状态机
│   ├── pll.rs               # PLL 锁相环
│   └── bus.rs               # ResonanceBus 消息总线
├── tracing_init.rs          # tracing-subscriber 初始化
└── bin/
    ├── sandbox.rs           # trit-sandbox CLI
    └── node.rs              # trit-node CLI

tests/
└── integration_test.rs      # 18 集成测试

benches/
└── trit_bench.rs            # 5 组 Criterion 基准测试

scenarios/
└── *.json                   # 17 个 JSON 场景文件
```

## 附录 C：场景 ID 完整列表

```
bridge_safety, bridge_safety.zh
career_value_conflict, career_value_conflict.zh, career_value_conflict_02, career_value_conflict_03
medical_conflict_01, medical_conflict_01.zh, medical_conflict_02, medical_conflict_03
general_negotiation, general_negotiation.zh, general_negotiation_02
physical_crane_overload, physical_runway_length
engineering_bridge_retrofit, engineering_material_tradeoff
```

---

*本白皮书由系统自反性审计生成，所有描述均基于实际代码状态。*

*生成时间戳：2026-06-17 | 代码版本：0.1.0 (MVP)*
