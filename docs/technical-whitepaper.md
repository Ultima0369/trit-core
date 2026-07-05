# Trit-Core 工程学技术白皮书

> **三值决策引擎：冲突感知 AI 对齐的工程实现**

**版本**：0.3.0  
**日期**：2026-07-04  
**协议**：MIT License  
**实现语言**：Rust 2021 Edition（`#![forbid(unsafe_code)]`）  
**代码规模**：~15,000 行 Rust | ~350 测试 | 40 场景  
**状态**：核心代数语义已冻结

> **提醒，而非指令**：本项目所有内容作为"邀请检验的提醒"，而非"必须服从的指令"。完整声明见 [`docs/explanation/insights/EPISTEMIC-HUMILITY.md`](explanation/insights/EPISTEMIC-HUMILITY.md)。

---

## 摘要

Trit-Core 是一个**面向冲突感知 AI 对齐的三值决策引擎**。与二值逻辑或概率平滑方案不同，它在 `True`（肯定）和 `False`（否定）之外引入了独立的第三状态——`Hold`（悬置判断），用于显式表达"系统检测到跨域冲突，选择不强制判定"。

**核心工程主张**：

> 在人类中心的咨询场景（医疗伦理、价值冲突、工程安全、公共协商）中，**允许悬置的三值协议比二值概率输出更能保留冲突信息、尊重人格主权，并避免误导性的共识坍缩**。

本白皮书面向科学家与工程审稿人，以图表驱动的递进叙事，从"问题"到"方法"到"实现"到"证据"，完整呈现三值决策引擎的工程学基础。

```
┌──────────────────────────────────────────────────────────────────┐
│                      Trit-Core 决策全景                             │
│                                                                    │
│  输入（多源信号）                                                     │
│  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐                              │
│  │Science│ │ Indiv│ │Consen│ │FirstP│  ...                          │
│  │ Frame │ │ Frame│ │ Frame│ │ Frame│                              │
│  └──┬───┘ └──┬───┘ └──┬───┘ └──┬───┘                              │
│     │        │        │        │                                    │
│     └────────┴───────┬┴────────┘                                    │
│                      ▼                                              │
│            ┌─────────────────┐                                      │
│            │  三值代数 (HTA)   │  ← TAND / TOR / TNOT               │
│            │  同帧→热路径 4ns  │                                      │
│            │  跨帧→Hold+中断   │                                      │
│            └────────┬────────┘                                      │
│                     ▼                                               │
│            ┌─────────────────┐                                      │
│            │   策略仲裁        │  ← Domain → ResolutionPolicy        │
│            │   域感知判定      │                                      │
│            └────────┬────────┘                                      │
│                     ▼                                               │
│            ┌─────────────────┐                                      │
│            │  安全降级        │  ← IEC 61508: 危险域中"不知"="不做"  │
│            │  SafeFallback    │                                      │
│            └────────┬────────┘                                      │
│                     ▼                                               │
│  输出  True / Hold / False  +  MetaInterrupt[]（可审计冲突链）       │
└──────────────────────────────────────────────────────────────────┘
```

---

## 1. 问题：二值逻辑的盲区

### 1.1 强制坍缩的四种典型场景

当前主流 AI 对齐方法——无论是 RLHF、DPO 还是宪法式 AI——在处理多源信号时，最终都必须将结果压缩为单一的偏好方向或概率分布。这种"强制坍缩"在以下四类场景中产生系统性偏差：

```
场景类别              二值系统的做法           丢失了什么
──────────────────────────────────────────────────────────────────
医疗伦理              患者个体风险 vs 统计      多数票忽略少数关键风险；
                      证据 → 加权平均           患者自主权被"平均掉"

价值判断              高薪无聊 vs 低薪创造      没有算法应该替人回答"怎样活"；
                      → 多数投票                二值系统却必须选一边

工程安全              公众观感 vs 物理定律      加权平均稀释了安全的硬度；
                      → 偏好聚合                观感与定律不可公度

公共协商              不可排序的权利冲突        多数决制造虚假的"共识"；
                      → 票决逻辑                沉默的少数被系统抹除
```

