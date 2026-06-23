# CLAUDE.md

> **⚡ 新会话启动**：先读 `SESSION_START.md`（30 秒了解当前进度和上次决策），再回到本文件看技术约束。

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a **Rust workspace** with two crates:

- **`trit-core`** (v0.3.0): A ternary decision engine for conflict-aware AI alignment. Uses three-state logic (`True`, `Hold`, `False`) instead of binary. The `Hold` state represents intentional suspension of judgment when conflicting decision domains are detected.
- **`aurora`** (v0.1.0): A local-first cognitive sovereignty desktop tool built on Trit-Core. Currently at M1 — bounded context skeletons + SQLite persistence layer in place.

## Build & Test Commands

```bash
# Build everything
cargo build --release

# Run all tests (workspace-wide)
cargo test --workspace --all-features -- --test-threads=2

# Run a single test (any crate)
cargo test -- <test_name>

# Run only ethics gate tests (non-negotiable, 10 tests)
cargo test ethics_

# Format check (CI-enforced)
cargo fmt -- --check
cargo fmt          # auto-fix

# Clippy (CI-enforced, -D warnings)
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Run benchmarks
cargo bench

# ── Trit-Core binaries ──────────────────────────────────

# Sandbox CLI with a scenario
cargo run --bin trit-sandbox -- --scenario scenarios/medical_conflict_01.json
cargo run --release --bin trit-sandbox -- --scenario scenarios/career_value_conflict.json

# Heap profiling (dhat)
cargo run --release --bin dhat-profile

# ── Aurora binary ───────────────────────────────────────

# Run Aurora pipeline (M0 end-to-end)
cargo run --bin aurora -- --input synthetic_2hz.json --output report.html
```

## Architecture: Trit-Core (5-Layer Cognitive Stack)

The library is a **modular monolith** with five layers (bottom-up):

### Layer 1: `src/anchor/` — Steady-State Constraints (Veto Power)
Five non-negotiable constraints checked before every decision: `thermal_baseline`, `survival_motives`, `flourishing_pool`, `ecological_base`, `wellbeing_priority`. Any violation forces `Hold` + alert. No frame or domain can override an `Abort`-severity violation.

### Layer 2: `src/hook/` — Scenario Perception & Module Scheduling
The "perceptual foundation." `ScenarioRecognizer` identifies the current scenario type (`PhysicalReasoning`, `ValueConflict`, `MedicalEthics`, `SelfReflection`, `General`). `MountArbiter` decides which adapter modules to mount based on scenario + resource budget. `HookContext` is the read-only communication bus — modules read from it but never mutate it.

### Layer 3: `src/adapters/` — Cognitive Module Pool
Ten dynamically mounted modules, each implementing `CognitiveModule`:
`AdaptiveIteration`, `AttentionScheduler`/`BandwidthScheduler`, `CognitiveDeconstruction`, `ConflictSuspension`, `CouplingAdapter`, `CriticalThinking`, `EcologicalAssessment`, `EngineeringArchitecture`, `ReflexiveAudit`, `SelfKnowledge`.

Modules do NOT call each other. All cross-module communication goes through `HookContext`. Every module output includes a `confidence` score.

### Layer 4: `src/core/` + `src/meta/` — Ternary Algebra & Policy Engine
- **`TritValue`**: `True` (+1), `Hold` (0), `False` (-1), `Unknown` (⊥ — out-of-distribution, propagates through TAND/TOR).
- **`Phase`**: Continuous tendency 0.0–1.0 (0.5 = neutral). Wraps `f64` with strict construction (`Phase::new` returns `Result`).
- **`Frame`**: `Science`, `Individual`, `Consensus`, `Absolute`, `Meta`. Cross-frame operations trigger `MetaInterrupt`.
- **`TritWord`**: Bundles `TritValue` + `Phase` + `Frame`. Fields are private; invariants enforced by constructors. `Copy` type.
- **`TernaryAlgebra`** (HTA): Static methods `t_and`, `t_or`, `t_not`, `t_hold`, `t_sense`. Hot-path methods `t_and_hot`/`t_or_hot` panic on frame mismatch. `t_and_n` uses equal-weight Phase averaging for batch operations.
- **`ResolutionPolicy::arbitrate()`**: Domain-specific arbitration. `Physical`/`Engineering` prioritize `Science` frame. `MedicalEthics` prioritizes `Individual`. `ValueJudgment` always returns `Hold`.
- **`SafeFallback`**: IEC 61508-style safety override; forces `False` with `Phase::full_false()` in dangerous domains.

