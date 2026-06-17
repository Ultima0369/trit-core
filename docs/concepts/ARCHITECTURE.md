# ARCHITECTURE — Trit-Core 系统架构

本文档描述 Trit-Core 的分层架构、数据流和关键设计决策。

---

## 1. 分层架构

```
┌──────────────────────────────────────────────────┐
│  应用层 (CLI / API)                               │
│  trit-sandbox, trit-node 二进制                   │
├──────────────────────────────────────────────────┤
│  沙箱引擎 (sandbox/)                               │
│  场景解析 → TAND 级联 → 策略仲裁 → JSON 输出        │
├──────────────────────────────────────────────────┤
│  策略引擎 (meta/)                                  │
│  Domain → ResolutionPolicy → SafeFallback          │
│  FrameMask (O(1) 位掩码)                           │
├──────────────────────────────────────────────────┤
│  三元 ALU (trit/)                                  │
│  TernaryAlgebra: TAND, TOR, TNOT, THOLD, TSENSE    │
│  Phase 算术: mean, complement, quantize            │
├──────────────────────────────────────────────────┤
│  帧注册表 (frame/)                                  │
│  Frame 枚举 + FrameRegistry                        │
├──────────────────────────────────────────────────┤
│  数据模型 (trit/)                                   │
│  TritWord { value: TritValue, phase: Phase,        │
│             frame: Frame }                         │
└──────────────────────────────────────────────────┘
```

---

## 2. 热路径 vs 冷路径

这是 Trit-Core 最关键的微架构设计。

### 2.1 热路径（Hot Path）

**条件**：两个 TritWord 共享同一个 Frame。

**行为**：
- 标准三值真值表（TAND/TOR）
- Phase 取算术均值
- 不分配 MetaInterrupt
- 不触发 MetaMonitor

**性能**：约 3ns/操作（分支无关 LUT + 一次浮点加法 + 一次除法）

**占比**：典型决策中约 80% 的操作走热路径。

### 2.2 冷路径（Cold Path）

**条件**：两个 TritWord 的 Frame 不同。

**行为**：
- 返回 Hold + MetaInterrupt
- 记录冲突类型、原因、时间戳
- 触发后续策略仲裁

**性能**：约 95ns/操作（包含 String 分配和时间戳获取）

**占比**：约 20% 的操作走冷路径。跨域冲突在现实场景中不是常态，但必须被检测。

### 2.3 为什么这样设计？

热/冷路径分离遵循一个核心洞察：**大多数决策发生在同一个参考系内，不需要元认知开销。** 当两个科学家讨论同一个实验数据时，他们不需要先协商"我们是否在同一个认知框架中"——他们直接计算。只有当经济学家和生态学家讨论同一个政策时，才需要元层面的仲裁。

---

## 3. 数据流

```
JSON 场景文件
      │
      ▼
ScenarioInput (serde 反序列化)
      │
      ▼
SignalInput[] → TritWord[]
      │
      ▼
TAND 级联（从左到右折叠）
  ├── 同帧 → 热路径：真值表 + Phase 均值
  └── 跨帧 → 冷路径：Hold + MetaInterrupt
      │
      ▼
MetaInterrupt[] 收集
      │
      ▼
ResolutionPolicy::arbitrate()
  ├── FrameMask 位掩码 O(1) 帧检测
  ├── Domain 特定规则
  └── 返回 ArbitrationResult
      │
      ▼
SafeFallback::guard()
  ├── 检查域是否危险
  ├── 检查结果是否为 Hold/Unknown
  ├── 检查中断计数 > 0
  └── 必要时强制 False
      │
      ▼
SandboxOutput (JSON 序列化)
```

---

## 4. FrameMask — O(1) 帧检测

```rust
pub(crate) struct FrameMask(u8);
```

5 个 Frame 各占一个 bit：

| Frame | 位 |
|---|---|
| Science | bit 0 |
| Individual | bit 1 |
| Consensus | bit 2 |
| Absolute | bit 3 |
| Meta | bit 4 |

