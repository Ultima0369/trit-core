# Layer 4 DecisionEngine Facade — Design Spec

**Date**: 2026-06-20
**Status**: Approved
**Based on**: 5-layer architecture spec v1, trit-core v0.4.0-dev codebase

## Goal

Extract the core ternary decision logic (TAND cascade → arbitration → SafeFallback) from `SandboxPipeline` into a standalone `DecisionEngine` facade in `src/core/`. The pipeline continues to handle validation, OS sampling, attention scheduling, self-knowledge, anchor checks, and calibration — but delegates the "decide" step to `DecisionEngine`.

## Architecture

```
src/core/decision_engine.rs   ← NEW — DecisionEngine struct + decide() method
src/meta/interrupt.rs         ← MODIFIED — add ExplainImpulse variant to ConflictType
src/sandbox/pipeline.rs       ← MODIFIED — delegate core logic to DecisionEngine
src/core/mod.rs               ← MODIFIED — pub mod decision_engine + re-export
src/lib.rs                    ← MODIFIED — re-export DecisionEngine, DecisionResult
src/adapters/cognitive_deconstruction.rs ← MODIFIED — use ExplainImpulse variant
```

## DecisionEngine API

```rust
/// Result of a single ternary decision cycle.
#[derive(Debug, Clone)]
pub struct DecisionResult {
    pub final_word: TritWord,
    pub policy_action: ArbitrationResult,
    pub interrupts: Vec<MetaInterrupt>,
    pub reflexive_alert: Option<ReflexiveAlert>,
    pub safe_fallback_triggered: bool,
}

/// Facade for the ternary decision engine (Layer 4 of the 5-layer architecture).
///
/// Owns SafeFallback configuration and optional reflexive auditor.
/// Does NOT own the ternary algebra (that's stateless), validation logic,
/// attention scheduling, self-knowledge, anchor constraints, or calibration —
/// those remain in SandboxPipeline (the Layer 2–3–5 integrator).
pub struct DecisionEngine {
    safe_fallback: SafeFallback,
    reflexive: Option<ReflexiveAuditor>,
    trace_phase: bool,
}

impl DecisionEngine {
    pub fn new() -> Self
    pub fn with_reflexive(self, auditor: ReflexiveAuditor) -> Self
    pub fn with_trace_phase(self, enabled: bool) -> Self

    /// Run the full ternary decision cycle:
    /// 1. TAND cascade over all trits
    /// 2. Policy arbitration (domain-specific)
    /// 3. Reflexive guard (override forced decisions with unresolved conflicts)
    /// 4. SafeFallback (force False in dangerous domains when uncertain)
    pub fn decide(
        &mut self,
        trits: &[TritWord],
        domain: &Domain,
    ) -> Result<DecisionResult, SandboxError>
}
```

## ConflictType::ExplainImpulse

```rust
pub enum ConflictType {
    FrameMismatch,
    OutOfScope,
    PhaseDrift,
    PolicyViolation,
    /// Cognitive deconstruction detected an explanation impulse:
    /// input entropy is high but output determinacy is high —
    /// the system is about to produce a confident answer without
    /// sufficient evidence.
    ExplainImpulse,
}
```

The `ExplainImpulse` variant is treated by the reflexive guard the same as `FrameMismatch`: if the decision is forced True/False while `ExplainImpulse` interrupts remain unresolved, the reflexive guard overrides to Hold.

## Data Flow

```
SandboxPipeline::run_with_diagnostics()
  │
  ├─ stage_validate_and_build()          ← STAYS: validation + trit building + registry
  │
  ├─ DecisionEngine::decide()            ← EXTRACTED:
  │    ├─ TAND cascade (TernaryAlgebra::t_and_n)
  │    ├─ Policy arbitration (ResolutionPolicy::arbitrate)
  │    ├─ Reflexive guard (override forced + unresolved)
  │    └─ SafeFallback (force False in dangerous domains)
  │
  ├─ stage_sample_budget()               ← STAYS: OS metrics → ComputeBudget
  ├─ stage_optional_extensions()         ← STAYS: attention + self_knowledge
  ├─ stage_tick_clock()                  ← STAYS: harmonic oscillator
  ├─ stage_anchor_check()                ← STAYS: Layer 1 constraints
  └─ stage_calibrate()                   ← STAYS: CalibrationLog + SelfKnowledge
```

