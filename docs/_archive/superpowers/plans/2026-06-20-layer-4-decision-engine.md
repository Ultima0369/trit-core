# Layer 4 DecisionEngine Facade — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract core ternary decision logic from `SandboxPipeline` into a standalone `DecisionEngine` facade in `src/core/`, and add `ExplainImpulse` variant to `ConflictType`.

**Architecture:** `DecisionEngine` owns `SafeFallback` and optional `ReflexiveAuditor`. Its `decide()` runs TAND → arbitration → reflexive guard → SafeFallback. `SandboxPipeline` delegates to it while keeping validation, OS sampling, attention, self-knowledge, anchor checks, and calibration.

**Tech Stack:** Rust, no new dependencies.

---

### Task 1: Add `ExplainImpulse` variant to `ConflictType`

**Files:**
- Modify: `src/meta/interrupt.rs`

- [ ] **Step 1: Add the variant**

In `src/meta/interrupt.rs`, change the `ConflictType` enum (lines 64-70):

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
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

- [ ] **Step 2: Update the `conflict_type_equality` test**

In the same file, update the test:

```rust
#[test]
fn conflict_type_equality() {
    assert_eq!(ConflictType::FrameMismatch, ConflictType::FrameMismatch);
    assert_ne!(ConflictType::FrameMismatch, ConflictType::OutOfScope);
    assert_eq!(ConflictType::ExplainImpulse, ConflictType::ExplainImpulse);
    assert_ne!(ConflictType::ExplainImpulse, ConflictType::FrameMismatch);
}
```

- [ ] **Step 3: Build and test**

Run: `cargo test -p trit-core --lib meta::interrupt`
Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/meta/interrupt.rs
git commit -m "feat: add ExplainImpulse variant to ConflictType"
```

---

### Task 2: Create `DecisionEngine` and `DecisionResult` in `src/core/decision_engine.rs`

**Files:**
- Create: `src/core/decision_engine.rs`

- [ ] **Step 1: Write the full file**

Create `src/core/decision_engine.rs`:

```rust
//! DecisionEngine facade — ternary decision pipeline (Layer 4).
//!
//! Extracted from `SandboxPipeline`, this module owns the core decision
//! logic: TAND cascade → policy arbitration → reflexive guard → SafeFallback.
//! It does NOT handle validation, OS sampling, attention scheduling,
//! self-knowledge, anchor checks, or calibration — those remain in the
//! sandbox pipeline (Layer 2–3–5 integrator).

use crate::adapters::reflexive_audit::{ReflexiveAlert, ReflexiveAuditor};
use crate::core::frame::Frame;
use crate::core::value::TritValue;
use crate::core::word::TritWord;
use crate::core::TernaryAlgebra;
use crate::meta::{
    ArbitrationResult, ConflictType, Domain, MetaInterrupt, ResolutionPolicy, SafeFallback,
};
use crate::sandbox::SandboxError;

/// Result of a single ternary decision cycle.
#[derive(Debug, Clone)]
pub struct DecisionResult {
    /// The final word after arbitration, reflexive guard, and SafeFallback.
    pub final_word: TritWord,
    /// The policy action taken by arbitration.
    pub policy_action: ArbitrationResult,
    /// All interrupts collected during the decision cycle.
    pub interrupts: Vec<MetaInterrupt>,
    /// Optional alert from the reflexive guard.
    pub reflexive_alert: Option<ReflexiveAlert>,
    /// Whether SafeFallback was triggered.
    pub safe_fallback_triggered: bool,
}

/// Facade for the ternary decision engine (Layer 4 of the 5-layer architecture).
///
/// Owns SafeFallback configuration and optional reflexive auditor.
/// Does NOT own the ternary algebra (stateless), validation logic,
/// attention scheduling, self-knowledge, anchor constraints, or calibration —
/// those remain in [`SandboxPipeline`](crate::sandbox::SandboxPipeline).
pub struct DecisionEngine {
    safe_fallback: SafeFallback,
    reflexive: Option<ReflexiveAuditor>,
    trace_phase: bool,
}

