# Layer 5 Feedback Loop — Design Spec

**Date:** 2026-06-20
**Status:** Draft
**Scope:** Option 2 — Correction Loop (~350 lines)

## Purpose

Close the 5-layer cognitive architecture loop. Every decision output is tested
against a proxy environment's prediction. Deviations trigger calibration of
Layer 3 modules. Severe deviations trigger immediate pipeline re-entry with a
correction signal.

This is the algorithmic form of "an unwise True/False can be corrected
immediately."

## Architecture

```
SandboxPipeline::run_with_diagnostics()
    ...
    stage_calibrate()          ← existing (Layer 4 → calibration log)
    stage_feedback_loop()      ← NEW Layer 5
        ProxyEnvironment::predict(decision) → ConsequencePrediction
        PracticeTest::compare(decision, prediction) → Δ
        ConsequenceReview::classify(Δ) → Matched | Deviated | Erroneous
        if Deviated | Erroneous:
            CorrectionTrigger::fire() → FeedbackSignal
            calibrate all CognitiveModules with FeedbackSignal
            if severity == Severe:
                re-enter pipeline with correction as additional signal
        ExperienceRecorder::record(scenario, decision, result)
```

## Files

| File | Lines (est.) | Responsibility |
|------|-------------|----------------|
| `src/feedback/mod.rs` | ~80 | `FeedbackLoop` struct, `PracticeTestResult`, `CorrectionSeverity`, `CorrectionHint`, `ConsequencePrediction`, `FeedbackSignal` |
| `src/feedback/proxy_env.rs` | ~60 | `ProxyEnvironment` trait, `StaticRuleModel` impl |
| `src/feedback/practice_test.rs` | ~50 | `PracticeTest::compare()` — computes deviation Δ |
| `src/feedback/consequence_review.rs` | ~60 | `ConsequenceReview::classify()` — maps Δ to severity |
| `src/feedback/correction.rs` | ~50 | `CorrectionTrigger` — threshold check, signal building |
| `src/feedback/experience_recorder.rs` | ~50 | Ring-buffer of `ExperienceRecord` entries |

**Modified files:**

| File | Change |
|------|--------|
| `src/adapters/mod.rs` | Replace placeholder `FeedbackSignal` with real type; re-export from `crate::feedback` |
| `src/sandbox/pipeline.rs` | Add `proxy: Option<Box<dyn ProxyEnvironment>>` field, `with_proxy()` builder, `stage_feedback_loop()` method |
| `src/lib.rs` | Register `pub mod feedback;` |

## Core Types

### ProxyEnvironment trait

```rust
pub trait ProxyEnvironment: Send + Sync {
    /// Predict the expected consequence of a decision.
    /// Returns None if the decision falls outside the proxy's modeling range.
    fn predict(&self, decision: &SandboxOutput) -> Option<ConsequencePrediction>;

    /// The confidence of this proxy's predictions, in [0.0, 1.0].
    fn confidence(&self) -> f64;

    /// Human-readable name of the proxy.
    fn name(&self) -> &'static str;
}

pub struct ConsequencePrediction {
    pub expected_value: TritValue,
    pub expected_phase: f64,
    pub confidence: f64,
    pub reasoning: String,
}
```

### PracticeTestResult

```rust
pub enum PracticeTestResult {
    /// Decision matches prediction within tolerance.
    Matched { confidence: f64 },
    /// Decision deviates from prediction — correctable.
    Deviated { delta: f64, correction: CorrectionHint },
    /// Decision is fundamentally wrong — requires re-entry.
    Erroneous { reason: String, severity: CorrectionSeverity },
}

pub struct CorrectionHint {
    pub suggested_value: Option<TritValue>,
    pub suggested_phase: Option<f64>,
    pub reason: String,
}

pub enum CorrectionSeverity {
    Mild,      // delta < 0.2 — record only, no correction
    Moderate,  // delta < 0.5 — calibrate modules
    Severe,    // delta >= 0.5 — calibrate + re-enter pipeline
}
```

