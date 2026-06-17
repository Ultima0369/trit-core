# Trit-Core 公共 API 契约

**版本**：0.1.0  
**稳定性**：不稳定（alpha）  

---

## 1. `trit_core::trit`

### `TritValue`（枚举）
```rust
pub enum TritValue {
    True,    // +1
    Hold,    // 0
    False,   // -1
    Unknown, // ―（不可计算，表示缺失/未定义信号）
}
```
方法：
- `fn negate(self) -> TritValue`
- `fn to_i8(self) -> i8`
- `fn is_computable(self) -> bool` — Unknown 返回 false
- `Default` → `Hold`

### `Phase`（结构体）
```rust
pub struct Phase(f64);
```
常量：
- `Phase::NEUTRAL` = 0.5
- `Phase::FULL_TRUE` = 1.0
- `Phase::FULL_FALSE` = 0.0

方法：
- `fn new(v: f64) -> Phase`（越界则 panic）
- `fn inner(self) -> f64`
- `fn mean(a: Phase, b: Phase) -> Phase`
- `fn complement(self) -> Phase`
- `fn commitment(self) -> Commitment`（趋向真/趋向假/中立）

### `TritWord`（结构体）
```rust
pub struct TritWord {
    pub value: TritValue,
    pub phase: Phase,
    pub frame: Frame,
}
```
构造器：
- `fn new(value: TritValue, phase: f64, frame: Frame) -> TritWord`
- `fn hold(frame: Frame) -> TritWord`
- `fn tru(frame: Frame) -> TritWord`
- `fn fals(frame: Frame) -> TritWord`

### `TernaryAlgebra`（结构体）
```rust
pub struct TernaryAlgebra;
```
静态方法：
- `fn t_and(a: &TritWord, b: &TritWord) -> (TritWord, Option<MetaInterrupt>)`
- `fn t_or(a: &TritWord, b: &TritWord) -> (TritWord, Option<MetaInterrupt>)`
- `fn t_not(a: &TritWord) -> TritWord`
- `fn t_hold(a: &TritWord) -> TritWord`
- `fn t_sense(phase: f64, frame: Frame) -> TritWord`

---

## 2. `trit_core::frame`

### `Frame`（枚举）
```rust
pub enum Frame {
    Science,     // 科学域：实证数据
    Individual,  // 个体域：用户情境
    Consensus,   // 共识域：统计偏好
    Absolute,    // 绝对域：不可知（必须悬置）
    Meta,        // 元域：冲突裁决
}
```
实现：`Display`、`Clone`、`Debug`、`PartialEq`、`Eq`、`Hash`。

### `FrameRegistry`（结构体）
```rust
pub struct FrameRegistry;
```
方法：
- `fn new() -> FrameRegistry`
- `fn register(&mut self, frame: Frame)`
- `fn is_registered(&self, frame: &Frame) -> bool`

---

## 3. `trit_core::meta`

### `Domain`（枚举）
```rust
pub enum Domain {
    Physical,       // 物理约束：科学优先，允许强制坍缩
    Engineering,    // 工程约束：科学优先，允许强制坍缩
    MedicalEthics,  // 医疗伦理：个体优先，禁止强制坍缩
    ValueJudgment,  // 价值判断：无优先，必须悬置
    General,        // 通用：协商，失败则悬置
    Custom(String), // 用户自定义域
}
```

### `ResolutionPolicy`（结构体）
```rust
pub struct ResolutionPolicy {
    pub domain: Domain,
}
```
方法：
- `fn new(domain: Domain) -> ResolutionPolicy`
- `fn arbitrate(&self, inputs: &[TritWord]) -> ArbitrationResult`

### `ArbitrationResult`（枚举）
```rust
pub enum ArbitrationResult {
    Commit(TritWord),     // 选择此三态作为输出
    Preserve(TritWord),   // 保留个体主权
    ForceCollapse,        // 强制坍缩至最近确定态
    Hold,                 // 显式悬置
    Negotiate,            // 需外部干预
}
```

### `MetaInterrupt`（结构体）
```rust
pub struct MetaInterrupt {
    pub conflict: ConflictType,
    pub reason: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
```
构造器：`fn new(conflict: ConflictType, reason: String) -> MetaInterrupt`

