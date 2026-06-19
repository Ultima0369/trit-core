# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Trit-Core is a **ternary decision engine** for conflict-aware AI alignment, implemented in Rust. Instead of binary logic (true/false), it uses a three-state system: `True`, `Hold`, and `False`. The `Hold` state represents intentional suspension of judgment when conflicting decision domains are detected.

**v0.3.0 is a single-machine decision engine.** The experimental distributed protocol layer (`src/net/`, `trit-node`) was removed to focus on core correctness, type-safety, and testability.

## Build & Test Commands

```bash
# Build (release)
cargo build --release

# Run all tests
cargo test --all-features -- --test-threads=2

# Run a single test
cargo test -- <test_name>

# Format check (CI-enforced)
cargo fmt -- --check
cargo fmt          # auto-fix

# Clippy (CI-enforced, -D warnings)
cargo clippy --all-targets --all-features -- -D warnings

# Run benchmarks
cargo bench

# Run the sandbox CLI with a scenario
cargo run --bin trit-sandbox -- --scenario scenarios/medical_conflict_01.json
cargo run --release --bin trit-sandbox -- --scenario scenarios/career_value_conflict.json

# Heap profiling (dhat)
cargo run --release --bin dhat-profile
```

## Architecture

The codebase is a **modular monolith** with these layers (bottom-up):

### 1. `src/core/` — Core Ternary Algebra
- **`TritValue`**: Four discrete states: `True` (+1), `Hold` (0), `False` (-1), `Unknown` (⊥ — out-of-distribution, propagates through TAND/TOR).
- **`Phase`**: Continuous tendency 0.0–1.0 (0.5 = neutral). Wraps `f64` with strict construction (`Phase::new` returns `Result`), explicit clamping (`Phase::new_clamped`), and constant constructors `Phase::neutral()`, `Phase::full_true()`, `Phase::full_false()`.
- **`Frame`** enum: `Science`, `Individual`, `Consensus`, `Absolute`, `Meta`. Each trit belongs to a frame; cross-frame operations trigger `MetaInterrupt`.
- **`FrameRegistry`**: Tracks active frames in a session.
- **`TritWord`**: The fundamental computation unit — bundles a `TritValue`, `Phase`, and `Frame`. Fields are private; invariants are enforced by constructors.
- **`TernaryAlgebra`** (HTA — Harmonic Ternary Algebra): Static methods `t_and`, `t_or`, `t_not`, `t_hold`, `t_sense`/`t_sense_clamped`. Cross-frame operations return `(TritWord, Option<MetaInterrupt>)`. Same-frame operations use standard ternary truth tables with phase averaging. Hot-path methods `t_and_hot` / `t_or_hot` panic on frame mismatch in all build modes.

### 2. `src/meta/` — Meta-Monitor & Policy Engine
- **`Domain`** enum: `Physical`, `Engineering`, `MedicalEthics`, `ValueJudgment`, `General`, `Custom(String)`. Each domain has different conflict resolution rules.
- **`ResolutionPolicy::arbitrate()`**: Core arbitration logic returning `Result<ArbitrationResult, PolicyError>`. Physical/Engineering prioritize `Science` frame and allow forced collapse. `MedicalEthics` prioritizes `Individual`. `ValueJudgment` always returns `Hold`.
- **`CustomRule` / `RuleLoader` / `JsonRuleLoader`**: External domain rule loading via JSON.
- **`FallbackBehavior`**: Type-safe enum (`Hold`, `Negotiate`, `CommitFirst`, `SafeFallback`) replacing previous string-based fallback.
- **`Domain`**: Implements `FromStr` and `Display` for type-safe domain parsing.
- **`DomainParseError`**: Error type for domain parsing failures.
- **`MetaMonitor`**: Records `MetaInterrupt` events and enforces invariants (e.g., `Absolute` frame must remain `Hold`).
- **`SafeFallback`**: IEC 61508-style safety override; forces `False` with `Phase::full_false()` in dangerous domains when the result is `Unknown` or `Hold` with interrupts.
- **`ConflictType`**: `FrameMismatch`, `OutOfScope`, `PhaseDrift`, `PolicyViolation`.

