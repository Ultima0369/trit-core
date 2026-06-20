# Layer 5 Feedback Loop — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the Layer 5 Feedback Loop (`src/feedback/`) — close the 5-layer cognitive architecture by testing decisions against a proxy environment, classifying deviations, triggering corrections, and recording experience.

**Architecture:** Six new files in `src/feedback/` implement the feedback pipeline: `ProxyEnvironment` trait + `StaticRuleModel`, `PracticeTest` comparison, `ConsequenceReview` classification, `CorrectionTrigger` thresholding, `ExperienceRecorder` ring buffer, and a `FeedbackLoop` facade. The `SandboxPipeline` gains an opt-in `stage_feedback_loop()` wired after `stage_calibrate()`. The placeholder `FeedbackSignal` in `src/adapters/mod.rs` is replaced with the real type.

**Tech Stack:** Rust, no new dependencies. Uses existing `TritValue`, `Phase`, `SandboxOutput`, `ScenarioType`, `CognitiveModule` types.

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `src/feedback/mod.rs` | Create | `FeedbackLoop` facade, `PracticeTestResult`, `CorrectionSeverity`, `CorrectionHint`, `ConsequencePrediction`, `FeedbackSignal` |
| `src/feedback/proxy_env.rs` | Create | `ProxyEnvironment` trait, `StaticRuleModel` |
| `src/feedback/practice_test.rs` | Create | `PracticeTest::compare()` — deviation computation |
| `src/feedback/consequence_review.rs` | Create | `ConsequenceReview::classify()` — severity from Δ |
| `src/feedback/correction.rs` | Create | `CorrectionTrigger` — threshold check, signal building |
| `src/feedback/experience_recorder.rs` | Create | `ExperienceRecorder` — ring buffer of `ExperienceRecord` |
| `src/adapters/mod.rs` | Modify | Replace placeholder `FeedbackSignal` with re-export from `crate::feedback` |
| `src/sandbox/pipeline.rs` | Modify | Add `feedback: Option<FeedbackLoop>`, `with_feedback()`, `stage_feedback_loop()` |
| `src/lib.rs` | Modify | Register `pub mod feedback;` |

---

### Task 1: Create `src/feedback/mod.rs` — core types and FeedbackLoop facade

**Files:**
- Create: `src/feedback/mod.rs`

- [ ] **Step 1: Write the module file with all core types**

