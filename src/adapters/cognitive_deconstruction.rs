//! Cognitive deconstruction adapter — explanation impulse detection.
//!
//! Detects when the system is about to produce a confident answer without
//! sufficient evidence. This is the algorithmic form of "disenchantment"
//! — catching the moment when the mind fills in a gap with a story.
//!
//! The core heuristic compares input entropy H(I) against output determinacy
//! D(O). When the input is highly ambiguous but the output is highly certain,
//! an explanation impulse is detected and the module recommends Hold.

use crate::adapters::{CognitiveModule, ModuleInput, ModuleOutput};
use crate::core::TritValue;
use crate::hook::module_registry::{ModuleId, ModuleState};
use crate::hook::HookContext;
use crate::meta::{ConflictType, MetaInterrupt};

/// Threshold for input entropy H(I) — above this, the input is "ambiguous."
const AMBIGUITY_THRESHOLD: f64 = 0.5;

/// Threshold for output determinacy D(O) — above this, the output is "certain."
const DETERMINACY_THRESHOLD: f64 = 0.7;

/// Cognitive module for explanation impulse detection.
pub struct CognitiveDeconstruction {
    state: ModuleState,
    /// Running count of explanation impulses detected.
    impulse_count: usize,
}

impl CognitiveDeconstruction {
    /// Create a new cognitive deconstruction module.
    pub fn new() -> Self {
        CognitiveDeconstruction {
            state: ModuleState::Idle,
            impulse_count: 0,
        }
    }

    /// Number of explanation impulses detected so far.
    pub fn impulse_count(&self) -> usize {
        self.impulse_count
    }
}

impl Default for CognitiveDeconstruction {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute entropy H(I) of the input signal distribution.
///
/// Uses the proportions of True, False, and Hold values as a simple
/// 3-class distribution. Range: [0.0, ln(3) ≈ 1.099].
fn input_entropy(signals: &[crate::core::TritWord]) -> f64 {
    if signals.is_empty() {
        return 0.0;
    }
    let n = signals.len() as f64;
    let tru = signals
        .iter()
        .filter(|s| s.value() == TritValue::True)
        .count() as f64;
    let fals = signals
        .iter()
        .filter(|s| s.value() == TritValue::False)
        .count() as f64;
    let hold = signals
        .iter()
        .filter(|s| s.value() == TritValue::Hold)
        .count() as f64;
    let unknown = signals
        .iter()
        .filter(|s| s.value() == TritValue::Unknown)
        .count() as f64;

    let mut entropy = 0.0;
    for &count in &[tru, fals, hold, unknown] {
        let p = count / n;
        if p > 0.0 {
            entropy -= p * p.ln();
        }
    }
    entropy
}

/// Compute output determinacy D(O) from signal phases.
///
/// Determinacy is high when most phases are near 0.0 or 1.0 (extreme).
/// Range: [0.0, 1.0].
fn output_determinacy(signals: &[crate::core::TritWord]) -> f64 {
    if signals.is_empty() {
        return 0.0;
    }
    let mean_deviation: f64 = signals
        .iter()
        .map(|s| (s.phase().inner() - 0.5).abs() * 2.0) // 0.0 = neutral, 1.0 = extreme
        .sum::<f64>()
        / signals.len() as f64;
    mean_deviation
}

impl CognitiveModule for CognitiveDeconstruction {
    fn id(&self) -> ModuleId {
        ModuleId::CognitiveDeconstruction
    }

    fn name(&self) -> &'static str {
        "cognitive_deconstruction"
    }

    fn process(&mut self, input: &ModuleInput, _ctx: &HookContext) -> ModuleOutput {
        self.state = ModuleState::Processing;

        if input.signals.is_empty() {
            self.state = ModuleState::Completed;
            return ModuleOutput::new(
                TritValue::Hold,
                0.5,
                "deconstruction: no signals — insufficient evidence",
            );
        }

        let h_i = input_entropy(&input.signals);
        let d_o = output_determinacy(&input.signals);

        let impulse_detected = h_i > AMBIGUITY_THRESHOLD && d_o > DETERMINACY_THRESHOLD;

        self.state = ModuleState::Completed;

        if impulse_detected {
            self.impulse_count += 1;
            let interrupt = MetaInterrupt::new(
                ConflictType::PolicyViolation,
                format!(
                    "explanation impulse: input entropy H(I)={:.3} > {} (ambiguous) AND output determinacy D(O)={:.3} > {} (certain) — system may be fabricating certainty",
                    h_i, AMBIGUITY_THRESHOLD, d_o, DETERMINACY_THRESHOLD,
                ),
            );
            ModuleOutput::new(
                TritValue::Hold,
                0.3,
                format!(
                    "deconstruction: EXPLANATION IMPULSE #{} — H(I)={:.3}, D(O)={:.3}. Choose Hold.",
                    self.impulse_count, h_i, d_o
                ),
            )
            .with_interrupts(vec![interrupt])
        } else {
            ModuleOutput::new(
                TritValue::True,
                0.7,
                format!(
                    "deconstruction: no impulse — H(I)={:.3}, D(O)={:.3}",
                    h_i, d_o
                ),
            )
        }
    }

