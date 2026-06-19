# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-06-18

### Added
- `SandboxDiagnostics` timing precision upgraded to nanoseconds (`elapsed_ns`, `stage_timings_ns`) so per-stage telemetry reports meaningful non-zero values.
- Three new cross-level conflict scenarios inspired by the `dao-science` L0–L7 cognitive spectrum:
  - `medical_pain_dismissed.json` — L2 individual reality vs L3 social consensus in `MedicalEthics`.
  - `general_conceptual_spin.json` — L4 rational collaboration drifting toward L6 conceptual spinning in `General`.
  - `engineering_evacuation_consensus.json` — L1 physical safety vs L3 tenant consensus vs L2 resident report in `Engineering`.
  - Bilingual Chinese counterparts for all new scenarios: `medical_pain_dismissed.zh.json`, `general_conceptual_spin.zh.json`, `engineering_evacuation_consensus.zh.json`.
  - Full Chinese translations for the existing English-only scenarios: `career_value_conflict_02.zh.json`, `career_value_conflict_03.zh.json`, `engineering_bridge_retrofit.zh.json`, `engineering_material_tradeoff.zh.json`, `general_negotiation_02.zh.json`, `medical_conflict_02.zh.json`, `medical_conflict_03.zh.json`, `physical_crane_overload.zh.json`, `physical_runway_length.zh.json`.
  - Three additional cross-domain scenarios:
    - `value_algorithmic_displacement.json` — ValueJudgment on efficiency vs human dignity.
    - `general_water_rights.json` — General-domain negotiation among hydrology, indigenous rights, and farmer survival.
    - `engineering_dam_breach_risk.json` — Engineering safety vs tourism economy vs individual home loss.
- `docs/explanation/insights/EPISTEMIC-HUMILITY.md` — epistemic humility statement: reminder, not instruction.
- `docs/explanation/insights/HUMANITIES-INDEX.md` — scientifically annotated humanities keyword index.
- `docs/explanation/insights/DAO-SCIENCE-REFERENCES.md` — curated cross-project references to `dao-science` for cognitive-spectrum, stopping-criteria, first-person epistemology, and deviation-cost support.
- `docs/explanation/PHILOSOPHY.md` §11 — cross-project mapping between Trit-Core and `dao-science`.
- `docs/technical-whitepaper.md` — comprehensive v0.3.0 technical whitepaper and audit index.
- Comprehensive observability for `trit-sandbox`: structured logging, per-stage diagnostics, CLI verbosity controls, and actionable error reports.
  - `src/tracing_init.rs` rewritten: supports `TRIT_LOG_FILE`, `TRIT_LOG_FORMAT` (`json`|`pretty`|`compact`|`full`), programmatic `LogOptions`, and file + stderr writers.
  - `src/sandbox/diagnostic.rs`: new `SandboxDiagnostics` collector with stage timings, frame distribution, interrupt counts, and SafeFallback tracking.
  - `src/sandbox/pipeline.rs`: each stage now emits `tracing` spans/events; new `run_with_diagnostics()` API returns `(SandboxOutput, SandboxDiagnostics)` while `run()` remains backward-compatible.
  - `src/sandbox/error.rs`: `SandboxError` now exposes `category()`, `category_name()`, `help()`, and `report()` for actionable error context.
  - `src/bin/sandbox.rs`: new CLI flags `--verbose`, `--quiet`, `--trace`, `--log-file`, `--log-format`, `--diagnostic`, `--validate-only`, `--dry-run`, plus structured error reports on failure.
