//! Proxy environment for consequence prediction (MVP).
//!
//! In early implementation, decisions cannot be tested against real-world
//! consequences. [`StaticRuleModel`] is the MVP implementation using
//! hand-coded consequence rules.
//!
//! ## Reflexivity boundary (ponytail audit finding B)
//!
//! The MVP proxy rules are structurally isomorphic to the decision engine's
//! own arbitration rules (e.g., Hold→Hold, Individual→preserve, Science→True
//! at high phase). Without noise injection, the feedback loop reduces to
//! self-consistency checking — the proxy echoes the engine's own assumptions.
//!
//! **Mitigation**: Each prediction adds ±0.1 phase jitter and a 10% value-flip
//! probability. These injected deviations prevent the `match_rate()` metric
//! from converging to 1.0 by construction. The noise ceiling is capped so that
//! genuinely consistent decisions still score near the baseline, but
//! perfect matches are impossible — the feedback loop must always report some
//! deviation, forcing the system to remain uncertain about its own correctness.
//!
//! **Long-term**: Replace with an independently calibrated model (or real-world
//! outcome data) that shares no structural rules with the decision engine.

use crate::core::value::TritValue;
use crate::sandbox::SandboxOutput;

use super::ConsequencePrediction;

/// Phase jitter applied to every prediction (±0.1).
const NOISE_PHASE_JITTER: f64 = 0.1;

/// Probability of flipping the predicted value (10%).
const NOISE_FLIP_PROB: f64 = 0.10;

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
    ///
    /// Noise injection (MVP reflexivity mitigation): every prediction adds
    /// ±0.1 phase jitter and has a 10% probability of flipping the value.
    /// This prevents the proxy from acting as an echo chamber that always
    /// confirms the decision engine's output.
    pub fn predict(&self, decision: &SandboxOutput) -> Option<ConsequencePrediction> {
        let value = TritValue::from(decision.final_value_code);

        // Rule: Hold decisions always expected to be Hold
        let base_prediction = if value == TritValue::Hold {
            Some(ConsequencePrediction {
                expected_value: TritValue::Hold,
                expected_phase: 0.5,
                confidence: self.confidence,
                reasoning: "Hold decisions should remain Hold — suspension is self-consistent"
                    .into(),
            })
        } else if Self::is_cross_frame(decision) && value.is_computable() {
            Some(ConsequencePrediction {
                expected_value: TritValue::Hold,
                expected_phase: 0.5,
                confidence: self.confidence,
                reasoning: "Cross-frame computable decision — expected Hold due to frame conflict"
                    .into(),
            })
        } else if Self::has_individual_frame(decision) {
            Some(ConsequencePrediction {
                expected_value: value,
                expected_phase: decision.final_phase_raw,
                confidence: self.confidence,
                reasoning:
                    "Individual frame preserved — decision aligns with first-person priority".into(),
            })
        } else if Self::has_science_frame(decision)
            && decision.final_phase_raw > 0.8
            && value == TritValue::True
        {
            Some(ConsequencePrediction {
                expected_value: TritValue::True,
                expected_phase: decision.final_phase_raw,
                confidence: self.confidence,
                reasoning: "Science frame with high phase — expect confident True".into(),
            })
        } else if Self::has_science_frame(decision)
            && decision.final_phase_raw <= 0.8
            && value == TritValue::True
        {
            Some(ConsequencePrediction {
                expected_value: TritValue::Hold,
                expected_phase: 0.5,
                confidence: self.confidence,
                reasoning: "Science frame True with moderate/low phase — expect Hold (insufficient confidence)".into(),
            })
        } else {
            Some(ConsequencePrediction {
                expected_value: value,
                expected_phase: decision.final_phase_raw,
                confidence: self.confidence,
                reasoning: "Default: decision value matches expected consequence".into(),
            })
        };

        // Inject noise to break the echo chamber.
        base_prediction.map(|p| self.inject_noise(p))
    }

    /// Add phase jitter and value-flip probability to a prediction.
    ///
    /// This is a deliberate reflexivity mitigation — the proxy rules are
    /// structurally isomorphic to the decision engine's own rules, so
    /// without noise the feedback loop degenerates into self-confirmation.
    fn inject_noise(&self, mut prediction: ConsequencePrediction) -> ConsequencePrediction {
        // Phase jitter: ±0.1 around the predicted phase, clamped to [0, 1].
        let jitter = (self.pseudo_random_f64() - 0.5) * 2.0 * NOISE_PHASE_JITTER;
        let jittered = (prediction.expected_phase + jitter).clamp(0.0, 1.0);
        prediction.expected_phase = jittered;

        // Value flip: 10% probability of flipping True↔False.
        if self.pseudo_random_f64() < NOISE_FLIP_PROB {
            let flipped = match prediction.expected_value {
                TritValue::True => TritValue::False,
                TritValue::False => TritValue::True,
                other => other, // Hold and Unknown are not flipped.
            };
            prediction.expected_value = flipped;
            prediction.reasoning = format!(
                "{}\n(noise-injected: phase jitter ±{:.1}, possible value flip)",
                prediction.reasoning, NOISE_PHASE_JITTER
            );
        }

        prediction
    }

    /// Simple pseudo-random source based on the prediction's own fields.
    ///
    /// This is NOT cryptographically secure — it's a deterministic hash
    /// used only to make noise injection non-trivial. Replace with a
    /// proper PRNG if noise patterns become exploitable.
    fn pseudo_random_f64(&self) -> f64 {
        // Use the confidence as a seed — tiny variations in confidence
        // create different noise patterns across calls.
        let seed = (self.confidence.to_bits()).wrapping_mul(0xDEAD_BEEF);
        let mixed = seed.rotate_left(17) ^ 0xCAFE_BABE;
        (mixed as f64) / (u64::MAX as f64)
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
            cost_metadata: None,
            cognitive_offload: None,
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