### `ConflictType`（枚举）
```rust
pub enum ConflictType {
    FrameMismatch,       // 参考系不匹配
    OutOfScope,          // 超出有效范围
    PhaseDrift,          // 相位漂移
    PolicyViolation,     // 违反域规则
}
```

### `MetaMonitor`（结构体）
```rust
pub struct MetaMonitor;
```
方法：
- `fn new(policy: ResolutionPolicy) -> MetaMonitor`
- `fn record(&mut self, interrupt: MetaInterrupt)`
- `fn log(&self) -> &[MetaInterrupt]`
- `fn inspect(&self, word: &TritWord) -> Option<MetaInterrupt>`

### `SafeFallback`（结构体）
IEC 61508 故障安全语义。对危险域中产生 Hold + 中断的操作强制返回 False。
```rust
pub struct SafeFallback;
```
方法：
- `fn new() -> SafeFallback`
- `fn register_dangerous(&mut self, domain: &str)` — 注册危险域
- `fn is_dangerous(&self, domain: &str) -> bool`
- `fn is_enabled(&self) -> bool`
- `fn set_enabled(&mut self, enabled: bool)`
- `fn guard(&self, result, interrupts, domain) -> TritWord`

### `CustomRule`（结构体）
自定义域的仲裁规则。
```rust
pub struct CustomRule {
    pub name: String,
    pub domain: String,
    pub priority_frame: String,
    pub fallback_policy: String, // "hold" | "safe_fallback" | "negotiate"
}
```

### `RuleLoader`（特征）
```rust
pub trait RuleLoader {
    fn load(&self, source: &str) -> Result<Vec<CustomRule>, String>;
}
```

### `JsonRuleLoader`（结构体）
实现 `RuleLoader` 特征，从 JSON 加载自定义规则。

### `FrameMask`（结构体，pub(crate)）
内部 O(1) 位掩码，用于帧存在性检查。使用 u8，最多支持 8 种帧类型。

---

## 4. `trit_core::clock`

### `HarmonicClock`（结构体）
```rust
pub struct HarmonicClock;
```
方法：
- `fn new(omega: f64, phi0: f64) -> HarmonicClock`
- `fn tick(&mut self, dt: f64) -> bool`
- `fn phase_now(&self) -> f64`
- `fn physical() -> HarmonicClock`（快速时钟，ω=10.0）
- `fn deliberative() -> HarmonicClock`（慢速时钟，ω=0.5）

---

## 5. `trit_core::net` — 分布式协议（M4-M6）

### `ResonanceBus`（结构体）
内存消息总线，支持多节点模拟。最大 256 节点，10,000 条消息日志（环形缓冲区）。
```rust
pub struct ResonanceBus;
```
方法：
- `fn new() -> ResonanceBus`
- `fn register(&mut self, node: Node)`
- `fn get_node(&self, id: &str) -> Option<&Node>`
- `fn log(&self) -> Iter<Message>`
- `fn handle_resonate_req(&mut self, from, to, msg) -> Option<Message>`
- `fn handle_resonate_ack(&mut self, node_id, ack)`
- `fn handle_decouple_req(&mut self, node_id, msg, cycles) -> Message`
- `fn negotiate(&mut self, participants) -> (NegotiatePayload, bool)`

### `Message`（结构体）
协议消息，包含头部和类型化负载。
```rust
pub struct Message {
    pub header: MessageHeader,
    pub payload: MessagePayload,
}
```
构造器：
- `fn resonate_req(sender, frame, phase, history) -> Message`
- `fn resonate_ack(sender, coupled_phase, interference, conflict, recommendation) -> Message`
- `fn decouple_req(sender, reason) -> Message`
- `fn decouple_ack(sender, restored_phase, cycles) -> Message`
- `fn negotiate(sender, participants, frames, phases, result) -> Message`
- `fn heartbeat(sender, state, phase) -> Message`

### `MessageHeader`（结构体）
```rust
pub struct MessageHeader {
    pub proto: String,      // "TRIT/0.1"
    pub msg_id: String,     // UUID v4
    pub timestamp: String,  // ISO 8601
    pub sender: String,
}
```