impl DecisionEngine {
    /// Create a new DecisionEngine with default SafeFallback.
    pub fn new() -> Self {
        DecisionEngine {
            safe_fallback: SafeFallback::new(),
            reflexive: None,
            trace_phase: false,
        }
    }

    /// Attach a reflexive auditor for post-decision self-checking.
    pub fn with_reflexive(mut self, auditor: ReflexiveAuditor) -> Self {
        self.reflexive = Some(auditor);
        self
    }

    /// Enable phase-trace collection in the reflexive auditor.
    pub fn with_trace_phase(mut self, enabled: bool) -> Self {
        self.trace_phase = enabled;
        self
    }

    /// Set a custom SafeFallback configuration.
    pub fn with_safe_fallback(mut self, safe_fallback: SafeFallback) -> Self {
        self.safe_fallback = safe_fallback;
        self
    }

    /// Run the full ternary decision cycle:
    ///
    /// 1. TAND cascade over all trits
    /// 2. Policy arbitration (domain-specific)
    /// 3. Reflexive guard (override forced decisions with unresolved conflicts)
    /// 4. SafeFallback (force False in dangerous domains when uncertain)
    ///
    /// # Errors
    ///
    /// Returns `SandboxError::InvalidScenario` if policy arbitration fails.
    pub fn decide(
        &mut self,
        trits: &[TritWord],
        domain: &Domain,
    ) -> Result<DecisionResult, SandboxError> {
        // Stage 1: TAND cascade
        let (current, mut interrupts) = TernaryAlgebra::t_and_n(trits);

        // Stage 2: policy arbitration
        let policy = ResolutionPolicy::new(domain.clone());
        let policy_result = policy.arbitrate(trits).map_err(|e| {
            SandboxError::InvalidScenario(format!("arbitration failed: {e}"))
        })?;

        let arbitrated_word = self.resolve_arbitrated_word(&policy_result, &current);

        // Stage 3: reflexive guard
        let reflexive_alert =
            self.run_reflexive_guard(&policy, &arbitrated_word, &interrupts);

        // Stage 4: SafeFallback
        let force = matches!(&policy_result, ArbitrationResult::ForceCollapse);
        let (final_word, fb_interrupt) = self.safe_fallback.guard_with_force(
            &policy.domain,
            &arbitrated_word,
            interrupts.len(),
            force,
        );

        let safe_fallback_triggered = fb_interrupt.is_some();
        if let Some(int) = fb_interrupt {
            interrupts.push(int);
        }

        // If reflexive guard fired and output is still forced True/False, override to Hold
        let final_word = if reflexive_alert.is_some() && final_word.value().is_computable() {
            TritWord::hold(Frame::Meta)
        } else {
            final_word
        };

        Ok(DecisionResult {
            final_word,
            policy_action: policy_result,
            interrupts,
            reflexive_alert,
            safe_fallback_triggered,
        })
    }

    // ── Private methods ───────────────────────────────────────────

    /// Resolve the word to use after arbitration.
    fn resolve_arbitrated_word(
        &self,
        policy_result: &ArbitrationResult,
        current: &TritWord,
    ) -> TritWord {
        match policy_result {
            ArbitrationResult::Commit(w) => {
                if current.value() == TritValue::Hold && w.value().is_computable() {
                    TritWord::hold(Frame::Meta)
                } else {
                    *w
                }
            }
            ArbitrationResult::Preserve(w) => *w,
            ArbitrationResult::Hold => TritWord::hold(Frame::Meta),
            ArbitrationResult::ForceCollapse => TritWord::hold(Frame::Meta),
            ArbitrationResult::Negotiate => *current,
        }
    }

