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
        self.experience
            .record(experience_recorder::ExperienceRecord {
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
    use proxy_env::StaticRuleModel;

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
    fn feedback_loop_disabled_returns_none() {
        let proxy = Box::new(StaticRuleModel::new());
        let mut fl = FeedbackLoop::new(proxy).with_enabled(false);
        let out = output(1, "Science", 0.9, "Commit(Science)");
        assert!(fl.run_cycle(&out).is_none());
    }

    #[test]
    fn feedback_loop_enabled_runs_cycle() {
        let proxy = Box::new(StaticRuleModel::new());
        let mut fl = FeedbackLoop::new(proxy);
        let out = output(1, "Science", 0.9, "Commit(Science)");
        let result = fl.run_cycle(&out);
        // Single-frame Science True with high phase should match StaticRuleModel
        // May be Some or None depending on rule match — just verify no panic
        assert!(fl.experience_count() > 0);
        let _ = result;
    }

    #[test]
    fn feedback_loop_experience_count_increments() {
        let proxy = Box::new(StaticRuleModel::new());
        let mut fl = FeedbackLoop::new(proxy);
        let out = output(1, "Science", 0.9, "Commit(Science)");
        fl.run_cycle(&out);
        fl.run_cycle(&out);
        assert_eq!(fl.experience_count(), 2);
    }

    #[test]
    fn feedback_loop_match_rate() {
        let proxy = Box::new(StaticRuleModel::new());
        let mut fl = FeedbackLoop::new(proxy);
        let out = output(1, "Science", 0.9, "Commit(Science)");
        fl.run_cycle(&out);
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
