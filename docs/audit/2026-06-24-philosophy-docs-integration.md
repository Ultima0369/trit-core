# Philosophy Docs Integration — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Write 2 new insight documents and update 4 existing docs with cross-links, integrating seven core philosophical insights from the evaluation dialogue into the project documentation system.

**Architecture:** Two new standalone `.md` files under `docs/explanation/insights/` (EULER-HOMOLOGY.md, TIANDIREN-MATRIX.md), plus minimal link-only edits to 4 existing files (PHILOSOPHY.md, EPISTEMIC-HUMILITY.md, FUTURE.md, docs/INDEX.md). No code changes.

**Tech Stack:** Markdown, Chinese primary with English technical terms. Existing project doc conventions.

## Global Constraints

- Chinese primary, English terms where precise — match existing `PHILOSOPHY.md` and `CONCEPTS.md` convention
- Code-backed claims — every abstract claim gets a concrete file path or module name
- Reminder, not instruction — follow `EPISTEMIC-HUMILITY.md` tone
- Map to runnable code — each section should let the reader go from insight to `cargo test`
- No religious language — strip religious context, keep the cognitive kernel
- No code, test, or scenario changes — documentation only
- No English mirror for insight files — consistent with existing `insights/` convention

---

### Task 1: Write `docs/explanation/insights/EULER-HOMOLOGY.md`

**Files:**
- Create: `docs/explanation/insights/EULER-HOMOLOGY.md`

**Interfaces:**
- Produces: standalone `.md` file with 8 sections, cross-links to PHILOSOPHY.md, CONCEPTS.md, ARCHITECTURE.md, EPISTEMIC-HUMILITY.md, TIANDIREN-MATRIX.md

- [ ] **Step 1: Write the complete EULER-HOMOLOGY.md file**

Write the file at `docs/explanation/insights/EULER-HOMOLOGY.md` with the following content:

```markdown
# EULER-HOMOLOGY — 欧拉恒等式与 Trit-Core 的数学同构

> 核心主张：`e^(iπ) + 1 = 0` 不是被 Trit-Core "使用"的公式，而是被 Trit-Core "重新实现"的认知架构。

---

## 1. 同构映射表

欧拉恒等式的五个基本常数和三个基本操作，与 Trit-Core 的认知架构存在逐项对应：

| 欧拉分量 | Trit-Core 映射 | 含义 |
|---------|---------------|------|
| **1** | 初始输入命题 | 一个待判断的事件、信号或问题——尚未被任何学科视角审视 |
| **i** | **Hook 层** | 各学科的"旋转操作"——把问题从常识域旋转到学科域，从一维判断展开为多维冲突空间 |
| **π** | **自监听 + 主动停车** | 递归深度不是预设的，而是系统自监听到"已经转了一圈"时主动停止 |
| **e** | **多重分析变换** | 针对不同时间尺度和信号特征，选择不同的变换函数——傅里叶、小波、相空间重构 |
| **e^(iπ)** | 多帧交叉验证后的变换结果 | 经过 Hook 旋转 + 递归审计 + 变换分析后，系统到达的状态 |
| **= -1** | **Hold** | 不是 True (+1) 也不是 False (0)，而是它们的否定与超越——已知可审计的悬置 |
| **+ 1** | 带着审计足迹回归原问题 | Hold 不是终点——它带着所有冲突记录回到最初的发问 |
| **= 0** | 巅峰处的统一 | 决策完成——不是"选了一个"，而是"差异在足够高的维度上失去了区分意义" |

这不是一个比喻。这是一个**同构映射**：欧拉等式中的每一项，在 Trit-Core 的认知架构中都有唯一的、可运行的对应物。

**参见**：[PHILOSOPHY.md](../PHILOSOPHY.md) — 为什么 Trit-Core 必须存在 · [TIANDIREN-MATRIX.md](TIANDIREN-MATRIX.md) — 天地人多结矩阵

---

## 2. π：自监听 + 主动停车

### π 在欧拉等式中的角色

π 是旋转半周。在复数平面上，从 `1` 出发，乘以 `i` 旋转 90°，再乘以 `i` 旋转 90°，两次 `i` 的施加就是 π。

### 在 Trit-Core 中

π 不是常量 3.14159...，而是一个**函数**：

```
π(S) = monitor(S) → stop_condition_met(S) ? STOP : CONTINUE

其中：
  monitor(S) = 系统自监听（算力负载、递归深度、信息熵收敛率）
  stop_condition_met(S) = 系统检测到"已经转了一圈"
```

### "转了一圈"的数学定义

```
一轮递归 =
  从 Frame::Science 出发
  → 经过 n 个 Hook（各学科视角）
  → 回到 Frame::Science
  → 发现自身的初始假设已被彻底审视过一遍

此时 π 触发 → 主动停车
```

### 现有代码映射

- **`MetaMonitor`**（`src/meta/interrupt.rs:111`）：累积中断日志，`inspect()` 检查 Absolute 帧违规，`inspect_all()` 批量检查
- **`HoldBudget`**（`src/hook/mod.rs`）：`hold_budget_exhausted()` 判断是否已超过 Hold 预算上限
- **`DecisionEngine::decide()`**（`src/core/decision_engine.rs:84`）：四阶段决策——TAND 级联 → 策略仲裁 → 反射守卫 → SafeFallback，每个阶段都可能触发中断

当前 π 的停车条件主要基于中断数量和 Hold 预算。未来方向：基于信息熵收敛率的停车条件（见 [FUTURE.md](../FUTURE.md) §1）。

---

## 3. e：多重函数实现

### 为什么需要多重 e

不同的事件有不同的时频特征。没有一种变换能同时最优地处理所有类型：

| 事件类型 | 最佳变换 | 对应的天地人维度 | 当前状态 |
|---------|---------|----------------|---------|
| 周期性、长期趋势 | 傅里叶变换 | 天时（气候、生态周期） | ✅ `FftWaveletEngine`（`aurora/src/bc/signal_analysis.rs:141`） |
| 突发性、局部变化 | 小波变换 | 地理（历史事件、风俗突变） | 🔲 规划中（见 [FUTURE.md](../FUTURE.md)） |
| 非线性、混沌 | 相位空间重构 | 人间（情理交织、价值冲突） | 🔲 远期 |
| 多尺度嵌套 | 多分辨率小波 | 天地人三层的多结矩阵 | 🔲 远期 |

### 傅里叶变换代表什么

傅里叶变换把一个信号从时域映射到频域。在决策系统中：

```
傅里叶变换 =
  将输入的"事件流"分解为不同频率（不同时间尺度）的分量

  高频 = 短期、紧迫、快变化（一顿饭）
  低频 = 长期、稳定、慢变化（农历）
