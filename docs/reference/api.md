# Trit-Core Public API Contract

**Version**: 0.3.0  
**Stability**: Core algebra semantics are stable. 0.3.0 added structured logging, diagnostics, and CLI observability flags; expect further evolution in the `sandbox` and `tracing_init` surfaces.

---

## 1. `trit_core::core`

### `TritValue` (enum)

```rust
pub enum TritValue {
    True,    // +1
    Hold,    // 0
    False,   // -1
    Unknown, // ⊥ — out-of-distribution, not computable
}
```

Methods:
- `fn negate(self) -> TritValue`
- `fn to_i8(self) -> i8`
- `fn is_computable(self) -> bool`
- `fn discriminant(self) -> u8`
- `Default` → `Hold`

Conversion:
- `From<i8>` maps `1→True`, `-1→False`, otherwise `Hold`.
- `TritValue::from_i8_strict(v: i8) -> Result<Self, &'static str>` only accepts `-1, 0, 1`.

### `Phase` (struct)

Wraps a finite `f64` in `[0.0, 1.0]`.

Constants:
- `Phase::NEUTRAL` = 0.5
- `Phase::FULL_TRUE` = 1.0
- `Phase::FULL_FALSE` = 0.0

Methods:
- `fn new(v: f64) -> Result<Phase, PhaseError>` — strict constructor
- `fn new_clamped(v: f64) -> Phase` — silent normalization with `tracing::warn`
- `const fn neutral() -> Phase` — constant 0.5 phase
- `const fn full_true() -> Phase` — constant 1.0 phase
- `const fn full_false() -> Phase` — constant 0.0 phase
- `fn inner(self) -> f64`
- `fn mean(a: Phase, b: Phase) -> Phase`
- `fn complement(self) -> Phase`
- `fn quantize(self, epsilon: f64) -> Phase`
- `fn commitment(self) -> Commitment`

### `Commitment` (enum)

```rust
pub enum Commitment {
    TowardTrue,
    TowardFalse,
    Neutral,
}
```

### `Frame` (enum)

```rust
pub enum Frame {
    Science,
    Individual,
    Consensus,
    Absolute,
    Meta,
    FirstPerson,
    Embodied,
    Relational,
    GeoEco,
    Developmental,
    Role,
    Environmental,
    Instrumental,
}
```

13 variants (see `src/core/frame.rs`). Trit-Core base 8 + Aurora extension 5 (ADR-004 + ADR-005, implemented). `Meta` is system-internal, not valid for external signal input.

Implements `Display`, `FromStr`, `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`, `Hash`.

### `FrameError` (enum)

Returned when `Frame::from_str` receives an unknown frame name.

### `FrameMask` (struct, `pub(crate)`)

Frame presence tracked as `u16` bitmask. See `src/meta/frame_mask.rs`. No public `FrameRegistry` struct exists — use `FrameMask::from_inputs(&[TritWord])` for frame presence checks.

### `TritWord` (struct)

The fundamental computation unit. Fields are **private**; invariants are enforced by constructors.

Both `TritWord` and `Frame` are `Copy`, so lightweight value semantics are used throughout the pipeline.

Constructors:
- `fn new(value: TritValue, phase: Phase, frame: Frame) -> Self`
- `fn try_new(value: TritValue, phase: f64, frame: &str) -> Result<Self, WordError>`
- `fn from_parts(value: TritValue, phase: Phase, frame: Frame) -> Result<Self, WordError>`
- `fn hold(frame: Frame) -> Self`
- `fn tru(frame: Frame) -> Self`
- `fn fals(frame: Frame) -> Self`
- `fn unknown(frame: Frame) -> Self`
- `fn absolute() -> Self` — always `Hold` + neutral phase + `Frame::Absolute`

Accessors:
- `fn value(&self) -> TritValue`
- `fn phase(&self) -> Phase`
- `fn frame(&self) -> Frame`

Transformers (preserve invariants):
- `fn with_value(&self, value: TritValue) -> Result<Self, WordError>`
- `fn with_phase(&self, phase: Phase) -> Result<Self, WordError>`
- `fn with_frame(&self, frame: Frame) -> Result<Self, WordError>`
- `fn invariant_holds(&self) -> bool`

### `WordError` (enum)

```rust
pub enum WordError {
    Phase(PhaseError),
    Frame(FrameError),
    AbsoluteInvariant,
}
```

### `TernaryAlgebra` (struct)

