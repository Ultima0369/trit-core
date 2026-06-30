# GLOSSARY — 术语表

本文档定义 Trit-Core 中使用的所有术语，包括项目自创术语、借用术语以及跨学科对应。

---

## A

### Absolute (帧)
不可知/不可观测的决策域。任何 Absolute 帧的 TritWord 必须永远是 Hold——系统承认这些问题不可判定。
**参见**: Frame, MetaMonitor

### Arbitration (仲裁)
策略引擎根据 Domain 规则解决跨域冲突的过程。
**参见**: ResolutionPolicy, ArbitrationResult

### ArbitrationResult (仲裁结果)
```rust
pub enum ArbitrationResult {
    Commit(TritWord),     // 提交到特定值
    Preserve(TritWord),   // 保留特定值（MedicalEthics）
    ForceCollapse,        // 强制安全坍缩
    Hold,                // 有意暂停
    Negotiate,           // 尝试协商
}
```
**参见**: ResolutionPolicy

---

## B

### BinaryBaseline (二元基线)
用于与三值系统对比的二元多数投票实现。证明二元逻辑会丢失跨域冲突信号。
**参见**: M2 验证

---

## C

### Cold Path (冷路径)
跨 Frame 的操作路径。产生 Hold + MetaInterrupt，触发策略仲裁。约占操作总量的 20%。
**延迟**: ~95ns
**参见**: Hot Path

### Commitment (承诺方向)
Phase 的三种方向性解释：
- `TowardTrue` (phase > 0.5)
- `Neutral` (phase ≈ 0.5)
- `TowardFalse` (phase < 0.5)

### Computable (可计算)
TritValue 中 True、Hold、False 是可计算的——系统理解输入并能执行逻辑运算。Unknown 是不可计算的。
**参见**: TritValue, Unknown

### ConflictType (冲突类型)
```rust
pub enum ConflictType {
    FrameMismatch,     // 跨帧操作
    OutOfScope,        // 超出系统范围（SafeFallback 触发）
    PhaseDrift,        // 相位漂移异常
    PolicyViolation,   // 策略违反（Absolute 帧非 Hold）
}
```

### Consensus (帧)
统计/群体偏好的决策域。来自多数人投票或众包数据。
**参见**: Frame

### CustomRule (自定义规则)
通过 JSON 文件加载的外部仲裁规则。使用 `RuleLoader` 特质加载。
**参见**: RuleLoader, Domain::Custom

---

## D

### Decouple (解耦)
分布式协议中节点断开耦合连接，恢复到 Sovereign 状态的过程。
**参见**: NodeState, RESONATE

### Domain (仲裁域)
决定冲突解决策略的域分类：
- Physical: 硬科学约束（物理定律优先）
- Engineering: 应用约束（安全系数优先）
- MedicalEthics: 软约束（患者自主权优先）
- ValueJudgment: 不可通约（永远 Hold）
- General: 默认协商
- Custom(name): 外部规则

**参见**: ResolutionPolicy

---

## F

### ForceCollapse (强制坍缩)
ArbitrationResult 的一种。表示物理/工程域中缺少 Science 信号时的安全默认。交由 SafeFallback 处理。
**参见**: SafeFallback

### Frame (帧/参考系)
决策域。每个 TritWord 属于一个 Frame。同帧操作正常计算，跨帧操作触发仲裁。

五种内置帧：
- Science（经验/证据驱动）
- Individual（个人上下文）
- Consensus（统计/群体偏好）
- Absolute（不可知）
- Meta（冲突仲裁/策略输出）

**参见**: FrameMask

### FrameMask (帧掩码)
u8 位掩码，用于 O(1) 检测输入中是否存在特定帧。每个帧占一个 bit。
**参见**: Frame

---

## H

### Harmonic Ternary Algebra (HTA，谐波三值代数)
Trit-Core 的核心计算模型：TAND、TOR、TNOT 加 Phase 算术。
**参见**: TernaryAlgebra

### Hold (暂停判断)
系统的有意暂停判断。"我理解这个问题，但基于当前信息或在跨域冲突下，我选择不决定。"与 Unknown 不同——Hold 是可计算状态。
**参见**: TritValue, Unknown

### Hot Path (热路径)
同 Frame 的操作路径。使用标准三值真值表 + Phase 均值，不产生 MetaInterrupt。约占操作总量的 80%。
**延迟**: ~1.5ns
**参见**: Cold Path

---

## I

### IEC 61508
国际电工委员会的功能安全标准。SafeFallback 的设计依据。"在危险域中，不能决定必须默认为不做。"
**参见**: SafeFallback

### Individual (帧)
个人上下文/个人事实的决策域。患者自主权、个人偏好、存在性事实。
**参见**: Frame

### Interference (干扰类型)
两个节点之间的帧兼容性分类：
- Constructive: 同帧（相位共振）
- Neutral: 跨帧但相位接近（可协商）
- Destructive: 跨帧且相位发散（冲突）

**参见**: Node

---

## M

### Meta (帧)
元层面的帧。用于冲突仲裁和策略输出。当系统产生 Hold（跨帧冲突）时，结果被放置在 Meta 帧中。

### MetaInterrupt (元中断)
跨域冲突的结构化记录。包含冲突类型、原因、时间戳。确保可审计性。
**参见**: ConflictType