## What Moves

From `src/sandbox/pipeline.rs` → `src/core/decision_engine.rs`:

| Source | Destination |
|--------|-------------|
| `stage_tand_cascade()` body | `DecisionEngine::run_tand_cascade()` |
| `stage_arbitrate_and_guard()` body | Core of `DecisionEngine::decide()` |
| `resolve_arbitrated_word()` | `DecisionEngine` private method |
| `stage_reflexive_guard()` body | `DecisionEngine` private method |
| `stage_safe_fallback()` body | `DecisionEngine` private method |
| `reflexive_guard()` free fn | `DecisionEngine` private method |

## What Stays

In `src/sandbox/pipeline.rs`:

- `stage_validate_and_build()` — input validation, trit construction, registry check
- `stage_sample_budget()` — OS-level compute budget sampling
- `stage_optional_extensions()` — attention scheduling, self-knowledge inference
- `stage_tick_clock()` — harmonic clock advancement
- `stage_anchor_check()` — Layer 1 anchor constraint enforcement
- `stage_calibrate()` — calibration log recording + SelfKnowledge feedback
- `build_policy()` / `build_trits()` — input conversion helpers
- `parse_attention_cmd()` / `modulate_attention_with_clock_phase()` — attention utilities

## Cognitive Deconstruction Wiring

The `CognitiveDeconstruction` module currently uses `ConflictType::OutOfScope` for explanation impulses. Changed to `ConflictType::ExplainImpulse` so the reflexive guard can distinguish "this is a cognitive pattern problem" from "this is out of distribution."

## Error Handling

`DecisionEngine::decide()` returns `Result<DecisionResult, SandboxError>`. The only error path is policy arbitration failure (`PolicyError`), which is mapped to `SandboxError::InvalidScenario`. TAND cascade and SafeFallback are infallible.

## Testing Strategy

### decision_engine.rs unit tests (~10)
- TAND cascade same-frame commits True
- TAND cascade cross-frame produces Hold + interrupts
- Arbitration honors domain-specific rules (MedicalEthics preserves Individual)
- Reflexive guard overrides forced True with unresolved FrameMismatch
- Reflexive guard overrides forced True with unresolved ExplainImpulse
- Reflexive guard does NOT override SafeFallback in dangerous domains
- SafeFallback forces False in Physical domain with Hold + interrupts
- DecisionResult fields are correctly populated
- ExplainImpulse interrupts are counted in interrupt list
- Dry-run mode (future: skip arbitration)

### pipeline_test.rs (existing, unchanged)
All 30+ existing integration tests continue to pass — the pipeline internally delegates to DecisionEngine but exposes the same `run()` / `run_with_diagnostics()` API.

### interrupt.rs tests (modified)
- `ExplainImpulse` variant equality and display
- MetaMonitor correctly records `ExplainImpulse` interrupts

## Immutable Design Principles (from 5-layer spec)

1. ✅ Anchor layer does not participate in dynamic adaptation — anchors stay in pipeline, outside DecisionEngine
2. ✅ Module mounting is scenario-driven — unchanged
3. ✅ Hold is not a failure state — DecisionEngine preserves Hold semantics
4. ✅ Unwise True/False can be corrected — unchanged (Layer 5)
5. ✅ Unmount = release — unchanged
6. ✅ Output must pass through practice testing — unchanged (Layer 5)
7. ✅ All decisions operate within non-negotiable baseline — anchor check stays in pipeline
8. ✅ Explanation impulse is detectable and actionable — dedicated `ExplainImpulse` variant
9. ✅ Adaptation is bounded — unchanged
10. ✅ 行业在做工具，我们在做生态位的自我定位 — unchanged

## Files Touched

| File | Action | Lines |
|------|--------|-------|
| `src/core/decision_engine.rs` | CREATE | ~250 |
| `src/core/mod.rs` | MODIFY | +2 |
| `src/meta/interrupt.rs` | MODIFY | +3 (variant + match arm) |
| `src/sandbox/pipeline.rs` | MODIFY | ~80 removed, ~30 added |
| `src/adapters/cognitive_deconstruction.rs` | MODIFY | ~3 changed |
| `src/lib.rs` | MODIFY | +2 re-exports |
| `tests/pipeline_test.rs` | MODIFY | imports only |