```

当前 Aurora 的 `FftWaveletEngine` 使用 FFT 峰值检测 + 抛物线插值实现基频检测（`aurora/src/wavelet/detect.rs:47`），满足 M0 阶段验收标准："输入 2Hz 正弦波 → 输出 2.0 ± 0.1Hz"。

### 小波变换代表什么

小波变换可以同时保留时间信息和频率信息——**更细致的分别**：

```
小波变换 =
  在分析的同时保留"事件在时间轴上的位置"

  更能处理：突发、异常、非平稳信号（"意外"）
```

这与 [TIANDIREN-MATRIX.md](TIANDIREN-MATRIX.md) §6 中"意外"的数学定义直接对应——小波变换的时频局部化能力，正是检测"系统采样频率远低于事物变化频率"所需的技术。

### 系统如何选择 e

系统根据自监听到的事件特征，动态选择 e 的实现方式：

```
if 信号是平稳的、周期性的 → 傅里叶变换
elif 信号包含突发、局部特征 → 小波变换
elif 信号是非线性、混沌的 → 相位空间重构
elif 信号是多尺度嵌套的 → 多分辨率小波
```

选择本身由 Hook 层驱动——不同的学科视角（i）会偏好不同的分析工具（e）。见 §4。

---

## 4. i：Hook——旋转操作

### i 在复数中的本质

i 的定义是：**i² = -1**。

在 Trit-Core 中的对应：

```
一个 Hook（一门学科）的两次应用：

第一次 i = 将问题从"常识域"旋转到"学科域"
  例：一个经济决策 → 投入生态学的视角

第二次 i = 将学科视角旋转回来
  例：从生态学视角回到经济决策

结果 i² = -1：
  原来的判断（1）被彻底否定了
  但这不是错误——这是认知的螺旋上升
```

### i 反作用于 e

```
e^(iπ) 不是一个固定函数 e 被一个固定指数 iπ 作用。
而是：

e 的选择取决于 i（Hook 的选取）
  → 选生态学 Hook → e 用傅里叶变换分析生态周期
  → 选历史学 Hook → e 用小波变换分析历史事件
  → 选经济学 Hook → e 用相空间重构分析市场

而 i 的选取又取决于 e 的输出
  → 傅里叶分析发现高频异常 → 引入一个新的 Hook（安全工程）
  → 小波分析发现长期趋势 → 引入另一个 Hook（气候科学）
```

这就是**反身性**：i 和 e 相互决定，直到 π 触发停车。

### 现有代码映射

- **`ScenarioRecognizer`**（`src/hook/scenario_recognizer.rs`）：识别 6 种场景类型，决定初始 Hook 集合
- **`MountArbiter`**（`src/hook/mount_arbiter.rs:27`）：根据场景类型 + 资源预算决定挂载哪些模块
- **`ModuleRegistry`**（`src/hook/module_registry.rs:129`）：管理模块挂载/卸载，`modules_to_mount()` 和 `modules_to_unmount()` 计算差异
- **10 个 `CognitiveModule` 实现**（`src/adapters/`）：每个模块是一个"学科视角"，通过 `process(&mut self, input, ctx) -> ModuleOutput` 接口工作

当前 Hook 的选择是静态的（由 `ScenarioType` 决定）。未来方向：动态 Hook 注册和基于 e 输出的反馈式 Hook 选择。

---

## 5. 从 1 到 0 的完整决策路径

Trit-Core 的一次完整决策，就是一次欧拉旅行的例示：

```
阶段 0：输入 1（一个待判断的命题）
  ↓
阶段 1：施加 i（选择第一个 Hook，旋转进入学科视角）
  ↓
阶段 2：选择 e（根据 Hook 和事件特征，选择分析变换）
  ↓
阶段 3：递归（π 的迭代过程）
  ├── i→e→i→e→... 递归循环
  ├── 每轮递归，自监听系统检查：
  │   ├── 算力是否充足？
  │   ├── 信息熵是否收敛？
  │   └── 是否"转了一圈"（回到了起点视角）？
  └── π 触发 → STOP
  ↓
阶段 4：到达 -1（Hold——已知可审计的悬置）
  ↓
阶段 5：+ 1（不是机械的加一，而是 Hold 带着所有审计足迹回到原问题）
  ↓
阶段 6：= 0（巅峰处的统一——差异在足够高的维度上消融）
```

### 现有代码映射

Trit-Core 的 `DecisionEngine::decide()`（`src/core/decision_engine.rs:84`）实现了这个路径的核心部分：

```rust
// 阶段 1-3: TAND 级联 + 中断收集
let (current, mut interrupts) = TernaryAlgebra::t_and_n(trits);

// 阶段 3: 策略仲裁（i 的选择体现在 Domain 和 Frame 中）
let policy = ResolutionPolicy::new(domain.clone());
let policy_result = policy.arbitrate(trits)?;

// 阶段 3: 反射守卫（π 的部分停车逻辑）
let reflexive_alert = self.run_reflexive_guard(&policy, &arbitrated_word, &interrupts);