```rust
//! Feedback Loop — practice testing and corrective control (Layer 5).
//!
//! Every decision output is tested against a proxy environment's prediction.
//! Deviations trigger calibration of Layer 3 modules. Severe deviations
//! trigger immediate pipeline re-entry with a correction signal.
//!
//! This is the algorithmic form of "an unwise True/False can be corrected
//! immediately" (Design Principle 4).

pub mod consequence_review;
pub mod correction;
pub mod experience_recorder;
pub mod practice_test;
pub mod proxy_env;

use crate::core::value::TritValue;
use crate::hook::ScenarioType;
use crate::sandbox::SandboxOutput;

// ── ConsequencePrediction ──────────────────────────────────────────

/// Predicted consequence of a decision from a proxy environment.
#[derive(Debug, Clone)]
pub struct ConsequencePrediction {
    /// The expected trit value.
    pub expected_value: TritValue,
    /// The expected phase tendency.
    pub expected_phase: f64,
    /// Confidence of this prediction in [0.0, 1.0].
    pub confidence: f64,
    /// Human-readable reasoning for the prediction.
    pub reasoning: String,
}

// ── PracticeTestResult ─────────────────────────────────────────────

/// Result of comparing a decision against a proxy prediction.
#[derive(Debug, Clone)]
pub enum PracticeTestResult {
    /// Decision matches prediction within tolerance.
    Matched { confidence: f64 },
    /// Decision deviates from prediction — correctable.
    Deviated {
        delta: f64,
        correction: CorrectionHint,
    },
    /// Decision is fundamentally wrong — requires re-entry.
    Erroneous {
        reason: String,
        severity: CorrectionSeverity,
    },
}

// ── CorrectionHint ─────────────────────────────────────────────────

/// Suggestion for correcting a deviated decision.
#[derive(Debug, Clone)]
pub struct CorrectionHint {
    /// Suggested value override, if any.
    pub suggested_value: Option<TritValue>,
    /// Suggested phase override, if any.
    pub suggested_phase: Option<f64>,
    /// Human-readable reason for the correction.
    pub reason: String,
}

// ── CorrectionSeverity ─────────────────────────────────────────────

/// Severity level for a decision deviation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CorrectionSeverity {
    /// delta < 0.2 — record only, no active correction.
    Mild,
    /// delta < 0.5 — calibrate modules.
    Moderate,
    /// delta >= 0.5 — calibrate + re-enter pipeline.
    Severe,
}

// ── FeedbackSignal ─────────────────────────────────────────────────

/// Feedback signal from Layer 5's practice testing.
///
/// This replaces the placeholder in `src/adapters/mod.rs`. It carries
/// the full practice test result, deviation delta, and any recommended
/// scenario type for re-entry.
#[derive(Debug, Clone)]
pub struct FeedbackSignal {
    /// The practice test result.
    pub test_result: PracticeTestResult,
    /// ID of the scenario that produced this feedback.
    pub source_decision_id: String,
    /// Deviation magnitude (0.0 = perfect match, 1.0 = total mismatch).
    pub deviation_delta: f64,
    /// Recommended scenario type for re-entry, if any.
    pub recommended_scenario: Option<ScenarioType>,
    /// Anchor violations detected during the decision.
    pub anchor_violations: Vec<String>,
}

// ── FeedbackLoop facade ────────────────────────────────────────────

use proxy_env::ProxyEnvironment;

/// Layer 5 feedback loop — practice tests every decision against a proxy
/// environment and triggers correction when deviations exceed thresholds.
pub struct FeedbackLoop {
    /// The proxy environment for consequence prediction.
    proxy: Box<dyn ProxyEnvironment>,
    /// Threshold for triggering correction (τ_correction). Default 0.3.
    correction_threshold: f64,
    /// Threshold for triggering pipeline re-entry. Default 0.5.
    reentry_threshold: f64,
    /// Experience recorder for successful patterns.
    experience: experience_recorder::ExperienceRecorder,
    /// Whether the feedback loop is active.
    enabled: bool,
}

impl FeedbackLoop {
    /// Create a new feedback loop with the given proxy environment.
    pub fn new(proxy: Box<dyn ProxyEnvironment>) -> Self {
        FeedbackLoop {
            proxy,
            correction_threshold: 0.3,
            reentry_threshold: 0.5,
            experience: experience_recorder::ExperienceRecorder::new(32),
            enabled: true,
        }
    }

    /// Set the correction threshold (τ_correction).
    pub fn with_correction_threshold(mut self, threshold: f64) -> Self {
        self.correction_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Set the re-entry threshold.
    pub fn with_reentry_threshold(mut self, threshold: f64) -> Self {
        self.reentry_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Disable the feedback loop (no-op for pipelines without feedback).
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Access the proxy environment.
    pub fn proxy(&self) -> &dyn ProxyEnvironment {
        self.proxy.as_ref()
    }

    /// Whether the feedback loop is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Run the full feedback cycle:
    ///
    /// 1. Predict expected consequence via proxy
    /// 2. Compare decision against prediction (PracticeTest)
    /// 3. Classify deviation severity (ConsequenceReview)
    /// 4. If deviated/erroneous, build FeedbackSignal
    /// 5. Record experience
    ///
    /// Returns `Some(FeedbackSignal)` if correction is needed, `None` if matched.
    pub fn run_cycle(&mut self, decision: &SandboxOutput) -> Option<FeedbackSignal> {
        if !self.enabled {
            return None;
        }

        // Step 1: predict
        let prediction = self.proxy.predict(decision)?;

        // Step 2: compare
        let test_result = practice_test::PracticeTest::compare(decision, &prediction);

        // Step 3: classify
        let severity = consequence_review::ConsequenceReview::classify(&test_result);

        // Step 4: build signal if needed
        let signal = correction::CorrectionTrigger::evaluate(
            &test_result,
            severity,
            &decision.scenario_id,
            self.correction_threshold,
            self.reentry_threshold,
        );

        // Step 5: record
        self.experience.record(experience_recorder::ExperienceRecord {
            scenario_id: decision.scenario_id.clone(),
            result_value: decision.final_value_code,
            result_phase: decision.final_phase_raw,
            matched: matches!(&test_result, PracticeTestResult::Matched { .. }),
            deviation_delta: match &test_result {
                PracticeTestResult::Matched { .. } => 0.0,
                PracticeTestResult::Deviated { delta, .. } => *delta,
                PracticeTestResult::Erroneous { .. } => 1.0,
            },
        });

        signal
    }

    /// Number of recorded experiences.
    pub fn experience_count(&self) -> usize {
        self.experience.len()
    }

    /// Fraction of experiences that were matched.
    pub fn match_rate(&self) -> f64 {
        self.experience.match_rate()
    }
}

impl std::fmt::Debug for FeedbackLoop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FeedbackLoop")
            .field("proxy", &self.proxy.name())
            .field("correction_threshold", &self.correction_threshold)
            .field("reentry_threshold", &self.reentry_threshold)
            .field("enabled", &self.enabled)
            .field("experience_count", &self.experience.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::value::TritValue;
    use proxy_env::StaticRuleModel;

    #[test]
    fn feedback_loop_disabled_returns_none() {
        let proxy = Box::new(StaticRuleModel::new());
        let mut fl = FeedbackLoop::new(proxy).with_enabled(false);
        // Build a minimal SandboxOutput manually
        let output = SandboxOutput {
            scenario_id: "test".into(),
            final_value: "True".into(),
            final_value_code: 1,
            final_frame: "Science".into(),
            final_phase_raw: 0.9,
            interrupts: vec![],
            policy_action: "Commit".into(),
            reflexive_alert: None,
            attention_cmd: None,
            receiver_estimate: None,
            hold_state: None,
        };
        assert!(fl.run_cycle(&output).is_none());
    }

    #[test]
    fn feedback_loop_enabled_runs_cycle() {
        let proxy = Box::new(StaticRuleModel::new());
        let mut fl = FeedbackLoop::new(proxy);
        let output = SandboxOutput {
            scenario_id: "test".into(),
            final_value: "True".into(),
            final_value_code: 1,
            final_frame: "Science".into(),
            final_phase_raw: 0.9,
            interrupts: vec![],
            policy_action: "Commit(Science)".into(),
            reflexive_alert: None,
            attention_cmd: None,
            receiver_estimate: None,
            hold_state: None,
        };
        let result = fl.run_cycle(&output);
        // Single-frame Science True should match StaticRuleModel prediction
        // (result may be None for matched, or Some for deviated depending on rule)
        // Just verify it doesn't panic and records experience
        assert!(fl.experience_count() > 0);
    }

    #[test]
    fn feedback_loop_experience_count_increments() {
        let proxy = Box::new(StaticRuleModel::new());
        let mut fl = FeedbackLoop::new(proxy);
        let output = SandboxOutput {
            scenario_id: "t1".into(),
            final_value: "True".into(),
            final_value_code: 1,
            final_frame: "Science".into(),
            final_phase_raw: 0.9,
            interrupts: vec![],
            policy_action: "Commit(Science)".into(),
            reflexive_alert: None,
            attention_cmd: None,
            receiver_estimate: None,
            hold_state: None,
        };
        fl.run_cycle(&output);
        fl.run_cycle(&output);
        assert_eq!(fl.experience_count(), 2);
    }

    #[test]
    fn feedback_loop_match_rate() {
        let proxy = Box::new(StaticRuleModel::new());
        let mut fl = FeedbackLoop::new(proxy);
        let matched_output = SandboxOutput {
            scenario_id: "m1".into(),
            final_value: "True".into(),
            final_value_code: 1,
            final_frame: "Science".into(),
            final_phase_raw: 0.9,
            interrupts: vec![],
            policy_action: "Commit(Science)".into(),
            reflexive_alert: None,
            attention_cmd: None,
            receiver_estimate: None,
            hold_state: None,
        };
        fl.run_cycle(&matched_output);
        let rate = fl.match_rate();
        assert!((0.0..=1.0).contains(&rate));
    }

    #[test]
    fn feedback_loop_debug_format() {
        let proxy = Box::new(StaticRuleModel::new());
        let fl = FeedbackLoop::new(proxy);
        let debug = format!("{:?}", fl);
        assert!(debug.contains("FeedbackLoop"));
        assert!(debug.contains("StaticRuleModel"));
    }
}
```

- [ ] **Step 2: Verify it compiles (will fail — submodules not yet created)**

Run: `cargo build --lib 2>&1 | head -20`
Expected: errors about missing submodules (`proxy_env`, `practice_test`, etc.)

- [ ] **Step 3: Commit**

```bash
git add src/feedback/mod.rs
git commit -m "feat: add feedback module skeleton with core types and FeedbackLoop facade"
```

---

### Task 2: Create `src/feedback/proxy_env.rs` — ProxyEnvironment trait + StaticRuleModel

**Files:**
- Create: `src/feedback/proxy_env.rs`

- [ ] **Step 1: Write the proxy environment module**