Static methods:
- `fn t_and(a: &TritWord, b: &TritWord) -> (TritWord, Option<MetaInterrupt>)`
- `fn t_or(a: &TritWord, b: &TritWord) -> (TritWord, Option<MetaInterrupt>)`
- `fn t_not(a: &TritWord) -> TritWord`
- `fn t_hold(a: &TritWord) -> TritWord`
- `fn t_sense(phase: f64, frame: Frame) -> Result<TritWord, PhaseError>` — returns `Err` on invalid phase
- `fn t_sense_clamped(phase: f64, frame: Frame) -> TritWord` — clamps invalid phase
- `fn precheck_same_frame(a: &TritWord, b: &TritWord) -> bool`
- `fn t_and_hot(a: &TritWord, b: &TritWord) -> TritWord` — panics if frames differ
- `fn t_or_hot(a: &TritWord, b: &TritWord) -> TritWord` — panics if frames differ
- `fn t_and_n(words: &[TritWord]) -> (TritWord, Vec<MetaInterrupt>)` — batch TAND with equal-weight phase averaging; used by the sandbox pipeline to avoid left-fold bias for 3+ signals

---

## 2. `trit_core::meta`

### `Domain` (enum)

```rust
pub enum Domain {
    Physical,
    Engineering,
    MedicalEthics,
    ValueJudgment,
    General,
    Custom(String),
    Organizational,
    Relational,
    Cognitive,
    Environmental,
    Climate,
}
```

11 variants (see `src/meta/domain.rs`). Trit-Core base 6 + Aurora extension 5 (ADR-004 + ADR-005, implemented).

### `ResolutionPolicy` (struct)

```rust
pub struct ResolutionPolicy {
    pub domain: Domain,
    pub custom_rule: Option<CustomRule>,
}
```

Methods:
- `fn new(domain: Domain) -> Self`
- `fn with_custom_rule(self, rule: CustomRule) -> Self`
- `fn arbitrate(&self, inputs: &[TritWord]) -> Result<ArbitrationResult, PolicyError>`

### `ArbitrationResult` (enum)

```rust
pub enum ArbitrationResult {
    Commit(TritWord),
    Preserve(TritWord),
    ForceCollapse,
    Hold,
    Negotiate,
    DryRun,
}
```

6 variants (see `src/meta/domain.rs:311`). `DryRun` = arbitration skipped on purpose.

### `PolicyError` (enum)

```rust
pub enum PolicyError {
    EmptyInputs,
    CustomRule(String),
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

Constructor: `fn new(conflict: ConflictType, reason: String) -> Self`

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

Methods:
- `fn new() -> Self`
- `fn record(&mut self, interrupt: MetaInterrupt)`
- `fn log(&self) -> impl Iterator<Item = &MetaInterrupt>`
- `fn drain_log(&mut self) -> Vec<MetaInterrupt>`
- `fn inspect(&self, word: &TritWord) -> Option<MetaInterrupt>`
- `fn inspect_all(&self, words: &[TritWord]) -> Vec<MetaInterrupt>`

### `SafeFallback` (struct)

IEC 61508-style fail-safe override.

Methods:
- `fn new() -> Self`
- `fn disabled() -> Self`
- `fn register_dangerous(&mut self, domain: &str)`
- `fn with_dangerous_domain(self, domain: impl Into<String>) -> Self`
- `fn enabled(self, enabled: bool) -> Self`
- `fn is_dangerous(&self, domain: &Domain) -> bool`
- `fn guard(&self, domain: &Domain, result: &TritWord, interrupt_count: usize) -> (TritWord, Option<MetaInterrupt>)`

### `FallbackBehavior` (enum)

```rust
pub enum FallbackBehavior {
    Hold,
    Negotiate,
    CommitFirst,
    SafeFallback,
}
```

Type-safe fallback behavior for custom rules, replacing the previous string-based field. Serializes as `hold`, `negotiate`, `commit_first`, `safe_fallback` via `#[serde(rename_all = "snake_case")]`.

Implements `Display`, `FromStr`, `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`, `Serialize`, `Deserialize`.

### `CustomRule` (struct)

```rust
pub struct CustomRule {
    pub name: String,
    pub priority_frame: Option<String>,
    pub allow_forced_collapse: bool,
    pub fallback: FallbackBehavior,
}
```

### `RuleLoader` (trait)

```rust
pub trait RuleLoader {
    type Error: std::fmt::Display;
    fn load<P: AsRef<Path>>(path: P) -> Result<CustomRule, Self::Error>;
    fn load_json(json: &str) -> Result<CustomRule, Self::Error>;
    fn apply(rule: &CustomRule, inputs: &[TritWord]) -> ArbitrationResult;
}
```

### `JsonRuleLoader` (struct)

Implements `RuleLoader` for JSON-format rules. `type Error = RuleError`.

### `RuleError` (enum)

```rust
pub enum RuleError {
    Read(String),
    Parse(String),
}
```

---

## 3. `trit_core::sandbox`

### `ScenarioInput` / `SignalInput` (structs)

Serde-deserializable JSON input types.

### `SandboxOutput` (struct)

Serde-serializable JSON output type.

