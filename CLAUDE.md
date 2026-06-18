# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Trit-Core is a **ternary decision engine** for conflict-aware AI alignment, implemented in Rust. Instead of binary logic (true/false), it uses a three-state system: `True`, `Hold`, and `False`. The `Hold` state represents intentional suspension of judgment when conflicting decision domains are detected — this is the core hypothesis: ternary protocols that respect domain conflicts produce more authentic results than binary RLHF systems that collapse to consensus averages.

## Build & Test Commands

```bash
# Build (release with LTO)
cargo build --release

# Run all tests
cargo test --all-features

# Run a single test
cargo test -- <test_name>

# Run tests in a specific module
cargo test -- trit_tests
cargo test -- meta_tests

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
```

## Architecture

The codebase is a **modular monolith** with these layers (bottom-up):

### 1. `src/trit/` — Core Ternary Algebra (frozen for 0.1.x)
- **`TritValue`**: Four discrete states: `True` (+1), `Hold` (0), `False` (-1), `Unknown` (⊥ — out-of-distribution, propagates through TAND/TOR).
- **`Phase`**: Continuous tendency 0.0–1.0 (0.5 = neutral). Wraps `f64` with bounds-clamping and NaN/Inf protection (logs warning via tracing). Use `try_new()` for strict validation that returns `Err`.
- **`TritWord`**: The fundamental computation unit — bundles a `TritValue`, `Phase`, and `Frame`.
- **`TernaryAlgebra`** (HTA — Harmonic Ternary Algebra): Static methods `t_and`, `t_or`, `t_not`, `t_hold`, `t_sense`. Cross-frame operations return `(TritWord, Option<MetaInterrupt>)` instead of forcing a binary collapse. Same-frame operations use standard ternary truth tables with phase averaging.

### 2. `src/frame/` — Decision Domains
- **`Frame`** enum: `Science`, `Individual`, `Consensus`, `Absolute`, `Meta`. Each trit belongs to a frame; cross-frame operations trigger `MetaInterrupt`.
- **`FrameRegistry`**: Tracks active frames in a session.

### 3. `src/meta/` — Meta-Monitor & Policy Engine
- **`Domain`** enum: `Physical`, `Engineering`, `MedicalEthics`, `ValueJudgment`, `General`. Each domain has different conflict resolution rules.
- **`ResolutionPolicy::arbitrate()`**: Core arbitration logic. Physical/Engineering prioritize `Science` frame and allow forced collapse. `MedicalEthics` prioritizes `Individual`. `ValueJudgment` always returns `Hold` (incommensurable values).
- **`MetaMonitor`**: Records `MetaInterrupt` events and enforces invariants (e.g., `Absolute` frame must remain `Hold`).
- **`ConflictType`**: `FrameMismatch`, `OutOfScope`, `PhaseDrift`, `PolicyViolation`.

### 4. `src/clock/` — Phase Oscillator
- **`HarmonicClock`**: Time-scale management via sinusoidal oscillator. Two presets: `physical()` (fast, ω=10.0) and `deliberative()` (slow, ω=0.5). Used for domain-specific sampling rates.

### 5. `src/sandbox/` — CLI Simulation
- `ScenarioInput` / `SandboxOutput`: Serde structs for JSON scenario I/O.
- `src/bin/sandbox.rs`: CLI that reads a JSON scenario, runs TAND cascade across signals, applies policy arbitration, and prints JSON output.
- `src/bin/node.rs`: Trit node CLI binary for sovereign node REPL (M4).
- `src/bin/dhat_profile.rs`: dhat heap profiling binary for allocation analysis.

### 6. `src/net/` — Distributed Protocol (M4-M6)
- **`bus/`** — `ResonanceBus`: in-memory message routing with VecDeque ring buffer (MAX_MESSAGE_LOG=10,000, MAX_NODES=256)
- **`coupling/`** — RESONATE_REQ/ACK and DECOUPLE_REQ handling
- **`discovery/`** — Seed-based peer bootstrapping via `--peers` / `TRIT_PEERS` (M6)
- **`frame_codec/`** — TCP length-prefix framing: 4-byte BE length + JSON payload (max 1 MiB) (M5)
- **`message/`** — Protocol message types: ResonateReq/Ack, DecoupleReq/Ack, NegotiatePayload, HeartbeatPayload
- **`negotiate/`** — Multi-node negotiation (single-pass)
- **`node/`** — `Node` state machine: Sovereign → Coupling → Coupled → Hold
- **`pll/`** — Software phase-locked loop controller (kp=0.3, deadband=0.05)
- **`tcp_client/`** — TCP client connector with resonate/decouple/heartbeat/negotiate methods (M5)
- **`tcp_server/`** — TCP node server dispatching messages to ResonanceBus (M5)
- **`gate/`** — Byzantine fault tolerance gatekeeper with 7 safety checks (M8)

### Data Flow
```
JSON scenario → ScenarioInput → TritWord[] → TAND cascade → MetaInterrupt[] → ResolutionPolicy::arbitrate() → SandboxOutput (JSON)
```

## Key Design Rules

- **`#![forbid(unsafe_code)]`** — no unsafe Rust anywhere.
- **`#![deny(warnings)]`** — warnings are errors.
- **Core algebra (`trit/`) is frozen** after M1: TAND/TOR/TNOT semantics are fixed for reproducibility. The `sandbox` and `net` modules are explicitly unstable and may be refactored in 0.2.0.
- **Cross-frame operations never force a binary decision** — they produce `Hold` + `MetaInterrupt` instead.
- **`Absolute` frame must always remain `Hold`** — enforced by `MetaMonitor::inspect()`.
- **`Phase` panics on out-of-range values** — callers must ensure `phase ∈ [0.0, 1.0]`.

## Scenario JSON Format

```json
{
  "id": "unique_id",
  "description": "human-readable scenario",
  "domain": "MedicalEthics|Physical|Engineering|ValueJudgment|General",
  "signals": [
    { "frame": "Science|Individual|Consensus|Absolute", "value": 1|0|-1, "phase": 0.0-1.0 }
  ],
  "expected_behavior": "hold|commit_true|commit_false|negotiate"
}
```

## Known Limitations

- `phase: f64` may drift over long cascades (see ADR-002).
- TCP transport requires tokio runtime; not suitable for embedded/no_std contexts.
- No formal verification (Coq/Lean).
- Performance target (10,000 TPS) validated at micro-benchmark and end-to-end level (~3ns/op hot path, ~333M ops/s theoretical, 658K-1.02M end-to-end TPS); see `docs/performance-validation.md`.