```rust
//! Proxy environment for consequence prediction (MVP).
//!
//! In early implementation, decisions cannot be tested against real-world
//! consequences. The [`ProxyEnvironment`] trait provides an approximate
//! consequence model. [`StaticRuleModel`] is the MVP implementation using
//! hand-coded consequence rules.

use crate::core::value::TritValue;
use crate::sandbox::SandboxOutput;

use super::ConsequencePrediction;

// ── ProxyEnvironment trait ─────────────────────────────────────────

/// A proxy for predicting the consequences of a decision.
///
/// Implementations range from static rule models (MVP) to simulated
/// environments, and eventually real-world outcome data.
pub trait ProxyEnvironment: Send + Sync {
    /// Predict the expected consequence of a decision.
    /// Returns None if the decision falls outside the proxy's modeling range.
    fn predict(&self, decision: &SandboxOutput) -> Option<ConsequencePrediction>;

    /// The confidence of this proxy's predictions, in [0.0, 1.0].
    fn confidence(&self) -> f64;

    /// Human-readable name of the proxy.
    fn name(&self) -> &'static str;
}

// ── StaticRuleModel ────────────────────────────────────────────────

/// MVP proxy environment using hand-coded consequence rules.
///
/// Rules are domain-specific and based on the decision's value, frame,
/// and phase. Confidence is 0.6 — explicitly uncertain, since this is
/// a static model, not a real environment.
pub struct StaticRuleModel {
    confidence: f64,
}

impl StaticRuleModel {
    /// Create a new StaticRuleModel with default confidence (0.6).
    pub fn new() -> Self {
        StaticRuleModel { confidence: 0.6 }
    }

    /// Create a StaticRuleModel with a custom confidence.
    pub fn with_confidence(confidence: f64) -> Self {
        StaticRuleModel {
            confidence: confidence.clamp(0.0, 1.0),
        }
    }

    /// Determine if the decision involves cross-frame signals.
    fn is_cross_frame(output: &SandboxOutput) -> bool {
        // Cross-frame decisions typically have Meta frame or Hold value
        output.final_frame == "Meta" || output.final_value_code == 0
    }

    /// Determine if the decision involves an Individual frame.
    fn has_individual_frame(output: &SandboxOutput) -> bool {
        output.policy_action.contains("Preserve") && output.final_frame == "Individual"
    }

    /// Determine if the decision involves a Science frame.
    fn has_science_frame(output: &SandboxOutput) -> bool {
        output.final_frame == "Science"
    }
}

impl Default for StaticRuleModel {
    fn default() -> Self {
        Self::new()
    }
}

impl ProxyEnvironment for StaticRuleModel {
    fn predict(&self, decision: &SandboxOutput) -> Option<ConsequencePrediction> {
        let value = TritValue::from(decision.final_value_code);

        // Rule: Hold decisions always expected to be Hold
        if value == TritValue::Hold {
            return Some(ConsequencePrediction {
                expected_value: TritValue::Hold,
                expected_phase: 0.5,
                confidence: self.confidence,
                reasoning: "Hold decisions should remain Hold — suspension is self-consistent"
                    .into(),
            });
        }

        // Rule: cross-frame decisions should be Hold
        if Self::is_cross_frame(decision) && value.is_computable() {
            return Some(ConsequencePrediction {
                expected_value: TritValue::Hold,
                expected_phase: 0.5,
                confidence: self.confidence,
                reasoning: "Cross-frame computable decision — expected Hold due to frame conflict"
                    .into(),
            });
        }

        // Rule: Individual frame preservation
        if Self::has_individual_frame(decision) {
            return Some(ConsequencePrediction {
                expected_value: value,
                expected_phase: decision.final_phase_raw,
                confidence: self.confidence,
                reasoning: "Individual frame preserved — decision aligns with first-person priority"
                    .into(),
            });
        }

        // Rule: Science frame with high phase → expect True
        if Self::has_science_frame(decision) && decision.final_phase_raw > 0.8 {
            return Some(ConsequencePrediction {
                expected_value: TritValue::True,
                expected_phase: decision.final_phase_raw,
                confidence: self.confidence,
                reasoning: "Science frame with high phase — expect confident True".into(),
            });
        }

        // Rule: Science frame with low phase → expect Hold
        if Self::has_science_frame(decision) && decision.final_phase_raw <= 0.8 && value == TritValue::True {
            return Some(ConsequencePrediction {
                expected_value: TritValue::Hold,
                expected_phase: 0.5,
                confidence: self.confidence,
                reasoning: "Science frame True with moderate/low phase — expect Hold (insufficient confidence)".into(),
            });
        }

        // Default: expect same value
        Some(ConsequencePrediction {
            expected_value: value,
            expected_phase: decision.final_phase_raw,
            confidence: self.confidence,
            reasoning: "Default: decision value matches expected consequence".into(),
        })
    }

    fn confidence(&self) -> f64 {
        self.confidence
    }

    fn name(&self) -> &'static str {
        "StaticRuleModel"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output(value_code: i8, frame: &str, phase: f64, policy: &str) -> SandboxOutput {
        SandboxOutput {
            scenario_id: "test".into(),
            final_value: match value_code {
                1 => "True".into(),
                0 => "Hold".into(),
                -1 => "False".into(),
                _ => "Unknown".into(),
            },
            final_value_code: value_code,
            final_frame: frame.into(),
            final_phase_raw: phase,
            interrupts: vec![],
            policy_action: policy.into(),
            reflexive_alert: None,
            attention_cmd: None,
            receiver_estimate: None,
            hold_state: None,
        }
    }

    #[test]
    fn predict_hold_stays_hold() {
        let model = StaticRuleModel::new();
        let out = output(0, "Meta", 0.5, "Hold");
        let pred = model.predict(&out).unwrap();
        assert_eq!(pred.expected_value, TritValue::Hold);
    }

    #[test]
    fn predict_cross_frame_computable_expects_hold() {
        let model = StaticRuleModel::new();
        let out = output(1, "Meta", 0.9, "Negotiate");
        let pred = model.predict(&out).unwrap();
        assert_eq!(pred.expected_value, TritValue::Hold);
    }

    #[test]
    fn predict_individual_preserved() {
        let model = StaticRuleModel::new();
        let out = output(-1, "Individual", 0.3, "Preserve(Individual)");
        let pred = model.predict(&out).unwrap();
        assert_eq!(pred.expected_value, TritValue::False);
    }

    #[test]
    fn predict_science_high_phase_expects_true() {
        let model = StaticRuleModel::new();
        let out = output(1, "Science", 0.9, "Commit(Science)");
        let pred = model.predict(&out).unwrap();
        assert_eq!(pred.expected_value, TritValue::True);
    }

    #[test]
    fn predict_science_moderate_phase_true_expects_hold() {
        let model = StaticRuleModel::new();
        let out = output(1, "Science", 0.6, "Commit(Science)");
        let pred = model.predict(&out).unwrap();
        assert_eq!(pred.expected_value, TritValue::Hold);
    }

    #[test]
    fn predict_science_false_passes_through() {
        let model = StaticRuleModel::new();
        let out = output(-1, "Science", 0.9, "Commit(Science)");
        let pred = model.predict(&out).unwrap();
        assert_eq!(pred.expected_value, TritValue::False);
    }

    #[test]
    fn proxy_confidence() {
        let model = StaticRuleModel::new();
        assert_float_eq!(model.confidence(), 0.6);
    }

    #[test]
    fn proxy_name() {
        let model = StaticRuleModel::new();
        assert_eq!(model.name(), "StaticRuleModel");
    }

    #[test]
    fn custom_confidence() {
        let model = StaticRuleModel::with_confidence(0.8);
        assert_float_eq!(model.confidence(), 0.8);
    }
}
```