### Layer 5: `src/feedback/` — Practice Testing & Correction
Every decision is tested against a `ProxyEnvironment` prediction. Deviations trigger calibration of Layer 3 modules. Severe deviations trigger immediate pipeline re-entry with a correction signal.

### Supporting Modules
- **`src/security/`**: Four-state machine — `Service`, `Refusal`, `Awareness`, `Transparency`.
- **`src/budget/`**: Hardware-aware compute budget and depth-level gating.
- **`src/calibration/`**: Decision history recording for feedback-driven learning.
- **`src/clock/`**: Phase oscillator (`HarmonicClock`) with `physical()` (ω=10.0) and `deliberative()` (ω=0.5) presets.
- **`src/sandbox/`**: Scenario I/O, validation, pipeline, and expected-behavior verification.
- **`src/baseline/`**: Binary baseline comparator for M2 validation.

### Data Flow
```
JSON scenario → ScenarioInput → validate → TritWord[] → t_and_n (batch TAND)
    → MetaInterrupt[] → ResolutionPolicy::arbitrate() → SafeFallback::guard()
    → SandboxOutput (JSON)
```

## Architecture: Aurora (M1 — Bounded Contexts + SQLite)

Aurora is a CLI binary (future: Tauri desktop app) with these layers:

### Pipeline (M0, working end-to-end)
```
Synthetic signal (2Hz sine) → FFT base frequency detection
    → Trit-Core decision (Embodied vs Individual frame)
    → CLI/JSON/HTML output
```

### Bounded Contexts (`aurora/src/bc/`, M1)
Six independent BCs with trait-defined boundaries, connected in a DAG:
```
SignalAnalysis ─────┐
                    ├──▶ TernaryDecision ──▶ AttentionGuidance ──▶ Presentation
RelationshipAnnotation ─┘        │                                    │
                                 │                                    │
                                 ▼                                    ▼
                            AuditTrail ◀──────────────────────────────┘
```
Each BC exposes exactly one public trait (its "port") and has one aggregate root.

### SQLite Data Layer (`aurora/src/db/`, M1)
Local database at `~/.aurora/data/aurora.db`. Schema: `contacts`, `frame_annotations`, `annotation_history`, `audit_log`, `communication_events`. Includes schema migration system.

### Other Aurora Modules
- **`aurora/src/wavelet/`**: Synthetic signal generation + FFT base frequency detection.
- **`aurora/src/ingest/`**: `DataSource` trait abstraction — JSON fallback + mail abstract.
- **`aurora/src/attention/`**: Attention Sovereignty Index (ASI) dashboard + reminder history + conflict panel.
- **`aurora/src/decision/`**: Maps signals to Trit-Core trits, conflict detection.
- **`aurora/src/render/`**: JSON and HTML output renderers.

## Key Design Rules

- **`#![forbid(unsafe_code)]`** — both crates enforce this.
- **Invariants are enforced by constructors** — `TritWord` and `Phase` fields are private.
- **`Frame` and `TritWord` are `Copy`** — `frame()` returns `Frame` by value; no `.clone()` needed.
- **Cross-frame operations never force a binary decision** — they produce `Hold` + `MetaInterrupt`.
- **`Absolute` frame must always remain `Hold` + neutral phase** — enforced by constructors, checked by `MetaMonitor::inspect()`.
- **`Phase::new` returns `Result`** — use `Phase::new_clamped` only when silent normalization is explicitly desired.
- **No panics in policy code** — `ResolutionPolicy::arbitrate` returns `Result`.
- **SafeFallback resets Phase to `full_false()`** — IEC 61508 "definitive safe state" semantics.
- **`t_and_n` uses equal-weight Phase averaging** — avoids left-fold bias for 3+ signal cascades.
- **`Meta` frame is system-internal** — only produced by cross-frame conflict resolution; not valid for external inputs.
- **Modules do NOT call each other** — all cross-module communication goes through `HookContext`.
- **Unmount = release** — no background processing after a module is unmounted.
- **`assert_float_eq!` macro** — use this for all `f64` comparisons in tests (replaces the `(a-b).abs() < f64::EPSILON` pattern).

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
- Aurora is CLI-only; Tauri desktop shell not yet started.
- SQLite encryption (SQLCipher) not yet enabled — plain SQLite for development.
