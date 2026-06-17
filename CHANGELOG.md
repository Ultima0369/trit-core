# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0-alpha] - 2026-06-17

### Added
- Core ternary algebra (HTA): TAND, TOR, TNOT with phase arithmetic.
- Five decision domains: Physical, Engineering, MedicalEthics, ValueJudgment, General.
- Meta-monitor with conflict detection and domain-based arbitration.
- Sandbox CLI (`trit-sandbox`) for JSON scenario input/output.
- trit-node CLI (`trit-node`) for sovereign node REPL (M4).
- Docker Compose 3-node cluster (Science/Individual/Consensus) with TCP mesh (M6).
- M5 TCP transport layer: length-prefix framing (frame_codec), TcpNodeServer, TcpClient.
- M6 seed node discovery: parse_seeds, bootstrap, --peers/TRIT_PEERS.
- 12 scenario JSON files covering 5 domains (plus 5 zh variants, total 17).
- Binary baseline comparator (`src/baseline/`) for M2 ternary vs binary validation.
- Integration test suite (18 tests) covering all scenarios end-to-end.
- 9 multi-node integration tests (M6): full mesh lifecycle, cross-frame conflict, seed bootstrap.
- Architecture Decision Records (ADRs): 001-ternary-logic, 002-phase-arithmetic, 003-domain-conflict, 004-distributed-protocol.
- Full Chinese documentation system (`docs/zh/`).
- Architecture audit report (`docs/zh/architecture-audit.zh.md`).
- Security audit report (`docs/security-audit.md`): all P1/P2 fixes applied.
- Code quality audit report (`docs/code-quality-audit.md`).
- GitHub Actions CI/CD pipeline: check, lint, test, benchmark, build.
- Benchmark suite (`criterion`) for TAND, TOR, TNOT, cascade operations.
- Preprint (`docs/preprint.md`): 10+ page research paper with abstract, architecture, validation, references.
- Chinese preprint (`docs/zh/preprint.zh.md`): 10+ page Chinese translation.
- M2 validation report (`docs/validation-report.md`): ternary vs binary comparison across 12 scenarios.
- M4-M6 distributed protocol: T_RESONATE/T_DECOUPLE with PLL, ResonanceBus, message types, TCP transport, seed discovery.
- 88 property-based tests (proptest) for formal invariant verification.
- 5-layer documentation system: getting-started, concepts, usage, development, insights (14 new docs).
- Observability via `tracing` in core algebra and policy engine.
- `#![deny(warnings)]` and `#![forbid(unsafe_code)]` enforced.
- CLAUDE.md for Claude Code guidance; Serena project memories initialized.
- Git repository initialized with 6 commits (no remote push).
- Total: 227 tests, 0 failures, 0 warnings, 0 clippy issues.

### Engineering
- Modular monolith structure: `trit/`, `frame/`, `meta/`, `clock/`, `sandbox/`, `net/`.
- Public API exported via `lib.rs` with SemVer stability guarantee for 0.1.x.
- Integration tests covering cross-frame conflict and domain arbitration.
- `cargo fmt` and `cargo clippy` enforced in CI.

### Known Limitations
- `phase: f64` may introduce precision drift over long cascades (ADR-002).
- TCP transport requires tokio runtime; not suitable for embedded/no_std contexts.
- No formal verification (Coq/Lean) attached.
- Performance target (10,000 TPS) validated at micro-benchmark level; end-to-end benchmarks pending.