### 1.2 二值 vs 三值：一个具体例子

```
场景：桥梁安全评估
┌─────────────────────────────────────────────────────────────┐
│  输入信号：                                                   │
│  - Science Frame:  "应力超标 15%，不安全"  (False, phase=0.9) │
│  - Consensus Frame: "公众投票认为安全"      (True,  phase=0.7) │
│                                                              │
│  二值系统（多数票）：                                          │
│    1票 False, 1票 True → tie → 默认 False                     │
│    → 输出: False（"碰巧对了"，但没有记录冲突过程）             │
│                                                              │
│  二值系统（加权平均）：                                        │
│    score = 0.5×(-1)×0.9 + 0.5×(+1)×0.7 = -0.1               │
│    → 输出: 略偏 False（"物理定律被公众观感稀释了 50%"）       │
│                                                              │
│  三值系统（Trit-Core）：                                       │
│    Science(False, 0.9) TAND Consensus(True, 0.7)              │
│    → 跨帧检测 → Hold + MetaInterrupt                          │
│    → 输出: Hold（"检测到 Science/Consensus 冲突，暂不裁决"）  │
│    → SafeFallback: Physical 域 → 强制 False（"物理定律不谈判"）│
└─────────────────────────────────────────────────────────────┘
```

### 1.3 Hold 不是失败

Trit-Core 的设计哲学是：**当输入来自不可通约的参考系时，正确的输出不是 True 也不是 False，而是 Hold + 可审计的冲突记录**。

Hold 表达的是三层含义：

| 层面 | 含义 |
|---|---|
| 认知层 | 系统**理解**了问题 |
| 检测层 | 系统**检测到**跨参考系冲突 |
| 决策层 | 系统**拒绝**在没有人类授权的情况下强制判定 |

这与"不确定性"或"模型能力不足"有本质区别——后者对应 `Unknown`（超出认知范围，不可计算）。

---

## 2. 方法：三值代数

### 2.1 四个基本类型

Trit-Core 的代数系统建立在四个相互正交的概念之上：

```
┌──────────────────────────────────────────────────────────────┐
│                    TritWord — 计算原子                         │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐              │
│  │ TritValue  │  │   Phase    │  │   Frame    │              │
│  │ 离散状态    │  │ 连续倾向度  │  │  参考系     │              │
│  └────────────┘  └────────────┘  └────────────┘              │
│                                                               │
│  "是什么"        "有多确定"       "在什么参考系中"             │
│  True/Hold/      [0.0, 1.0]       Science/Individual/         │
│  False/Unknown   0.5=中性         Consensus/FirstPerson...    │
└──────────────────────────────────────────────────────────────┘
```

#### TritValue — 四状态体系

| 状态 | 符号 | 数值 | 语义 | 可计算 |
|---|---|---|---|---|
| `True` | +1 | 1 | 肯定裁决 | 是 |
| `Hold` | 0 | 0 | 有意暂停判断 | 是 |
| `False` | -1 | -1 | 否定裁决 | 是 |
| `Unknown` | ⊥ | 0 | 超出认知范围 | 否 |

`True` / `Hold` / `False` 构成可计算空间（MVL-3），`Unknown` 是元层面的安全标记——它在合取运算中传染，防止不可知的信号参与决策。

#### Phase — 连续倾向度

```
0.0  ←──────────────── 0.5 ────────────────→ 1.0
完全倾向于False         完全中性             完全倾向于True

例子：
  Phase(0.9) = "倾向于 True，确信度很高"
  Phase(0.3) = "倾向于 False，但保留 30% 不确定性"
  Phase(0.5) = "完全中性，不偏向任何一方"
```

Phase 提供离散三值之外的**强度维度**。两个独立传感器都报告"可能为真"时，它们的 Phase 均值反映的是多源印证后的倾向强度增强——这是贝叶斯更新在连续空间的几何类比。

为防止长链级联中的浮点累积误差，Phase 在构造和运算后自动量化：`0.5 ± ε → 0.5`，`0.0 ± ε → 0.0`，`1.0 ± ε → 1.0`。中性锚点优先检查。