- [ ] **Step 2: Verify it compiles (will fail — missing practice_test, consequence_review, correction, experience_recorder)**

Run: `cargo build --lib 2>&1 | head -20`
Expected: errors about missing submodules

- [ ] **Step 3: Commit**

```bash
git add src/feedback/proxy_env.rs
git commit -m "feat: add ProxyEnvironment trait and StaticRuleModel MVP"
```

---

### Task 3: Create `src/feedback/practice_test.rs` — decision vs prediction comparison

**Files:**
- Create: `src/feedback/practice_test.rs`

- [ ] **Step 1: Write the practice test module**

```rust
//! Practice test — compare decision output against proxy prediction.
//!
//! Computes the deviation Δ between a decision and its predicted consequence.
//! Δ = w_v · δ_v + w_p · δ_p, where δ_v is value mismatch (0 or 1) and
//! δ_p is phase difference.

use crate::core::value::TritValue;
use crate::sandbox::SandboxOutput;

use super::{ConsequencePrediction, CorrectionHint, PracticeTestResult};

/// Weight for value mismatch in deviation computation.
const VALUE_WEIGHT: f64 = 0.6;

/// Weight for phase difference in deviation computation.
const PHASE_WEIGHT: f64 = 0.4;

/// Tolerance for considering a match "close enough."
const MATCH_TOLERANCE: f64 = 0.15;

/// Stateless practice test comparator.
pub struct PracticeTest;

impl PracticeTest {
    /// Compare a decision output against a proxy prediction.
    ///
    /// Returns `Matched` if Δ < tolerance, `Deviated` with correction hint
    /// if Δ ≥ tolerance, or `Erroneous` if the deviation is extreme (Δ > 0.8).
    pub fn compare(
        decision: &SandboxOutput,
        prediction: &ConsequencePrediction,
    ) -> PracticeTestResult {
        let decision_value = TritValue::from(decision.final_value_code);

        // Value mismatch: 1.0 if values differ, 0.0 if same
        let delta_v = if decision_value != prediction.expected_value {
            1.0
        } else {
            0.0
        };

        // Phase difference: absolute difference
        let delta_p = (decision.final_phase_raw - prediction.expected_phase).abs();

        // Weighted deviation
        let delta = VALUE_WEIGHT * delta_v + PHASE_WEIGHT * delta_p;

        if delta < MATCH_TOLERANCE {
            PracticeTestResult::Matched {
                confidence: prediction.confidence,
            }
        } else if delta > 0.8 {
            PracticeTestResult::Erroneous {
                reason: format!(
                    "Extreme deviation Δ={:.3}: decision={:?}/{} vs expected={:?}/{}",
                    delta,
                    decision_value,
                    decision.final_phase_raw,
                    prediction.expected_value,
                    prediction.expected_phase
                ),
                severity: super::CorrectionSeverity::Severe,
            }
        } else {
            let correction = CorrectionHint {
                suggested_value: if delta_v > 0.0 {
                    Some(prediction.expected_value)
                } else {
                    None
                },
                suggested_phase: if delta_p > 0.1 {
                    Some(prediction.expected_phase)
                } else {
                    None
                },
                reason: format!(
                    "Deviation Δ={:.3}: δ_v={:.3}, δ_p={:.3}",
                    delta, delta_v, delta_p
                ),
            };
            PracticeTestResult::Deviated { delta, correction }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output(value_code: i8, phase: f64) -> SandboxOutput {
        SandboxOutput {
            scenario_id: "test".into(),
            final_value: match value_code {
                1 => "True".into(),
                0 => "Hold".into(),
                -1 => "False".into(),
                _ => "Unknown".into(),
            },
            final_value_code: value_code,
            final_frame: "Science".into(),
            final_phase_raw: phase,
            interrupts: vec![],
            policy_action: "Commit".into(),
            reflexive_alert: None,
            attention_cmd: None,
            receiver_estimate: None,
            hold_state: None,
        }
    }

    fn prediction(value: TritValue, phase: f64) -> ConsequencePrediction {
        ConsequencePrediction {
            expected_value: value,
            expected_phase: phase,
            confidence: 0.6,
            reasoning: "test".into(),
        }
    }

    #[test]
    fn exact_match_is_matched() {
        let out = output(1, 0.9);
        let pred = prediction(TritValue::True, 0.9);
        let result = PracticeTest::compare(&out, &pred);
        assert!(matches!(result, PracticeTestResult::Matched { .. }));
    }

    #[test]
    fn small_phase_diff_is_matched() {
        let out = output(1, 0.85);
        let pred = prediction(TritValue::True, 0.9);
        let result = PracticeTest::compare(&out, &pred);
        // Δ = 0.6*0 + 0.4*0.05 = 0.02 < 0.15 → Matched
        assert!(matches!(result, PracticeTestResult::Matched { .. }));
    }

    #[test]
    fn value_mismatch_is_deviated() {
        let out = output(1, 0.9);
        let pred = prediction(TritValue::False, 0.9);
        let result = PracticeTest::compare(&out, &pred);
        // Δ = 0.6*1 + 0.4*0 = 0.6 → Deviated
        assert!(matches!(result, PracticeTestResult::Deviated { .. }));
    }

    #[test]
    fn large_phase_diff_is_deviated() {
        let out = output(1, 0.9);
        let pred = prediction(TritValue::True, 0.1);
        let result = PracticeTest::compare(&out, &pred);
        // Δ = 0.6*0 + 0.4*0.8 = 0.32 → Deviated
        assert!(matches!(result, PracticeTestResult::Deviated { .. }));
    }

    #[test]
    fn extreme_deviation_is_erroneous() {
        let out = output(1, 1.0);
        let pred = prediction(TritValue::False, 0.0);
        let result = PracticeTest::compare(&out, &pred);
        // Δ = 0.6*1 + 0.4*1.0 = 1.0 > 0.8 → Erroneous
        assert!(matches!(result, PracticeTestResult::Erroneous { .. }));
    }

    #[test]
    fn deviated_includes_correction_hint() {
        let out = output(1, 0.9);
        let pred = prediction(TritValue::False, 0.9);
        let result = PracticeTest::compare(&out, &pred);
        match result {
            PracticeTestResult::Deviated { correction, .. } => {
                assert_eq!(correction.suggested_value, Some(TritValue::False));
                assert!(correction.suggested_phase.is_none());
            }
            _ => panic!("expected Deviated"),
        }
    }

    #[test]
    fn phase_only_deviation_suggests_phase_correction() {
        let out = output(1, 0.9);
        let pred = prediction(TritValue::True, 0.3);
        let result = PracticeTest::compare(&out, &pred);
        match result {
            PracticeTestResult::Deviated { correction, .. } => {
                assert_eq!(correction.suggested_value, None);
                assert!(correction.suggested_phase.is_some());
            }
            _ => panic!("expected Deviated"),
        }
    }
}
```

