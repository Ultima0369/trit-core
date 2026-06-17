# MODULES — 模块参考

`src/` 下每个子模块的职责、关键函数和设计约束摘要。

---

## `trit/` — 核心三值代数（FROZEN in 0.1.x）

### 文件

| 文件 | 行数 | 职责 |
|---|---|---|
| `mod.rs` | 57 | TritWord 定义 + 构造器 |
| `value.rs` | 174 | TritValue 枚举 + LUT 实现 |
| `phase.rs` | 148 | Phase 连续倾向度 + 量化 |
| `algebra.rs` | 220 | TernaryAlgebra: TAND/TOR/TNOT/THOLD/TSENSE |

### 关键函数

| 函数 | 签名 | 说明 |
|---|---|---|
| `TritValue::negate` | `fn negate(self) -> TritValue` | LUT 驱动的分支无关取反 |
| `TritValue::to_i8` | `fn to_i8(self) -> i8` | LUT 驱动的数值转换 |
| `Phase::new` | `fn new(v: f64) -> Phase` | 构造 Phase，钳制 NaN/Inf |
| `Phase::mean` | `fn mean(a: Phase, b: Phase) -> Phase` | 算术均值 + 自动量化 |
| `Phase::complement` | `fn complement(self) -> Phase` | `1.0 - p` + 自动量化 |
| `Phase::quantize` | `fn quantize(self, epsilon: f64) -> Phase` | 锚点吸附：0.5→0.0→1.0 |
| `TernaryAlgebra::precheck_same_frame` | `fn precheck_same_frame(&TritWord, &TritWord) -> bool` | O(1) Frame 检查 |
| `TernaryAlgebra::t_and` | `fn t_and(&TritWord, &TritWord) -> (TritWord, Option<MetaInterrupt>)` | 完整路径 TAND |
| `TernaryAlgebra::t_and_hot` | `fn t_and_hot(&TritWord, &TritWord) -> TritWord` | 热路径 TAND（要求同帧） |
| `TernaryAlgebra::t_or` | `fn t_or(&TritWord, &TritWord) -> (TritWord, Option<MetaInterrupt>)` | 完整路径 TOR |
| `TernaryAlgebra::t_or_hot` | `fn t_or_hot(&TritWord, &TritWord) -> TritWord` | 热路径 TOR（要求同帧） |
| `TernaryAlgebra::t_not` | `fn t_not(&TritWord) -> TritWord` | 相位翻转否定 |
| `TernaryAlgebra::t_hold` | `fn t_hold(&TritWord) -> TritWord` | 强制 Hold |
| `TernaryAlgebra::t_sense` | `fn t_sense(phase: f64, frame: Frame) -> TritWord` | 从原始传感器数据创建 Hold |

### 设计约束

- TAND/TOR/TNOT 真值表**不可变**（冻结，保证结果可复现）
- 所有运算不分配堆内存（除跨帧冲突的 MetaInterrupt）
- Hot path 使用 `debug_assert_eq!` 验证 Frame 一致（release 下零开销）

---

## `frame/` — 决策域定义

### 文件

| 文件 | 行数 | 职责 |
|---|---|---|
| `mod.rs` | 118 | Frame 枚举 + FrameRegistry |

### 关键类型

| 类型 | 说明 |
|---|---|
| `Frame` | 5 个变体：Science、Individual、Consensus、Absolute、Meta |
| `FrameRegistry` | 跟踪活跃帧的 Vec |

### 设计约束

- Frame 通过 `Display`/`FromStr` 实现双射（已验证往返一致性）
- `Absolute` 帧必须永远 Hold（由 MetaMonitor 强制执行）

---

## `meta/` — 策略引擎与仲裁

### 文件

| 文件 | 行数 | 职责 |
|---|---|---|
| `mod.rs` | 237 | 模块声明 + 重导出 + 21 个测试 |
| `domain.rs` | 108 | Domain 枚举 + ResolutionPolicy + ArbitrationResult |
| `frame_mask.rs` | 52 | O(1) u8 位掩码帧检测 |
| `interrupt.rs` | 85 | MetaInterrupt + ConflictType + MetaMonitor |
| `rules.rs` | 90 | CustomRule + RuleLoader 特质 + JsonRuleLoader |
| `safe_fallback.rs` | 124 | SafeFallback: IEC 61508 安全降级 |

### 关键函数

| 函数 | 签名 | 说明 |
|---|---|---|
| `FrameMask::from_inputs` | `fn from_inputs(&[TritWord]) -> FrameMask` | 单次遍历 O(n) 构建位掩码 |
| `FrameMask::has` | `fn has(&Frame) -> bool` | O(1) 位检测 |
| `ResolutionPolicy::arbitrate` | `fn arbitrate(&[TritWord]) -> ArbitrationResult` | 域仲裁核心 |
| `MetaMonitor::inspect` | `fn inspect(&TritWord) -> Option<MetaInterrupt>` | Absolute 帧不变性检查 |
| `SafeFallback::is_dangerous` | `fn is_dangerous(&Domain) -> bool` | 危险域判定 |
| `SafeFallback::guard` | `fn guard(&Domain, &TritWord, interrupt_count) -> (TritWord, Option<MetaInterrupt>)` | 安全降级主入口 |
| `JsonRuleLoader::load` | `fn load(path) -> Result<CustomRule, String>` | 从文件加载自定义规则 |
| `JsonRuleLoader::apply` | `fn apply(&CustomRule, &[TritWord]) -> ArbitrationResult` | 应用自定义规则 |