#### Frame — 参考系（12 个变体）

```
Trit-Core 基础 8 帧              Aurora 扩展 4 帧
┌──────────────────┐          ┌──────────────────┐
│ Science          │ 经验证据  │ GeoEco           │ 地理生态  │
│ Individual       │ 个人事实  │ Developmental    │ 成长轨迹  │
│ Consensus        │ 群体偏好  │ Role             │ 社会角色  │
│ Absolute         │ 不可知    │ Environmental    │ 环境状态  │
│ Meta             │ 系统内部  │                  │          │
│ FirstPerson      │ 主观报告  │                  │          │
│ Embodied         │ 身体状态  │                  │          │
│ Relational       │ 关系互动  │                  │          │
└──────────────────┘          └──────────────────┘
```

Frame 是**参考系**，不是标签。在物理学中，两个不同参考系中测量的速度不能直接相加——你需要洛伦兹变换。在决策论中，两个不同 Frame 中产生的判断不能直接取平均——你需要域仲裁。

**核心规则**：

| 规则 | 说明 |
|---|---|
| 同 Frame 内 | 正常三值运算，Phase 取均值——热路径（~80% 的决策走这条） |
| 跨 Frame | 任何运算返回 `Hold` + `MetaInterrupt`——冲突不悄悄抹平 |
| FirstPerson 优先 | 当 `FirstPerson` 与 `Science` 冲突时，默认保留主观事实 |
| Absolute 永远 Hold | "绝对不可知"不能被判定为 True 或 False |

### 2.2 三值真值表

#### TNOT（谐波否定）

```
输入:  True  →  False
       Hold  →  Hold      （悬置的否定还是悬置）
       False →  True
       Unknown → Unknown  （不可知的否定依然不可知）
```

#### TAND（谐波合取）

```
 TAND   │ True  Hold  False  Unknown
────────┼────────────────────────────
 True   │ True  Hold  False  Unknown
 Hold   │ Hold  Hold  False  Unknown
 False  │ False False False  Unknown
 Unknown│ Unkn  Unkn  Unkn   Unknown
```

关键观察：
- **False 湮灭一切**：一个否定信号否定整个合取链（安全保守原则）
- **Unknown 传染**：一个未知因素污染整条推理链（不可知不可参与判定）
- **Hold 不湮灭**：悬置 + 悬置 = 悬置（冲突叠加不自动升级）

#### TOR（谐波析取）

```
 TOR    │ True  Hold  False  Unknown
────────┼────────────────────────────
 True   │ True  True  True   True
 Hold   │ True  Hold  Hold   Unknown
 False  │ True  Hold  False  Unknown
 Unknown│ True  Unkn  Unkn   Unknown
```

关键观察：
- **True 主导一切**：一个肯定信号释放整个析取链
- **Unknown 被 True 覆盖**：已知的肯定可以覆盖未知
- **False 不湮灭析取**：析取中 False 不主导，允许其他信号表达

### 2.3 热路径与冷路径

```
调用方
  │
  ├─ precheck_same_frame(a, b) == true
  │    └─ t_and_hot() / t_or_hot()
  │         · 帧枚举比较 ×1
  │         · 值匹配 ×1
  │         · Phase::mean() + quantize()
  │         · TritWord 构造
  │         ⏱ ~4 ns — 零堆分配
  │
  └─ precheck_same_frame(a, b) == false
       └─ t_and() / t_or()
            · 帧比较（确认冲突）
            · MetaInterrupt::with_frames() 构造
            · tracing 门控日志
            ⏱ ~100 ns — 仅分配 String 原因文本
```

热/冷路径分离是 Trit-Core 的核心性能设计。同帧操作（约占典型决策的 80%）走零分配热路径，跨帧冲突走带审计记录的冷路径。调用方不需要手动判断——代数层自动路由。

### 2.4 批量级联：t_and_n

对于 3+ 信号的批量合取，`t_and_n` 使用**等权 Phase 平均**而非顺序左折叠，消除级联偏差：