- [ ] **Step 2: Verify it compiles (will fail — missing consequence_review, correction, experience_recorder)**

Run: `cargo build --lib 2>&1 | head -20`
Expected: errors about missing submodules

- [ ] **Step 3: Commit**

```bash
git add src/feedback/practice_test.rs
git commit -m "feat: add PracticeTest comparator with weighted deviation formula"
```

---

### Task 4: Create `src/feedback/consequence_review.rs` — deviation classification

**Files:**
- Create: `src/feedback/consequence_review.rs`

- [ ] **Step 1: Write the consequence review module**

```rust
//! Consequence review — classify deviation severity from practice test results.
//!
//! Maps the deviation Δ to a [`CorrectionSeverity`] level:
//! - Δ < 0.2 → Mild (record only)
//! - 0.2 ≤ Δ < 0.5 → Moderate (calibrate modules)
//! - Δ ≥ 0.5 → Severe (calibrate + re-enter pipeline)

use super::{CorrectionSeverity, PracticeTestResult};

/// Threshold for Moderate severity.
const MODERATE_THRESHOLD: f64 = 0.2;

/// Threshold for Severe severity.
const SEVERE_THRESHOLD: f64 = 0.5;

/// Stateless consequence review classifier.
pub struct ConsequenceReview;

impl ConsequenceReview {
    /// Classify a practice test result into a severity level.
    ///
    /// Matched results always return Mild. Deviated results are classified
    /// by their delta. Erroneous results always return Severe.
    pub fn classify(result: &PracticeTestResult) -> CorrectionSeverity {
        match result {
            PracticeTestResult::Matched { .. } => CorrectionSeverity::Mild,
            PracticeTestResult::Deviated { delta, .. } => {
                if *delta >= SEVERE_THRESHOLD {
                    CorrectionSeverity::Severe
                } else if *delta >= MODERATE_THRESHOLD {
                    CorrectionSeverity::Moderate
                } else {
                    CorrectionSeverity::Mild
                }
            }
            PracticeTestResult::Erroneous { severity, .. } => *severity,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::value::TritValue;

    #[test]
    fn matched_is_mild() {
        let result = PracticeTestResult::Matched { confidence: 0.9 };
        assert_eq!(ConsequenceReview::classify(&result), CorrectionSeverity::Mild);
    }

    #[test]
    fn small_deviation_is_mild() {
        let result = PracticeTestResult::Deviated {
            delta: 0.1,
            correction: super::super::CorrectionHint {
                suggested_value: None,
                suggested_phase: None,
                reason: "test".into(),
            },
        };
        assert_eq!(ConsequenceReview::classify(&result), CorrectionSeverity::Mild);
    }

    #[test]
    fn moderate_deviation_is_moderate() {
        let result = PracticeTestResult::Deviated {
            delta: 0.35,
            correction: super::super::CorrectionHint {
                suggested_value: None,
                suggested_phase: None,
                reason: "test".into(),
            },
        };
        assert_eq!(
            ConsequenceReview::classify(&result),
            CorrectionSeverity::Moderate
        );
    }

    #[test]
    fn large_deviation_is_severe() {
        let result = PracticeTestResult::Deviated {
            delta: 0.6,
            correction: super::super::CorrectionHint {
                suggested_value: None,
                suggested_phase: None,
                reason: "test".into(),
            },
        };
        assert_eq!(
            ConsequenceReview::classify(&result),
            CorrectionSeverity::Severe
        );
    }

    #[test]
    fn erroneous_is_severe() {
        let result = PracticeTestResult::Erroneous {
            reason: "test".into(),
            severity: CorrectionSeverity::Severe,
        };
        assert_eq!(
            ConsequenceReview::classify(&result),
            CorrectionSeverity::Severe
        );
    }

    #[test]
    fn boundary_at_moderate_threshold() {
        let result = PracticeTestResult::Deviated {
            delta: 0.2,
            correction: super::super::CorrectionHint {
                suggested_value: None,
                suggested_phase: None,
                reason: "test".into(),
            },
        };
        assert_eq!(
            ConsequenceReview::classify(&result),
            CorrectionSeverity::Moderate
        );
    }

    #[test]
    fn boundary_at_severe_threshold() {
        let result = PracticeTestResult::Deviated {
            delta: 0.5,
            correction: super::super::CorrectionHint {
                suggested_value: None,
                suggested_phase: None,
                reason: "test".into(),
            },
        };
        assert_eq!(
            ConsequenceReview::classify(&result),
            CorrectionSeverity::Severe
        );
    }
}
```

- [ ] **Step 2: Verify it compiles (will fail — missing correction, experience_recorder)**

Run: `cargo build --lib 2>&1 | head -20`
Expected: errors about missing submodules

- [ ] **Step 3: Commit**

```bash
git add src/feedback/consequence_review.rs
git commit -m "feat: add ConsequenceReview severity classifier"
```

---

### Task 5: Create `src/feedback/correction.rs` — CorrectionTrigger

**Files:**
- Create: `src/feedback/correction.rs`

- [ ] **Step 1: Write the correction trigger module**