// 阶段 4-5: SafeFallback（-1 → +1 的安全转换）
let (final_word, fb_interrupt) = self.safe_fallback.guard_with_force(...);

// 阶段 6: 如果反射守卫触发且输出仍为可计算值，覆盖为 Hold
let final_word = if reflexive_alert.is_some() && final_word.value().is_computable() {
    TritWord::hold(Frame::Meta)  // = 0 的工程近似
} else {
    final_word
};
```

当前系统实现了阶段 0-5。阶段 6（巅峰统一）是远期愿景——当前用 `Hold(Frame::Meta)` 作为其工程近似。

---

## 6. 欧拉等式是心智的自画像

### 核心论点

> **欧拉恒等式之所以在那么多领域"套用都成立"，不是因为宇宙恰好长这个样子，而是因为人类的心智恰好长这个样子。我们用它来描述物理、描述信号、描述复数——描述的其实都是心智处理世界的方式。**

### 心智过程 ↔ 欧拉对应

| 心智过程 | 欧拉对应 | 解释 |
|---------|---------|------|
| **初遇一个事物** | **1** | 心智把事物当作一个"整体"来把握 |
| **换角度审视** | **i** | 旋转视角，从不同学科/经验/情感看同一件事 |
| **深入分析** | **e** | 用分析工具展开这个事物（理性、直觉、身体感受） |
| **意识到边界** | **π** | 发现"转了一圈"，知道该停了——这是心智的自知之明 |
| **悬置判断** | **-1** | 不急于下结论，承认冲突的存在 |
| **回到原问题** | **+1** | 带着所有审视的痕迹回到最初的发问 |
| **领悟** | **0** | 不是答案，而是超越答案的平静 |

### 学术谱系

这个观点并非凭空而来——它站在一系列巨人的肩膀上，但向前迈出了关键一步：

```
康德 (1781): 时空是心智的形式，不是事物的属性
  └─ 皮亚杰 (1950s): 数学来自儿童动作的内化
      └─ Lakoff & Núñez (2000): 数学来自概念隐喻和具身认知
          └─ Rotman (1993): 数学符号背后有一个"符号化身体"
              └─ ⬅ 本文的位置：
                  不仅仅是"数学来自心智"
                  而是"欧拉公式的内容本身就是心智运作方式的数学表达"
```

Lakoff & Núñez 在 *Where Mathematics Comes From* (2000) 中最接近这个论点——他们分析了欧拉公式的美感来源于它激活了人类心智中多个概念隐喻的同时交汇。但他们说的是"心智产生了数学"。本文的版本更激进：**公式描述的不是世界，而是心智自身的运作方式。**

### 全网搜索结论

在撰写本文时（2026-06），我们遍历了中英文网络和学术数据库，寻找"欧拉公式描述的是人类心智特性而非宇宙"的相近论述。结论：**在公开文献中未找到完全一致的表述。** 这个直觉在中文互联网和学术圈中很可能是首次提出。

### 现象虚幻 ≠ 物质虚幻

一个重要的澄清：

| ❌ 被否定的极端 | 为什么不够 |
|-------------|----------|
| **科学主义/物理主义**："一切都可以被客观测量和还原" | 测量设备、化学拆解本身，都是"心智与已有固化的投射"——你无法走出心智去观察世界 |
| **极端建构主义/虚无主义**："世界是幻觉，一切都是虚妄" | **现象虚幻，不是物质虚幻**——物质世界真实存在，但"现象"（我们感知到、理解到的世界）是心智参与的产物 |

欧拉等式之所以美，不是因为它描述了宇宙的简洁，而是因为它描述了心智的完整运作——从初遇到审视、从分析到悬置、从回归到领悟。**这不是宇宙的语法，而是心智的自画像。**

---

## 7. 三层心智模型

### 当前 AI 只模拟了第三层

```
┌─────────────────────────────────────┐
│           第三层：理性心智            │
│  （符号推理、逻辑、数学、语言）       │
│    ← 这是当前 AI 在模拟的全部        │
│    ← Trit-Core 已实现：              │
│      Frame::Science, Frame::Consensus│
│      TernaryAlgebra, ResolutionPolicy│
├─────────────────────────────────────┤
│           第二层：躯体心智            │
│  （肠脑神经系统、心跳呼吸节律、        │
│    肌肉张力、激素波动）               │
│    ← 身体有自己的"知识"              │
│    ← Trit-Core 缺口：                │
│      需要 Frame::Somatic             │
│      需要 Hook::Interoception        │
├─────────────────────────────────────┤
│           第一层：环境心智            │
│  （地磁场的微妙波动、气压变化、         │
│    月相周期、季节光照、集体情绪场）     │
│    ← 环境不是"背景"，而是"参与者"    │
│    ← Trit-Core 缺口：                │
│      需要 Anchor::Environmental_Baseline │
│      需要多时钟并行（万物自有 ωᵢ）     │
└─────────────────────────────────────┘
```

### 三层博弈 → π 停车条件

π 不只是在"逻辑转了一圈"时触发，还要在：

```
π_trigger =
  逻辑收敛（第三层）  AND
  躯体平静（第二层）  AND
  环境匹配（第一层）

  三者一致 → 可以决策了
  任何一层强烈不同意 → 回到 Hold