```
左折叠偏差（避免）：
  ((a TAND b) TAND c)  →  Phase 被前两步的平均权重扭曲

t_and_n（等权）：
  value = a.value TAND b.value TAND c.value（真值表优先级）
  phase = (a.phase + b.phase + c.phase) / 3  →  公平平均
```

---

## 3. 决策核心：五层认知栈

### 3.1 架构总览

```
┌──────────────────────────────────────────────────────────────┐
│  Layer 5: 反馈层 (feedback/)                                    │
│  实践测试 · 代理环境预测 · 偏差校准 · 纠错信号                    │
├──────────────────────────────────────────────────────────────┤
│  Layer 4: 核心代数 + 策略引擎 (core/ + meta/)                    │
│  TritValue · Phase · Frame · TritWord · TernaryAlgebra         │
│  Domain · ResolutionPolicy · SafeFallback · MetaInterrupt       │
├──────────────────────────────────────────────────────────────┤
│  Layer 3: 认知模块池 (adapters/)                                 │
│  10 个可动态挂载的认知模块 · HookContext 通信总线                 │
├──────────────────────────────────────────────────────────────┤
│  Layer 2: 场景感知 (hook/)                                       │
│  ScenarioRecognizer · MountArbiter · HookContext               │
├──────────────────────────────────────────────────────────────┤
│  Layer 1: 稳态约束 (anchor/)                                     │
│  五个否决级约束 (Veto Power) · 任何违反 → Hold + 告警            │
└──────────────────────────────────────────────────────────────┘
```

### 3.2 五层交互关系

```
                    ┌──────────────────┐
                    │   Layer 5: 反馈   │ ← 测试、校准、纠错
                    └────────┬─────────┘
                             │ 偏差信号
                    ┌────────▼─────────┐
                    │   Layer 4: 决策   │ ← 代数运算 + 策略仲裁
                    └──┬──────────┬────┘
                       │          │
             模块输出   │          │ 仲裁结果
                       │          │
          ┌────────────▼──┐  ┌───▼──────────────┐
          │ Layer 3: 模块  │  │  Layer 2: 感知    │
          │ 10个认知模块   │◄─┤ 场景识别·挂载决策 │
          └────────┬───────┘  └──────────────────┘
                   │
                   │ 只读
          ┌────────▼───────┐
          │  HookContext   │ ← 模块间唯一通信通道
          │  不可变上下文    │
          └────────────────┘
                   ▲
                   │ 约束信号
          ┌────────┴───────┐
          │  Layer 1: 约束  │ ← 否决权：任何违反 → Hold
          │  5个稳态约束    │
          └────────────────┘
```

### 3.3 各层职责

#### Layer 1 — 稳态约束（Veto Power）

五个不可协商的约束，在任何决策前先检查。任何 `Abort` 级违反直接返回 `Hold` + 告警，任何 Frame 或 Domain 无法覆盖。

| 约束 | 语义 |
|---|---|
| `thermal_baseline` | 系统热基线——计算资源的物理边界 |
| `survival_motives` | 生存动机——系统完整性底线 |
| `flourishing_pool` | 繁荣池——长期健康运行的资源储备 |
| `ecological_base` | 生态基础——对外部环境的不可逆影响 |
| `wellbeing_priority` | 福祉优先——人类福祉不可被优化目标覆盖 |

#### Layer 2 — 场景感知

`ScenarioRecognizer` 将输入模式映射到场景类型（`PhysicalReasoning` / `ValueConflict` / `MedicalEthics` / `SelfReflection` / `General`）。`MountArbiter` 根据场景 + 资源预算决定挂载哪些 Layer 3 模块。`HookContext` 是只读通信总线——模块只能读它，不能写它。

#### Layer 3 — 认知模块池

10 个实现 `CognitiveModule` trait 的模块，可动态挂载/卸载：

```
AdaptiveIteration · AttentionScheduler · BandwidthScheduler
CognitiveDeconstruction · ConflictSuspension · CouplingAdapter
CriticalThinking · EcologicalAssessment · EngineeringArchitecture
ReflexiveAudit · SelfKnowledge
```

