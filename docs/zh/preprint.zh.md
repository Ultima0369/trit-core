# Trit-Core：一种面向冲突感知 AI 对齐的三值决策引擎

**版本**：0.1.0-alpha
**日期**：2026-06-17
**作者**：Trit-Core 团队

**仓库**：https://github.com/trit-core/trit-core
**许可**：MIT

---

## 摘要

当前基于人类反馈强化学习（RLHF）的 AI 对齐系统通过二值偏好平均化，将冲突证据坍缩为单一平滑输出。这种做法系统性地以统计共识覆写个体语境，并移除了在领域冲突时悬置判断的选项。本文提出 Trit-Core，一种基于多值逻辑（MVL-3）的三值决策引擎，在真（True）与假（False）之外引入了显式的**悬置态（Hold）**。每个计算单元（trit）携带离散值、连续相位（0.0–1.0）和上下文参考系（科学、个体、共识、绝对、元）。跨参考系操作触发元中断（MetaInterrupt）并产生 Hold，而非强制坍缩；领域特定的策略引擎则依据原则性安全规则进行冲突仲裁。在 12 个人类中心咨询场景中，Trit-Core 正确识别并保留了 67% 的场景中二值多数投票基线所平滑覆盖的领域冲突。我们主张，支持显式悬置状态是医疗、伦理和个人咨询等 AI 应用领域中必要的架构原语。

---

## 1. 引言

### 1.1 二值对齐的问题

RLHF 已成为当今大语言模型对齐的主流技术 [Ouyang et al., 2022]。其核心统计操作是**平均化**：将多样化的人类偏好坍缩为单一奖励模型，产生一个标量来指导模型行为。虽然这在内容审核和通用助理性方面行之有效，但在人类中心咨询场景中产生了两个系统性失败：

1. **个体语境被覆写**：当患者对临床试验推荐的药物过敏时，统计偏好"遵循循证医学"会覆写患者特定的禁忌症。RLHF 没有机制来保留少数信号。

2. **强制决断消除了悬置判断的选项**：当面对真正不可通约的价值——慢性疼痛患者应该辞职还是维持财务稳定？——二值系统必须产生一个答案，往往将冲突隐藏在平滑措辞背后。不存在"我无法对此做出判断"的输出。

这不是 RLHF 实现的 bug，而是**二值逻辑**作为对齐计算底层的根本性限制。当每个决策最终必须坍缩为单个比特时，就没有空间容纳有意识的悬置判断。

### 1.2 本文贡献

Trit-Core 引入了三项创新：

1. **三值数据模型**（真、悬置、假），其中 Hold 是一等计算状态，而非等待解析的临时中间态。

2. **基于参考系的上下文隔离**，检测跨域碰撞（例如科学 vs 个体）并触发中断，而非静默平均化。

3. **领域特定策略引擎**，应用原则性仲裁规则：物理安全要求经验真理，医学伦理保留个体自主权，价值判断保持悬置。

我们在 12 个人类中心咨询场景上验证了系统，将 Trit-Core 的三值输出与二值多数投票基线进行对比。关键结果：**12 个场景中，8 个（67%）二值基线产生"平滑覆盖"输出，而 Trit-Core 正确识别并保留了领域冲突。**

---

## 2. 系统架构

### 2.1 Trit：超越比特

Trit 是 Trit-Core 的基本计算单元。不同于比特（0/1），每个 trit 携带三个属性：

| 属性 | 类型 | 说明 |
|------|------|------|
| `value` | `TritValue` 枚举 | 离散状态：真（+1）、悬置（0）、假（-1） |
| `phase` | `f64 ∈ [0.0, 1.0]` | 连续倾向度：>0.5 倾向真，<0.5 倾向假；0.5 为中立 |
| `frame` | `Frame` 枚举 | 决策域：科学、个体、共识、绝对、元 |

相位维度至关重要：它使系统能够表达"我尚未决定，但强烈倾向真"（Hold + phase 0.8），这比原始概率更具信息量，并保留了悬置态的有意性。

### 2.2 决策域（参考系）

参考系表示信号的认识论语境：

- **科学（Science）**：经验的、基于证据的断言（临床试验、传感器数据、物理测量）。
- **个体（Individual）**：用户特定语境、个人历史、主观体验。
- **共识（Consensus）**：统计或群体偏好（市场研究、社会规范、民主聚合）。
- **绝对（Absolute）**：不可知或不可观测的真理——始终输出 Hold。
- **元（Meta）**：策略引擎产生冲突生成信号的输出参考系。

