//! Ecological assessment adapter — practice testing and irreversibility.
//!
//! Assesses whether a proposed True decision risks irreversible consequences.
//! Flags overconfidence (extreme phase), high-complexity scenarios, and
//! physical interventions that may have permanent effects.

use crate::adapters::{adapter_lifecycle_no_unmount, CognitiveModule, ModuleInput, ModuleOutput};
use crate::core::{Frame, TritValue};
use crate::hook::module_registry::{ModuleId, ModuleState};
use crate::hook::HookContext;
use crate::meta::{MetaInterrupt, PolicyViolation};

/// Cognitive module for ecological consequence assessment.
pub struct EcologicalAssessment {
    state: ModuleState,
    /// How many irreversibility warnings have been raised.
    warning_count: usize,
}

impl EcologicalAssessment {
    /// Create a new ecological assessment module.
    pub fn new() -> Self {
        EcologicalAssessment {
            state: ModuleState::Idle,
            warning_count: 0,
        }
    }

    /// Number of irreversibility warnings raised.
    pub fn warning_count(&self) -> usize {
        self.warning_count
    }
}

impl Default for EcologicalAssessment {
    fn default() -> Self {
        Self::new()
    }
}

impl CognitiveModule for EcologicalAssessment {
    adapter_lifecycle_no_unmount!();

    fn id(&self) -> ModuleId {
        ModuleId::EcologicalAssessment
    }

    fn name(&self) -> &'static str {
        "ecological_assessment"
    }

    fn process(&mut self, input: &ModuleInput, _ctx: &HookContext) -> ModuleOutput {
        self.state = ModuleState::Processing;

        if input.signals.is_empty() {
            self.state = ModuleState::Completed;
            return ModuleOutput::new(
                TritValue::Hold,
                0.5,
                "ecological: no signals — cannot assess consequences",
            );
        }

        let mut interrupts = Vec::new();
        let mut concerns: Vec<String> = Vec::new();

        // Check 1: Extreme phase certainty (>0.9) → overconfidence flag.
        let overconfident: Vec<_> = input
            .signals
            .iter()
            .filter(|s| s.phase().inner() > 0.9)
            .collect();
        if !overconfident.is_empty() {
            concerns.push(format!(
                "{} signal(s) with extreme phase > 0.9 — possible overconfidence",
                overconfident.len()
            ));
        }

        // Check 2: High complexity (≥3 distinct frames) → harder to predict consequences.
        let mut frames_seen = std::collections::HashSet::new();
        for signal in &input.signals {
            frames_seen.insert(signal.frame());
        }
        if frames_seen.len() >= 3 {
            concerns.push(format!(
                "{} distinct frames — high complexity, consequence prediction uncertain",
                frames_seen.len()
            ));
        }

        // Check 3: Science-frame True signals with high phase → physical interventions
        // may be irreversible.
        let physical_interventions: Vec<_> = input
            .signals
            .iter()
            .filter(|s| {
                s.frame() == Frame::Science
                    && s.value() == TritValue::True
                    && s.phase().inner() > 0.7
            })
            .collect();
        if !physical_interventions.is_empty() {
            concerns.push(format!(
                "{} physical intervention(s) with high certainty — irreversibility risk",
                physical_interventions.len()
            ));
        }

        // Build interrupts from concerns.
        for concern in &concerns {
            interrupts.push(MetaInterrupt::policy_violation(
                PolicyViolation::SurvivalBoundaryOverride,
                format!("ecological: {}", concern),
            ));
        }

        self.state = ModuleState::Completed;

        if concerns.is_empty() {
            ModuleOutput::new(
                TritValue::True,
                0.7,
                "ecological: no irreversibility concerns detected",
            )
        } else {
            self.warning_count += concerns.len();
            ModuleOutput::new(
                TritValue::Hold,
                0.4,
                format!(
                    "ecological: {} concern(s) — {}",
                    concerns.len(),
                    concerns.join("; ")
                ),
            )
            .with_interrupts(interrupts)
        }
    }

    // ponytail: on_mount + state via adapter_lifecycle_no_unmount!()
    fn on_unmount(&mut self) {
        self.warning_count = 0;
        self.state = ModuleState::Completed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::phase::Phase;

    fn word(frame: Frame, value: TritValue, phase: f64) -> crate::core::TritWord {
        crate::core::TritWord::new(value, Phase::new_clamped(phase), frame)
    }

    #[test]
    fn empty_input_hold() {
        let mut m = EcologicalAssessment::new();
        let input = ModuleInput {
            signals: vec![],
            interrupts: vec![],
            attention_cmd: None,
        };
        let out = m.process(&input, &HookContext::default());
        assert_eq!(out.recommendation, TritValue::Hold);
    }

    #[test]
    fn no_concerns_clean_signals() {
        let mut m = EcologicalAssessment::new();
        let input = ModuleInput {
            signals: vec![
                word(Frame::Individual, TritValue::True, 0.6),
                word(Frame::Individual, TritValue::True, 0.7),
            ],
            interrupts: vec![],
            attention_cmd: None,
        };
        let out = m.process(&input, &HookContext::default());
        assert_eq!(out.recommendation, TritValue::True);
    }

    #[test]
    fn overconfidence_detected() {
        let mut m = EcologicalAssessment::new();
        let input = ModuleInput {
            signals: vec![word(Frame::Science, TritValue::True, 0.95)],
            interrupts: vec![],
            attention_cmd: None,
        };
        let out = m.process(&input, &HookContext::default());
        assert_eq!(out.recommendation, TritValue::Hold);
        assert!(!out.interrupts.is_empty());
    }

    #[test]
    fn physical_intervention_irreversibility() {
        let mut m = EcologicalAssessment::new();
        let input = ModuleInput {
            signals: vec![word(Frame::Science, TritValue::True, 0.85)],
            interrupts: vec![],
            attention_cmd: None,
        };
        let out = m.process(&input, &HookContext::default());
        assert_eq!(out.recommendation, TritValue::Hold);
        assert!(out.trace.contains("irreversibility"));
    }

    #[test]
    fn high_complexity_warning() {
        let mut m = EcologicalAssessment::new();
        let input = ModuleInput {
            signals: vec![
                word(Frame::Science, TritValue::True, 0.5),
                word(Frame::Individual, TritValue::True, 0.5),
                word(Frame::Consensus, TritValue::True, 0.5),
            ],
            interrupts: vec![],
            attention_cmd: None,
        };
        let out = m.process(&input, &HookContext::default());
        assert_eq!(out.recommendation, TritValue::Hold);
    }

    #[test]
    fn warning_count_resets_on_unmount() {
        let mut m = EcologicalAssessment::new();
        let input = ModuleInput {
            signals: vec![word(Frame::Science, TritValue::True, 0.95)],
            interrupts: vec![],
            attention_cmd: None,
        };
        m.process(&input, &HookContext::default());
        assert!(m.warning_count() > 0);

        m.on_unmount();
        assert_eq!(m.warning_count(), 0);
    }

    #[test]
    fn module_id_and_name() {
        let m = EcologicalAssessment::new();
        assert_eq!(m.id(), ModuleId::EcologicalAssessment);
        assert_eq!(m.name(), "ecological_assessment");
    }
}
