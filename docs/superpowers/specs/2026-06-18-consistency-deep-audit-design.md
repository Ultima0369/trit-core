# Consistency Deep Audit & Doc System Update — Design Spec

**Date**: 2026-06-18
**Status**: Approved
**Builds on**: `2026-06-18-doc-system-update-design.md` (Phase 1-5 completed)

## Goal

Perform a deep consistency audit comparing actual source code against all documentation, then fix every discovered discrepancy. This is a **code→doc** audit (source of truth is the code).

## Audit Methodology

1. Read every public API surface in `src/`
2. Cross-reference against `docs/api.md`, `CLAUDE.md`, `README.md`, `CHANGELOG.md`
3. Check test counts via `cargo test -- --list`
4. Verify all documented behaviors match actual implementations
5. Check cross-references between docs for staleness

## Findings & Fix Plan

### Category F: Factual Errors (code disagrees with doc)

| ID | File | Current (Wrong) | Correct |
|----|------|----------------|---------|
| F1 | README.md:10 | "298 passing" | "305 passing" |
| F2 | CHANGELOG.md:27 | "298 tests" | "305 tests" |
| F3 | REVIEWER_GUIDE.md:30 | "227 个" | "305 个" |
| F4 | CLAUDE.md:17 | "Three discrete states" | "Four discrete states: True, Hold, False, Unknown" |
| F5 | CLAUDE.md:30 | "Phase panics on out-of-range" | "Phase clamps out-of-range with warning; use try_new() for strict rejection" |
| F6 | CLAUDE.md:47 | "bounds-checking" | "bounds-clamping with NaN/Inf protection" |
| F7 | api.md:35 | Phase::new() "panics if out of [0.0, 1.0]" | "clamps out-of-range values with tracing warning; try_new() returns Err for strict validation" |
| F8 | api.md:163-169 | SafeFallback API: is_enabled(), set_enabled(), guard(result, interrupts, domain) | enabled: bool (pub field), guard(domain, result, interrupt_count) |
| F9 | api.md:173-179 | CustomRule: domain, priority_frame, fallback_policy fields | name, priority_frame: Option<String>, allow_forced_collapse: bool, fallback |
| F10 | api.md:184-186 | RuleLoader::load(&self, source) -> Vec<CustomRule> | load(path) -> Result<CustomRule, Error>, load_json(json), apply(rule, inputs) |

### Category M: Missing Content (code has it, doc doesn't)

| ID | What | Where to Add |
|----|------|-------------|
| M1 | TritValue::Unknown variant description | CLAUDE.md architecture section |
| M2 | TritWord::unknown(frame) constructor | api.md §1 |
| M3 | Commitment enum (TowardTrue/False/Neutral) | api.md §1 |
| M4 | Phase::try_new() strict constructor | api.md §1 |
| M5 | TernaryAlgebra hot path: precheck_same_frame, t_and_hot, t_or_hot | api.md §1 |
| M6 | ByzantineGatekeeper + GateRejection (M8) | api.md §5, MODULES.md, README.md |
| M7 | M7 partition: stale_peers, purge_stale_peers, detect_split_brain | api.md §5, MODULES.md |
| M8 | HEARTBEAT_TIMEOUT_SECS, SPLIT_BRAIN_TIMEOUT_SECS constants | api.md §5 |
| M9 | dhat-profile binary | README.md build section |
| M10 | Frame::from_str() | api.md §2 |

### Category S: Stale References

| ID | File | Issue | Fix |
|----|------|-------|-----|
| S1 | ARCHITECTURE.md §7 | Title "M4–M6", missing M7/M8 | → "M4–M8", add M7+M8 subsections |
| S2 | MODULES.md net/ | "NEW" markers on old M5/M6 files, missing gate.rs | Remove NEW, add gate.rs |
| S3 | REVIEWER_GUIDE.md:88 | "M4–M7" | → "M4–M9" |
| S4 | api.md §5 title | "M4-M6" | → "M4-M8" |
| S5 | README.md:58 | docs/ description understates 37-file system | Expand description |
| S6 | CHANGELOG.md | Missing M9 entry | Add M9 concurrency stress testing |
| S7 | MODULES.md | Line counts drifted (phase.rs 148→149, algebra.rs 220→221) | Remove line counts (they rot) |

### Category I: Minor Inconsistencies

| ID | Issue | Fix |
|----|-------|-----|
| I1 | CHANGELOG.md:78 verbose perf detail | Shorten, reference perf doc |
| I2 | api.md §3 FrameMask listed as pub(crate) in public API doc | Note it's internal, or remove |
| I3 | CLAUDE.md missing gate.rs in net/ listing | Add gate.rs |
| I4 | CLAUDE.md missing dhat_profile.rs binary | Add to binary listing |

## Files to Modify

1. `README.md` — test count, doc description, net/ description, dhat-profile binary
2. `CHANGELOG.md` — test counts, M9 entry, shorten perf note
3. `CLAUDE.md` — TritValue 4-state, Phase clamping, gate.rs, dhat_profile.rs
4. `docs/api.md` — Phase, SafeFallback, CustomRule, RuleLoader, M7/M8 APIs, hot path, Commitment, Unknown
5. `docs/concepts/ARCHITECTURE.md` — M4→M8, M7/M8 subsections
6. `docs/development/MODULES.md` — remove NEW, add gate.rs, M7/M8 functions, remove line counts
7. `docs/REVIEWER_GUIDE.md` — test count, M4→M9

## Verification

```bash
cargo test --all-features  # must pass 305 tests
cargo clippy --all-targets --all-features -- -D warnings  # must pass
cargo fmt -- --check  # must pass

# No stale test counts
grep -rn "298" --include="*.md" README.md CHANGELOG.md docs/ CLAUDE.md
grep -rn "227" --include="*.md" README.md CHANGELOG.md docs/ CLAUDE.md
# 227 is correct only in CHANGELOG alpha section — verify context
```