    /// Run the reflexive guard — check for forced decisions with unresolved conflicts.
    fn run_reflexive_guard(
        &mut self,
        policy: &ResolutionPolicy,
        arbitrated_word: &TritWord,
        interrupts: &[MetaInterrupt],
    ) -> Option<ReflexiveAlert> {
        if let Some(ref mut auditor) = self.reflexive {
            for int in interrupts {
                auditor.record_interrupt(int.clone());
            }
            if self.trace_phase {
                auditor.record_phase_shift(
                    crate::adapters::reflexive_audit::PhaseShift::new(
                        arbitrated_word.phase().inner(),
                        arbitrated_word.phase().inner(),
                        "arbitration",
                    ),
                );
            }
            return Self::check_reflexive_guard(
                &policy.domain,
                arbitrated_word,
                interrupts,
                &self.safe_fallback,
            );
        }
        None
    }

    /// Check whether a forced True/False decision was made while unresolved
    /// cross-frame conflicts or explanation impulses remain.
    fn check_reflexive_guard(
        domain: &Domain,
        decision: &TritWord,
        interrupts: &[MetaInterrupt],
        safe_fallback: &SafeFallback,
    ) -> Option<ReflexiveAlert> {
        let unresolved_conflicts = interrupts
            .iter()
            .filter(|i| {
                matches!(
                    i.conflict,
                    ConflictType::FrameMismatch | ConflictType::ExplainImpulse
                )
            })
            .count();

        let is_forced =
            decision.value() == TritValue::True || decision.value() == TritValue::False;

        if unresolved_conflicts > 0 && is_forced {
            let dangerous = safe_fallback.is_dangerous(domain);
            if dangerous {
                return None;
            }
            let alert = ReflexiveAlert {
                reason: format!(
                    "Forced {:?} output with {} unresolved conflict(s)",
                    decision.value(),
                    unresolved_conflicts
                ),
                recommendation: "Reflexive guard suggests returning Hold.".to_string(),
            };
            return Some(alert);
        }

        None
    }
}