    fn on_mount(&mut self) {
        self.state = ModuleState::Idle;
    }

    fn on_unmount(&mut self) {
        self.impulse_count = 0;
        self.state = ModuleState::Completed;
    }

    fn state(&self) -> ModuleState {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::phase::Phase;
    use crate::core::Frame;

    fn word(frame: Frame, value: TritValue, phase: f64) -> crate::core::TritWord {
        crate::core::TritWord::new(value, Phase::new_clamped(phase), frame)
    }

    #[test]
    fn empty_input_insufficient_evidence() {
        let mut m = CognitiveDeconstruction::new();
        let input = ModuleInput {
            signals: vec![],
            interrupts: vec![],
            attention_cmd: None,
        };
        let out = m.process(&input, &HookContext::default());
        assert_eq!(out.recommendation, TritValue::Hold);
    }

    #[test]
    fn low_entropy_low_determinacy_no_impulse() {
        // All same value with neutral phases → low entropy, low determinacy
        let mut m = CognitiveDeconstruction::new();
        let input = ModuleInput {
            signals: vec![
                word(Frame::Science, TritValue::True, 0.5),
                word(Frame::Science, TritValue::True, 0.5),
            ],
            interrupts: vec![],
            attention_cmd: None,
        };
        let out = m.process(&input, &HookContext::default());
        assert_eq!(out.recommendation, TritValue::True);
    }

    #[test]
    fn high_ambiguity_high_certainty_detects_impulse() {
        // Mixed values (high entropy) + extreme phases (high determinacy) → impulse
        let mut m = CognitiveDeconstruction::new();
        let input = ModuleInput {
            signals: vec![
                word(Frame::Science, TritValue::True, 0.95),
                word(Frame::Individual, TritValue::False, 0.9),
                word(Frame::Consensus, TritValue::Hold, 0.88),
            ],
            interrupts: vec![],
            attention_cmd: None,
        };
        let out = m.process(&input, &HookContext::default());
        assert_eq!(out.recommendation, TritValue::Hold);
        assert!(!out.interrupts.is_empty());
        assert!(out.trace.contains("EXPLANATION IMPULSE"));
        assert_eq!(m.impulse_count(), 1);
    }

    #[test]
    fn mixed_values_neutral_phases_no_impulse() {
        // Mixed values (high entropy) but neutral phases (low determinacy) → no impulse
        let mut m = CognitiveDeconstruction::new();
        let input = ModuleInput {
            signals: vec![
                word(Frame::Science, TritValue::True, 0.5),
                word(Frame::Individual, TritValue::False, 0.5),
                word(Frame::Consensus, TritValue::Hold, 0.5),
            ],
            interrupts: vec![],
            attention_cmd: None,
        };
        let out = m.process(&input, &HookContext::default());
        // High entropy but low determinacy → no impulse
        assert_eq!(out.recommendation, TritValue::True);
    }

    #[test]
    fn impulse_count_resets_on_unmount() {
        let mut m = CognitiveDeconstruction::new();
        // Trigger an impulse
        let input = ModuleInput {
            signals: vec![
                word(Frame::Science, TritValue::True, 0.95),
                word(Frame::Individual, TritValue::False, 0.9),
                word(Frame::Consensus, TritValue::Hold, 0.88),
            ],
            interrupts: vec![],
            attention_cmd: None,
        };
        m.process(&input, &HookContext::default());
        assert_eq!(m.impulse_count(), 1);

        m.on_unmount();
        assert_eq!(m.impulse_count(), 0);
    }

    #[test]
    fn input_entropy_all_same_is_low() {
        let signals = vec![
            word(Frame::Science, TritValue::True, 0.5),
            word(Frame::Science, TritValue::True, 0.6),
        ];
        let h = input_entropy(&signals);
        assert!(h < 0.5);
    }

    #[test]
    fn input_entropy_all_different_is_high() {
        let signals = vec![
            word(Frame::Science, TritValue::True, 0.5),
            word(Frame::Science, TritValue::False, 0.5),
            word(Frame::Science, TritValue::Hold, 0.5),
        ];
        let h = input_entropy(&signals);
        assert!(h > 0.5);
    }

    #[test]
    fn output_determinacy_neutral_phases_is_low() {
        let signals = vec![
            word(Frame::Science, TritValue::True, 0.5),
            word(Frame::Science, TritValue::False, 0.5),
        ];
        let d = output_determinacy(&signals);
        assert!(d < 0.3);
    }

    #[test]
    fn output_determinacy_extreme_phases_is_high() {
        let signals = vec![
            word(Frame::Science, TritValue::True, 0.95),
            word(Frame::Science, TritValue::False, 0.05),
        ];
        let d = output_determinacy(&signals);
        assert!(d > 0.7);
    }

    #[test]
    fn module_id_and_name() {
        let m = CognitiveDeconstruction::new();
        assert_eq!(m.id(), ModuleId::CognitiveDeconstruction);
        assert_eq!(m.name(), "cognitive_deconstruction");
    }
}
