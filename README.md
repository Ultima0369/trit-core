# Trit-Core MVP

A ternary decision engine for conflict-aware AI alignment.

## Overview

Trit-Core implements a **multi-valued logic (MVL) computation framework** where each decision unit (trit) carries three states: `True`, `Hold`, and `False`. Unlike binary logic which forces a forced determination, Trit-Core introduces a **Hold state** that represents intentional suspension of judgment when conflicting decision domains are detected.

This project validates the hypothesis: *In human-centric advisory scenarios, a ternary decision protocol that respects domain conflicts and preserves undetermined states produces more authentic user satisfaction than binary RLHF (Reinforcement Learning from Human Feedback) systems that collapse to consensus averages.*

## Architecture at a Glance

```
Input Layer (multi-source signals)
    ├── Science Domain (empirical data)
    ├── Individual Domain (user context)
    ├── Consensus Domain (statistical preference)
    └── Absolute Domain (unknowable / unobservable)
         │
         ▼
    Ternary ALU (Harmonic Ternary Algebra)
         ├── TAND / TOR / TNOT
         ├── Phase Arithmetic (0.0 ~ 1.0)
         └── Domain Conflict Detection
         │
         ▼
    Meta-Monitor (Policy Engine)
         ├── Conflict Interrupt
         ├── Domain Rules
         └── Resolution Arbitration
         │
         ▼
    Output Layer
         ├── Determined: +1 / -1
         ├── Undetermined: 0 (Hold) + reason
         └── Decision Log (JSONL)
```

## Project Structure

| Path | Description |
|------|-------------|
| `src/lib.rs` | Public API |
| `src/trit/` | Ternary algebra and data types |
| `src/frame/` | Decision domain / context frame registry |
| `src/meta/` | Meta-monitor, policy engine, ADR-enforced rules |
| `src/clock/` | Phase oscillator and time-scale management |
| `src/sandbox/` | CLI simulation environment |
| `src/net/` | Distributed node protocol (M2 stage) |
| `docs/` | Architecture Decision Records (ADRs) |
| `tests/` | Unit tests and scenario integration tests |
| `scenarios/` | Human-centric advisory cases (JSON) |

## Technology Stack

- **Language**: Rust 2021 Edition
- **Serialization**: serde + serde_json (decision logs)
- **Error Handling**: thiserror
- **Timestamping**: chrono + uuid

## Build & Run

```bash
cargo build --release
cargo test
cargo run --bin trit-sandbox -- --scenario scenarios/career_conflict.json
```

## Documentation

- `docs/adr/` — Architecture Decision Records
- `docs/whitepaper.md` — Technical specification
- `docs/roadmap.md` — Milestone plan and acceptance criteria
- `docs/api.md` — Public API contract

## License

MIT
