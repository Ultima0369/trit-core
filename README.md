# Trit-Core v0.3.0

[![CI](https://github.com/trit-core/trit-core/actions/workflows/ci.yml/badge.svg)](https://github.com/trit-core/trit-core/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-blue.svg)](https://www.rust-lang.org)

A ternary decision engine for conflict-aware AI alignment.

**Status**: v0.3.0 — observability-enhanced single-machine ternary decision engine
**License**: MIT

> **A reminder, not instruction**: Everything in this project is offered as a reminder to inspect, not as instruction to obey. We invite every reader to practice-test, cross-reference, and keep exploring. See [`docs/explanation/insights/EPISTEMIC-HUMILITY.md`](docs/explanation/insights/EPISTEMIC-HUMILITY.md) for the full statement.

## Overview

Trit-Core implements a **multi-valued logic (MVL) computation framework** where each decision unit (trit) carries three states: `True`, `Hold`, and `False`. Unlike binary logic which forces a determination, Trit-Core introduces a **Hold state** that represents intentional suspension of judgment when conflicting decision domains are detected.

This project tests the hypothesis on synthetic scenarios: *In human-centric advisory contexts, a ternary decision protocol that respects domain conflicts and preserves undetermined states may avoid the misleading consensus collapses common to binary RLHF proxies.* Human-subject validation of "authentic user satisfaction" is planned for future work.

## Architecture at a Glance

```
Input Layer (multi-source signals)
    ├── Science Domain (empirical data)
    ├── Individual Domain (user context)
    ├── Consensus Domain (statistical preference)
    └── Absolute Domain (unknowable / unobservable)
         │
         ▼
    Core Ternary ALU (Harmonic Ternary Algebra)
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
    Sandbox Pipeline
         ├── Input Validation
         ├── SafeFallback (dangerous-domain guard)
         └── JSON Output
```

## Project Structure

| Path | Description |
|------|-------------|
| `src/lib.rs` | Public API |
| `src/core/` | Ternary algebra and data types (value, phase, frame, word, algebra) |
| `src/meta/` | Meta-monitor, policy engine, custom rules, safe fallback |
| `src/sandbox/` | Scenario I/O, validation, pipeline, and expected-behavior verification |
| `src/clock/` | Phase oscillator and time-scale management |
| `src/baseline/` | Binary baseline comparator for validation |
| `docs/` | Full documentation system — see [docs/INDEX.md](docs/INDEX.md) |
| `tests/` | Unit tests, integration tests, property tests, scenario validation, CLI tests |
| `scenarios/` | Human-centric advisory cases (JSON, including Chinese `.zh.json` variants) |

## Technology Stack

- **Language**: Rust 2021 Edition
- **Serialization**: serde + serde_json (decision logs)
- **Error Handling**: thiserror
- **Timestamping**: chrono
- **Observability**: tracing
- **Heap Profiling**: dhat (zero-allocation hot path verified)
- **Property Testing**: proptest

## Build & Run

```bash
# Build
cargo build --release

# Run all tests
cargo test --all-features -- --test-threads=2

# Run a single test
cargo test -- <test_name>

# Lint & format
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings

# Benchmarks
cargo bench

# Scenario sandbox
cargo run --release --bin trit-sandbox -- --scenario scenarios/medical_conflict_01.json

# Heap profiling (dhat)
cargo run --release --bin dhat-profile --features dhat-profile

# Check public API snapshot (CI gate)
cargo public-api -ss --all-features > /tmp/current-public-api.txt
diff -u api/public-api.txt /tmp/current-public-api.txt
```

## Key Results (M2 Validation)

Across human-centric advisory scenarios:
- **Binary baseline produces misleading output** when domain conflicts exist; Trit-Core correctly preserves conflicts as `Hold`.
- **100% of ValueJudgment cases**: binary cannot express "this should not be decided by algorithm".
- **100% of MedicalEthics cases**: binary ignores patient-specific context.

## Documentation

→ **Start here**: [docs/INDEX.md](docs/INDEX.md) — full navigation map.

### New to Trit-Core?
1. [What is Trit?](docs/tutorials/WHAT_IS_TRIT.md) — three stories that explain why ternary decisions matter
2. [Quickstart](docs/tutorials/QUICKSTART.md) — 3 minutes from clone to first scenario
3. [Concepts](docs/explanation/CONCEPTS.md) — core types and their mathematical foundations
4. [Architecture](docs/explanation/ARCHITECTURE.md) — v0.3.0 module layers and invariants

### Integrating Trit-Core?
- [API Reference](docs/reference/api.md) — public API contract
- [Module Reference](docs/reference/MODULES.md) — per-module responsibilities and key functions
- [Custom Rules](docs/how-to/CUSTOM_RULE.md) — defining external arbitration rules via JSON

### Contributing?
- [Contributing Guide](docs/how-to/CONTRIBUTING.md) — code style, CI gates, test strategy
- [Benchmarks](docs/reference/BENCHMARK.md) — performance data and how to run benchmarks

### Deep Dives
- [Conflict Catalog](docs/explanation/insights/CONFLICT_CATALOG.md) — systematic classification of cross-frame conflict patterns
- [Future](docs/explanation/insights/FUTURE.md) — known limitations and possible resolution paths
- [Glossary](docs/explanation/insights/GLOSSARY.md) — all terms defined, with cross-disciplinary mappings

### Reports & Audits
- [Technical Whitepaper](docs/technical-whitepaper.md) — comprehensive v0.3.0 overview and audit index
- [Validation Report](docs/reports/validation-report.md) — M2 ternary vs binary comparison
- [Performance Validation](docs/reports/performance-validation.md) — end-to-end TPS benchmarks and bottleneck analysis
- [Security Audit](docs/reports/security-audit.md) — AppSec audit
- [Code Quality Audit](docs/reports/code-quality-audit.md) — SOLID/DRY/complexity audit
- [CTO Audit Report](docs/reports/cto-audit-report.md) — comprehensive architecture & quality audit
- [Reviewer Guide](docs/how-to/REVIEWER_GUIDE.md) — how to verify core claims

## Milestones

| Milestone | Status |
|-----------|--------|
| M0: Foundation | ✅ Complete |
| M1: Sandbox CLI | ✅ Complete |
| M2: Scenario Validation | ✅ Complete |
| M3: Preprint & Open Source | ✅ Core complete |
| M4–M9: Distributed Prototype | ⚠️ Removed in v0.2.0; planned as separate crate |

## v0.2.0 Highlights

- **Architecture refactor**: `src/core/` now centralizes `TritWord`, `Phase`, `Frame`, and algebra invariants.
- **Type-safe invariants**: `TritWord` fields are private; `Phase::new` returns `Result`; `Frame::Absolute` invariant is enforced at construction.
- **Sandbox layer**: scenario validation and pipeline execution are now library code, tested independently of the CLI.
- **Automated scenario validation**: every `scenarios/*.json` is checked against its `expected_behavior`.
- **Removed network layer**: `src/net/`, `trit-node`, `tokio`, and `uuid` removed to focus on core correctness.
- **CI/CD improvements**: scenario-validation job, standalone dhat-profile job, reduced release-profile memory pressure, public-API snapshot gate.

## License

MIT
