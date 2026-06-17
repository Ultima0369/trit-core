# CONCEPTS — Trit-Core 核心概念

本文档定义 Trit-Core 中所有核心类型的语义、设计理由和数学基础。阅读本文档后，你应该能理解每一个 `TritWord` 在系统中意味着什么，以及为什么它被设计成这样。

---

## 1. TritValue — 三值逻辑单元

### 1.1 定义

```rust
pub enum TritValue {
    True,    // +1 — 肯定
    Hold,    //  0 — 有意暂停判断
    False,   // -1 — 否定
    Unknown, //  ⊥ — 超出认知范围，不可计算
}
```

### 1.2 为什么不是布尔值？

布尔逻辑只有两个状态：`true` 和 `false`。在工程安全、医疗决策、价值判断等场景中，存在两类布尔逻辑无法表达的关键状态：

1. **有意暂停判断**（Hold）：系统知道这个问题，但检测到跨域冲突，**选择不决定**。这不是失败——这是诚实。

2. **超出认知范围**（Unknown）：输入本身不在系统的训练分布内。这不是"不确定"——系统根本不知道这是什么。

Hold 和 Unknown 的区别是核心设计：

| 状态 | 含义 | 系统是否理解输入 | 是否可计算 |
|---|---|---|---|
| True | 肯定 | 是 | 是 |
| Hold | 有意暂停 | 是 | 是 |
| False | 否定 | 是 | 是 |
| Unknown | 超出认知 | 否 | 否 |

### 1.3 数学基础：MVL-3

Trit-Core 的 True/Hold/False 构成一个三值逻辑系统（MVL-3，Multi-Valued Logic with 3 computable states）。Unknown 不在 MVL-3 内——它是元层面的标记，用于输入门控和安全降级。

### 1.4 运算法则

**否定（negate）**：分支无关的 LUT 实现。

| 输入 | 输出 |
|---|---|
| True | False |
| Hold | Hold |
| False | True |
| Unknown | Unknown |

**TAND（合取）**：

| TAND | True | Hold | False | Unknown |
|---|---|---|---|---|
| True | True | Hold | False | Unknown |
| Hold | Hold | Hold | False | Unknown |
| False | False | False | False | Unknown |
| Unknown | Unknown | Unknown | Unknown | Unknown |

**TOR（析取）**：

| TOR | True | Hold | False | Unknown |
|---|---|---|---|---|
| True | True | True | True | True |
| Hold | True | Hold | Hold | Unknown |
| False | True | Hold | False | Unknown |
| Unknown | True | Unknown | Unknown | Unknown |

关键观察：
- False 在 TAND 中湮灭一切（安全保守：一个否定信号否定整个合取链）
- True 在 TOR 中主导一切（一个肯定信号释放整个析取链）
- Unknown 在 TAND 中传染（一个未知因素污染整条推理链）
- Unknown 在 TOR 中被 True 覆盖（已知的肯定可以覆盖未知）

---

## 2. Phase — 连续倾向度

### 2.1 定义

```rust
pub struct Phase(f64);  // 范围 [0.0, 1.0]
```

| 值 | 含义 |
|---|---|
| 0.0 | 完全倾向于 False |
| 0.5 | 完全中性 |
| 1.0 | 完全倾向于 True |
| 0.73 | 倾向于 True，但保留 27% 的不确定性空间 |

### 2.2 为什么不是离散的？

真实世界的信念不是开关。两个独立传感器都报告"可能为真"时，它们的相位均值反映的是"多个来源互相印证后的倾向强度的提高"——这是贝叶斯更新在连续空间的几何类比。

### 2.3 量化（Quantization）

为防止长链级联中的浮点漂移，Phase 在构造和运算后自动量化：

```
0.5 ± ε → 0.5（中性锚点，最先检查）
0.0 ± ε → 0.0
1.0 ± ε → 1.0
```

中性锚点优先检查，因为在大量级联中，0.50000001 和 0.49999999 在语义上应该被视为相同。

### 2.4 运算

- **均值** `mean(a, b)`：`(a + b) / 2`，自动量化
- **互补** `complement(p)`：`1.0 - p`，自动量化
- **承诺方向** `commitment(p)`：p > 0.5 → TowardTrue，p < 0.5 → TowardFalse，p ≈ 0.5 → Neutral

---

## 3. Frame — 决策域

### 3.1 定义

```rust
pub enum Frame {
    Science,     // 经验/证据驱动
    Individual,  // 个人上下文/个人事实
    Consensus,   // 统计/群体偏好
    Absolute,    // 不可知/不可观测（永远 Hold）
    Meta,        // 冲突仲裁/策略输出
}
```

### 3.2 核心规则

**同 Frame 内**：正常三元逻辑运算，Phase 取均值。这是"热路径"——约占典型决策的 80%。

