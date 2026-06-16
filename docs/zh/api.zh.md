# Trit-Core 公共 API 契约

**版本**：0.1.0  
**稳定性**：不稳定（alpha）  

---

## 1. `trit_core::trit`

### `TritValue`（枚举）
```rust
pub enum TritValue {
    True,   // +1
    Hold,   // 0
    False,  // -1
}
```
方法：
- `fn negate(self) -> TritValue`
- `fn to_i8(self) -> i8`
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

---

## 4. `trit_core::clock`（MVP 占位）

### `HarmonicClock`（结构体）
```rust
pub struct HarmonicClock;
```
方法：
- `fn new(omega: f64, phi0: f64) -> HarmonicClock`
- `fn tick(&mut self, dt: f64) -> bool`
- `fn phase_now(&self) -> f64`
- `fn physical() -> HarmonicClock`（快速时钟）
- `fn deliberative() -> HarmonicClock`（慢速时钟）

---

## 5. `trit_core::sandbox`（MVP 占位）

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
