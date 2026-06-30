# Trit-Core Architecture (v0.3.0)

This document describes the architecture of `trit-core` at version 0.3.0. The project is a **single-machine ternary decision engine** with structured observability. The experimental distributed protocol layer (`src/net/`) was removed in v0.2.0 to focus engineering effort on correctness, type-safety, and testability of the core algebra and policy engine.

## Design Principles

1. **Invariants are enforced by the type system** whenever possible.
   - `Phase` is always finite and within `[0.0, 1.0]`.
   - `TritWord` fields are private; constructors enforce the `Frame::Absolute` invariant.
2. **Fail-safe by default** in dangerous domains (`Physical`, `Engineering`, registered dangerous `Custom` domains).
3. **Explicit over implicit**: `Phase::new` returns `Result`; silent clamping is available only via `Phase::new_clamped`.
4. **No unsafe code**: `#![forbid(unsafe_code)]` is enforced crate-wide.

## Module Layers

```
┌─────────────────────────────────────────────────────────────┐
│  Application Layer                                            │
│  - src/bin/sandbox.rs    (thin CLI over sandbox::pipeline)   │
│  - src/bin/dhat_profile.rs (heap profiling)                  │
├─────────────────────────────────────────────────────────────┤
│  Sandbox Layer (src/sandbox/)                                 │
│  - input.rs      ScenarioInput / SignalInput                 │
│  - output.rs     SandboxOutput                                │
│  - validate.rs   input validation & sanitization             │
│  - pipeline.rs   t_and_n batch → arbitrate → SafeFallback    │
│  - diagnostic.rs runtime telemetry collection                │
│  - error.rs      SandboxError / ErrorCategory / help text    │
│  - validator.rs  expected_behavior verification              │
├─────────────────────────────────────────────────────────────┤
│  Meta Layer (src/meta/)                                       │
│  - interrupt.rs  MetaInterrupt, ConflictType, MetaMonitor    │
│  - domain.rs     Domain, ResolutionPolicy, ArbitrationResult │
│  - rules.rs      CustomRule, RuleLoader, JsonRuleLoader      │
│  - safe_fallback.rs  IEC 61508-style safety override         │
├─────────────────────────────────────────────────────────────┤
│  Core Algebra Layer (src/core/)                               │
│  - value.rs      TritValue                                    │
│  - phase.rs      Phase, Commitment, PhaseError               │
│  - frame.rs      Frame, FrameRegistry, FrameError            │
│  - word.rs       TritWord (invariant-centralized)            │
│  - algebra.rs    TernaryAlgebra (TAND/TOR/TNOT)              │
├─────────────────────────────────────────────────────────────┤
│  Support Layer                                                │
│  - src/clock.rs / src/clock/   Phase oscillator              │
│  - src/baseline/               Binary baseline comparator    │
│  - src/tracing_init.rs         tracing subscriber setup      │
└─────────────────────────────────────────────────────────────┘
```

## Data Flow

```
scenarios/*.json
       │
       ▼
ScenarioInput ──validate──► SandboxPipeline::run()
                               │
                               ▼
                         SignalInput[] ──► TritWord[]
                               │
                               ▼
                         t_and_n batch TAND (bias-free for 3+ signals)
                               │
                               ▼
                         MetaInterrupt[] + current TritWord
                               │
                               ▼
                         ResolutionPolicy::arbitrate()
                               │
                               ▼
                         SafeFallback::guard()
                               │
                               ▼
                         SandboxOutput ──► stdout (JSON)
                               │
                               ▼
                         SandboxDiagnostics ──► stderr (with --diagnostic)
```

## Key Invariants

### Phase
- `Phase(f64)` is private.
- `Phase::new(f64) -> Result<Phase, PhaseError>` rejects NaN, infinite, and out-of-range values.
- `Phase::new_clamped(f64) -> Phase` is the only silent-normalization path and logs a `tracing::warn`.

### TritWord
- Fields `value`, `phase`, `frame` are private.
- `TritWord::new(value, phase, frame)` accepts a validated `Phase`.
- `TritWord::try_new(value, phase: f64, frame: &str) -> Result<Self, WordError>` is the one-stop constructor from raw input.
- `TritWord::absolute()` is the only way to create an `Absolute` frame word; it forces `Hold` + neutral phase.
- `with_value`, `with_phase`, `with_frame` return `Result` to preserve the `Absolute` invariant.

### TernaryAlgebra
- `t_and` / `t_or` are safe: cross-frame inputs return `Hold` + `MetaInterrupt`.
- `t_and_hot` / `t_or_hot` are unchecked fast paths and **panic** if frames differ (active in all build modes via `assert!`).

### Meta Layer
- `ResolutionPolicy::arbitrate` returns `Result<ArbitrationResult, PolicyError>`; it never panics.
- `Domain::Custom(name)` resolves through an attached `CustomRule` if available; otherwise it falls back to `Negotiate`.
- `SafeFallback` forces `False` in dangerous domains when the result is `Unknown` or `Hold` with interrupts.

## API Stability

v0.3.0 builds on the v0.2.0 refactor:
- `TritWord` and `Frame` are now `Copy`.
- `TritWord::frame()` returns `Frame` by value.
- `SandboxPipeline::run(&self)` is an instance method; `run_with_diagnostics(&self)` returns both output and telemetry.
- Batch TAND via `TernaryAlgebra::t_and_n` replaces sequential cascades for 3+ signals.
- `CustomRule.fallback` is now the type-safe `FallbackBehavior` enum.
- New observability surfaces: `SandboxDiagnostics`, `LogFormat`, `LogOptions`, `init_with_opts`.
- `src/net/` and `trit-node` binary remain removed; no network dependencies.

## Future Directions

- Property-based verification of full algebraic laws (distributivity, associativity over computable values).
- Optional no-std core profile.
- Reintroduction of a distributed protocol as a separate crate with cryptographic identity and formal wire specification.