**关键约束**：模块之间不直接调用。所有跨模块通信通过 `HookContext`。每个模块输出包含 `confidence` 分数。卸载 = 释放，无后台处理。

#### Layer 4 — 核心代数 + 策略引擎

这是 Trit-Core 的数学与工程核心。已在第 2 节详述三值代数，在第 4 节详述策略引擎。

#### Layer 5 — 反馈

每个决策在 `ProxyEnvironment` 中被测试。偏差触发 Layer 3 模块校准；严重偏差触发管道重入 + 纠错信号。

### 3.4 端到端数据流

```
scenarios/*.json
       │
       ▼
 ScenarioInput ──validate──► SandboxPipeline
                               │
       ┌───────────────────────┘
       ▼
 SignalInput[] ──► TritWord[]（12 个管道阶段）
       │
       ├─ 1. validate          场景级 + 信号级校验
       ├─ 2. build_policy      解析 Domain，构建 ResolutionPolicy
       ├─ 3. build_trits       SignalInput → TritWord
       ├─ 4. registry_check    可选 Frame 白名单校验
       ├─ 5. t_and_n           批量 TAND 级联（等权平均）
       ├─ 6. arbitrate         域策略仲裁
       ├─ 7. reflexive_guard   可选自反审计 guard
       ├─ 8. safe_fallback     危险域安全降级
       ├─ 9. attention         可选注意力调度
       ├─ 10. self_knowledge   可选自我知识推断
       ├─ 11. phase_trace      可选相位追踪
       └─ 12. build_output     JSON 输出
       │
       ▼
 SandboxOutput ──► stdout (JSON)
       │
       ▼
 SandboxDiagnostics ──► stderr (--diagnostic)
```

---

## 4. 策略引擎：从冲突到安全

### 4.1 Domain — 10 个仲裁域

Domain 定义了"同一组信号在不同上下文中如何被仲裁"。

| Domain | 优先 Frame | 可强制坍缩 | 逻辑 |
|---|---|---|---|
| `Physical` | Science | **是** | 物理定律不谈判 |
| `Engineering` | Science | **是** | 安全系数不妥协 |
| `MedicalEthics` | Individual | 否 | 患者自主权是安全默认 |
| `ValueJudgment` | 无 | 否 | 不可通约——永远 Hold |
| `General` | 首个（同帧时）| 否 | 同帧提交，跨帧协商 |
| `Custom(name)` | 由规则定义 | 由规则定义 | 外部 JSON 规则文件 |
| `Organizational` | 多 Frame 协商 | 否 | 跨角色/流程协商 |
| `Relational` | Relational | 否 | 关系在场时优先关系帧 |
| `Cognitive` | Embodied | 否 | 优先身体信号胜过抽象 |
| `Environmental` | GeoEco | 否 | 环境适应优先地理生态约束 |

### 4.2 仲裁决策树

```
arbitrate(domain, trit_words, interrupts)
  │
  ├─ domain == Physical / Engineering
  │    ├─ 存在 Science Frame → Commit(Science)
  │    └─ 无 Science Frame   → ForceCollapse → SafeFallback 接管
  │
  ├─ domain == MedicalEthics
  │    ├─ 存在 Individual Frame → Preserve(Individual)
  │    └─ 无 Individual Frame   → Negotiate
  │
  ├─ domain == ValueJudgment
  │    └─ 永远 Hold（算法不应替人做价值判断）
  │
  ├─ domain == General
  │    ├─ 所有信号同 Frame → Commit(首个)
  │    └─ 多 Frame 存在     → Negotiate
  │
  ├─ domain == Organizational
  │    └─ 多 Frame 跨角色协商 → Negotiate
  │
  ├─ domain == Relational
  │    ├─ 存在 Relational Frame → 优先 Relational
  │    └─ 无 Relational Frame   → Negotiate
  │
  ├─ domain == Cognitive
  │    ├─ 存在 Embodied Frame → 优先 Embodied
  │    └─ 无 Embodied Frame   → Negotiate
  │
  ├─ domain == Environmental
  │    ├─ 存在 GeoEco Frame → 优先 GeoEco
  │    └─ 无 GeoEco Frame   → Negotiate
  │
  └─ domain == Custom(name)
       ├─ 加载了 CustomRule → 按规则执行
       └─ 无规则            → Negotiate
```