- `docs/explanation/insights/DIALOGUE-ORIGIN.md` documenting the intellectual lineage between `开悟.md` and Trit-Core.
- Expanded `docs/explanation/PHILOSOPHY.md` with insights from `开悟.md`: "statistical consensus ≠ truth", "mind is ternary", "verifiability", and "careful use of assertions".
- `TernaryAlgebra::t_and_n()` batch TAND method with equal-weight Phase averaging, eliminating left-fold bias for 3+ signal cascades.
- `FallbackBehavior` enum (`Hold`, `Negotiate`, `CommitFirst`, `SafeFallback`) replacing `CustomRule.fallback: String` for type-safe rule configuration.
- `Domain::from_str()` and `Domain::display()` implementations, centralizing domain string parsing.
- `DomainParseError` type for domain parsing failures.
- `ArbitrationResult::fmt::Display` implementation for human-readable output.
- `SandboxOutput` custom `Deserialize` with validation: `final_phase ∈ [0.0, 1.0]`, `final_value_code ∈ {-1, 0, 1}`.
- `HarmonicClock::to_phase()` method mapping `[-1.0, 1.0]` to `[0.0, 1.0]` for Phase compatibility.
- Unified adaptive scheduling layer (Layers 4–5 of cognitive architecture):
  - `src/budget/` — `ComputeBudget` + `DepthLevel` enum: OS-level CPU/memory/thread sampling gating how deep the pipeline computes.
  - `src/calibration/` — `CalibrationLog`: fixed-size ring buffer recording decision history for pattern calibration.
  - `src/attention/scheduler.rs` — depth-gated bandwidth via `bandwidth_from_depth()`, consecutive `HoldCurrent` escalation to `Recalibrate`.
  - `src/knowledge/self_model.rs` — `calibrate_from_result()` feedback loop with tiered confidence ceiling (0.6→0.95).
  - `src/clock.rs` — `for_domain()` preset mapping (Physical→ω=10.0, deliberative→ω=0.5) and `elapsed_time()`.
  - `src/sandbox/pipeline.rs` — three new stages: 8b (sample OS budget), 10b (clock tick), 13 (calibrate + feedback); depth gating for optional extensions.
  - `src/sandbox/diagnostic.rs` — `depth_level: u8` and `clock_phase: f64` fields for telemetry.
  - 354 passing tests (+11 pipeline integration tests).
- `FrameRegistry::register_from_words()` and `FrameRegistry::validate_all()` methods for frame whitelisting.
- `tests/error_path_test.rs` — 16 error path tests covering all `SandboxError` variants.
- Expanded `tests/cli_test.rs` with end-to-end CLI coverage for new scenarios, `--validate-only`, `--dry-run`, path-traversal rejection, and unknown-argument rejection.
- `tests/sandbox_test.rs` now includes `diagnostics_shape_matches_expected_fields`, asserting `SandboxDiagnostics` JSON serialization and stage timing coverage.
- `t_and_n` proptest coverage: value consistency, global mean Phase, cross-frame behavior.
- CI coverage job using `cargo-tarpaulin` with Codecov upload.

### Changed
- Documentation system reorganized into Diátaxis-style categories under `docs/`:
  - `tutorials/` — `WHAT_IS_TRIT.md`, `QUICKSTART.md`
  - `how-to/` — `CLI_REFERENCE.md`, `CONFIGURATION.md`, `CUSTOM_RULE.md`, `CONTRIBUTING.md`, `REVIEWER_GUIDE.md`
  - `explanation/` — `CONCEPTS.md`, `ARCHITECTURE.md`, `PHILOSOPHY.md`, `roadmap.md`, plus `insights/`
  - `reference/` — `api.md`, `MODULES.md`, `BENCHMARK.md`
  - `reports/` — `validation-report.md`, `performance-validation.md`, `security-audit.md`, `code-quality-audit.md`, `cto-audit-report.md`
  - `archive/` — historical `preprint.md`, `technical-whitepaper.md`
  - All internal Markdown links updated accordingly.
- **BREAKING**: `Frame` now derives `Copy`; `TritWord::frame()` returns `Frame` instead of `&Frame`.
- **BREAKING**: `TritWord` now derives `Copy`; all `.clone()` calls on `TritWord` replaced with implicit copy.
- **BREAKING**: `CustomRule.fallback` is now `FallbackBehavior` instead of `String`. JSON remains backward-compatible via `#[serde(rename_all = "snake_case")]`.
- **BREAKING**: `SandboxPipeline::run()` uses `t_and_n` (batch TAND) instead of sequential `t_and` cascade, producing different Phase values for 3+ signal inputs.
- **BREAKING**: `SafeFallback::guard()` now resets Phase to `Phase::full_false()` (0.0) when forcing `False`, instead of preserving the original Phase.
- **BREAKING**: `SandboxOutput.policy_action` now uses `Display` formatting instead of `Debug` formatting.
- `build_policy()` in `pipeline.rs` now uses `Domain::from_str()` instead of manual string matching.
- `validate_domain()` in `validate.rs` now delegates to `Domain::from_str()`.
- `Meta` frame is now documented as system-internal; `validate_signal` comments explain why it's excluded from external inputs.
- CLI `--validate-only` now runs full `validate_scenario()` instead of only parsing JSON.
- JSON parse failures and oversized scenario files are classified as `SandboxError::InvalidScenario` rather than `SandboxError::Io`.
- Path-traversal denials are classified as `ErrorCategory::Security`.
- `InvalidFrame` help text now notes that `Meta` is reserved for system-internal use.
- `--log-file` help text and docs clarified: logs are written to the file *instead of* stderr.
- Pipeline completion log now emits both `elapsed_ns` and `elapsed_us` for correlation with `SandboxDiagnostics`.
- `--dry-run` no longer validates `expected_behavior` against the full-pipeline output (arbitration is intentionally skipped in dry-run mode).
- `HarmonicClock` documented as experimental with `to_phase()` bridge method.
- `TritWord::fals` doc comment explains naming (avoids `false` keyword).