**跨 Frame**：任何跨 Frame 操作不产生真假结论。返回值强制为 Hold + MetaInterrupt。这是"冷路径"——确保跨域冲突不被悄悄抹平。

### 3.3 Frame 的哲学

Frame 不是标签，是**参考系**。在物理学中，两个在不同参考系中测量的速度不能直接相加——你需要洛伦兹变换。在决策论中，两个在不同决策域中产生的判断不能直接取平均——你需要域仲裁。

Analogous principle: 你不能把"物理学说这座桥不安全"和"公众投票说这座桥看起来很安全"放在一起来做加权平均。这两个判断不属于同一个可比较的空间。

---

## 4. Domain — 仲裁域

### 4.1 定义

```rust
pub enum Domain {
    Physical,        // 硬科学约束
    Engineering,     // 应用约束
    MedicalEthics,   // 软约束
    ValueJudgment,   // 不可通约
    General,         // 默认协商
    Custom(String),  // 外部加载的域规则
}
```

### 4.2 仲裁规则

| Domain | 优先 Frame | 可强制坍缩 | 逻辑 |
|---|---|---|---|
| Physical | Science | 是 | 物理定律不谈判 |
| Engineering | Science | 是 | 安全系数不妥协 |
| MedicalEthics | Individual | 否 | 患者自主权是安全默认 |
| ValueJudgment | 无 | 否 | 不可通约的价值——永远 Hold |
| General | 首个（同帧时）| 否 | 同帧提交，跨帧协商 |
| Custom(name) | 由规则定义 | 由规则定义 | 外部 JSON 规则文件 |

### 4.3 为什么 ValueJudgment 永远 Hold？

有些价值判断在数学上是不可通约的。"我应该做高薪但无聊的工作还是低薪但有创造性的工作"——没有算法应该替人回答这个问题。系统的诚实体现在说"I cannot decide this for you"。

---

## 5. TritWord — 计算原子

### 5.1 定义

```rust
pub struct TritWord {
    pub value: TritValue,  // 三值状态
    pub phase: Phase,      // 连续倾向度
    pub frame: Frame,      // 决策域
}
```

一个 TritWord 携带了做出一个可审计决策所需的全部信息：
- **是什么**（value）
- **有多确定**（phase）
- **在什么参考系中**（frame）

### 5.2 构造器

| 构造器 | value | phase | 语义 |
|---|---|---|---|
| `tru(frame)` | True | 1.0 | 完全确定的肯定 |
| `fals(frame)` | False | 0.0 | 完全确定的否定 |
| `hold(frame)` | Hold | 0.5 | 中性暂停 |
| `unknown(frame)` | Unknown | 0.5 | 超出认知范围 |
| `new(v, p, f)` | 任意 | 任意 | 完全自定义 |

---

## 6. MetaInterrupt — 冲突记录

```rust
pub struct MetaInterrupt {
    pub conflict: ConflictType,  // FrameMismatch, OutOfScope, PhaseDrift, PolicyViolation
    pub reason: String,          // 人类可读原因
    pub timestamp: DateTime<Utc>, // UTC 时间戳
}
```

每一个跨域冲突、安全降级、策略违反都产生一个 MetaInterrupt。这确保了：
- **可审计性**：每个"为什么系统说 Hold"都有记录
- **可追溯性**：每个安全决策都有时间戳和原因
- **不可篡改性**：日志是追加的

---

## 7. SafeFallback — 安全降级

核心原则（IEC 61508 / ISO 26262）：

> 在危险域中，系统"不能决定"必须默认为"不做"。

当一个 Domain 被标记为危险的（Physical、Engineering、chemistry、genetics、nuclear 等），并且系统产生了 Hold 或 Unknown，同时存在 MetaInterrupt 时，SafeFallback 强制将结果改为 False。

这意味着：如果你在处理一个化工厂的安全系统，检测到跨域冲突（例如操作员想打开阀门但压力传感器报警），系统不会"保持中立"——它说 False（不操作），把决定权交还给人类。

### 7.1 设计理由

- `Hold` = "还没准备好决定，收集更多数据"（非危险域适用）
- `SafeFallback → False` = "不能决定，但失败意味着人员伤亡 → 阻止"（危险域适用）

---

## 8. BinaryBaseline — 二元基线对比

Trit-Core 包含一个二元对照系统，用于**证明三值逻辑确实检测到了二元逻辑会丢失的信号**。

二元基线的工作方式：
1. 计数 True 和 False（Hold 视为弃权）
2. True > False → True，否则 → False（tie → False，保守）
3. 完全忽略 Frame 差异

实验结果（12 个场景）：
- 67% 的案例：二元基线产生误导性输出
- 100% 的 ValueJudgment 案例：二元无法表达"算法不应该决定这个"
- 100% 的 MedicalEthics 案例：二元忽略患者特定上下文