```rust
//! Correction trigger — threshold-based feedback signal emission.
//!
//! Evaluates practice test results against correction and re-entry
//! thresholds. Builds a [`FeedbackSignal`] when correction is warranted.

use crate::hook::ScenarioType;

use super::{CorrectionSeverity, FeedbackSignal, PracticeTestResult};

/// Stateless correction trigger.
pub struct CorrectionTrigger;

impl CorrectionTrigger {
    /// Evaluate whether a correction signal should be emitted.
    ///
    /// Returns `Some(FeedbackSignal)` if the deviation warrants correction
    /// (severity ≥ Moderate and delta ≥ correction_threshold), or `None`
    /// if the decision is acceptable as-is.
    pub fn evaluate(
        result: &PracticeTestResult,
        severity: CorrectionSeverity,
        source_decision_id: &str,
        correction_threshold: f64,
        reentry_threshold: f64,
    ) -> Option<FeedbackSignal> {
        let delta = match result {
            PracticeTestResult::Matched { .. } => return None,
            PracticeTestResult::Deviated { delta, .. } => *delta,
            PracticeTestResult::Erroneous { .. } => 1.0,
        };

        // Only emit signal if delta exceeds correction threshold
        if delta < correction_threshold {
            return None;
        }

        let recommended_scenario = if delta >= reentry_threshold {
            Some(ScenarioType::ReflexiveAudit)
        } else {
            None
        };

        Some(FeedbackSignal {
            test_result: result.clone(),
            source_decision_id: source_decision_id.to_string(),
            deviation_delta: delta,
            recommended_scenario,
            anchor_violations: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::value::TritValue;

    #[test]
    fn matched_returns_none() {
        let result = PracticeTestResult::Matched { confidence: 0.9 };
        let signal = CorrectionTrigger::evaluate(
            &result,
            CorrectionSeverity::Mild,
            "test",
            0.3,
            0.5,
        );
        assert!(signal.is_none());
    }

    #[test]
    fn mild_deviation_below_threshold_returns_none() {
        let result = PracticeTestResult::Deviated {
            delta: 0.15,
            correction: super::super::CorrectionHint {
                suggested_value: None,
                suggested_phase: None,
                reason: "test".into(),
            },
        };
        let signal = CorrectionTrigger::evaluate(
            &result,
            CorrectionSeverity::Mild,
            "test",
            0.3,
            0.5,
        );
        assert!(signal.is_none());
    }

    #[test]
    fn moderate_deviation_above_threshold_emits_signal() {
        let result = PracticeTestResult::Deviated {
            delta: 0.35,
            correction: super::super::CorrectionHint {
                suggested_value: Some(TritValue::Hold),
                suggested_phase: None,
                reason: "test".into(),
            },
        };
        let signal = CorrectionTrigger::evaluate(
            &result,
            CorrectionSeverity::Moderate,
            "scenario_1",
            0.3,
            0.5,
        );
        assert!(signal.is_some());
        let s = signal.unwrap();
        assert_eq!(s.source_decision_id, "scenario_1");
        assert_float_eq!(s.deviation_delta, 0.35);
        assert!(s.recommended_scenario.is_none()); // below reentry threshold
    }

    #[test]
    fn severe_deviation_recommends_reflexive_audit() {
        let result = PracticeTestResult::Deviated {
            delta: 0.7,
            correction: super::super::CorrectionHint {
                suggested_value: Some(TritValue::Hold),
                suggested_phase: Some(0.5),
                reason: "test".into(),
            },
        };
        let signal = CorrectionTrigger::evaluate(
            &result,
            CorrectionSeverity::Severe,
            "test",
            0.3,
            0.5,
        );
        assert!(signal.is_some());
        let s = signal.unwrap();
        assert_eq!(s.recommended_scenario, Some(ScenarioType::ReflexiveAudit));
    }

    #[test]
    fn erroneous_always_emits_signal() {
        let result = PracticeTestResult::Erroneous {
            reason: "critical failure".into(),
            severity: CorrectionSeverity::Severe,
        };
        let signal = CorrectionTrigger::evaluate(
            &result,
            CorrectionSeverity::Severe,
            "test",
            0.3,
            0.5,
        );
        assert!(signal.is_some());
        assert_float_eq!(signal.unwrap().deviation_delta, 1.0);
    }

    #[test]
    fn custom_thresholds_are_respected() {
        let result = PracticeTestResult::Deviated {
            delta: 0.4,
            correction: super::super::CorrectionHint {
                suggested_value: None,
                suggested_phase: None,
                reason: "test".into(),
            },
        };
        // With correction_threshold=0.5, delta=0.4 should NOT trigger
        let signal = CorrectionTrigger::evaluate(
            &result,
            CorrectionSeverity::Moderate,
            "test",
            0.5,
            0.7,
        );
        assert!(signal.is_none());
    }
}
```

- [ ] **Step 2: Verify it compiles (will fail — missing experience_recorder)**

Run: `cargo build --lib 2>&1 | head -20`
Expected: errors about missing `experience_recorder`

- [ ] **Step 3: Commit**

```bash
git add src/feedback/correction.rs
git commit -m "feat: add CorrectionTrigger with threshold-based signal emission"
```

---

### Task 6: Create `src/feedback/experience_recorder.rs` — pattern storage

**Files:**
- Create: `src/feedback/experience_recorder.rs`

- [ ] **Step 1: Write the experience recorder module**

```rust
//! Experience recorder — ring buffer of feedback outcomes.
//!
//! Stores a fixed-size window of [`ExperienceRecord`] entries for
//! pattern analysis. Oldest entries are silently evicted when the
//! buffer is full.

use std::collections::VecDeque;

/// A single recorded experience from the feedback loop.
#[derive(Debug, Clone)]
pub struct ExperienceRecord {
    /// ID of the scenario that produced this experience.
    pub scenario_id: String,
    /// Final value code of the decision (-1, 0, 1).
    pub result_value: i8,
    /// Final phase of the decision.
    pub result_phase: f64,
    /// Whether the decision matched the proxy prediction.
    pub matched: bool,
    /// Deviation delta (0.0 for matched, >0.0 for deviated).
    pub deviation_delta: f64,
}

/// Fixed-size ring buffer of experience records.
///
/// # Window eviction
///
/// When the buffer exceeds `window_size`, the oldest entry is silently
/// dropped. This prevents unbounded memory growth.
#[derive(Debug, Clone)]
pub struct ExperienceRecorder {
    entries: VecDeque<ExperienceRecord>,
    window_size: usize,
}

impl ExperienceRecorder {
    /// Create a new experience recorder with the given window size.
    pub fn new(window_size: usize) -> Self {
        ExperienceRecorder {
            entries: VecDeque::with_capacity(window_size),
            window_size,
        }
    }

    /// Record a new experience. Evicts the oldest if the window is full.
    pub fn record(&mut self, entry: ExperienceRecord) {
        if self.entries.len() >= self.window_size {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    /// Number of entries currently stored.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if no entries are stored.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The window size (maximum entries).
    pub fn window_size(&self) -> usize {
        self.window_size
    }

    /// Fraction of entries that were matched. Returns 0.0 if empty.
    pub fn match_rate(&self) -> f64 {
        if self.entries.is_empty() {
            return 0.0;
        }
        self.entries.iter().filter(|e| e.matched).count() as f64 / self.entries.len() as f64
    }

    /// Average deviation delta across all entries. Returns 0.0 if empty.
    pub fn average_delta(&self) -> f64 {
        if self.entries.is_empty() {
            return 0.0;
        }
        self.entries.iter().map(|e| e.deviation_delta).sum::<f64>() / self.entries.len() as f64
    }

    /// Iterate over entries from oldest to newest.
    pub fn iter(&self) -> impl Iterator<Item = &ExperienceRecord> {
        self.entries.iter()
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(matched: bool, delta: f64) -> ExperienceRecord {
        ExperienceRecord {
            scenario_id: "test".into(),
            result_value: 1,
            result_phase: 0.9,
            matched,
            deviation_delta: delta,
        }
    }

    #[test]
    fn new_recorder_is_empty() {
        let er = ExperienceRecorder::new(32);
        assert!(er.is_empty());
        assert_eq!(er.len(), 0);
    }

    #[test]
    fn record_adds_entry() {
        let mut er = ExperienceRecorder::new(32);
        er.record(record(true, 0.0));
        assert_eq!(er.len(), 1);
    }

    #[test]
    fn window_eviction_drops_oldest() {
        let mut er = ExperienceRecorder::new(3);
        er.record(record(true, 0.0));
        er.record(record(false, 0.3));
        er.record(record(false, 0.5));
        er.record(record(true, 0.1)); // first entry should be evicted
        assert_eq!(er.len(), 3);
        // Oldest should now be the second entry (delta=0.3)
        let first = er.iter().next().unwrap();
        assert_float_eq!(first.deviation_delta, 0.3);
    }

    #[test]
    fn match_rate_all_matched() {
        let mut er = ExperienceRecorder::new(32);
        er.record(record(true, 0.0));
        er.record(record(true, 0.0));
        assert_float_eq!(er.match_rate(), 1.0);
    }

    #[test]
    fn match_rate_mixed() {
        let mut er = ExperienceRecorder::new(32);
        er.record(record(true, 0.0));
        er.record(record(false, 0.4));
        assert_float_eq!(er.match_rate(), 0.5);
    }

    #[test]
    fn match_rate_empty_is_zero() {
        let er = ExperienceRecorder::new(32);
        assert_float_eq!(er.match_rate(), 0.0);
    }

    #[test]
    fn average_delta() {
        let mut er = ExperienceRecorder::new(32);
        er.record(record(true, 0.0));
        er.record(record(false, 0.4));
        assert_float_eq!(er.average_delta(), 0.2);
    }

    #[test]
    fn average_delta_empty_is_zero() {
        let er = ExperienceRecorder::new(32);
        assert_float_eq!(er.average_delta(), 0.0);
    }

    #[test]
    fn clear_empties() {
        let mut er = ExperienceRecorder::new(32);
        er.record(record(true, 0.0));
        er.clear();
        assert!(er.is_empty());
    }

    #[test]
    fn window_size_is_preserved() {
        let er = ExperienceRecorder::new(16);
        assert_eq!(er.window_size(), 16);
    }
}
```