### Fixed
- `ValueJudgment` domain now consistently returns `Hold` even when all input signals share the same frame (regression guard added).
- `benches/trit_bench.rs` field name updated from `final_phase` to `final_phase_raw` to match `SandboxOutput` struct.
- `api/public-api.txt` snapshot regenerated to reflect current public API surface.
- `cargo fmt` and `cargo clippy` now pass cleanly (previously had formatting and `manual_range_contains` issues).
- `clock.rs:116` uses `(-1.0..=1.0).contains(&p)` instead of manual range check.
- Documentation inconsistencies from global audit:
  - `docs/reference/api.md` `ErrorCategory` variants corrected to match `src/sandbox/error.rs`.
  - `docs/reference/api.md` and `docs/how-to/CLI_REFERENCE.md` clarify that `final_value_code: 0` covers both `Hold` and `Unknown`.
  - `docs/reports/validation-report.md` scenario table rewritten to match the actual 16 English + 16 Chinese files; quantitative summary corrected.
  - `docs/explanation/CONCEPTS.md` `TritWord` snippet updated to show private fields; BinaryBaseline stats synced with 16-scenario validation report.
  - `docs/how-to/CLI_REFERENCE.md` scenario list updated to real files; `policy_action` example format corrected.
  - `docs/tutorials/QUICKSTART.md` updated to use `physical_crane_overload.json` and current test count.
  - `README.md` hypothesis claim softened; architecture link updated to v0.3.0; bilingual scenarios noted.
  - `docs/INDEX.md` stale "待更新到 0.3.0" note removed.

## [0.2.0] - 2026-06-18

### Added
- New `src/core/` module unifying `TritValue`, `Phase`, `Frame`, `TritWord`, and `TernaryAlgebra`.
- `Phase::new` strict constructor returning `Result<Phase, PhaseError>`.
- `Phase::new_clamped` for explicit graceful degradation on invalid input.
- `TritWord` invariant-centralized design with private fields and `try_new` / `from_parts` constructors.
- `TritWord::absolute()` factory enforcing `Hold` + neutral phase for the `Absolute` frame.
- `WordError` type for construction failures.
- `FrameError` type for invalid frame strings.
- `sandbox/` layer: `SandboxPipeline`, `ScenarioValidator`, `SandboxError`, and reusable validation logic.
- `tests/sandbox_test.rs` automatically validates all `scenarios/*.json` against `expected_behavior`.
- `tests/core_invariants_test.rs` for `Phase` / `TritWord` / `Absolute` invariant coverage.
- `tests/cli_test.rs` for end-to-end CLI smoke tests.
- `docs/ARCHITECTURE.md` documenting v0.2.0 module layers and invariants.
- `.github/workflows/ci.yml` scenario-validation job and standalone `dhat-profile` job.

### Changed
- **Breaking**: `TritWord` fields are no longer public; use constructors and accessors.
- **Breaking**: `Phase::new` now returns `Result` instead of silently clamping.
- **Breaking**: `ResolutionPolicy::arbitrate` now returns `Result<ArbitrationResult, PolicyError>`.
- **Breaking**: `MetaMonitor::new` no longer takes a `ResolutionPolicy` argument.
- **Breaking**: `SafeFallback` fields are now private; use builder methods.
- `ResolutionPolicy` can now hold an optional `CustomRule` for `Domain::Custom` arbitration.
- `TernaryAlgebra::t_and_hot` / `t_or_hot` now use `assert!` (active in release) instead of `debug_assert!`.
- `Cargo.toml` keywords reduced to 5 to comply with crates.io limits.
- Release profile changed to `lto = "thin"` and `codegen-units = 16` to reduce Windows link memory pressure.