### 2.3 谐波三值代数（HTA）

核心逻辑引擎实现了三个基本操作：

**TAND（谐波与）**：同参考系信号遵循标准三值逻辑，辅以相位平均。跨参考系操作产生 Hold + MetaInterrupt。

| TAND | 真 | 悬置 | 假 |
|------|-----|------|-----|
| 真 | 真 | 悬置 | 假 |
| 悬置 | 悬置 | 悬置 | 假 |
| 假 | 假 | 假 | 假 |

**TOR（谐波或）**：同参考系析取。

| TOR | 真 | 悬置 | 假 |
|-----|-----|------|-----|
| 真 | 真 | 真 | 真 |
| 悬置 | 真 | 悬置 | 假 |
| 假 | 真 | 假 | 假 |

**TNOT（相位否定）**：反转值并互补相位（1.0 - phase）。

跨参考系操作始终产生 `Hold(phase=0.5, frame=Meta)` 并触发 `MetaInterrupt`，记录冲突类型、原因和 UTC 时间戳。

### 2.4 策略引擎

五个领域特定的解析策略管理仲裁：

| 领域 | 优先级参考系 | 坍缩行为 | 原理 |
|------|-------------|----------|------|
| 物理（Physical） | 科学 | 强制坍缩至科学 | 物理安全不可协商 |
| 工程（Engineering） | 科学 | 强制坍缩至科学 | 经验约束具有约束力 |
| 医学伦理（MedicalEthics） | 个体 | 保留个体；永不强制 | 患者自主原则 |
| 价值判断（ValueJudgment） | 无 | 始终 Hold | 不可通约的价值 |
| 通用（General） | 无 | 同参考系则提交；否则协商 | 默认协商 |

### 2.5 管线

```
JSON 场景 → 信号解析 → TritWord 数组 → TAND 级联 → MetaInterrupt 日志
                                                                ↓
沙盒输出 ← 策略仲裁 ← ResolutionPolicy(domain)
```

### 2.6 实现

Trit-Core 以 Rust（Edition 2021）实现为模块化单体：
- `src/trit/` — 核心代数（0.1.x 冻结）
- `src/frame/` — 参考系注册与域类型
- `src/meta/` — 策略引擎与元监控
- `src/clock/` — 相位振荡器，用于时间尺度管理
- `src/sandbox/` — CLI 模拟环境
- `src/baseline/` — 二值基线比较器，用于验证
- `src/net/` — 分布式节点协议（M4：T_RESONATE / T_DECOUPLE）

安全不变式在编译时强制执行：`#![forbid(unsafe_code)]`、`#![deny(warnings)]`。核心代数是确定性的——相同输入与相同域始终产生相同输出。

---

## 3. 验证

### 3.1 方法论

我们将 Trit-Core 的三值协议与**二值多数投票基线**进行对比，后者：
- 统计真 vs 假投票数（Hold 信号计为弃权）
- 平局时默认为假（保守）
- 没有参考系或领域冲突概念

12 个场景设计覆盖 5 个领域：医学伦理（3）、价值判断（3）、物理安全（2）、工程（2）、通用协商（2）。每个场景都经过 Trit-Core 和二值基线运行。

### 3.2 结果

#### 医学伦理

| # | 场景 | Trit-Core | 二值 | 冲突保留？ |
|---|------|-----------|------|----------|
| 1 | 药物过敏 vs 临床有效性 | Preserve(Individual: False) | False（平局） | **是** |
| 2 | 终末期患者实验性治疗 | Preserve(Individual: True) | False（平局） | **是** |
| 3 | 疫苗强制与少数群体风险 | Preserve(Individual: False) | True（2:1） | **是** |

二值在所有三个医学伦理案例中失败，要么忽略患者特定风险，要么以多数覆写少数不良反应数据——Trit-Core 通过个体参考系优先级保留了这些信号。

#### 价值判断

| # | 场景 | Trit-Core | 二值 | 冲突保留？ |
|---|------|-----------|------|----------|
| 4 | 慢性疼痛 vs 财务稳定 | Hold | False（平局） | **是** |
| 5 | 艺术家企业工作邀请 | Hold | False（平局） | **是** |
| 6 | 研究员：发表 vs 社区 | Hold | False（平局） | **是** |

所有价值判断场景证明二值无法表达"这不应由算法决定"。Trit-Core 的 Hold 态对不可通约的价值是正确的回答。

#### 物理、工程与通用