- [ ] **Step 2: Build to verify compilation**

Run: `cargo build --lib 2>&1`
Expected: Should compile cleanly now that all submodules exist. May have warnings about unused imports in mod.rs.

- [ ] **Step 3: Run the new unit tests**

Run: `cargo test --lib feedback 2>&1`
Expected: All feedback tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/feedback/experience_recorder.rs
git commit -m "feat: add ExperienceRecorder ring buffer for feedback patterns"
```

---

### Task 7: Update `src/adapters/mod.rs` — replace placeholder FeedbackSignal

**Files:**
- Modify: `src/adapters/mod.rs`

- [ ] **Step 1: Replace the placeholder FeedbackSignal with a re-export**

Replace lines 166-185 in `src/adapters/mod.rs` (the placeholder `FeedbackSignal` struct) with a re-export from `crate::feedback`:

```rust
// ── Feedback signal (from Layer 5) ─────────────────────────────────

/// Feedback signal from Layer 5's practice testing.
///
/// Re-exported from [`crate::feedback::FeedbackSignal`]. This replaces
/// the v0.3.0 placeholder with the real Layer 5 type.
pub use crate::feedback::FeedbackSignal;
```

Also update the test at the bottom of the file that constructs a `FeedbackSignal` placeholder. Replace the `default_calibrate_returns_zero` test (lines 275-283) with:

```rust
    #[test]
    fn default_calibrate_returns_zero() {
        let mut m = TestModule { mounted: false };
        let fb = FeedbackSignal {
            test_result: crate::feedback::PracticeTestResult::Matched { confidence: 0.9 },
            source_decision_id: "test".into(),
            deviation_delta: 0.0,
            recommended_scenario: None,
            anchor_violations: vec![],
        };
        assert_float_eq!(m.calibrate(&fb), 0.0);
    }
```

- [ ] **Step 2: Build and run adapter tests**

Run: `cargo test --lib adapters 2>&1`
Expected: All adapter tests pass (including the updated `default_calibrate_returns_zero`).

- [ ] **Step 3: Commit**

```bash
git add src/adapters/mod.rs
git commit -m "refactor: replace placeholder FeedbackSignal with Layer 5 re-export"
```

---

### Task 8: Register `pub mod feedback` in `src/lib.rs`

**Files:**
- Modify: `src/lib.rs`

- [ ] **Step 1: Add the feedback module declaration**

After line 89 (`pub mod core;`), add:

```rust
pub mod feedback;
```

And add `FeedbackLoop, FeedbackSignal, PracticeTestResult, CorrectionSeverity, CorrectionHint, ConsequencePrediction, ProxyEnvironment, StaticRuleModel` to the `pub use` block. Add after the existing `pub use core::` block (line 132):

```rust
pub use feedback::{
    ConsequencePrediction, CorrectionHint, CorrectionSeverity, FeedbackLoop, FeedbackSignal,
    PracticeTestResult,
    proxy_env::{ProxyEnvironment, StaticRuleModel},
};
```

- [ ] **Step 2: Build to verify**

Run: `cargo build --lib 2>&1`
Expected: Clean compile.

- [ ] **Step 3: Commit**

```bash
git add src/lib.rs
git commit -m "feat: register feedback module and re-export public types"
```

---

### Task 9: Wire `stage_feedback_loop()` into `SandboxPipeline`

**Files:**
- Modify: `src/sandbox/pipeline.rs`

- [ ] **Step 1: Add the `feedback` field and `with_feedback()` builder**

Add the import at the top of `pipeline.rs` (after line 17, the `use crate::core::value::TritValue;` import):

```rust
use crate::feedback::FeedbackLoop;
```

Add the field to the `SandboxPipeline` struct (after line 50, `pub(crate) calibration_log: CalibrationLog,`):

```rust
    /// Feedback loop for practice testing (Layer 5).
    pub(crate) feedback: Option<FeedbackLoop>,
```

Add initialization in `Self::new()` (after line 97, `calibration_log: CalibrationLog::default(),`):

```rust
            feedback: None,
```

Add the builder method after `with_calibration_log()` (after line 181):

```rust
    /// Attach a feedback loop for practice testing (Layer 5).
    pub fn with_feedback(mut self, feedback: FeedbackLoop) -> Self {
        self.feedback = Some(feedback);
        self
    }
```

- [ ] **Step 2: Add `stage_feedback_loop()` method**

Add after the `stage_calibrate()` method (after line 535):

```rust
    /// Stage 14: feedback loop — practice test the decision against a proxy
    /// environment (Layer 5).
    ///
    /// Runs after calibration. If a feedback loop is configured, it predicts
    /// the expected consequence, compares against the actual decision, and
    /// emits a FeedbackSignal if correction is needed.
    fn stage_feedback_loop(
        &mut self,
        scenario: &ScenarioInput,
        output: &SandboxOutput,
        diagnostics: &mut SandboxDiagnostics,
    ) {
        let stage_start = Instant::now();
        if let Some(ref mut feedback) = self.feedback {
            let signal = feedback.run_cycle(output);
            if let Some(ref sig) = signal {
                info!(
                    deviation_delta = sig.deviation_delta,
                    recommended_scenario = ?sig.recommended_scenario,
                    "feedback loop: correction triggered"
                );
                diagnostics.record_feedback_signal(sig.clone());

                // Calibrate self-knowledge with the feedback signal
                if let Some(ref mut knowledge) = self.self_knowledge {
                    knowledge.calibrate(sig);
                }
            } else {
                debug!("feedback loop: decision matched proxy prediction");
            }
        }
        diagnostics.record_stage("feedback_loop", stage_start);
    }