impl Default for DecisionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::frame::Frame;
    use crate::core::phase::Phase;

    fn word(frame: Frame, value: TritValue, phase: f64) -> TritWord {
        TritWord::new(value, Phase::new_clamped(phase), frame)
    }

    #[test]
    fn decide_same_frame_commits_true() {
        let mut engine = DecisionEngine::new();
        let trits = vec![
            word(Frame::Science, TritValue::True, 0.9),
            word(Frame::Science, TritValue::True, 0.8),
        ];
        let result = engine.decide(&trits, &Domain::General).unwrap();
        assert_eq!(result.final_word.value(), TritValue::True);
        assert_eq!(result.final_word.frame(), Frame::Science);
        assert!(!result.safe_fallback_triggered);
    }

    #[test]
    fn decide_cross_frame_produces_hold() {
        let mut engine = DecisionEngine::new();
        let trits = vec![
            word(Frame::Science, TritValue::True, 0.9),
            word(Frame::Individual, TritValue::False, 0.8),
        ];
        let result = engine.decide(&trits, &Domain::General).unwrap();
        assert_eq!(result.final_word.value(), TritValue::Hold);
        assert!(!result.interrupts.is_empty());
    }

    #[test]
    fn decide_medical_ethics_preserves_individual() {
        let mut engine = DecisionEngine::new();
        let trits = vec![
            word(Frame::Science, TritValue::True, 0.8),
            word(Frame::Individual, TritValue::False, 0.2),
        ];
        let result = engine.decide(&trits, &Domain::MedicalEthics).unwrap();
        assert_eq!(result.final_word.value(), TritValue::False);
        assert_eq!(result.final_word.frame(), Frame::Individual);
    }

    #[test]
    fn decide_value_judgment_holds() {
        let mut engine = DecisionEngine::new();
        let trits = vec![
            word(Frame::Individual, TritValue::False, 0.3),
            word(Frame::Consensus, TritValue::True, 0.7),
        ];
        let result = engine.decide(&trits, &Domain::ValueJudgment).unwrap();
        assert_eq!(result.final_word.value(), TritValue::Hold);
    }

    #[test]
    fn reflexive_guard_overrides_forced_true_with_frame_mismatch() {
        let mut engine = DecisionEngine::new().with_reflexive(ReflexiveAuditor::new());
        let trits = vec![
            word(Frame::Science, TritValue::True, 0.9),
            word(Frame::Individual, TritValue::True, 0.8),
        ];
        let result = engine.decide(&trits, &Domain::General).unwrap();
        assert_eq!(result.final_word.value(), TritValue::Hold);
        assert!(result.reflexive_alert.is_some());
    }

    #[test]
    fn reflexive_guard_overrides_forced_true_with_explain_impulse() {
        let explain_interrupt = MetaInterrupt::new(
            ConflictType::ExplainImpulse,
            "explanation impulse detected".to_string(),
        );
        let decision = TritWord::tru(Frame::Science);
        let sf = SafeFallback::new();
        let alert = DecisionEngine::check_reflexive_guard(
            &Domain::General,
            &decision,
            &[explain_interrupt],
            &sf,
        );
        assert!(alert.is_some());
    }

    #[test]
    fn reflexive_guard_does_not_override_in_dangerous_domain() {
        let mut engine = DecisionEngine::new().with_reflexive(ReflexiveAuditor::new());
        let trits = vec![
            word(Frame::Science, TritValue::True, 0.9),
            word(Frame::Individual, TritValue::True, 0.8),
        ];
        let result = engine.decide(&trits, &Domain::Physical).unwrap();
        // In Physical domain with cross-frame True, arbitration may ForceCollapse
        // → SafeFallback forces False. Reflexive guard should not override.
        assert!(result.safe_fallback_triggered || result.reflexive_alert.is_none());
    }

    #[test]
    fn safe_fallback_forces_false_in_physical_with_hold_and_interrupts() {
        let mut engine = DecisionEngine::new();
        let trits = vec![
            word(Frame::Individual, TritValue::True, 0.9),
            word(Frame::Consensus, TritValue::False, 0.2),
        ];
        let result = engine.decide(&trits, &Domain::Physical).unwrap();
        assert_eq!(result.final_word.value(), TritValue::False);
        assert!(result.safe_fallback_triggered);
    }

    #[test]
    fn decision_result_fields_are_populated() {
        let mut engine = DecisionEngine::new();
        let trits = vec![word(Frame::Science, TritValue::True, 0.9)];
        let result = engine.decide(&trits, &Domain::General).unwrap();
        assert_eq!(result.final_word.value(), TritValue::True);
        assert!(!result.safe_fallback_triggered);
        assert!(result.reflexive_alert.is_none());
        assert!(result.interrupts.is_empty());
    }

    #[test]
    fn single_signal_false_passes_through() {
        let mut engine = DecisionEngine::new();
        let trits = vec![word(Frame::Science, TritValue::False, 0.9)];
        let result = engine.decide(&trits, &Domain::General).unwrap();
        assert_eq!(result.final_word.value(), TritValue::False);
        assert_eq!(result.final_word.frame(), Frame::Science);
    }
}
```

- [ ] **Step 2: Build and test**

Run: `cargo test -p trit-core --lib core::decision_engine`
Expected: All 10 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/core/decision_engine.rs
git commit -m "feat: add DecisionEngine facade with decide() method"
```

---

### Task 3: Register module and re-export types