### 设计约束

- `FrameMask` 最多支持 8 种 Frame（u8 限制）
- `Domain::Custom` 的仲裁始终返回 `Negotiate`（实际仲裁由 `RuleLoader::apply` 完成）
- SafeFallback 的预置危险域列表在 `SafeFallback::new()` 中硬编码

---

## `net/` — 分布式协议（M4，存根）

### 文件

| 文件 | 行数 | 职责 |
|---|---|---|
| `mod.rs` | 27 | 模块声明 + 重导出 |
| `bus.rs` | 91 | ResonanceBus 消息总线 |
| `coupling.rs` | 209 | 耦合生命周期（RESONATE/DECOUPLE） |
| `message.rs` | 192 | 协议消息类型与构造器 |
| `negotiate.rs` | 109 | 多节点协商（单次遍历） |
| `node.rs` | 255 | Node 状态机 |
| `pll.rs` | 121 | 软件锁相环 |

### 关键函数

| 函数 | 签名 | 说明 |
|---|---|---|
| `Node::initiate_coupling` | `fn initiate_coupling(&mut self, peer_id: &str)` | Sovereign → Coupling |
| `Node::confirm_coupling` | `fn confirm_coupling(&mut self)` | Coupling → Coupled |
| `Node::decouple` | `fn decouple(&mut self)` | 任意状态 → Sovereign |
| `Node::interference_with` | `fn interference_with(&Node) -> Interference` | 帧兼容性检测 |
| `PllController::compute_correction` | `fn compute_correction(&mut self, local, peer) -> f64` | 比例校正（含死区） |
| `PllController::is_conflict_phase_gap` | `fn is_conflict_phase_gap(a, b) -> bool` | 冲突相位差检测 |
| `ResonanceBus::register` | `fn register(&mut self, node: Node)` | 节点注册（上限 MAX_NODES=256） |
| `ResonanceBus::handle_resonate_req` | `fn handle_resonate_req(&mut self, from, to, msg) -> Option<Message>` | 处理 RESONATE_REQ |
| `ResonanceBus::handle_resonate_ack` | `fn handle_resonate_ack(&mut self, node_id, ack)` | 处理 RESONATE_ACK |
| `ResonanceBus::handle_decouple_req` | `fn handle_decouple_req(&mut self, node_id, msg, cycles) -> Message` | 处理 DECOUPLE_REQ |
| `ResonanceBus::negotiate` | `fn negotiate(&mut self, participant_ids) -> (TritWord, bool)` | 单次遍历协商 |

### 设计约束

- 消息日志是有上限的（`MAX_MESSAGE_LOG = 10_000`），实现为 VecDeque 环形缓冲区
- PLL 参数（kp=0.3、deadband=0.05、max_correction=0.1）使用 ADR-004 中的值
- `net/` 模块是显式不稳定的——可能在 0.2.0 中进行重构

---

## `clock/` — 相位振荡器

### 文件

| 文件 | 行数 | 职责 |
|---|---|---|
| `clock.rs` | 83 | HarmonicClock 正弦振荡器 |

### 关键函数

| 函数 | 说明 |
|---|---|
| `HarmonicClock::tick(dt)` | 推进时间 dt，返回是否上升过零 |
| `HarmonicClock::phase_now()` | 返回 `sin(ω·t + φ₀)` |
| `HarmonicClock::physical()` | 快速时钟（ω=10.0） |
| `HarmonicClock::deliberative()` | 慢速时钟（ω=0.5） |

---

## `baseline/` — 二元基线对比

### 文件

| 文件 | 行数 | 职责 |
|---|---|---|
| `mod.rs` | 162 | BinaryBaseline 二元多数投票 |

### 关键函数

| 函数 | 说明 |
|---|---|
| `BinaryBaseline::evaluate(&[TritWord]) -> BinaryResult` | 二元多数投票（tie → False） |
| `BinaryBaseline::compare(&TritWord, &BinaryResult) -> BinaryResult` | 二元 vs 三元对比 |
| `BinaryBaseline::has_hidden_conflict(&[TritWord]) -> bool` | 检测二进制会忽略的跨帧冲突 |

---

## `sandbox/` — CLI 沙箱

### 文件

| 文件 | 行数 | 职责 |
|---|---|---|
| `sandbox.rs` | 33 | ScenarioInput + SandboxOutput 类型定义 |

### 二进制入口

| 文件 | 行数 | 说明 |
|---|---|---|
| `src/bin/sandbox.rs` | 230 | trit-sandbox CLI |
| `src/bin/node.rs` | 233 | trit-node CLI |
