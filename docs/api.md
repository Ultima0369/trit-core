# Trit-Core Public API Contract

**Version**: 0.1.0  
**Stability**: Unstable (alpha)  

---

## 1. `trit_core::trit`

### `TritValue` (enum)
```rust
pub enum TritValue {
    True,   // +1
    Hold,   // 0
    False,  // -1
}
```
Methods:
- `fn negate(self) -> TritValue`
- `fn to_i8(self) -> i8`
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

---

## 4. `trit_core::clock` (MVP stub)

### `HarmonicClock` (struct)
```rust
pub struct HarmonicClock;
```
Methods:
- `fn new(omega: f64, phi0: f64) -> HarmonicClock`
- `fn tick(&mut self, dt: f64) -> bool`
- `fn phase_now(&self) -> f64`
- `fn physical() -> HarmonicClock` (fast)
- `fn deliberative() -> HarmonicClock` (slow)

---

## 5. `trit_core::sandbox` (MVP stub)

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