Fields (as serialized):
- `scenario_id: String`
- `final_value: String` — `"True"`, `"Hold"`, `"False"`, or `"Unknown"`
- `final_value_code: i8` — `1`=True, `0`=Hold/Unknown (use `final_value` string to distinguish), `-1`=False
- `final_frame: String`
- `final_phase: f64` — serialized name for the internal `final_phase_raw` field; always in `[0.0, 1.0]`
- `interrupts: Vec<String>`
- `policy_action: String`
- `reflexive_alert: Option<String>` — present when `--reflexive` triggers a guard
- `attention_cmd: Option<String>` — present when an attention scheduler suggests an action
- `receiver_estimate: Option<ReceiverEstimate>` — present when `--self-knowledge` is enabled
- `hold_state: Option<HoldState>` — present when the final value is `Hold`

Methods:
- `fn is_commit_true(&self) -> bool`
- `fn is_commit_false(&self) -> bool`
- `fn is_hold(&self) -> bool`
- `fn final_phase(&self) -> Phase`
- `fn final_trit_value(&self) -> TritValue`

### `SandboxPipeline` (struct)

```rust
impl SandboxPipeline {
    pub fn new() -> Self;
    pub fn with_registry(registry: FrameRegistry) -> Self;
    pub fn with_dry_run(self, dry_run: bool) -> Self;
    pub fn with_reflexive(self, auditor: ReflexiveAuditor) -> Self;
    pub fn with_attention(self, scheduler: AttentionScheduler) -> Self;
    pub fn with_self_knowledge(self, knowledge: SelfKnowledge) -> Self;
    pub fn with_holder_config(self, config: HolderConfig) -> Self;
    pub fn with_trace_phase(self, enabled: bool) -> Self;
    pub fn with_hold_final(self, enabled: bool) -> Self;
    pub fn run(&mut self, scenario: &ScenarioInput) -> Result<SandboxOutput, SandboxError>;
    pub fn run_with_diagnostics(&mut self, scenario: &ScenarioInput)
        -> Result<(SandboxOutput, SandboxDiagnostics), SandboxError>;
}
```

`run_with_diagnostics` is the primary observable entry point: it runs the full pipeline and returns both the output and a `SandboxDiagnostics` telemetry record.

### `SandboxDiagnostics` (struct)

Runtime telemetry collector. This type is **output-only** and is not intended to be deserialized; `started_at` is serialized as approximate epoch millis for human readability. Fields include:
- `started_at: Option<Instant>` (serialized as approximate epoch millis)
- `elapsed_ns: u64`
- `signal_count: usize`
- `frame_distribution: HashMap<String, usize>`
- `interrupt_count: usize`
- `interrupt_types: Vec<String>`
- `policy_action: String`
- `safe_fallback_triggered: bool`
- `stage_timings_ns: HashMap<String, u64>`

Methods:
- `fn elapsed_us(&self) -> u64`

### `ErrorCategory` (enum)

```rust
pub enum ErrorCategory {
    Input,
    Security,
    Internal,
    Validation,
    Io,
}
```

Returned by `SandboxError::category()`. Each error also provides `category_name()` and `help()` for actionable reporting. Path-traversal attempts are classified as `Security`.

### `ScenarioValidator` (struct)

```rust
impl ScenarioValidator {
    pub fn validate(output: &SandboxOutput, expected_behavior: &str) -> Result<(), SandboxError>;
}
```

Supported `expected_behavior` values: `"hold"`, `"commit_true"`, `"commit_false"`, `"negotiate"`.

### `SandboxError` (enum)

Unified error type for validation and pipeline failures. Provides:
- `fn category(&self) -> ErrorCategory`
- `fn category_name(&self) -> &'static str`
- `fn help(&self) -> String`
- `fn report(&self) -> String`

### Validation limits (re-exported constants)

- `MAX_JSON_SIZE: usize = 64 * 1024`
- `MAX_SIGNALS: usize = 100`
- `MAX_STRING_LEN: usize = 1024`

Functions:
- `fn validate_scenario(scenario: &ScenarioInput) -> Result<(), SandboxError>`
- `fn validate_signal(index: usize, signal: &SignalInput) -> Result<(), SandboxError>`
- `fn sanitize_log_field(s: &str) -> String`

### `SensorSignal` / `EnvironmentalContext` (structs)

Mind-engineering input types for multi-modal signals.

`SensorSignal` variants: `BodyState(BodyState)`, `Environmental(EnvSnapshot)`, `Cognitive(CogState)`, `Text(TextInput)`.

`EnvironmentalContext` carries `bandwidth`, `noise_level`, `social_density`, and `time_pressure` to help attention scheduling.

### `HoldState` / `HolderConfig` (structs)

`HoldState` describes whether a Hold is `Awaiting` further information or `Final`. `HolderConfig` allows per-domain Hold semantics.

---

## 3.1 `trit_core::reflexive`