### 3. `src/sandbox/` — Scenario Pipeline
- **`SandboxOutput`**: Serde structs for JSON scenario I/O. Custom deserializer validates `final_phase ∈ [0.0, 1.0]` and `final_value_code ∈ {-1, 0, 1}`.
- **`SandboxPipeline::run()`**: Validates input, builds `TritWord`s, runs batch TAND via `t_and_n` (equal-weight Phase averaging), arbitrates, applies SafeFallback, and returns `SandboxOutput`.
- **`ScenarioValidator`**: Compares output against `expected_behavior` (`hold`, `commit_true`, `commit_false`, `negotiate`).
- **`SandboxError`**: Unified error type for pipeline failures.
- `src/bin/sandbox.rs`: Thin CLI that calls `SandboxPipeline::run()`.

### 4. `src/clock/` — Phase Oscillator
- **`HarmonicClock`**: Time-scale management via sinusoidal oscillator. Two presets: `physical()` (fast, ω=10.0) and `deliberative()` (slow, ω=0.5). Used for domain-specific sampling rates.

### 5. `src/baseline/` — Binary Baseline Comparator
- **`BinaryBaseline`**: Simple majority-rule comparator used in M2 validation to demonstrate where binary systems fail to preserve conflicts.

### Data Flow
```
JSON scenario → ScenarioInput → validate → TritWord[] → t_and_n (batch TAND)
    → MetaInterrupt[] → ResolutionPolicy::arbitrate() → SafeFallback::guard()
    → SandboxOutput (JSON)
```

## Key Design Rules

- **`#![forbid(unsafe_code)]`** — no unsafe Rust anywhere.
- **Invariants are enforced by constructors** — `TritWord` and `Phase` fields are private.
- **`Frame` and `TritWord` are `Copy`** — `frame()` returns `Frame` by value; no `.clone()` needed.
- **Cross-frame operations never force a binary decision** — they produce `Hold` + `MetaInterrupt` instead.
- **`Absolute` frame must always remain `Hold` + neutral phase** — enforced by `TritWord` constructors and checked by `MetaMonitor::inspect()`.
- **`Phase::new` returns `Result`** — callers must handle invalid input; use `Phase::new_clamped` only when silent normalization is explicitly desired.
- **No panics in policy code** — `ResolutionPolicy::arbitrate` returns `Result`.
- **SafeFallback resets Phase to `full_false()`** — when forcing `False` in dangerous domains, the Phase is set to 0.0 (not the original Phase), matching IEC 61508 "definitive safe state" semantics.
- **`t_and_n` uses equal-weight Phase averaging** — avoids left-fold bias for 3+ signal cascades.
- **`Meta` frame is system-internal** — only produced by cross-frame conflict resolution; not valid for external signal inputs.
- **`FallbackBehavior` is a type-safe enum** — `CustomRule.fallback` uses `FallbackBehavior` instead of `String`.
- **`Domain` implements `FromStr` and `Display`** — domain parsing is centralized in one place.

## Scenario JSON Format

```json
{
  "id": "unique_id",
  "description": "human-readable scenario",
  "domain": "MedicalEthics|Physical|Engineering|ValueJudgment|General|Custom(name)",
  "signals": [
    { "frame": "Science|Individual|Consensus|Absolute", "value": 1|0|-1, "phase": 0.0-1.0 }
  ],
  "expected_behavior": "hold|commit_true|commit_false|negotiate"
}
```

## Known Limitations

- `phase: f64` may drift over long cascades (see ADR-002).
- No formal verification (Coq/Lean).
- Distributed protocol removed in v0.2.0; planned as a separate crate.