**Files:**
- Modify: `src/core/mod.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Add module declaration to `src/core/mod.rs`**

Add `pub mod decision_engine;` after `pub mod algebra;` (line 10):

```rust
pub mod algebra;
pub mod decision_engine;
pub mod frame;
```

Add re-exports after the existing `pub use` block:

```rust
pub use algebra::TernaryAlgebra;
pub use decision_engine::{DecisionEngine, DecisionResult};
pub use frame::{Frame, FrameError, FrameRegistry};
```

- [ ] **Step 2: Add re-exports to `src/lib.rs`**

In the `pub use core::` block, add `DecisionEngine` and `DecisionResult`:

```rust
pub use core::{
    algebra::TernaryAlgebra,
    decision_engine::{DecisionEngine, DecisionResult},
    frame::{Frame, FrameError, FrameRegistry},
    // ... rest unchanged
};
```

- [ ] **Step 3: Build and test**

Run: `cargo test -p trit-core --lib`
Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/core/mod.rs src/lib.rs
git commit -m "feat: register DecisionEngine module and re-export types"
```

---

### Task 4: Modify `SandboxPipeline` to delegate to `DecisionEngine`

**Files:**
- Modify: `src/sandbox/pipeline.rs`

This is the main extraction task. The pipeline struct changes to own a `DecisionEngine` instead of separate `safe_fallback`, `reflexive`, and `trace_phase` fields. The `stage_tand_cascade`, `stage_arbitrate_and_guard`, `resolve_arbitrated_word`, `stage_reflexive_guard`, `stage_safe_fallback`, and `reflexive_guard` methods are removed and replaced by a single call to `DecisionEngine::decide()`.

- [ ] **Step 1: Update imports**

Remove the `SafeFallback` import (line 19) since it's now owned by `DecisionEngine`:

```rust
use crate::meta::{ArbitrationResult, Domain, MetaInterrupt, ResolutionPolicy};
```

(Remove `SafeFallback` from this import — it was: `ArbitrationResult, Domain, MetaInterrupt, ResolutionPolicy, SafeFallback`)

- [ ] **Step 2: Update struct definition**

Replace lines 34-52 (struct definition):

```rust
pub struct SandboxPipeline {
    pub(crate) registry: Option<FrameRegistry>,
    pub(crate) dry_run: bool,
    pub(crate) decision_engine: crate::core::decision_engine::DecisionEngine,
    pub(crate) attention: Option<AttentionScheduler>,
    pub(crate) self_knowledge: Option<SelfKnowledge>,
    pub(crate) holder_config: Option<HolderConfig>,
    pub(crate) trace_phase: bool,
    pub(crate) hold_final: bool,
    /// Anchor constraints checked before every decision.
    pub(crate) anchor_constraints: Vec<Box<dyn AnchorConstraint>>,
    /// Hardware-aware compute budget for depth gating.
    pub(crate) budget: ComputeBudget,
    /// Harmonic clock for temporal context.
    pub(crate) clock: HarmonicClock,
    /// Calibration log for feedback-driven learning.
    pub(crate) calibration_log: CalibrationLog,
}
```

- [ ] **Step 3: Update `Debug` impl**

Replace lines 54-72:

```rust
impl std::fmt::Debug for SandboxPipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SandboxPipeline")
            .field("registry", &self.registry)
            .field("dry_run", &self.dry_run)
            .field("decision_engine", &"DecisionEngine { .. }")
            .field("attention", &self.attention)
            .field("self_knowledge", &self.self_knowledge)
            .field("holder_config", &self.holder_config)
            .field("trace_phase", &self.trace_phase)
            .field("hold_final", &self.hold_final)
            .field("anchor_count", &self.anchor_constraints.len())
            .field("budget", &self.budget)
            .field("clock", &self.clock)
            .field("calibration_log", &self.calibration_log)
            .finish()
    }
}
```

- [ ] **Step 4: Update `new()` and builder methods**

Replace `new()` (lines 86-102):

```rust
pub fn new() -> Self {
    Self {
        registry: None,
        dry_run: false,
        decision_engine: crate::core::decision_engine::DecisionEngine::new(),
        attention: None,
        self_knowledge: None,
        holder_config: None,
        trace_phase: false,
        hold_final: false,
        anchor_constraints: Vec::new(),
        budget: ComputeBudget::conservative(),
        clock: HarmonicClock::deliberative(),
        calibration_log: CalibrationLog::default(),
    }
}
```

