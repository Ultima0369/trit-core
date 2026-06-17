# Trit-Core Public API Contract

**Version**: 0.1.0  
**Stability**: Unstable (alpha)  

---

## 1. `trit_core::trit`

### `TritValue` (enum)
```rust
pub enum TritValue {
    True,    // +1
    Hold,    // 0
    False,   // -1
    Unknown, // ― (non-computable, represents missing/undefined signal)
}
```
Methods:
- `fn negate(self) -> TritValue`
- `fn to_i8(self) -> i8`
- `fn is_computable(self) -> bool` — returns false for Unknown
- `Default` → `Hold`

### `Phase` (struct)
```rust
pub struct Phase(f64);
```
Constants:
- `Phase::NEUTRAL` = 0.5
- `Phase::FULL_TRUE` = 1.0
- `Phase::FULL_FALSE` = 0.0

Methods:
- `fn new(v: f64) -> Phase` (panics if out of [0.0, 1.0])
- `fn inner(self) -> f64`
- `fn mean(a: Phase, b: Phase) -> Phase`
- `fn complement(self) -> Phase`
- `fn commitment(self) -> Commitment` (TowardTrue / TowardFalse / Neutral)

### `TritWord` (struct)
```rust
pub struct TritWord {
    pub value: TritValue,
    pub phase: Phase,
    pub frame: Frame,
}
```
Constructors:
- `fn new(value: TritValue, phase: f64, frame: Frame) -> TritWord`
- `fn hold(frame: Frame) -> TritWord`
- `fn tru(frame: Frame) -> TritWord`
- `fn fals(frame: Frame) -> TritWord`

### `TernaryAlgebra` (struct)
```rust
pub struct TernaryAlgebra;
```
Static methods:
- `fn t_and(a: &TritWord, b: &TritWord) -> (TritWord, Option<MetaInterrupt>)`
- `fn t_or(a: &TritWord, b: &TritWord) -> (TritWord, Option<MetaInterrupt>)`
- `fn t_not(a: &TritWord) -> TritWord`
- `fn t_hold(a: &TritWord) -> TritWord`
- `fn t_sense(phase: f64, frame: Frame) -> TritWord`

---

## 2. `trit_core::frame`

### `Frame` (enum)
```rust
pub enum Frame {
    Science,
    Individual,
    Consensus,
    Absolute,
    Meta,
}
```
Implements `Display`, `Clone`, `Debug`, `PartialEq`, `Eq`, `Hash`.

### `FrameRegistry` (struct)
```rust
pub struct FrameRegistry;
```
Methods:
- `fn new() -> FrameRegistry`
- `fn register(&mut self, frame: Frame)`
- `fn is_registered(&self, frame: &Frame) -> bool`

---

## 3. `trit_core::meta`

### `Domain` (enum)
```rust
pub enum Domain {
    Physical,
    Engineering,
    MedicalEthics,
    ValueJudgment,
    General,
    Custom(String),
}
```

### `ResolutionPolicy` (struct)
```rust
pub struct ResolutionPolicy {
    pub domain: Domain,
}
```
Methods:
- `fn new(domain: Domain) -> ResolutionPolicy`
- `fn arbitrate(&self, inputs: &[TritWord]) -> ArbitrationResult`

### `ArbitrationResult` (enum)
```rust
pub enum ArbitrationResult {
    Commit(TritWord),
    Preserve(TritWord),
    ForceCollapse,
    Hold,
    Negotiate,
}
```

### `MetaInterrupt` (struct)
```rust
pub struct MetaInterrupt {
    pub conflict: ConflictType,
    pub reason: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
```
Constructor: `fn new(conflict: ConflictType, reason: String) -> MetaInterrupt`

### `ConflictType` (enum)
```rust
pub enum ConflictType {
    FrameMismatch,
    OutOfScope,
    PhaseDrift,
    PolicyViolation,
}
```

### `MetaMonitor` (struct)
```rust
pub struct MetaMonitor;
```
Methods:
- `fn new(policy: ResolutionPolicy) -> MetaMonitor`
- `fn record(&mut self, interrupt: MetaInterrupt)`
- `fn log(&self) -> &[MetaInterrupt]`
- `fn inspect(&self, word: &TritWord) -> Option<MetaInterrupt>`