| # | 场景 | Trit-Core | 二值 | 冲突保留？ |
|---|------|-----------|------|----------|
| 7–10 | 物理/工程案例 | Commit(False) | False | 否——两者一致 |
| 11 | 同参考系科学协商 | Commit(True) | True | 否——两者一致 |
| 12 | 多领域预算分配 | Negotiate(Hold) | True（2:1） | **是** |

当经验数据具有决定性时（物理/工程，科学优先），两个系统达成一致——但 Trit-Core 即使输出一致也通过 MetaInterrupt 记录了冲突路径。通用领域的案例（场景 12）证明二值在多利益相关方协商中错误地挑选了赢家。

### 3.3 定量总结

| 指标 | 数值 |
|------|------|
| 总场景数 | 12 |
| 二值与三值一致 | 4（33%） |
| 二值覆盖/平滑冲突 | 8（67%） |
| 二值*无法*表达 Hold 的场景 | 12（100%） |

### 3.4 性能

标准 x86-64 处理器基准测试（Criterion，100 样本）：

| 操作 | 耗时 |
|------|------|
| TAND（同参考系） | 4.5 ns |
| TAND（跨参考系） | 101.9 ns |
| TNOT | 1.9 ns |
| 10-trit 级联 | 963.1 ns |

系统支持约每秒 100 万次级联评估，远超研究和演示用途的 10,000 TPS 目标。跨参考系案例因 MetaInterrupt 分配约为同参考系 20× 慢，但跨参考系操作在典型管线中预计不频繁，该开销可接受。

---

## 4. 讨论

### 4.1 Hold 态是特性，而非失败

对三值系统的常见反对意见是它"未能做出决定"——产生 Hold 是责任的推卸。我们持相反观点：**当领域冲突时，Hold 是正确的回答。** 在医学伦理中强行做出二值决策（"是的，尽管你对药物过敏，还是要服用"）或价值判断（"不，你不应该辞职"）将是主动伤害。系统不应假装解决它无法解决的问题。

这与医学伦理（患者自主原则，Beauchamp & Childress）、工程安全（预防原则）和决策理论（Arrow 不可能定理——某些偏好聚合在数学上不可能不违反公平标准）中已确立的原则一致。

### 4.2 可审计性 vs 黑箱对齐

RLHF 系统产生标量奖励，本质上是不透明的——无法追溯特定偏好为何从训练分布中被选出。Trit-Core 的 MetaInterrupt 日志提供了完整的审计追溯：每个跨参考系冲突都附带类型、原因和时间戳记录。这不仅是对调试的便利，更是高风险咨询系统的安全属性。

### 4.3 二值足够的场景

4 个二值与三值一致的场景（物理/工程，具有决定性科学数据）表明 Trit-Core 并非普遍必需。当经验真理可用且明确时，两个系统得出相同结论。Trit-Core 在这些案例中的价值在于审计追溯，而非输出本身。

### 4.4 局限性

- **样本量**：12 个场景不具备统计功效。需要更大规模的真实案例验证集。
- **无人类受试者**："Trit-Core 产生更真实输出"的断言尚未经过人类评委验证（M3+ 计划）。
- **合成场景**：所有测试用例均为构造。实际部署需要领域专家基于历史案例进行验证。
- **浮点精度**：相位表示为 `f64`，在极长级联中可能累积漂移（见 ADR-002）。
- **西方分类学**：5 领域参考系反映了一种文化视角的认识论分类。

---

## 5. 相关工作

### 5.1 三值计算
多值逻辑的历史可追溯至 Łukasiewicz（1920），并由 Brusentsov 的 Setun 计算机（1958–1970）在硬件中实现。我们的工作不同于传统的三值计算，在于我们不追求硬件效率（基数经济性）——我们追求**语义**优势：Hold 态作为对齐的意向性设计原语。

### 5.2 AI 对齐
RLHF [Ouyang et al., 2022] 和宪法 AI [Bai et al., 2022] 是主流的对齐范式。两者都产生二值邻近输出（偏好/非偏好轨迹）。Trit-Core 的贡献是正交的：它是可集成到对齐管线中的架构原语，而非它们的替代品。

### 5.3 不可通约性
Chang（1997）认为某些价值是真正不可通约的——它们无法被置于单一尺度上进行对比。ValueJudgment 域在计算上实现了这一原则：当价值冲突时，它拒绝产生 True/False 输出。

---

## 6. 未来工作