Replace `with_safe_fallback` (lines 118-121):

```rust
/// Inject a custom SafeFallback configuration.
pub fn with_safe_fallback(mut self, safe_fallback: SafeFallback) -> Self {
    self.decision_engine = self.decision_engine.with_safe_fallback(safe_fallback);
    self
}
```

Replace `with_reflexive` (lines 125-128):

```rust
/// Attach a reflexive auditor.
pub fn with_reflexive(mut self, auditor: ReflexiveAuditor) -> Self {
    self.decision_engine = self.decision_engine.with_reflexive(auditor);
    self
}
```

Replace `with_trace_phase` (lines 149-152):

```rust
/// Enable phase-trace collection.
pub fn with_trace_phase(mut self, enabled: bool) -> Self {
    self.trace_phase = enabled;
    self.decision_engine = self.decision_engine.with_trace_phase(enabled);
    self
}
```

Note: `trace_phase` stays on `SandboxPipeline` (used at line 234 for `diagnostics.record_phase`) in addition to being set on `DecisionEngine` (used in `run_reflexive_guard`).

- [ ] **Step 5: (Removed — consolidated into Steps 6–7)**

- [ ] **Step 6: Add `stage_decide` method**

Add a new method that delegates to `DecisionEngine::decide()` and records diagnostics. Note: diagnostics recording for interrupts/stages is done in `run_with_diagnostics` after the call returns, not inside `stage_decide`, to avoid partial moves:

```rust
    /// Stages 5–8: delegate to DecisionEngine for TAND → arbitration → guard → SafeFallback.
    fn stage_decide(
        &mut self,
        domain_str: &str,
        trits: &[TritWord],
    ) -> Result<crate::core::decision_engine::DecisionResult, SandboxError> {
        if self.dry_run {
            return Ok(crate::core::decision_engine::DecisionResult {
                final_word: trits.first().copied().unwrap_or_else(|| TritWord::hold(Frame::Meta)),
                policy_action: ArbitrationResult::Negotiate,
                interrupts: Vec::new(),
                reflexive_alert: None,
                safe_fallback_triggered: false,
            });
        }

        let domain: Domain = domain_str
            .parse()
            .map_err(|e| SandboxError::InvalidDomain(format!("{}", e)))?;

        self.decision_engine.decide(trits, &domain)
    }
```

- [ ] **Step 7: Update `run_with_diagnostics` to call `stage_decide` and record diagnostics**

Replace the decision section in `run_with_diagnostics` (lines 210-220):

```rust
        // Stages 1–4: validate, build policy, build trits, registry check
        let trits = self.stage_validate_and_build(scenario, &mut diagnostics)?;

        // Stages 5–8: TAND cascade → arbitration → reflexive guard → SafeFallback
        let stage_start = Instant::now();
        let decision_result = self.stage_decide(&scenario.domain, &trits)?;
        diagnostics.record_stage("t_and_n", stage_start);

        // Record arbitration result
        diagnostics.record_policy_action(&decision_result.policy_action);
        diagnostics.record_stage("arbitrate", Instant::now());

        // Record interrupts
        diagnostics.record_interrupts(&decision_result.interrupts);

        // Record reflexive guard
        if decision_result.reflexive_alert.is_some() {
            diagnostics.mark_reflexive_guard();
        }
        diagnostics.record_stage("reflexive_guard", Instant::now());

        // Record SafeFallback
        if decision_result.safe_fallback_triggered {
            diagnostics.mark_safe_fallback();
        }
        diagnostics.interrupts = decision_result.interrupts.clone();
        diagnostics.record_stage("safe_fallback", Instant::now());

        // Stage 8b: sample OS → ComputeBudget.depth_level
        self.stage_sample_budget(&mut diagnostics);

        // Stages 9–10: attention scheduling, self-knowledge inference
        let final_word = decision_result.final_word;
        let policy_action_str = format!("{}", decision_result.policy_action);
        let reflexive_alert = decision_result.reflexive_alert;
        self.stage_optional_extensions(&trits, &final_word, &mut diagnostics);

        // Stage 10b: clock tick — advance the harmonic oscillator
        self.stage_tick_clock(&mut diagnostics);

        // Stage 11: phase trace
        let mut final_word = final_word;
        if self.trace_phase {
            diagnostics.record_phase(final_word.phase().inner());
        }

        // Stage 11b: anchor check (Layer 1)
        final_word = self.stage_anchor_check(scenario, final_word, &mut diagnostics);

        // Stage 12: build output
        let output = self.stage_build_output_with_timing(
            scenario,
            &final_word,
            &policy_action_str,
            reflexive_alert.as_ref(),
            &mut diagnostics,
        );

        // Stage 13: calibrate — record entry + update SelfKnowledge patterns
        self.stage_calibrate(scenario, &final_word, &mut diagnostics);
```