### `SafeFallback` (struct)
IEC 61508 fail-safe semantics. Forces False on dangerous-domain operations that produce Hold with active interrupts.
```rust
pub struct SafeFallback;
```
Methods:
- `fn new() -> SafeFallback`
- `fn register_dangerous(&mut self, domain: &str)` — register a domain as dangerous
- `fn is_dangerous(&self, domain: &str) -> bool`
- `fn is_enabled(&self) -> bool`
- `fn set_enabled(&mut self, enabled: bool)`
- `fn guard(&self, result: &TritWord, interrupts: &[MetaInterrupt], domain: &Domain) -> TritWord`

### `CustomRule` (struct)
User-defined arbitration rule for custom domains.
```rust
pub struct CustomRule {
    pub name: String,
    pub domain: String,
    pub priority_frame: String,
    pub fallback_policy: String, // "hold" | "safe_fallback" | "negotiate"
}
```

### `RuleLoader` (trait)
```rust
pub trait RuleLoader {
    fn load(&self, source: &str) -> Result<Vec<CustomRule>, String>;
}
```

### `JsonRuleLoader` (struct)
Implements `RuleLoader` for JSON-format custom rules.

### `FrameMask` (struct, pub(crate))
Internal O(1) bitmask for frame presence checks. Uses u8, supports up to 8 frame types.

---

## 4. `trit_core::clock`

### `HarmonicClock` (struct)
```rust
pub struct HarmonicClock;
```
Methods:
- `fn new(omega: f64, phi0: f64) -> HarmonicClock`
- `fn tick(&mut self, dt: f64) -> bool`
- `fn phase_now(&self) -> f64`
- `fn physical() -> HarmonicClock` (fast, ω=10.0)
- `fn deliberative() -> HarmonicClock` (slow, ω=0.5)

---

## 5. `trit_core::net` — Distributed Protocol (M4-M6)

### `ResonanceBus` (struct)
In-memory message bus for multi-node simulation. Max 256 nodes, 10,000 message log (ring buffer).
```rust
pub struct ResonanceBus;
```
Methods:
- `fn new() -> ResonanceBus`
- `fn register(&mut self, node: Node)`
- `fn get_node(&self, id: &str) -> Option<&Node>`
- `fn log(&self) -> Iter<Message>`
- `fn handle_resonate_req(&mut self, from: &str, to: &str, msg: &Message) -> Option<Message>`
- `fn handle_resonate_ack(&mut self, node_id: &str, ack: &Message)`
- `fn handle_decouple_req(&mut self, node_id: &str, msg: &Message, cycles: u64) -> Message`
- `fn negotiate(&mut self, participants: &[String]) -> (NegotiatePayload, bool)`

### `Message` (struct)
Protocol message with header + typed payload.
```rust
pub struct Message {
    pub header: MessageHeader,
    pub payload: MessagePayload,
}
```
Constructors:
- `fn resonate_req(sender, frame, phase, history) -> Message`
- `fn resonate_ack(sender, coupled_phase, interference, conflict, recommendation) -> Message`
- `fn decouple_req(sender, reason) -> Message`
- `fn decouple_ack(sender, restored_phase, cycles) -> Message`
- `fn negotiate(sender, participants, frames, phases, result) -> Message`
- `fn heartbeat(sender, state, phase) -> Message`

### `MessageHeader` (struct)
```rust
pub struct MessageHeader {
    pub proto: String,      // "TRIT/0.1"
    pub msg_id: String,     // UUID v4
    pub timestamp: String,  // ISO 8601
    pub sender: String,
}
```

### `OpCode` (enum)
```rust
pub enum OpCode {
    ResonateReq, ResonateAck, DecoupleReq, DecoupleAck, Negotiate, Heartbeat,
}
```

### `MessagePayload` (enum)
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