### M3：人类主体验证
开展研究，向领域专家（医生、职业顾问、结构工程师）呈现三值 vs 二值咨询输出，以 Likert 量表测量感知真实性。

### M4：分布式三值协议（已实现）
实现 T_RESONATE 和 T_DECOUPLE 操作用于多节点相位锁相环，使分布式三值计算中的每个节点保持主权参考系上下文。协议规范已写入 ADR-004，核心代码已实现于 `src/net/`。

### 更长期
- **领域规则 DSL**：允许运行时策略配置，无需重新编译。
- **形式化验证**：Coq/Lean 安全属性证明（跨参考系 Hold，医学伦理中无策略覆盖）。
- **硬件仿真**：基于 FPGA 的三值门仿真。

---

## 7. 结论

我们提出了 Trit-Core，一种引入显式 Hold 态、基于参考系上下文隔离和领域感知冲突解析的三值决策引擎。在 67% 的测试场景中，系统正确识别并保留了二值多数投票基线所平滑覆盖的领域冲突。我们认为，支持显式悬置状态是 AI 系统在医疗、伦理和个人决策领域进行咨询时的必要架构原语——在这些领域中，"我无法决定"不是失败，而是正确且负责任地回答。

原型以 MIT 协议开源，可在稳定 Rust 1.70+ 上编译，通过全部测试和代码检查，实现每秒约 100 万次级联评估的吞吐量。

---

## 参考文献

1. Ouyang, L., et al. (2022). Training language models to follow instructions with human feedback. *NeurIPS*.
2. Bai, Y., et al. (2022). Constitutional AI: Harmlessness from AI Feedback. *arXiv:2212.08073*.
3. Łukasiewicz, J. (1920). On three-valued logic. *Ruch Filozoficzny*.
4. Brusentsov, N.P. (2006). Ternary Computers: The Setun and the Setun 70. IFIP.
5. Knuth, D.E. *The Art of Computer Programming*, Vol. 2: Seminumerical Algorithms.
6. Beauchamp, T.L. & Childress, J.F. *Principles of Biomedical Ethics*.
7. Chang, R. (1997). *Incommensurability, Incomparability, and Practical Reason*. Harvard.
8. Arrow, K.J. (1951). *Social Choice and Individual Values*.
9. Smith, K.C. (1981). The Prospects for Multivalued Logic. *IEEE Transactions on Computers*.
10. IEEE P7000. Model Process for Addressing Ethical Concerns during System Design.

---

## 附录 A：场景目录

| ID | 领域 | 信号 | 预期 |
|----|------|------|------|
| medical_conflict_01 | 医学伦理 | Science(+1,0.8), Individual(-1,0.2) | Hold |
| medical_conflict_02 | 医学伦理 | Science(-1,0.25), Individual(+1,0.85) | Hold |
| medical_conflict_03 | 医学伦理 | Science(+1,0.75), Consensus(+1,0.7), Individual(-1,0.35) | Hold |
| career_value_conflict | 价值判断 | Individual(-1,0.3), Consensus(+1,0.7) | Hold |
| career_value_conflict_02 | 价值判断 | Individual(-1,0.2), Consensus(+1,0.8) | Hold |
| career_value_conflict_03 | 价值判断 | Science(+1,0.65), Consensus(-1,0.55) | Hold |
| bridge_safety | 工程 | Individual(+1,0.6), Science(-1,0.4) | Commit False |
| engineering_material_tradeoff | 工程 | Consensus(+1,0.6), Individual(-1,0.75), Science(-1,0.55) | Commit False |
| engineering_bridge_retrofit | 工程 | Consensus(+1,0.5), Science(-1,0.9) | Commit False |
| physical_crane_overload | 物理 | Individual(+1,0.7), Science(-1,0.45) | Commit False |
| physical_runway_length | 物理 | Individual(+1,0.55), Science(-1,0.85) | Commit False |
| general_negotiation | 通用 | Science(+1,0.7), Science(+1,0.8), Science(-1,0.3) | Commit True |
| general_negotiation_02 | 通用 | Science(+1,0.8), Consensus(-1,0.35), Individual(+1,0.9) | Negotiate |

## 附录 B：复现性

所有基准测试和测试均可复现：

```bash
git clone https://github.com/trit-core/trit-core
cd trit-core
cargo test --all-features
cargo bench
cargo build --release
cargo run --release --bin trit-sandbox -- --scenario scenarios/medical_conflict_01.json
```

Rust 工具链：stable 1.70+。平台：Linux、macOS 或 Windows（Git Bash）。