### 4.3 SafeFallback — IEC 61508 安全降级

```
SafeFallback::guard(domain, result, interrupt_count)
  │
  ├─ domain 不是危险域？
  │    └─ 直接通过（不干预）
  │
  ├─ result == Unknown？
  │    └─ → 强制 False
  │         （系统无法计算 → 在危险域中默认"不做"）
  │
  ├─ result == Hold 且 (interrupt_count > 0 或 force)？
  │    └─ → 强制 False
  │         （悬置 + 有冲突 → 在危险域中默认"不做"）
  │         （Phase 重置为 full_false() = 0.0）
  │
  └─ 其他情况 → 通过
```

**设计原理**（IEC 61508 / ISO 26262）：

> 在安全关键系统中，无法确认"安全"的状态，必须默认为"不安全"。

| 概念 | 非危险域 | 危险域 |
|---|---|---|
| `Hold` | "还没准备好，收集更多数据" | → `SafeFallback → False`："不能决定但失败=伤亡 → 阻止" |
| `Unknown` | "超出范围，标记以便审查" | → `SafeFallback → False`："完全不知道但可能致命 → 阻止" |

**内置危险域**：
- `Physical` 和 `Engineering`：始终危险（物理定律不谈判）
- 默认注册的 Custom 危险域：`chemistry`、`genetics`、`structural`、`nuclear`、`pharmaceutical`
- `MedicalEthics` **不是**危险域——因为患者自主权（`Individual` Frame）本身就是安全默认

**实例**：化工厂安全系统检测到跨域冲突（操作员想开阀，压力传感器报警），三值运算返回 `Hold`（Scientific Frame 与 Role Frame 冲突），SafeFallback 强制 `False`——"不操作"——把决定权交还给人类。

### 4.4 MetaInterrupt — 审计链

每一次跨域冲突、安全降级、策略违反都产生一条 `MetaInterrupt`：

```
MetaInterrupt {
    conflict: FrameMismatch | OutOfScope | PhaseDrift | PolicyViolation,
    reason:   "TAND: Science vs Consensus — cross-frame conflict",
    timestamp: 2026-07-04T12:34:56Z,
}
```

这确保了：
- **可审计性**：每个"为什么系统说 Hold"都有完整的冲突记录
- **可追溯性**：每个安全决策都有精确的时间戳和原因
- **不可篡改性**：审计日志是追加的

---

## 5. 工程证据

### 5.1 测试矩阵

| 类型 | 数量 | 说明 |
|---|---|---|
| 单元测试 | ~250 | 核心代数、Phase、Frame、TritWord、策略、SafeFallback、心智工程模块 |
| 集成测试 | ~20 | 端到端场景、元策略、三值不变量 |
| 属性测试 | 19 | proptest：代数定律、Phase 有界性、SafeFallback |
| 场景校验 | 40 | 20 英文 + 20 中文 JSON 场景，全部匹配 expected_behavior |
| 伦理门测试 | 10 | 非可协商的 Aurora 伦理约束 |
| CLI 测试 | ~19 | 路径遍历、未知参数、dry-run、validate-only |
| 不变量测试 | 9 | 核心不变量 |
| 错误路径测试 | ~16 | 非法 domain/frame/phase/value |
| **总计** | **~375** | **全部通过** |

### 5.2 性能基准

| 操作 | 延迟 |
|---|---|
| `t_and_hot`（同帧热路径） | ~4.1 ns |
| `precheck_same_frame` | ~0.75 ns |
| TAND 跨帧（冷路径） | ~104 ns |
| 10 元素热路径级联 | ~3.5 ns/op |
| 10 元素跨帧级联 | ~101 ns/op |

**端到端吞吐量**：

| 管道 | 吞吐量 | 相对 10,000 TPS 目标 |
|---|---|---|
| MedicalEthics 管道 | ~602K signals/s | **60×** |
| Physical 管道 | ~558K signals/s | **56×** |

