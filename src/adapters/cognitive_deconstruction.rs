//! Cognitive deconstruction: concept reduction, disenchantment, explanation impulse detection.
//!
//! This module detects when the system is about to "fill in" an answer without
//! sufficient evidence — the explanation impulse. When input ambiguity is high
//! but output determinacy is high, this module fires an alert.
//!
//! ## Mathematical Foundation
//!
//! Let H(I) be the entropy of the input signal distribution, and D(O) the
//! determinacy of the output (how close Phase is to 0.0 or 1.0). The
//! explanation impulse fires when:
//!
//!   H(I) > tau_ambiguity  AND  D(O) > tau_determinacy

use crate::adapters::{CognitiveModule, ModuleId, ModuleInput, ModuleOutput, ModuleState};
use crate::core::word::TritWord;
use crate::feedback::FeedbackSignal;
use crate::hook::{HookContext, UnmountReason};

/// Configuration for explanation impulse detection.
#[derive(Debug, Clone)]
pub struct DeconstructionConfig {
    /// Entropy threshold above which input is considered ambiguous.
    pub ambiguity_threshold: f64,
    /// Determinacy threshold above which output is considered too certain.
    pub determinacy_threshold: f64,
}

impl Default for DeconstructionConfig {
    fn default() -> Self {
        DeconstructionConfig {
            ambiguity_threshold: 0.5,
            determinacy_threshold: 0.8,
        }
    }
}

pub struct CognitiveDeconstruction {
    id: ModuleId,
    state: ModuleState,
    config: DeconstructionConfig,
}

impl CognitiveDeconstruction {
    pub fn new() -> Self {
        CognitiveDeconstruction {
            id: ModuleId::new("cognitive_deconstruction"),
            state: ModuleState::Unmounted,
            config: DeconstructionConfig::default(),
        }
    }

    /// Compute input entropy from signal distribution.
    ///
    /// Uses Phase variance as a proxy for entropy: higher variance = more ambiguity.
    fn input_entropy(&self, signals: &[TritWord]) -> f64 {
        if signals.is_empty() {
            return 1.0; // No signals = maximum ambiguity
        }
        let n = signals.len() as f64;
        let mean: f64 = signals.iter().map(|w| w.phase().inner()).sum::<f64>() / n;
        let variance: f64 = signals
            .iter()
            .map(|w| {
                let d = w.phase().inner() - mean;
                d * d
            })
            .sum::<f64>()
            / n;
        // Normalize: max variance for [0,1] bounded values is 0.25
        (variance / 0.25).min(1.0)
    }

    /// Compute output determinacy from the most recent result.
    ///
    /// Determinacy is how close the Phase is to 0.0 or 1.0 (i.e., how "certain").
    fn output_determinacy(&self, results: &[TritWord]) -> f64 {
        if results.is_empty() {
            return 0.0;
        }
        let last = results.last().unwrap();
        let p = last.phase().inner();
        // Distance from neutral (0.5): 0.0 at neutral, 1.0 at extremes
        ((p - 0.5).abs() * 2.0).min(1.0)
    }

    /// Detect whether an explanation impulse is occurring.
    pub fn detect_explanation_impulse(&self, signals: &[TritWord], results: &[TritWord]) -> bool {
        let entropy = self.input_entropy(signals);
        let determinacy = self.output_determinacy(results);
        entropy > self.config.ambiguity_threshold && determinacy > self.config.determinacy_threshold
    }
}

impl Default for CognitiveDeconstruction {
    fn default() -> Self {
        Self::new()
    }
}

impl CognitiveModule for CognitiveDeconstruction {
    fn id(&self) -> ModuleId {
        self.id.clone()
    }

    fn name(&self) -> &'static str {
        "cognitive_deconstruction"
    }

    fn process_signals(&mut self, input: &ModuleInput, _ctx: &HookContext) -> ModuleOutput {
        let impulse = self.detect_explanation_impulse(&input.signals, &[]);
        ModuleOutput {
            results: vec![],
            confidence: if impulse { 0.3 } else { 0.7 },
            explanation_impulse_detected: impulse,
            summary: if impulse {
                "Explanation impulse detected: input ambiguous but output would be too certain"
                    .into()
            } else {
                "No explanation impulse detected".into()
            },
            warnings: if impulse {
                vec!["Consider choosing Hold instead of forcing an answer".into()]
            } else {
                vec![]
            },
        }
    }

    fn on_mount(&mut self) {
        self.state = ModuleState::Active;
    }

    fn on_unmount(&mut self, _reason: UnmountReason) {
        self.state = ModuleState::Unmounted;
    }

    fn state(&self) -> ModuleState {
        self.state
    }

    fn calibrate(&mut self, _feedback: &FeedbackSignal) -> f64 {
        0.7
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::frame::Frame;
    use crate::core::phase::Phase;
    use crate::core::value::TritValue;

    fn word(frame: Frame, value: TritValue, phase: f64) -> TritWord {
        TritWord::new(value, Phase::new(phase).unwrap(), frame)
    }

    #[test]
    fn no_impulse_when_input_is_clear() {
        let module = CognitiveDeconstruction::new();
        let signals = vec![word(Frame::Science, TritValue::True, 0.9)];
        let entropy = module.input_entropy(&signals);
        assert!(
            entropy < 0.5,
            "clear input should have low entropy, got {entropy}"
        );
    }

    #[test]
    fn impulse_detected_when_ambiguous_input_but_certain_output() {
        let module = CognitiveDeconstruction::new();
        // Highly ambiguous input (wide phase spread across extremes)
        let signals = vec![
            word(Frame::Science, TritValue::True, 1.0),
            word(Frame::Individual, TritValue::False, 0.0),
            word(Frame::Consensus, TritValue::True, 0.5),
        ];
        // But output is highly certain
        let results = vec![word(Frame::Meta, TritValue::True, 0.95)];
        let entropy = module.input_entropy(&signals);
        assert!(
            entropy > 0.5,
            "ambiguous input should have high entropy, got {entropy}"
        );
        assert!(module.detect_explanation_impulse(&signals, &results));
    }

    #[test]
    fn no_impulse_when_output_is_uncertain() {
        let module = CognitiveDeconstruction::new();
        let signals = vec![
            word(Frame::Science, TritValue::True, 0.9),
            word(Frame::Individual, TritValue::False, 0.1),
        ];
        let results = vec![word(Frame::Meta, TritValue::Hold, 0.5)];
        assert!(!module.detect_explanation_impulse(&signals, &results));
    }

    #[test]
    fn empty_signals_max_entropy() {
        let module = CognitiveDeconstruction::new();
        assert_eq!(module.input_entropy(&[]), 1.0);
    }
}