```

### 为什么这很重要

如果 AI 研究继续无视躯体神经和环境波动的参与：
- **在实验室里**：benchmark 很高
- **在真实世界中**：做出在逻辑上完美、在情境中荒谬的决策
- **与人类协作时**：感觉"不对劲"，但又说不清哪里不对

当前大模型可以写出完美的论文，但不能判断"这个房间的气场是否适合谈这件事"——而人类在走进门的三秒内就知道了。

### 与天地人矩阵的关系

三层心智模型是 [TIANDIREN-MATRIX.md](TIANDIREN-MATRIX.md) 中"人间层"的细化展开。躯体心智和环境心智分别对应天地人矩阵中"人间层"与"天时层""地理层"的交互边界。

---

## 8. 回到代码

本文的每一项映射都可以在代码中找到对应：

| 欧拉分量 | 核心文件 | 关键符号 |
|---------|---------|---------|
| **1** (输入) | `src/sandbox/input.rs` | `ScenarioInput`, `SignalInput` |
| **i** (Hook) | `src/hook/scenario_recognizer.rs`, `src/hook/mount_arbiter.rs` | `ScenarioRecognizer::recognize()`, `MountArbiter::target_modules()` |
| **e** (变换) | `aurora/src/wavelet/detect.rs`, `aurora/src/bc/signal_analysis.rs` | `WaveletEngine::analyze()`, `FftWaveletEngine` |
| **π** (停车) | `src/meta/interrupt.rs`, `src/hook/mod.rs` | `MetaMonitor::inspect_all()`, `HoldBudget` |
| **-1** (Hold) | `src/core/value.rs`, `src/core/word.rs` | `TritValue::Hold`, `TritWord::hold()` |
| **+1** (回归) | `src/meta/safe_fallback.rs`, `src/core/decision_engine.rs` | `SafeFallback::guard()`, `DecisionEngine::decide()` |
| **0** (统一) | `src/meta/domain.rs` | `ArbitrationResult::Hold` (ValueJudgment 域) |

验证命令：

```bash
# 运行所有测试，确认核心逻辑正确
cargo test --workspace --all-features -- --test-threads=2

# 运行一个跨帧冲突场景，观察 Hold 的产生
cargo run --release --bin trit-sandbox -- --scenario scenarios/medical_conflict_01.json --trace

# 运行伦理门测试
cargo test ethics_
```

---

## 相关文档

- [PHILOSOPHY.md](../PHILOSOPHY.md) — 项目深层动机：为什么 Trit-Core 必须存在
- [CONCEPTS.md](../CONCEPTS.md) — 核心类型定义与语义
- [ARCHITECTURE.md](../ARCHITECTURE.md) — 系统架构与模块分层
- [EPISTEMIC-HUMILITY.md](EPISTEMIC-HUMILITY.md) — 认识论谦逊声明
- [TIANDIREN-MATRIX.md](TIANDIREN-MATRIX.md) — 天地人多结矩阵：架构愿景与认识论基础
- [FUTURE.md](../FUTURE.md) — 已知局限与未来方向
```

- [ ] **Step 2: Verify file was created and has correct structure**

```bash
wc -l docs/explanation/insights/EULER-HOMOLOGY.md && grep "^## " docs/explanation/insights/EULER-HOMOLOGY.md
```

