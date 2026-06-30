//! Proxy environment for consequence prediction (MVP).
//!
//! In early implementation, decisions cannot be tested against real-world
//! consequences. [`StaticRuleModel`] is the MVP implementation using
//! hand-coded consequence rules.

use crate::core::value::TritValue;
use crate::sandbox::SandboxOutput;

use super::ConsequencePrediction;

// ── StaticRuleModel ────────────────────────────────────────────────

/// MVP proxy environment using hand-coded consequence rules.
///
/// Rules are domain-specific and based on the decision's value, frame,
/// and phase. Confidence is 0.6 — explicitly uncertain, since this is
/// a static model, not a real environment.
pub struct StaticRuleModel {
    confidence: f64,
}

impl Default for StaticRuleModel {
    fn default() -> Self {
        Self { confidence: 0.6 }
    }
}

impl StaticRuleModel {
    /// Create a new StaticRuleModel with default confidence (0.6).
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a StaticRuleModel with a custom confidence.
    pub fn with_confidence(confidence: f64) -> Self {
        StaticRuleModel {
            confidence: confidence.clamp(0.0, 1.0),
        }
    }

    /// Predict the expected consequence of a decision.
    /// Returns None if the decision falls outside the proxy's modeling range.
    pub fn predict(&self, decision: &SandboxOutput) -> Option<ConsequencePrediction> {
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
                reasoning:
                    "Individual frame preserved — decision aligns with first-person priority".into(),
            });
        }

        // Rule: Science frame True with high phase → expect True
        if Self::has_science_frame(decision)
            && decision.final_phase_raw > 0.8
            && value == TritValue::True
        {
            return Some(ConsequencePrediction {
                expected_value: TritValue::True,
                expected_phase: decision.final_phase_raw,
                confidence: self.confidence,
                reasoning: "Science frame with high phase — expect confident True".into(),
            });
        }

        // Rule: Science frame with moderate/low phase True → expect Hold
        if Self::has_science_frame(decision)
            && decision.final_phase_raw <= 0.8
            && value == TritValue::True
        {
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

    /// The confidence of this proxy's predictions, in [0.0, 1.0].
    pub fn confidence(&self) -> f64 {
        self.confidence
    }

    /// Human-readable name of the proxy.
    pub fn name(&self) -> &'static str {
        "StaticRuleModel"
    }

    /// Determine if the decision involves cross-frame signals.
    fn is_cross_frame(output: &SandboxOutput) -> bool {
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