### `OpCode`（枚举）
```rust
pub enum OpCode {
    ResonateReq, ResonateAck, DecoupleReq, DecoupleAck, Negotiate, Heartbeat,
}
```

### `MessagePayload`（枚举）
```rust
pub enum MessagePayload {
    ResonateReq(ResonateReq),
    ResonateAck(ResonateAck),
    DecoupleReq(DecoupleReq),
    DecoupleAck(DecoupleAck),
    Negotiate(NegotiatePayload),
    Heartbeat(HeartbeatPayload),
}
```

### 负载类型
- **`ResonateReq`**: `{ frame, phase, history }`
- **`ResonateAck`**: `{ coupled_phase, interference, conflict_detected, recommendation }`
- **`DecoupleReq`**: `{ reason }`
- **`DecoupleAck`**: `{ restored_phase, cycles_coupled }`
- **`NegotiatePayload`**: `{ participants, frames, phases, consensus_phase, conflict_resolution }`
- **`HeartbeatPayload`**: `{ node_state, current_phase }`

### `Node`（结构体）
节点状态机和身份信息。
```rust
pub struct Node {
    pub id: String,
    pub frame: Frame,
    pub current_phase: f64,
    pub sovereign_phase: f64,
    pub state: NodeState,
    pub peers: Vec<String>,
    pub cycles_coupled: u64,
    pub interrupts: Vec<String>,
}
```
方法：`new()`, `initiate_coupling()`, `confirm_coupling()`, `decouple()`, `adjust_phase()`, `tick()`, `to_trit()`

### `NodeState`（枚举）
```rust
pub enum NodeState { Sovereign, Coupling, Coupled, Hold }
```

### `Interference`（枚举）
```rust
pub enum Interference { Constructive, Neutral, Destructive }
```

### `PllController`（结构体）
软件锁相环。kp=0.3，deadband=0.05，max_correction=0.1。
方法：`new()`, `compute_correction()`, `reset()`, `is_conflict_phase_gap()`, `is_phase_jump_anomaly()`

### `TcpNodeServer`（结构体）
TCP 传输服务器，接受连接并将消息分发至 ResonanceBus。
方法：`new()`, `with_bus()`, `bus_handle()`, `serve()`

### `TcpClient`（结构体）
TCP 客户端连接器，用于远程节点通信。
方法：`connect()`, `resonate()`, `decouple()`, `heartbeat()`, `negotiate()`

### `discovery` 模块
种子对等节点发现：
- `fn parse_seeds(seeds: &str) -> Vec<String>` — 解析逗号分隔的 `host:port` 列表
- `async fn bootstrap(bus, local_node_id, seeds) -> usize` — 连接种子节点，交换心跳

### `frame_codec` 模块
TCP 长度前缀帧协议（4 字节大端长度 + JSON 负载，最大 1 MiB）：
- `const MAX_FRAME_SIZE: usize = 1_048_576`
- `async fn read_frame(reader) -> io::Result<Vec<u8>>`
- `async fn write_frame(writer, payload) -> io::Result<()>`

---

## 6. `trit_core::sandbox`

### `ScenarioInput`（结构体）
支持 Serde 从 JSON 反序列化。

### `SandboxOutput`（结构体）
支持 Serde 序列化为 JSON。

---

## 稳定性保证（MVP）

- **0.1.x 内无破坏性变更**：一旦发布，补丁版仅增场景与文档。
- **0.2.0 可能重构**：`sandbox` 与 `net` 模块明确不稳定。
- **核心代数（`trit/`）M1 后冻结**：TAND、TOR、TNOT 语义固定，确保可复现性。

---

## 示例代码

```rust
use trit_core::trit::{TernaryAlgebra, TritWord, TritValue};
use trit_core::frame::Frame;

let science = TritWord::tru(Frame::Science);
let individual = TritWord::fals(Frame::Individual);

let (result, interrupt) = TernaryAlgebra::t_and(&science, &individual);

assert_eq!(result.value, TritValue::Hold);
assert!(interrupt.is_some());
```