### MetaMonitor (元监控器)
记录 MetaInterrupt 并强制执行不变性（如 Absolute 帧必须 Hold）的组件。

### MVL-3 (三值逻辑)
Multi-Valued Logic with 3 computable states（True、Hold、False）。Unknown 不在 MVL-3 内——它是元层面的标记。

---

## N

### Negotiate (协商)
多节点之间通过消息交换解决冲突的过程。也指 ArbitrationResult 的一种（General 域跨帧时）。

### Node (节点)
分布式协议中的参与方。每个节点有 Frame、Phase、状态（Sovereign/Coupling/Coupled/Hold）。
**参见**: NodeState

### NodeState (节点状态)
```
Sovereign → Coupling → Coupled → Hold
    ↑                      │
    └────── 解耦 ──────────┘
```

---

## P

### Phase (相位/倾向度)
连续值 f64 ∈ [0.0, 1.0]。0.5 = 中性，>0.5 = 倾向 True，<0.5 = 倾向 False。
模拟真实世界信念的连续性和不确定性。
**参见**: Quantization

### PLL Controller (锁相环控制器)
软件锁相环。用于分布式节点的相位同步。参数：kp=0.3、deadband=0.05、max_correction=0.1。
**参见**: ResonanceBus

---

## Q

### Quantization (量化)
Phase 在运算后自动吸附到锚点（0.0、0.5、1.0）以防止级联中的浮点漂移。
**参见**: Phase

---

## R

### RESONATE (谐振)
分布式协议中节点请求与另一个节点耦合的消息（RESONATE_REQ/RESONATE_ACK）。
**参见**: Node, PLL Controller

### ResolutionPolicy (仲裁策略)
根据 Domain 规则决定冲突解决方式的策略引擎。
**参见**: Domain, ArbitrationResult

### ResonanceBus (谐振总线)
内存中的消息总线，用于节点注册、消息路由、PLL 校正和耦合生命周期管理。
**参见**: Node, PLL Controller

### RuleLoader (规则加载器)
外部仲裁规则的加载和应用特质。默认实现：`JsonRuleLoader`。
**参见**: CustomRule, Domain::Custom

---

## S

### SafeFallback (安全降级)
IEC 61508 安全原则的实现。危险域 + Hold/Unknown + 中断 > 0 → 强制 False。
**参见**: Domain, ForceCollapse

### Science (帧)
经验/证据驱动的决策域。科学数据、实验测量、物理定律。
**参见**: Frame

### Sovereign (主权状态)
节点的默认状态。独立振荡，无对等耦合。节点的主权相位在耦合期间被保留，解耦时恢复。
**参见**: NodeState

---

## T

### TAND (三值与)
Harmonic Ternary AND。False 湮灭一切，Unknown 传染。
**参见**: TernaryAlgebra, TritValue

### TernaryAlgebra (三值代数)
TAND、TOR、TNOT、THOLD、TSENSE 的静态方法集合。
**参见**: HTA

### THOLD (强制 Hold)
将任意 TritWord 强制转为 Hold 状态（phase=0.5）。
**参见**: TernaryAlgebra

### TNOT (三值非)
Phase 翻转否定。True→False，False→True，Hold→Hold，Unknown→Unknown。
**参见**: TernaryAlgebra

### TOR (三值或)
Harmonic Ternary OR。True 主导一切。
**参见**: TernaryAlgebra, TritValue

### Trit (三值位)
单数。TritWord 的非正式简写。类比：bit → trit。
**别名**: `pub type Trit = TritWord;`

### TritValue (三值)
TritWord 的三值状态枚举。
- True (+1): 肯定
- Hold (0): 有意暂停
- False (-1): 否定
- Unknown (⊥): 超出认知范围

**参见**: MVL-3

### TritWord (三值词)
Trit-Core 的原子计算单元。`{ value: TritValue, phase: Phase, frame: Frame }`。
**参见**: TritValue, Phase, Frame

### TSENSE (传感器输入)
从原始传感器数据创建 Hold 状态的 TritWord。
**参见**: TernaryAlgebra

---

## U

### Unknown (未知)
超出系统认知范围的标记（⊥）。在 MVL-3 之外。与 Hold 不同——Unknown 表示"我不知道你在说什么"，Hold 表示"我理解但选择不判断"。
**参见**: TritValue, Hold

---

## V

### ValueJudgment (价值判断)
不可通约的价值判断域。在所有 Domain 中唯一永远返回 Hold 的域——承认算法不应替代人类做价值判断。
**参见**: Domain

---

## 跨学科对应

| Trit-Core 术语 | 数学/逻辑学 | 物理/工程学 | 神经科学 |
|---|---|---|---|
| TritValue::Hold | 直觉主义"未证明" | 不确定性原理 | 认知抑制 |
| Phase | 贝叶斯置信度 | 波函数相位 | 神经同步性 |
| Frame | 参考系 | 惯性参考系 | 认知框架 |
| FrameMask | 位向量 | 多路复用 | 并行探测 |
| MetaInterrupt | 元逻辑断言 | 故障中断 | 冲突监测 |
| SafeFallback | 安全自动化 | IEC 61508 | 防御行为 |
| Quantization | 数值方法 | 测量精度 | 感知阈值 |
| RESONATE | 耦合振子 | 锁相环 | 神经元同步 |