### Removed
- **Removed**: `src/net/` distributed protocol layer.
- **Removed**: `trit-node` binary.
- **Removed**: `tokio` and `uuid` dependencies.
- **Removed**: Network-related tests (`concurrency_test.rs`, `byzantine_test.rs`, `partition_test.rs`, `multi_node_test.rs`).
- **Removed**: Network-related benchmarks from `benches/trit_bench.rs`.

### Fixed
- Corrected `expected_behavior` in 6 scenario files (`medical_conflict_01*`, `medical_conflict_02`, `medical_conflict_03`, `general_negotiation*`).
- Eliminated all `.expect()` panic paths in `ResolutionPolicy::arbitrate`.
- Removed silent `Frame::Meta` fallback on unknown frame strings in the sandbox pipeline.
- Removed redundant `Phase` reconstruction in `SafeFallback::guard`.
- Fixed broken intra-doc links in `TernaryAlgebra` hot-path documentation.

### Refactored (post-audit iteration)
- `TernaryAlgebra::t_sense` now returns `Result<TritWord, PhaseError>`; added `t_sense_clamped` for non-failing sensor input.
- Added `Phase::neutral()`, `Phase::full_true()`, `Phase::full_false()` constant constructors and removed unnecessary `.unwrap()` calls in `TritWord` factory functions.
- `src/bin/dhat_profile.rs` now returns `Result` and uses `?` instead of `.unwrap()` / `.expect()`.
- Re-ran `cargo bench` and rewrote `docs/reference/BENCHMARK.md` and `docs/reports/performance-validation.md` with v0.2.0 measured numbers.
- Added historical-version notices to all v0.1.x documents (whitepaper, preprint, audit reports, ADR-004, and Chinese translations).
- Added `cargo-public-api` CI gate with snapshot `api/public-api.txt` and `scripts/update-public-api.sh`.

## [0.1.0] - 2026-06-18

### Added (M7)
- Network partition tolerance: heartbeat monitoring with per-node timestamps.
- Stale peer detection (`stale_peers()`, `purge_stale_peers()`) with 30s timeout.
- Split-brain detection (`detect_split_brain()`) with 60s timeout.
- TcpClient multi-message session support (BufReader/BufWriter rewrite).
- Connection timeout (5s), read timeout (30s), write timeout (10s).
- 6 partition fault-tolerance tests (connection loss, reconnect, partial partition, standalone, split-brain, heartbeat keepalive).
- dhat heap profiling binary (`src/bin/dhat_profile.rs`): zero-allocation hot path verified.

### Added (M8)
- Byzantine fault tolerance: `ByzantineGatekeeper` with 7 safety checks.
- Message validation layer: phase bounds, sender validation, frame name validation, payload consistency.
- Rate limiting (100 msg/s per peer) and per-peer log cap (1000 entries).
- Known-node enforcement with register/unregister lifecycle.
- Gatekeeper integration in ResonanceBus (optional, zero overhead when disabled).
- TCP server validate-then-dispatch pipeline with REJECTED response prefix.
- 7 Byzantine TCP integration tests + 25 gatekeeper unit tests + 31 message validation tests.
- Total: 305 tests, 0 failures, 0 warnings, 0 clippy issues.

### Added (M9)
- Multi-threaded concurrency stress testing: concurrent bus operations under load.
- Thread-safe ResonanceBus access patterns validated.
- Concurrency test suite (6 tests) covering race conditions and deadlock prevention.

### Changed
- README updated: M0-M9 milestones, 305 tests, updated tech stack.
- All docs: version bumped from 0.1.0-alpha to 0.1.0.
- Roadmap status: Draft → Complete.

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
- Architecture audit report (`docs/zh/explanation/architecture-audit.zh.md`).
- Security audit report (`docs/reports/security-audit.md`): all P1/P2 fixes applied.
- Code quality audit report (`docs/reports/code-quality-audit.md`).
- GitHub Actions CI/CD pipeline: check, lint, test, benchmark, build.
- Benchmark suite (`criterion`) for TAND, TOR, TNOT, cascade operations.
- Preprint (`docs/archive/preprint.md`): 10+ page research paper with abstract, architecture, validation, references.
- Chinese preprint (`docs/zh/archive/preprint.zh.md`): 10+ page Chinese translation.
- M2 validation report (`docs/reports/validation-report.md`): ternary vs binary comparison across 12 scenarios.
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
- Performance validated: 29 criterion benchmarks across 9 groups; 10,000 TPS target exceeded by 65-101x (see docs/reports/performance-validation.md).
