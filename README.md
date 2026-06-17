# Trit-Core MVP

A ternary decision engine for conflict-aware AI alignment.

**Status**: v0.1.0-alpha — M0/M1/M2/M3/M4 core deliverables complete
**Tests**: 34 passing, 0 failures
**License**: MIT

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
| `src/baseline/` | Binary baseline comparator for validation |
| `src/net/` | Distributed node protocol (M4: T_RESONATE / T_DECOUPLE) |
| `docs/` | Architecture Decision Records (ADRs), whitepaper, preprint |
| `tests/` | Unit tests and scenario integration tests |
| `scenarios/` | Human-centric advisory cases (JSON, 17 files) |

## Technology Stack

- **Language**: Rust 2021 Edition
- **Serialization**: serde + serde_json (decision logs)
- **Error Handling**: thiserror
- **Timestamping**: chrono + uuid
- **Observability**: tracing

## Build & Run

```bash
# Build
cargo build --release

# Run all tests (34 tests)
cargo test --all-features

# Run a single test
cargo test -- <test_name>

# Lint & format
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings

# Benchmarks
cargo bench

# Scenario sandbox
cargo run --release --bin trit-sandbox -- --scenario scenarios/medical_conflict_01.json

# Distributed node (M4)
cargo run --release --bin trit-node -- --frame Science --phase 0.75 --id my-node

# Docker 3-node cluster
docker compose up --build
```

## Key Results (M2 Validation)

Across 12 human-centric advisory scenarios:
- **67% of cases**: binary baseline produces misleading output; Trit-Core correctly preserves domain conflicts
- **100% of ValueJudgment cases**: binary cannot express "this should not be decided by algorithm"
- **100% of MedicalEthics cases**: binary ignores patient-specific context

## Documentation

| Document | Description |
|----------|-------------|
| `docs/whitepaper.md` | Technical specification |
| `docs/preprint.md` | Research paper (10+ pages, English) |
| `docs/zh/preprint.zh.md` | Research paper (Chinese) |
| `docs/roadmap.md` | Milestone plan and acceptance criteria |
| `docs/api.md` | Public API contract |
| `docs/validation-report.md` | M2 ternary vs binary comparison |
| `docs/security-audit.md` | AppSec audit (P1/P2 fixes applied) |
| `docs/code-quality-audit.md` | SOLID/DRY/complexity audit |
| `docs/adr/` | Architecture Decision Records (4 ADRs) |
| `CLAUDE.md` | Claude Code guidance |

## Milestones

| Milestone | Status |
|-----------|--------|
| M0: Foundation | ✅ Complete |
| M1: Sandbox CLI | ✅ Complete |
| M2: Scenario Validation | ✅ Complete |
| M3: Preprint & Open Source | ✅ Core complete |
| M4: Distributed Prototype | ✅ Core complete |

## License

MIT