操作：
- `from_inputs()`：一次遍历，O(n)，设置对应位
- `has(frame)`：位与运算，O(1)
- `count()`：`popcount` 指令，O(1)

当所有 5 位都设置时（`mask == 0b11111`），提前退出遍历。

---

## 5. ResolutionPolicy — 域仲裁

```rust
pub fn arbitrate(&self, inputs: &[TritWord]) -> ArbitrationResult
```

仲裁结果：

| 结果 | 含义 |
|---|---|
| `Commit(TritWord)` | 提交到特定 TritWord |
| `Preserve(TritWord)` | 保留特定 TritWord（MedicalEthics） |
| `ForceCollapse` | 强制安全坍缩（交由 SafeFallback 处理） |
| `Hold` | 有意暂停判断 |
| `Negotiate` | 尝试多轮协商 |

### 5.1 仲裁逻辑

```
Physical/Engineering:
  Science 存在 → Commit(Science 信号)
  Science 不存在 → ForceCollapse

MedicalEthics:
  Individual 存在 → Preserve(Individual 信号)
  Individual 不存在 → Negotiate

ValueJudgment:
  无条件 → Hold

General:
  单一帧 → Commit(第一个信号)
  多帧 → Negotiate

Custom(name):
  无条件 → Negotiate（由外部 RuleLoader 覆盖）
```

---

## 6. SafeFallback — IEC 61508 安全原则

### 6.1 触发条件（三个条件必须同时满足）

1. Domain 是危险的（Physical、Engineering、chemistry、genetics、structural、nuclear、pharmaceutical）
2. 仲裁结果是 Hold 或 Unknown
3. 存在至少一个 MetaInterrupt

### 6.2 行为

强制将结果改为 `False`，并生成 `OutOfScope` 类型的 MetaInterrupt。

### 6.3 为什么 MedicalEthics 不是危险的？

患者自主权（Individual frame）本身就是安全默认。在医疗场景中，"不做"（不治疗）可能比"做"（强制治疗）更危险。因此 MedicalEthics 不触发 SafeFallback——它通过 Preserve(Individual) 机制来保护患者自主权。

---

## 7. 分布式协议（M4，存根）

### 7.1 Node 状态机

```
Sovereign → Coupling → Coupled → Hold
    ↑                      │
    └────── 解耦 ──────────┘
```

### 7.2 消息类型

| 操作码 | 方向 | 含义 |
|---|---|---|
| RESONATE_REQ | 请求方→目标 | 请求相位耦合 |
| RESONATE_ACK | 目标→请求方 | 确认耦合（含干扰类型） |
| DECOUPLE_REQ | 任意→目标 | 请求解耦 |
| DECOUPLE_ACK | 目标→请求方 | 确认解耦（恢复主权相位） |
| NEGOTIATE | 总线广播 | 多节点协商结果 |
| HEARTBEAT | 节点→总线 | 存活信号 |

### 7.3 PLL 锁相环

软件锁相环用于耦合节点的相位同步：

- 比例增益 `kp = 0.3`
- 死区 `deadband = 0.05`（忽略微小相位差，防止振荡）
- 最大单步校正 `max_correction = 0.1`
- 冲突相位差阈值：`|phase_a - phase_b| > 0.3`

---

## 8. 关键设计约束

1. **`#![forbid(unsafe_code)]`** — 零 unsafe 代码
2. **`#![deny(warnings)]`** — 警告即错误
3. **核心代数冻结** — `trit/` 模块的 TAND/TOR/TNOT 语义在 0.1.x 中不可变
4. **跨帧操作不强制二元决策** — 始终产生 Hold + MetaInterrupt
5. **Absolute 帧必须永远 Hold** — 由 MetaMonitor::inspect() 强制执行
6. **Phase 构造时钳制 NaN/Inf** — 防止浮点异常传播