**堆分析**（dhat）：
- 热路径（TAND/TOR/TNOT）：**零堆分配**
- 冷路径唯一分配：`MetaInterrupt::new()` 的 String 原因文本
- 端到端管道主要分配：serde JSON 序列化/反序列化

### 5.3 二元基线对比

Trit-Core 包含一个二元对照系统 `BinaryBaseline`，用于**证明三值逻辑确实检测到了二元逻辑会丢失的冲突信号**。

| 指标 | 结果 |
|---|---|
| 二值与三值输出一致 | 8 / 19（42%） |
| 二值覆盖/平滑真实冲突 | 11 / 19（58%） |
| 二值无法表达 Hold | 19 / 19（100%） |

关键发现：
- **100% ValueJudgment 案例**：二元无法表达"算法不应该决定这个"
- **100% MedicalEthics 案例**：二元忽略患者特定上下文
- 即使 Physical/Engineering 输出一致，Trit-Core 仍通过 `MetaInterrupt` 记录冲突路径——二元系统只输出结论，不记录过程

完整数据见 [`docs/reports/validation-report.md`](reports/validation-report.md)。

### 5.4 安全措施

| 风险 | 缓解措施 |
|---|---|
| 路径遍历 (CWE-22) | 输入路径规范化，限制在 `scenarios/` 目录 |
| 不可信反序列化 (CWE-502) | JSON ≤ 64KB，信号 ≤ 100，字符串 ≤ 1KB，Phase/Frame/Domain 严格校验 |
| 断言崩溃 (CWE-617) | `Phase::new` 返回 `Result`；`Phase::new_clamped` 仅显式调用 |
| 日志注入 (CWE-117) | `sanitize_log_field` 替换控制字符并截断 |
| 内存安全 | `#![forbid(unsafe_code)]` 编译时强制 |
| 依赖安全 | `cargo-audit` 通过，无已知 CVE |

### 5.5 质量门禁

| 门禁 | 状态 |
|---|---|
| `cargo fmt -- --check` | ✅ 通过 |
| `cargo clippy -- -D warnings` | ✅ 通过 |
| `cargo test --workspace --all-features` | ✅ ~375 测试通过 |
| `#![forbid(unsafe_code)]` | ✅ 零 unsafe |
| 公共 API 快照 | `cargo public-api -ss` 与 `api/public-api.txt` 比对 |

---

## 6. 局限与边界

### 6.1 已知局限

| 局限 | 严重度 | 说明 |
|---|---|---|
| 无形式化验证 | 高 | 核心真值表和仲裁逻辑仅靠测试覆盖，未经 Coq/Lean 验证 |
| 场景样本有限 | 中 | 40 个场景，未经统计学意义上的充分验证 |
| 无人类被试研究 | 中 | "真实用户满意度"和"感知真实性"尚未通过实验验证 |
| `Unknown` 传播偏保守 | 中 | TAND 中传染，可能过于严格 |
| 分布式协议未实现 | 低 | v0.2.0 已移除，计划作为独立 crate 重新引入 |
| Phase 长级联漂移 | 低 | `quantize()` 缓解但未根除，>10⁶ 的理论误差边界未计算 |

### 6.2 适用场景

**适合使用三值决策系统的场景**：
- 安全关键的多信号融合（物理/工程安全、医疗决策）
- 跨价值体系冲突（个人自主权 vs 集体利益 vs 科学证据）
- 冲突审计要求（合规场景：需要证明"系统检测到了冲突且未忽略"）
- 多领域 AGI 对齐实验（替代 RLHF 平均化的对齐范式）

**不适合的场景**：
- 简单二值决策（门禁开关、是/否审批——二元逻辑更高效）
- 纳秒级实时控制（Trit-Core 仲裁管道为微秒级）
- 无需冲突感知的统计聚合（推荐系统、评分融合）

---

## 7. 结论

Trit-Core v0.3.0 是一个**类型安全、零 unsafe、经过系统验证的三值决策引擎**。它通过独立的 `Hold` 状态、参考系感知的代数运算、域特定的仲裁策略和 IEC 61508 风格的安全降级，在二值系统会强制坍缩的场景中保留了关键冲突信息。