### Payload types
- **`ResonateReq`**: `{ frame: String, phase: f64, history: Vec<f64> }`
- **`ResonateAck`**: `{ coupled_phase: f64, interference: String, conflict_detected: bool, recommendation: String }`
- **`DecoupleReq`**: `{ reason: String }`
- **`DecoupleAck`**: `{ restored_phase: f64, cycles_coupled: u64 }`
- **`NegotiatePayload`**: `{ participants, frames, phases, consensus_phase, conflict_resolution }`
- **`HeartbeatPayload`**: `{ node_state: String, current_phase: f64 }`

### `Node` (struct)
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
Methods:
- `fn new(id, frame, phase) -> Node`
- `fn initiate_coupling(&mut self, peer: &str)`
- `fn confirm_coupling(&mut self)`
- `fn decouple(&mut self)`
- `fn adjust_phase(&mut self, delta: f64)`
- `fn tick(&mut self)`
- `fn to_trit(&self) -> TritWord`

### `NodeState` (enum)
```rust
pub enum NodeState { Sovereign, Coupling, Coupled, Hold }
```

### `Interference` (enum)
```rust
pub enum Interference { Constructive, Neutral, Destructive }
```

### `PllController` (struct)
Software phase-locked loop. kp=0.3, deadband=0.05, max_correction=0.1.
Methods:
- `fn new() -> PllController`
- `fn compute_correction(&mut self, local: f64, peer: f64) -> f64`
- `fn reset(&mut self)`
- `fn is_conflict_phase_gap(a: f64, b: f64) -> bool` — gap > 0.3
- `fn is_phase_jump_anomaly(old: f64, new: f64) -> bool` — delta > 0.5

### `TcpNodeServer` (struct)
TCP transport server accepting connections and dispatching messages to ResonanceBus.
Methods:
- `fn new(bind_addr) -> TcpNodeServer`
- `fn with_bus(bind_addr, bus) -> TcpNodeServer`
- `fn bus_handle(&self) -> Arc<Mutex<ResonanceBus>>`
- `async fn serve(&self) -> io::Result<()>`

### `TcpClient` (struct)
TCP client connector for remote node communication.
Methods:
- `async fn connect(addr) -> io::Result<TcpClient>`
- `async fn resonate(&mut self, node_id, frame, phase, history) -> io::Result<Message>`
- `async fn decouple(&mut self, node_id, reason, cycles) -> io::Result<Message>`
- `async fn heartbeat(&mut self, node_id, state, phase) -> io::Result<Message>`
- `async fn negotiate(&mut self, node_id, participants, frames, phases) -> io::Result<Message>`

### `discovery` module
Seed-based peer discovery functions:
- `fn parse_seeds(seeds: &str) -> Vec<String>` — parse comma-separated `host:port` list
- `async fn bootstrap(bus, local_node_id, seeds) -> usize` — connect to seeds, exchange heartbeats

### `frame_codec` module
TCP length-prefix framing protocol (4-byte BE length + JSON payload, max 1 MiB):
- `const MAX_FRAME_SIZE: usize = 1_048_576`
- `async fn read_frame<R: AsyncRead + Unpin>(reader: &mut R) -> io::Result<Vec<u8>>`
- `async fn write_frame<W: AsyncWrite + Unpin>(writer: &mut W, payload: &[u8]) -> io::Result<()>`

---

## 6. `trit_core::sandbox`

### `ScenarioInput` (struct)
Serde-deserializable from JSON.

### `SandboxOutput` (struct)
Serde-serializable to JSON.

---

## Stability Guarantees (MVP)

- **No breaking changes within 0.1.x**: Once a crate is published, 0.1.x patch releases add only new scenarios and docs.
- **0.2.0 may refactor**: The `sandbox` and `net` modules are explicitly unstable.
- **Core algebra (`trit/`) is frozen after M1**: TAND, TOR, TNOT semantics are fixed for reproducibility.

---

## Example Usage

```rust
use trit_core::trit::{TernaryAlgebra, TritWord, TritValue};
use trit_core::frame::Frame;

let science = TritWord::tru(Frame::Science);
let individual = TritWord::fals(Frame::Individual);

let (result, interrupt) = TernaryAlgebra::t_and(&science, &individual);

assert_eq!(result.value, TritValue::Hold);
assert!(interrupt.is_some());
```