- [ ] **Step 8: Remove old methods**

Delete these methods from `SandboxPipeline`:
- `stage_tand_cascade()` (lines 322-339)
- `stage_arbitrate_and_guard()` (lines 342-394)
- `resolve_arbitrated_word()` (lines 397-430)
- `stage_reflexive_guard()` (lines 433-464)
- `stage_safe_fallback()` (lines 467-495)

Also delete the free function `reflexive_guard()` (lines 818-851).

- [ ] **Step 9: Build and test**

Run: `cargo test -p trit-core --lib`
Expected: All tests pass (core + pipeline).

- [ ] **Step 10: Run integration tests**

Run: `cargo test -p trit-core`
Expected: All tests pass including `tests/pipeline_test.rs`.

- [ ] **Step 11: Format and clippy**

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

- [ ] **Step 12: Commit**

```bash
git add src/sandbox/pipeline.rs
git commit -m "refactor: delegate core decision logic to DecisionEngine"
```

---

### Task 5: Update `CognitiveDeconstruction` to use `ExplainImpulse`

**Files:**
- Modify: `src/adapters/cognitive_deconstruction.rs`

- [ ] **Step 1: Change the ConflictType**

In `src/adapters/cognitive_deconstruction.rs`, line 134, change:

```rust
ConflictType::PolicyViolation,
```

To:

```rust
ConflictType::ExplainImpulse,
```

- [ ] **Step 2: Build and test**

Run: `cargo test -p trit-core --lib adapters::cognitive_deconstruction`
Expected: All tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/adapters/cognitive_deconstruction.rs
git commit -m "feat: use ExplainImpulse variant in cognitive deconstruction"
```

---

### Task 6: Run full test suite and final checks

- [ ] **Step 1: Run all tests**

```bash
cargo test --all-features -- --test-threads=2
```

Expected: All 430+ tests pass.

- [ ] **Step 2: Format check**

```bash
cargo fmt -- --check
```

- [ ] **Step 3: Clippy**

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

- [ ] **Step 4: Update CHANGELOG**

Add to `CHANGELOG.md` under `[0.4.0] - Unreleased`:

```markdown
- **Layer 4 DecisionEngine facade** (`src/core/decision_engine.rs`): extracted core ternary decision logic.
  - `DecisionEngine` struct with `decide()` method: TAND cascade → arbitration → reflexive guard → SafeFallback.
  - `DecisionResult` type: bundles final_word, policy_action, interrupts, reflexive_alert, safe_fallback_triggered.
  - `ConflictType::ExplainImpulse` variant for cognitive deconstruction detection.
  - `SandboxPipeline` delegates to `DecisionEngine` for the decision step.
```

- [ ] **Step 5: Final commit**

```bash
git add CHANGELOG.md
git commit -m "docs: update CHANGELOG for Layer 4 DecisionEngine"
```