### `ReflexiveAuditor` (struct)

Records interrupt history and phase shifts to audit whether a forced True/False decision is justified.

- `fn new() -> Self`
- `fn record_interrupt(&mut self, interrupt: MetaInterrupt)`
- `fn record_phase_shift(&mut self, shift: PhaseShift)`
- `fn auto_post_audit(&self, output: &SandboxOutput) -> Option<ReflexiveAlert>`
- `fn audit_last_decision(&self) -> AuditReport`
- `fn reflexive_posture(&self) -> ReflexivePosture`

### `ReflexiveAlert` (struct)

`reason` + `recommendation` pair emitted when the guard fires.

### `AuditReport` (enum)

`Clean`, `ForcedCollapse`, `ExplanationImpulse`, or `HoldOutput`.

### `ReflexivePosture` (enum)

`Proceed`, `Hold`, `Recalibrate`.

---

## 3.2 `trit_core::attention`

### `AttentionScheduler` (struct)

Suggests attention commands based on signal load, bandwidth, and embodied state.

- `fn new() -> Self`
- `fn record_event(&mut self, event: AttentionEvent)`
- `fn suggest_reprioritization(&self, inputs: &[TritWord]) -> AttentionCmd`

### `AttentionCmd` (enum)

`HoldCurrent`, `ZoomIn`, `WidenScope`, `Recalibrate`, `BodyShift`.

### `AttentionEvent` (struct)

Timestamped event with a label and load contribution.

---

## 3.3 `trit_core::knowledge`

### `SelfKnowledge` (struct)

A minimal self-model of response patterns and trigger signatures.

- `fn new() -> Self`
- `fn with_human_defaults() -> Self`
- `fn add_pattern(&mut self, pattern: ResponsePattern)`
- `fn add_trigger(&mut self, trigger: TriggerSignature)`
- `fn infer_receiver_state(&self, input: &TritWord) -> ReceiverEstimate`

### `ReceiverEstimate` (struct)

Estimated receiver value, phase, confidence, and attended frames.

---

## 4. `trit_core::clock`

### `HarmonicClock` (struct)

- `fn new(omega: f64, phi0: f64) -> Self`
- `fn tick(&mut self, dt: f64) -> bool`
- `fn phase_now(&self) -> f64`
- `fn physical() -> Self`
- `fn deliberative() -> Self`
- `fn to_phase(&self) -> Phase`

---

## 5. `trit_core::tracing_init`

Structured logging initialization, used by `trit-sandbox` and `dhat-profile`.

### `LogFormat` (enum)

```rust
pub enum LogFormat {
    Pretty,
    Compact,
    Full,
    Json, // default
}
```

Implements `FromStr` and `std::str::FromStr`.

### `LogOptions` (struct)

```rust
pub struct LogOptions {
    pub filter: String,
    pub format: LogFormat,
    pub file: Option<std::path::PathBuf>,
    pub span_events: bool,
}
```

Builder-style helpers:
- `fn from_env() -> Self`
- `fn with_filter(self, filter: impl Into<String>) -> Self`
- `fn with_format(self, format: LogFormat) -> Self`
- `fn with_file(self, path: impl AsRef<Path>) -> Self`

### Functions

- `fn init()` — initialize from environment variables (`TRIT_LOG`, `TRIT_LOG_FILE`, `TRIT_LOG_FORMAT`, `TRIT_LOG_JSON`)
- `fn init_with_opts(opts: LogOptions) -> Result<(), String>` — programmatic initialization with file or stderr output

---

## 6. `trit_core::baseline`

### `BinaryBaseline` / `BinaryResult`

Majority-rule binary comparator used for M2 validation.

---

## Stability Guarantees

- **0.3.0**: Core ternary algebra semantics (TAND/TOR/TNOT truth tables, `Phase` invariants, `TritWord` construction) are stable.
- **Sandbox and observability surfaces** (`SandboxPipeline`, `SandboxDiagnostics`, `LogOptions`, `tracing_init`) may continue to evolve in 0.3.x.
- **Network/distributed protocol** is not part of 0.3.0; planned as a separate crate in the future.

---

## Example Usage

```rust
use trit_core::core::{Frame, TernaryAlgebra, TritValue, TritWord};

let science = TritWord::tru(Frame::Science);
let individual = TritWord::fals(Frame::Individual);

let (result, interrupt) = TernaryAlgebra::t_and(&science, &individual);

assert_eq!(result.value(), TritValue::Hold);
assert!(interrupt.is_some());
```

For three or more signals, prefer the bias-free batch cascade:

```rust
let words = vec![
    TritWord::tru(Frame::Science),
    TritWord::fals(Frame::Individual),
    TritWord::tru(Frame::Consensus),
];
let (result, interrupts) = TernaryAlgebra::t_and_n(&words);
```
