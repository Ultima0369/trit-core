//! Survival motive weight constants.
//!
//! These are **not trainable** — they are constants of the architecture.
//! They represent the irreducible set of survival-level needs that must
//! be satisfied before any non-survival consideration.
//!
//! Weights are in [0.0, 1.0] and never change at runtime.

use crate::anchor::{AnchorConstraint, AnchorSeverity, AnchorViolation, DecisionPreview};

/// The five survival motive dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SurvivalDimension {
    Hunger,
    Thirst,
    PhysicalSafety,
    ThermalSafety,
    Belonging,
}

/// Immutable weight matrix for survival motives.
#[derive(Debug, Clone)]
pub struct SurvivalMotiveWeights {
    pub hunger: f64,
    pub thirst: f64,
    pub physical_safety: f64,
    pub thermal_safety: f64,
    pub belonging: f64,
}

impl Default for SurvivalMotiveWeights {
    fn default() -> Self {
        SurvivalMotiveWeights {
            hunger: 0.95,
            thirst: 0.98,
            physical_safety: 0.97,
            thermal_safety: 0.93,
            belonging: 0.85,
        }
    }
}

/// Survival motives anchor.
///
/// This anchor is special: it does not inspect DecisionPreview
/// but rather acts as a constant weight reference. Its `check` always
/// passes; the weights are queried by other system components.
pub struct SurvivalMotives {
    pub weights: SurvivalMotiveWeights,
}

impl SurvivalMotives {
    pub fn new() -> Self {
        SurvivalMotives {
            weights: SurvivalMotiveWeights::default(),
        }
    }

    /// Get the weight for a specific survival dimension.
    pub fn weight(&self, dim: SurvivalDimension) -> f64 {
        match dim {
            SurvivalDimension::Hunger => self.weights.hunger,
            SurvivalDimension::Thirst => self.weights.thirst,
            SurvivalDimension::PhysicalSafety => self.weights.physical_safety,
            SurvivalDimension::ThermalSafety => self.weights.thermal_safety,
            SurvivalDimension::Belonging => self.weights.belonging,
        }
    }

    /// Returns true if any survival motive weight is below 0.5
    /// (indicating a likely configuration error).
    pub fn is_valid(&self) -> bool {
        self.weights.hunger >= 0.5
            && self.weights.thirst >= 0.5
            && self.weights.physical_safety >= 0.5
            && self.weights.thermal_safety >= 0.5
            && self.weights.belonging >= 0.5
    }
}

impl Default for SurvivalMotives {
    fn default() -> Self {
        Self::new()
    }
}

impl AnchorConstraint for SurvivalMotives {
    fn name(&self) -> &'static str {
        "survival_motives"
    }

    fn severity(&self) -> AnchorSeverity {
        AnchorSeverity::Abort
    }

    fn check(&self, _decision: &DecisionPreview) -> Option<AnchorViolation> {
        // Survival motives do not inspect decisions directly.
        // They provide constant weight references. If the weights
        // are invalid (a configuration error), we flag it.
        if !self.is_valid() {
            return Some(AnchorViolation {
                anchor_name: self.name().to_string(),
                description: "Survival motive weights have been tampered with".to_string(),
                severity: AnchorSeverity::Abort,
                actual_value: 0.0,
                threshold: 0.5,
            });
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_weights_are_valid() {
        let motives = SurvivalMotives::new();
        assert!(motives.is_valid());
    }

    #[test]
    fn weight_ordering_is_correct() {
        let motives = SurvivalMotives::new();
        // Thirst > Physical Safety > Hunger > Thermal Safety > Belonging
        assert!(motives.weights.thirst > motives.weights.physical_safety);
        assert!(motives.weights.physical_safety > motives.weights.hunger);
        assert!(motives.weights.hunger > motives.weights.thermal_safety);
        assert!(motives.weights.thermal_safety > motives.weights.belonging);
    }

    #[test]
    fn anchor_check_always_passes_for_valid_weights() {
        let motives = SurvivalMotives::new();
        assert!(motives.check(&DecisionPreview::neutral()).is_none());
    }
}