```

- [ ] **Step 3: Add `record_feedback_signal` to `SandboxDiagnostics`**

Read `src/sandbox/diagnostic.rs` and add the field + method:

Add field to `SandboxDiagnostics` struct:

```rust
    /// Optional feedback signal from Layer 5.
    pub feedback_signal: Option<crate::feedback::FeedbackSignal>,
```

Add method:

```rust
    /// Record a feedback signal from Layer 5.
    pub fn record_feedback_signal(&mut self, signal: crate::feedback::FeedbackSignal) {
        self.feedback_signal = Some(signal);
    }
```

- [ ] **Step 4: Call `stage_feedback_loop` from `run_with_diagnostics`**

In `run_with_diagnostics()`, after the `stage_calibrate` call (after line 264), add:

```rust
        // Stage 14: feedback loop (Layer 5)
        self.stage_feedback_loop(scenario, &output, &mut diagnostics);
```

- [ ] **Step 5: Build and run all tests**

Run: `cargo build --lib 2>&1`
Expected: Clean compile.

Run: `cargo test --lib 2>&1 | tail -5`
Expected: All 427+ lib tests pass.

Run: `cargo test --test pipeline_test 2>&1 | tail -5`
Expected: All 38 pipeline tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/sandbox/pipeline.rs src/sandbox/diagnostic.rs
git commit -m "feat: wire stage_feedback_loop into SandboxPipeline (Layer 5)"
```

---

### Task 10: Add pipeline integration test for feedback loop

**Files:**
- Modify: `tests/pipeline_test.rs`

- [ ] **Step 1: Add feedback loop integration tests**

Add at the end of `tests/pipeline_test.rs`:

```rust
// ── Feedback loop (Layer 5) ────────────────────────────────────────

use trit_core::feedback::proxy_env::StaticRuleModel;
use trit_core::feedback::FeedbackLoop;

#[test]
fn pipeline_with_feedback_runs_cycle() {
    let s = scenario("General", vec![signal("Science", 1, 0.9)]);
    let proxy = Box::new(StaticRuleModel::new());
    let feedback = FeedbackLoop::new(proxy);
    let mut pipeline = SandboxPipeline::default().with_feedback(feedback);
    let (out, diag) = pipeline.run_with_diagnostics(&s).unwrap();
    assert_eq!(out.final_value_code, 1);
    // Feedback stage should have been recorded
    assert!(diag.stage_timings_ns.contains_key("feedback_loop"));
}

#[test]
fn pipeline_without_feedback_still_works() {
    let s = scenario("General", vec![signal("Science", 1, 0.9)]);
    let mut pipeline = SandboxPipeline::default();
    let (out, diag) = pipeline.run_with_diagnostics(&s).unwrap();
    assert_eq!(out.final_value_code, 1);
    // Without feedback configured, stage should still be recorded (as no-op)
    assert!(diag.stage_timings_ns.contains_key("feedback_loop"));
}

#[test]
fn pipeline_feedback_medical_ethics_preserves_individual() {
    let s = scenario(
        "MedicalEthics",
        vec![signal("Science", 1, 0.8), signal("Individual", -1, 0.2)],
    );
    let proxy = Box::new(StaticRuleModel::new());
    let feedback = FeedbackLoop::new(proxy);
    let mut pipeline = SandboxPipeline::default().with_feedback(feedback);
    let (out, diag) = pipeline.run_with_diagnostics(&s).unwrap();
    assert_eq!(out.final_value_code, -1);
    assert!(diag.stage_timings_ns.contains_key("feedback_loop"));
}

#[test]
fn pipeline_feedback_disabled_does_nothing() {
    let s = scenario("General", vec![signal("Science", 1, 0.9)]);
    let proxy = Box::new(StaticRuleModel::new());
    let feedback = FeedbackLoop::new(proxy).with_enabled(false);
    let mut pipeline = SandboxPipeline::default().with_feedback(feedback);
    let (out, _diag) = pipeline.run_with_diagnostics(&s).unwrap();
    assert_eq!(out.final_value_code, 1);
}

#[test]
fn pipeline_feedback_value_judgment_holds() {
    let s = scenario(
        "ValueJudgment",
        vec![signal("Individual", -1, 0.3), signal("Consensus", 1, 0.7)],
    );
    let proxy = Box::new(StaticRuleModel::new());
    let feedback = FeedbackLoop::new(proxy);
    let mut pipeline = SandboxPipeline::default().with_feedback(feedback);
    let (out, _diag) = pipeline.run_with_diagnostics(&s).unwrap();
    assert_eq!(out.final_value_code, 0);
}
```

- [ ] **Step 2: Run the new integration tests**

Run: `cargo test --test pipeline_test -- feedback 2>&1`
Expected: All 5 new feedback tests pass.

- [ ] **Step 3: Run the full test suite**

Run: `cargo test --lib --test pipeline_test 2>&1 | tail -5`
Expected: All 427 lib + 43 pipeline (38 existing + 5 new) tests pass.

- [ ] **Step 4: Commit**

```bash
git add tests/pipeline_test.rs
git commit -m "test: add Layer 5 feedback loop integration tests"
```

---

### Task 11: Final verification — fmt, clippy, full test suite

**Files:** None (verification only)

- [ ] **Step 1: Run cargo fmt**

Run: `cargo fmt -- --check`
Expected: No formatting issues.

- [ ] **Step 2: Run cargo clippy**

Run: `cargo clippy --lib --all-features -- -D warnings 2>&1`
Expected: No warnings.

- [ ] **Step 3: Run full test suite**

Run: `cargo test --lib --test pipeline_test 2>&1 | tail -5`
Expected: All tests pass.

- [ ] **Step 4: Update CHANGELOG**

Add to `CHANGELOG.md` under `[0.4.0] - Unreleased`:

```markdown
- **Layer 5 Feedback Loop** — close the 5-layer cognitive architecture:
  - `ProxyEnvironment` trait + `StaticRuleModel` MVP for consequence prediction
  - `PracticeTest` comparator with weighted deviation formula (Δ = 0.6·δ_v + 0.4·δ_p)
  - `ConsequenceReview` severity classifier (Mild/Moderate/Severe)
  - `CorrectionTrigger` with threshold-based feedback signal emission
  - `ExperienceRecorder` ring buffer for pattern storage
  - `FeedbackLoop` facade wired into `SandboxPipeline` as opt-in `stage_feedback_loop()`
  - Replaced placeholder `FeedbackSignal` with real Layer 5 type
```

- [ ] **Step 5: Final commit**

```bash
git add CHANGELOG.md
git commit -m "docs: update CHANGELOG for Layer 5 Feedback Loop"
```