**当前证据表明**：
- 核心代数和管道语义正确（~375 测试通过）
- 性能远超 10,000 TPS 目标（56–60×）
- 安全与质量问题已修复（v0.1.0 发现的 P1/P2 已全部解决）
- 58% 的真实冲突案例中，二值基线无法保留冲突信息

**适合当前的角色**：研究原型、对齐实验平台、安全关键决策系统的推理组件。生产部署前仍需更大规模的场景验证、人类被试研究和领域专家评审。

---

## 附录

### A. 术语表

| 术语 | 定义 |
|---|---|
| **TritValue** | 三值逻辑单元：True (+1), Hold (0), False (-1), Unknown (⊥) |
| **Hold** | 有意暂停判断——系统检测到冲突但选择不强制裁决 |
| **Unknown** | 超出认知范围——输入不可计算 |
| **Phase** | 连续倾向度 [0.0, 1.0]，0.5 = 中性 |
| **Frame** | 决策参考系（12 个变体），跨 Frame 运算触发冲突 |
| **Domain** | 仲裁域（10 个变体），定义"同一组信号如何被裁决" |
| **TritWord** | 计算原子 = TritValue + Phase + Frame |
| **TAND / TOR / TNOT** | 谐波三值合取 / 析取 / 否定 |
| **HTA** | 谐波三值代数 (Harmonic Ternary Algebra) |
| **MetaInterrupt** | 冲突审计记录——每次跨域冲突产生一条 |
| **SafeFallback** | IEC 61508 风格安全降级——危险域中强制 False |
| **MVL-3** | 三值逻辑可计算空间 |
| **IAM** | 信息代数模块 (Information Algebra Module) |

### B. 与认知科学框架的映射

Trit-Core 的 Frame 系统与 `dao-science` 的 L0–L7 认知频谱存在结构性对应：

| dao-science 层级 | Trit-Core 对应 |
|---|---|
| L0（绝对事实/觉知） | `Frame::Absolute` — 不可知/不可观测，永远 Hold |
| L1（物理规律） | `Frame::Science` + `Physical`/`Engineering` Domain |
| L2（个体实情） | `Frame::Individual` + `MedicalEthics` 优先级 |
| L3（群体共识） | `Frame::Consensus` — 不被升格为真理 |
| L4（理性合作） | `General` Domain — 可形式化协作 |
| L5–L7（高冲突/高风险） | 触发 `Hold` + `MetaInterrupt` 或 `SafeFallback` |

不同层级的事实需要不同的处理方式，跨层级操作不能直接取平均——这直接支撑了 Frame 和 Domain 的设计哲学。

### C. 参考文献地图

| 主题 | 文档 |
|---|---|
| 快速开始 | [`docs/tutorials/QUICKSTART.md`](tutorials/QUICKSTART.md) |
| 核心概念详解 | [`docs/explanation/CONCEPTS.md`](explanation/CONCEPTS.md) |
| 架构设计 | [`docs/explanation/ARCHITECTURE.md`](explanation/ARCHITECTURE.md) |
| M2/M3 验证报告 | [`docs/reports/validation-report.md`](reports/validation-report.md) |
| 性能验证 | [`docs/reports/performance-validation.md`](reports/performance-validation.md) |
| 安全审计 | [`docs/reports/security-audit.md`](reports/security-audit.md) |
| 代码质量审计 | [`docs/reports/code-quality-audit.md`](reports/code-quality-audit.md) |
| 认知科学引用 | [`docs/explanation/insights/DAO-SCIENCE-REFERENCES.md`](explanation/insights/DAO-SCIENCE-REFERENCES.md) |
| 认知架构愿景 | [`aurora/00_manifest/COGNITIVE_ARCHITECTURE_LAYERS.md`](../aurora/00_manifest/COGNITIVE_ARCHITECTURE_LAYERS.md) |

---

*本白皮书随 v0.3.0 发布，作为 Trit-Core 三值决策引擎的工程学技术总览。*