### FeedbackSignal (replaces placeholder)

```rust
pub struct FeedbackSignal {
    pub test_result: PracticeTestResult,
    pub source_decision_id: String,
    pub deviation_delta: f64,
    pub recommended_scenario: Option<ScenarioType>,
    pub anchor_violations: Vec<String>,
}
```

### FeedbackLoop struct

```rust
pub struct FeedbackLoop {
    proxy: Box<dyn ProxyEnvironment>,
    correction_threshold: f64,      // τ_correction, default 0.3
    reentry_threshold: f64,         // default 0.5
    experience: ExperienceRecorder,
    enabled: bool,
}
```

## StaticRuleModel (MVP ProxyEnvironment)

A hardcoded consequence model for the MVP. Rules:

| Domain | Decision Value | Condition | Expected |
|--------|---------------|-----------|----------|
| Physical | True | phase > 0.8 | True |
| Physical | True | phase ≤ 0.8 | Hold |
| Physical | False | — | False |
| MedicalEthics | True | Individual frame present | Individual value preserved |
| MedicalEthics | False | — | False |
| ValueJudgment | True/False | — | Hold (value judgments should not commit) |
| Engineering | False | Science frame present | False |
| Engineering | True | — | True |
| General (single-frame) | True/False | — | Same value |
| General (cross-frame) | True/False | — | Hold |
| All | Hold | — | Hold |

Confidence starts at 0.6 (explicitly uncertain — this is a static model, not a real environment).

## Integration Point

`SandboxPipeline::run_with_diagnostics()` gains one new stage after `stage_calibrate()`:

```rust
// Stage 14: feedback loop (Layer 5)
self.stage_feedback_loop(scenario, &output, &mut diagnostics);
```

The `SandboxPipeline` struct gains:
- `pub(crate) feedback: Option<FeedbackLoop>` field
- `pub fn with_proxy(mut self, proxy: Box<dyn ProxyEnvironment>) -> Self` builder

Feedback is **opt-in**: a pipeline without `with_proxy()` behaves exactly as before.

## Deviation Computation

Given decision output $o$ and predicted consequence $c$:

$$\Delta = w_v \cdot \delta_v + w_p \cdot \delta_p$$

Where:
- $\delta_v = 1.0$ if values differ, $0.0$ otherwise (weight $w_v = 0.6$)
- $\delta_p = |o.phase - c.phase|$ (weight $w_p = 0.4$)

## What Does NOT Change

- **DecisionEngine** — Layer 4 is unchanged
- **TernaryAlgebra** — core algebra is immutable per design principle 9
- **ResolutionPolicy / SafeFallback** — arbitration is unchanged
- **Anchor constraints** — Layer 1 is unchanged
- **Hook Manager** — Layer 2 is unchanged
- **All existing tests** — feedback is opt-in, pipeline default behavior is identical

## Testing Strategy

1. **Unit tests for StaticRuleModel** — each domain rule produces correct prediction
2. **Unit tests for PracticeTest::compare()** — Δ computation correct for matched/deviated cases
3. **Unit tests for ConsequenceReview::classify()** — correct severity from Δ
4. **Unit tests for CorrectionTrigger** — threshold behavior (below/at/above)
5. **Unit tests for ExperienceRecorder** — ring buffer eviction, record/recall
6. **Pipeline integration test** — `with_proxy()` triggers feedback, without does not
7. **FeedbackSignal round-trip** — placeholder replaced, calibrate() receives real signal

## Design Principles Enforced

- **Principle 4**: An unwise True/False can be corrected immediately — Severe triggers re-entry
- **Principle 6**: Output must pass through practice testing — feedback is the gate
- **Principle 9**: Adaptation is bounded — calibration only tunes module weights, never modifies anchors or algebra
- **Principle 10**: The feedback loop learns from consequences — ExperienceRecorder persists patterns