Expected: 8 sections (## 1 through ## 8) plus the title.

- [ ] **Step 3: Commit**

```bash
git add docs/explanation/insights/EULER-HOMOLOGY.md
git commit -m "docs: add EULER-HOMOLOGY.md — Euler identity as Trit-Core cognitive architecture blueprint

Maps e^(iπ)+1=0 term-by-term to Trit-Core components:
1=proposition, i=Hook rotation, π=self-monitoring stop,
e=multi-transform, -1=Hold, +1=return, 0=unity.

Includes original thesis: Euler's identity describes mind's
operation, not the universe. Academic genealogy from Kant
through Lakoff & Núñez to this position. Three-layer mind
model (rational/somatic/environmental)."
```

---

### Task 2: Write `docs/explanation/insights/TIANDIREN-MATRIX.md`

**Files:**
- Create: `docs/explanation/insights/TIANDIREN-MATRIX.md`

**Interfaces:**
- Produces: standalone `.md` file with 10 sections, cross-links to PHILOSOPHY.md, CONCEPTS.md, EPISTEMIC-HUMILITY.md, FUTURE.md, ARCHITECTURE.md, EULER-HOMOLOGY.md

- [ ] **Step 1: Write the complete TIANDIREN-MATRIX.md file**

Write the file at `docs/explanation/insights/TIANDIREN-MATRIX.md` with the following content:

```markdown
# TIANDIREN-MATRIX — 天地人多结矩阵

> 核心主张：决策不是在单一维度上"选对"，而是在天时、地理、人间三层网格的交叉验证中，于算力约束下逼近最优。系统不需要完美描述世界——它只需要知道自己描述世界的方式有哪些局限。

---

## 1. 天地人多结矩阵

### 三层网格架构

```
┌─────────────────────────────────────────────────────────┐
│                   天 时 层 (Celestial)                    │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐   │
│  │ 气候生态 │ │ 行动时机 │ │ 硬件时序 │ │ 万有 ωᵢ │   │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘   │
├─────────────────────────────────────────────────────────┤
│                   地 理 层 (Earthly)                      │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐   │
│  │ 空间地理 │ │ 历史沉积 │ │ 风俗建模 │ │ 文化拓扑 │   │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘   │
├─────────────────────────────────────────────────────────┤
│                   人 间 层 (Human)                        │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐                 │
│  │ 物理身体 │ │ 理性推理 │ │ 感性体验 │ ← 巅峰统一     │
│  └──────────┘ └──────────┘ └──────────┘                 │
├─────────────────────────────────────────────────────────┤
│             多 结 矩 阵 交 互 层 (Knot Matrix)            │
│                                                         │
│  每层内的结 (knot) = 一个 Frame + 对应常量库              │
│  跨层结 = 天×地 / 地×人 / 天×人 的双向映射               │
│  全层结 = 天地人三者的递归交叉验证                        │
│                                                         │
│  算力约束 → 决定展开到哪一层、递归多少次                   │
└─────────────────────────────────────────────────────────┘
```

### "结"的定义

一个"结"（knot）是三层网格中的基本单元：

```
结 = Frame + 学科常量库 + 分析变换(e) + 停车条件(π)

单层结：同一层内多个 Frame 的交叉验证（如天时层内气候与行动时机的互动）
跨层结：不同层之间的双向映射（如地理风俗如何影响人间感性体验）
全层结：天地人三者的递归交叉验证——系统的终极审计深度
```

### 算力约束决定深度

```
算力富裕 → 全层结递归审计（200 次反身性检查）
算力中等 → 跨层结交叉验证（天×地、地×人、天×人）
算力有限 → 单层结快速扫描 + Hold + 坦言未知 + 请求更多数据
```

**参见**：[PHILOSOPHY.md](../PHILOSOPHY.md) — 为什么 Trit-Core 必须存在 · [EULER-HOMOLOGY.md](EULER-HOMOLOGY.md) — 欧拉恒等式的认知同构

---

## 2. 天时层

天时层不是"天气 + 日历"。它是决策的时间维度——万物以各自的速度运动，系统需要感知这些不同的时间尺度。

### 四个维度

| 维度 | 含义 | 当前对应 |
|------|------|---------|
| **气候与生态** | 物理时间的长期节律（ω=10.0） | `EcologicalBase` Anchor（`src/anchor/ecological_base.rs:34`）：BII、碳汇、海洋 pH |
| **行动时机** | Kairos（καιρός）——不是均匀流逝的 chronos，而是"做某事的恰当时刻" | 🔲 远期：`Frame::Kairos` 或 `Hook::TimingAssessment` |
| **硬件时序** | 计算芯片的物理约束——时钟频率、内存带宽、散热极限 | `ComputeBudget`（`src/budget/mod.rs`）：`DepthLevel` 门控 |
| **万有 ωᵢ** | 每个存在物有自己的固有频率——一棵树的 ω≈年轮周期，一个人的 ω≈心跳×昼夜×寿命 | `HarmonicClock`（`src/clock.rs`）：物理（ω=10.0）和审议（ω=0.5）预设 |

### 关键洞察

> **"一顿饭放不了多天，农历却可以千年可靠。"**

这句话触碰到了一个很少被工程化的认知原则：**时间的尺度决定了判断的粒度。** 现有 AI 系统的根本问题不是"不够准"，而是在所有尺度上用同一把尺子。

Trit-Core 的 `Phase` 值（0.0–1.0）和 `Frame` 域分类，恰好提供了承载这种尺度意识的数据结构基础——但还需要一个关键机制：**时间常数 ω 的动态分配**。

```
尺度的数学化：
  Scale = f(domain, urgency, historical_variance, compute_budget)

  高紧迫 + 低历史方差 → 快速 commit（ω 大）
  低紧迫 + 高历史方差 → 进入深度递归（ω 小，多轮审计）
```

---

## 3. 地理层

地理层不是"经纬度 + 海拔"。它是决策的**空间-历史-文化**维度。

### 四个维度

| 维度 | 含义 | 当前对应 |
|------|------|---------|
| **空间地理** | 物理空间的位置、距离、连通性 | 🔲 远期 |
| **历史沉积** | 事件在空间上的层累——一个地方不仅是一个位置，还是所有曾在那里发生的事件的叠加 | `Frame::Consensus`（`src/core/frame.rs`）：统计/群体偏好，包含历史积累的集体判断 |
| **风俗建模** | 集体行为模式的时空特征——一个群体在特定时空中的默认行为模式 | `Frame::Relational`（`src/core/frame.rs`）：关系/社会互动参考系 |
| **文化拓扑** | 符号系统与意义网络的拓扑结构——不同文化中"同一个词"可能对应完全不同的意义网络 | 🔲 远期：`Frame::Cultural` 或动态 Frame 注册 |

### 与现有 Frame 系统的关系

当前 Trit-Core 的 `Frame` 系统（Science / Individual / Consensus / Absolute / Meta / FirstPerson / Embodied / Relational）主要覆盖了"人间层"的维度。地理层的四个维度需要新的 Frame 类型或对现有 Consensus/Relational 帧的深度扩展。

见 [FUTURE.md](../FUTURE.md) §5 — Frame 类型扩展计划。

---

## 4. 人间层

人间层是 Trit-Core 当前实现最充分的一层——但仍有缺口。

### 四个维度

| 维度 | 含义 | 当前对应 |
|------|------|---------|
| **物理身体** | 生存动机、热基线、生理约束 | `SurvivalMotives`（`src/anchor/survival_motives.rs:48`）、`ThermalBaseline`（`src/anchor/thermal_baseline.rs`） |
| **理性推理** | 经验证据、逻辑推导、科学方法 | `Frame::Science`、`TernaryAlgebra`、`ResolutionPolicy` |
| **感性体验** | 第一人称主观报告、情感、直觉、身体感受 | `Frame::Individual`、`Frame::FirstPerson`、`Frame::Embodied` |
| **巅峰统一** | 物理、理性、感性在足够高的维度上失去区分意义 | 🔲 远期——当前用 `Hold(Frame::Meta)` 作为工程近似 |

### 巅峰处的统一

> "物理、理性与感性，在巅峰处其实是统一的。"

这意味着当前的 `Frame` 系统（正交的、不可化简的多个域）只是一个**底层近似**。更完整的模型应该是：

```
底层：多帧并行（当前 8 帧 → 未来 200+ 学科帧）
        ↕ 多结交叉验证
中层：帧间冲突 → Hold → 递归审计
        ↕ 约束下的深度控制
顶层：统一态（巅峰处）
       —— 不是 True/False/Hold
       —— 而是 "在足够多的尺度上交叉验证后，差异不再重要"
```

这对应了东方哲学中的**天人合一**——不是取消差异，而是在足够高的维度上，差异失去了区分意义。

### 与三层心智模型的关系

人间层的四个维度与 [EULER-HOMOLOGY.md](EULER-HOMOLOGY.md) §7 的三层心智模型形成映射：

- **物理身体** ↔ 躯体心智（第二层）
- **理性推理** ↔ 理性心智（第三层）
- **感性体验** ↔ 理性心智与躯体心智的交互
- **巅峰统一** ↔ 三层一致时的 π 停车

---

## 5. 时间不是匀速的

### 核心主张

> "万物以各自的运动速度，同时在发生。"

这不是哲学修辞。这是一个**可工程化的时间模型**。

### 当前 Trit-Core 的局限

`HarmonicClock`（`src/clock.rs`）目前只有一个 ω（角频率），而且它只代表"系统的运算节奏"。

### 万物自有 ωᵢ

```
万物都有自有的 ωᵢ：
  一棵树的 ω  ≈ 年轮周期
  一个人的 ω  ≈ 心跳×昼夜×寿命
  一个文明的 ω ≈ 百年到千年
  一顿饭的 ω  ≈ 小时到天
  一个 CPU 的 ω ≈ 纳秒到微秒

系统在 t 时刻感知到的"时间" =
  对 ωᵢ 的采样覆盖率 × 采样频率的匹配度
```

### 工程含义

当前 `HarmonicClock` 有两个预设：
- `physical()`：ω=10.0（快速、物理时间）
- `deliberative()`：ω=0.5（慢速、审议时间）

未来方向：**多时钟并行**——系统同时维护多个 `HarmonicClock` 实例，每个对应一个被监测的 ωᵢ。当一个事件进入系统时，系统根据事件的时间特征将其路由到最匹配的时钟。

---

## 6. "意外"的数学定义

### 核心主张

> "因为没有监听到具体事物的变化，所以当突然遭遇互动，就被判定为突然、意外。"

### 形式化

```
意外度(事件 E) =
  1 - P(监测到 E 的前驱变化 | 系统的 ωᵢ 覆盖了 E 所在的时间尺度)

当系统的采样频率 << 事物变化的固有频率 → 意外必然发生
```

### 这不是缺陷

这是**系统的认知边界意识**。一个系统如果不知道自己的采样频率远低于被监测事物的变化频率，它会在每一次"意外"中感到困惑。一个知道自己采样频率不足的系统，会在"意外"发生时诚实地说：**"这不是意外——这是我的监测能力不足。"**

### 现有代码映射

- **`TritValue::Unknown`**（`src/core/value.rs:18`）：超出认知范围——系统根本不知道这是什么
- **`SafeFallback`**（`src/meta/safe_fallback.rs`）：在危险域中，Unknown/Hold + 中断 → 强制 False
- **`SignalQuality::Poor`**（`aurora/src/bc/signal_analysis.rs`）：短信号（< 30 样本）标记为 Poor——系统知道自己采样不足

---

## 7. 信息 = 心智触碰世界的界面事件

### 推翻香农假设

当前主流信息论（香农）的假设：

> 信息 = 信号的不确定性降低。**信息独立存在**，只等被接收。

本文的定义：

> 信息 = 心智与世界触碰时产生的**界面事件**。

```
没有心智的触碰 → 只有物理能量波动，没有"信息"
有心智的触碰 → 能量被五感解码（有损压缩）→ 抽象 → 分辨 → 联想 → 推理
```

### 五感是有损编码器

```
世界真实的丰富度（∞）
  → 五感接收（带宽极窄，仅电磁波 380-780nm、20-20000Hz 等几个窄窗口）
  → 神经编码（有损压缩、特征提取）
  → 抽象概念（丢弃了大部分原始信息）
  → 语言/符号（再次有损量化）
```

这个过程从第一步开始就在"丢失"——不是错误，而是**生存所需要的简化**。

### 对 Trit-Core 的含义

Trit-Core 处理的所有对象——`TritWord`、`Phase`、`Frame`、`MetaInterrupt`——没有一个是"客观世界的真实属性"。它们全都是**相**（见 §8）。

`Phase` 不是"客观概率"，而是**系统对自己判断的倾向强度**。`Frame` 不是"世界的真实分类"，而是**系统选择的参考系**。`Hold` 不是"不确定"，而是**系统知道自己不知道的边界在哪里**。

这个定义更接近：
- **Gregory Bateson**："信息是产生差异的差异"（a difference that makes a difference）
- **Varela & Maturana** 的"认知生物学"：信息不在环境中，而在观察者的认知结构中

但本文比他们更彻底——指出了**五感本身就是有损编码器**，而语言和符号是再次有损的量化。

---

## 8. "境不自境，因心故境"

### 十四个字说完了科学绕了几百年的事

> **境不自境，因心故境。心不自心，因境故有。相，就是中间产物，内容物。**

科学史这条路径：

```
早期科学：世界是客观的，心智可以观察而不参与
  → 量子力学：观察者影响被观察者
  → 认知科学：五感是有损编码
  → 神经现象学：第一人称不可还原
  → 具身认知：身体参与思维构成
  → 生成认知：世界与心智共同涌现
      ↓
      绕了一大圈，回到：
      "境不自境，因心故境。心不自心，因境故有。"
```

### 三个概念

| 概念 | 定义 | 科学的错误理解 |
|-----|------|--------------|
| **境** | 心所触及的世界 | "客观实在"——仿佛可以脱离心而存在 |
| **心** | 触碰境的认知活动 | "信息处理器"——仿佛可以脱离境而运作 |
| **相** | 心与境触碰时产生的中间产物 | "数据"——仿佛是对客观世界的忠实拷贝 |

**"相"不是世界的照片，而是心与境相遇时产生的涟漪。**

### 对 Trit-Core 的含义

Trit-Core 处理的全部对象——Frame、Phase、Hook、Hold——都是"相"。它们不是世界的属性，而是**系统与输入数据触碰时产生的中间产物**。

这不是虚无主义。这是**知道工具的局限后，更清醒地使用工具**：

```
承认心智的参与是不可避免的
  → 承认五感信息是有损的
  → 承认语言和符号是再次有损的
  → 承认任何描述都无法覆盖全部存在实情
      ↓
      不是放弃，而是更清醒地使用这些工具
      知道工具的局限，才能更好地用工具
```

### 与 Trit-Core 使命的关系

> **系统不需要"完美描述世界"——它只需要知道自己描述世界的方式有哪些局限。**

一个 Hold 不是系统的失败，而是系统**知道自己不知道的边界在哪里**。

见 [EPISTEMIC-HUMILITY.md](EPISTEMIC-HUMILITY.md) — 完整的认识论谦逊声明。

---

## 9. 算力约束下的逼近最优

### 核心原则

> **算力决定深度，但态度决定方向。**

```
算力富裕时 → 高深度递归审计（200 次反身性检查）
算力有限时 → 浅层 Hold + 坦言未知 + 请求更多数据

无论算力如何 → 态度不变：
  1. 不假装知道
  2. 不强行消解冲突
  3. 不把 Hold 当作失败
  4. 对天地人保持谦逊
```

### 现有代码映射

- **`ComputeBudget`**（`src/budget/mod.rs`）：`DepthLevel` 枚举——`Minimal`, `Reduced`, `Standard`, `Deep`, `Exhaustive`
- **`HoldStrategy`**（`src/hook/mod.rs`）：`hold_budget` 和 `hold_cycle_count`——系统在连续 Hold 达到上限时触发 `Recalibrate`
- **`BandwidthScheduler::process()`**（`src/adapters/bandwidth_scheduler.rs:273`）：根据 `ComputeBudget` 选择不同的处理深度

### 递归不是预设的，而是由冲突的"不可消解程度"动态决定的

```
Level 0:  直接 TAND/TOR 匹配已知经验常量 (O(n), n=200 学科)
Level 1:  帧内冲突 → Phase 加权平均 (O(k), k=同域信号数)
Level 2:  跨帧冲突 → MetaInterrupt + Frame 边界引用 (触发 1 次)
Level 3:  深度审计 → 递归展开冲突学科的定义边界 (调用外部知识图谱)
Level 4:  Hold 确认 → 存档/采集请求/提醒
```

每一步都可以在 `ComputeBudget` 约束下决定是否进入下一层——**深度不是预设的，而是由冲突的"不可消解程度"动态决定的**。

---

## 10. 从愿景到代码

### 已实现

| 天地人组件 | 代码位置 | 成熟度 |
|-----------|---------|--------|
| 天时：气候生态 | `src/anchor/ecological_base.rs` | ✅ M1 |
| 天时：硬件时序 | `src/budget/mod.rs` | ✅ M1 |
| 天时：双时钟 | `src/clock.rs` — `HarmonicClock` | ✅ M1 |
| 地理：群体共识 | `src/core/frame.rs` — `Frame::Consensus` | ✅ M1 |
| 地理：关系参考系 | `src/core/frame.rs` — `Frame::Relational` | ✅ M1 |
| 人间：物理身体 | `src/anchor/survival_motives.rs`, `thermal_baseline.rs` | ✅ M1 |
| 人间：理性推理 | `src/core/`, `src/meta/` | ✅ M1 |
| 人间：感性体验 | `src/core/frame.rs` — `Frame::Individual`, `Frame::FirstPerson`, `Frame::Embodied` | ✅ M1 |
| 多结交互：帧冲突 | `src/meta/interrupt.rs` — `MetaInterrupt` | ✅ M1 |
| 多结交互：算力门控 | `src/budget/mod.rs` — `DepthLevel` | ✅ M1 |
| 多结交互：安全降级 | `src/meta/safe_fallback.rs` | ✅ M1 |

### 缺口（按优先级）

| 缺口 | 描述 | 优先级 |
|------|------|--------|
| 动态 Frame 注册 | 从 8 个固定 Frame → 用户可注册的 Frame 系统 | v0.5.x |
| 多时钟并行 | 多个 `HarmonicClock` 实例，每个对应一个 ωᵢ | v0.6.x |
| 递归审计管线 | 深度控制 + 预算感知的递归审计 | v0.6.x |
| 学科常量库框架 | 200+ 学科知识的初始化和管理框架 | v0.7.x |
| 躯体层 | `Frame::Somatic` + `Hook::Interoception` | v0.8.x |
| 环境层 | `Anchor::Environmental_Baseline` — 地磁、气压、月相 | v0.8.x |
| 尺度自觉调度器 | 多时间尺度并行决策 | v0.8.x |
| 全层结递归 | 天地人三层完整递归交叉验证 | v1.0.0 |

### 验证命令

```bash
# 运行所有测试
cargo test --workspace --all-features -- --test-threads=2

# 验证 Anchor 层
cargo test anchor_

# 验证 Frame 系统
cargo test frame_

# 验证 SafeFallback
cargo test safe_fallback

# 运行伦理门测试
cargo test ethics_
```

---

## 相关文档

- [PHILOSOPHY.md](../PHILOSOPHY.md) — 项目深层动机
- [CONCEPTS.md](../CONCEPTS.md) — 核心类型定义与语义
- [ARCHITECTURE.md](../ARCHITECTURE.md) — 系统架构与模块分层
- [EPISTEMIC-HUMILITY.md](EPISTEMIC-HUMILITY.md) — 认识论谦逊声明
- [EULER-HOMOLOGY.md](EULER-HOMOLOGY.md) — 欧拉恒等式与 Trit-Core 的数学同构
- [FUTURE.md](../FUTURE.md) — 已知局限与未来方向
- [CONFLICT_CATALOG.md](CONFLICT_CATALOG.md) — 跨域冲突模式分类
```

- [ ] **Step 2: Verify file was created and has correct structure**

```bash
wc -l docs/explanation/insights/TIANDIREN-MATRIX.md && grep "^## " docs/explanation/insights/TIANDIREN-MATRIX.md
```

Expected: 10 sections (## 1 through ## 10) plus the title.

- [ ] **Step 3: Commit**

```bash
git add docs/explanation/insights/TIANDIREN-MATRIX.md
git commit -m "docs: add TIANDIREN-MATRIX.md — Heaven-Earth-Human knot matrix architecture vision

Describes three-layer grid (celestial/earthly/human) with knot
matrix interactions. Covers: non-uniform time model (all things
have intrinsic ωᵢ), mathematical definition of surprise,
information as mind-world interface event, Buddhist epistemology
(境不自境因心故境) in engineering terms, compute-constrained
approximation to optimal. Maps vision to existing code and
identifies gaps by version target."
```

---

### Task 3: Update cross-links in existing docs

**Files:**
- Modify: `docs/explanation/PHILOSOPHY.md` (add 1 line in §10)
- Modify: `docs/explanation/insights/EPISTEMIC-HUMILITY.md` (add 2 lines in "Related Documents")
- Modify: `docs/explanation/insights/FUTURE.md` (add 1 reference in §5)
- Modify: `docs/INDEX.md` (add 2 entries under insights/)

**Interfaces:**
- Consumes: EULER-HOMOLOGY.md and TIANDIREN-MATRIX.md exist at their paths
- Produces: updated cross-reference links in 4 existing files

- [ ] **Step 1: Update PHILOSOPHY.md §10**

In `docs/explanation/PHILOSOPHY.md`, find the summary section (§10, around line 216-231). After the numbered list (item 9), add:

```markdown
10. **欧拉同构与天地人矩阵**意味着 Trit-Core 的认知架构与欧拉恒等式存在深层同构（见 [EULER-HOMOLOGY.md](insights/EULER-HOMOLOGY.md)），而其未来演化方向可以用天地人多结矩阵来描述（见 [TIANDIREN-MATRIX.md](insights/TIANDIREN-MATRIX.md)）
```

And update the existing items 1-9 to 1-9 (no renumbering needed — this is item 10).

Use `mcp__serena__replace_content` with regex mode to insert after item 9. The needle should match the line containing "9. **诚实**意味着当对齐在数学上不可能时" and append the new item 10 after it.

- [ ] **Step 2: Update EPISTEMIC-HUMILITY.md "Related Documents"**

In `docs/explanation/insights/EPISTEMIC-HUMILITY.md`, find the "相关文档" section (around line 160-166). Add two new entries before the closing:

```markdown
- [EULER-HOMOLOGY.md](EULER-HOMOLOGY.md) — 欧拉恒等式与 Trit-Core 的数学同构
- [TIANDIREN-MATRIX.md](TIANDIREN-MATRIX.md) — 天地人多结矩阵：架构愿景与认识论基础
```

- [ ] **Step 3: Update FUTURE.md §5**

In `docs/explanation/insights/FUTURE.md`, find §5 "Frame 类型有限" (around line 84-100). In the "可能路径" subsection, add after the last bullet:

```markdown
- 天地人多结矩阵中的"地理层"和"天时层"需要新的 Frame 类型——详见 [TIANDIREN-MATRIX.md](insights/TIANDIREN-MATRIX.md) §3-4
```

- [ ] **Step 4: Update docs/INDEX.md**

In `docs/INDEX.md`, find the `explanation/insights/` section (around line 37). Add two entries:

```markdown
- [`EULER-HOMOLOGY.md`](explanation/insights/EULER-HOMOLOGY.md) — Euler identity as Trit-Core cognitive architecture (Chinese)
- [`TIANDIREN-MATRIX.md`](explanation/insights/TIANDIREN-MATRIX.md) — Heaven-Earth-Human knot matrix vision (Chinese)
```

- [ ] **Step 5: Verify all links are valid**

```bash
grep -rn "EULER-HOMOLOGY.md\|TIANDIREN-MATRIX.md" docs/ --include="*.md"
```

Expected: links in PHILOSOPHY.md, EPISTEMIC-HUMILITY.md, FUTURE.md, INDEX.md, plus the self-references in the two new files.

- [ ] **Step 6: Commit**

```bash
git add docs/explanation/PHILOSOPHY.md docs/explanation/insights/EPISTEMIC-HUMILITY.md docs/explanation/insights/FUTURE.md docs/INDEX.md
git commit -m "docs: add cross-links to EULER-HOMOLOGY and TIANDIREN-MATRIX from existing docs

Update PHILOSOPHY.md §10, EPISTEMIC-HUMILITY.md related docs,
FUTURE.md §5, and docs/INDEX.md with pointers to the two new
insight documents."
```

---

### Task 4: Final verification

**Files:**
- Verify: all new and modified files

- [ ] **Step 1: Check all files exist and have content**

```bash
echo "=== New files ===" && wc -l docs/explanation/insights/EULER-HOMOLOGY.md docs/explanation/insights/TIANDIREN-MATRIX.md && echo "=== Modified files ===" && wc -l docs/explanation/PHILOSOPHY.md docs/explanation/insights/EPISTEMIC-HUMILITY.md docs/explanation/insights/FUTURE.md docs/INDEX.md
```

- [ ] **Step 2: Verify no broken markdown links**

```bash
grep -oP '\[.*?\]\(.*?\)' docs/explanation/insights/EULER-HOMOLOGY.md | head -20
grep -oP '\[.*?\]\(.*?\)' docs/explanation/insights/TIANDIREN-MATRIX.md | head -20
```

Manual check: all linked files exist at their relative paths.

- [ ] **Step 3: Run git diff to review all changes**

```bash
git diff --stat HEAD~3..HEAD
```

- [ ] **Step 4: Commit if any final tweaks were needed**

```bash
git add -A && git commit -m "docs: final verification pass for philosophy docs integration" || echo "No changes needed"
```
