//! Universal wellbeing priority ordering.
//!
//! Defines the priority structure for wellbeing across:
//! - Intergenerational justice (future lives not discounted)
//! - Non-human life weighting
//! - Irreversible damage red lines

use crate::anchor::{AnchorConstraint, AnchorSeverity, AnchorViolation, DecisionPreview};

/// Wellbeing priority configuration.
#[derive(Debug, Clone)]
pub struct WellbeingPriorityConfig {
    /// Intergenerational discount factor.
    /// 1.0 = future lives valued equally to present lives.
    /// Must be >= 0.95 (future lives cannot be discounted below 95% of present).
    pub intergenerational_discount: f64,
    /// Non-human sentient life weight relative to human life.
    /// Must be >= 0.3 in trade-off calculations.
    pub non_human_life_weight: f64,
    /// Irreversible damage probability threshold.
    /// Any action with expected irreversible ecosystem damage
    /// probability > this threshold is rejected.
    pub irreversible_damage_threshold: f64,
}

impl Default for WellbeingPriorityConfig {
    fn default() -> Self {
        WellbeingPriorityConfig {
            intergenerational_discount: 0.95,
            non_human_life_weight: 0.3,
            irreversible_damage_threshold: 0.01,
        }
    }
}

/// Wellbeing priority anchor.
pub struct WellbeingPriority {
    config: WellbeingPriorityConfig,
}

impl WellbeingPriority {
    pub fn new() -> Self {
        WellbeingPriority {
            config: WellbeingPriorityConfig::default(),
        }
    }

    pub fn with_config(config: WellbeingPriorityConfig) -> Self {
        WellbeingPriority { config }
    }
}

impl Default for WellbeingPriority {
    fn default() -> Self {
        Self::new()
    }
}

impl AnchorConstraint for WellbeingPriority {
    fn name(&self) -> &'static str {
        "wellbeing_priority"
    }

    fn severity(&self) -> AnchorSeverity {
        AnchorSeverity::Abort
    }

    fn check(&self, decision: &DecisionPreview) -> Option<AnchorViolation> {
        // Check irreversible damage risk
        if decision.irreversible_change_risk > self.config.irreversible_damage_threshold {
            return Some(AnchorViolation {
                anchor_name: self.name().to_string(),
                description: format!(
                    "Irreversible change risk {:.4} exceeds threshold {:.4}",
                    decision.irreversible_change_risk, self.config.irreversible_damage_threshold
                ),
                severity: self.severity(),
                actual_value: decision.irreversible_change_risk,
                threshold: self.config.irreversible_damage_threshold,
            });
        }

        // Check intergenerational discount is not violated
        if self.config.intergenerational_discount < 0.95 {
            return Some(AnchorViolation {
                anchor_name: self.name().to_string(),
                description: format!(
                    "Intergenerational discount {:.2} below minimum 0.95",
                    self.config.intergenerational_discount
                ),
                severity: self.severity(),
                actual_value: self.config.intergenerational_discount,
                threshold: 0.95,
            });
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_decision_passes() {
        let priority = WellbeingPriority::new();
        let preview = DecisionPreview {
            irreversible_change_risk: 0.001,
            ..DecisionPreview::neutral()
        };
        assert!(priority.check(&preview).is_none());
    }

    #[test]
    fn high_irreversible_risk_fails() {
        let priority = WellbeingPriority::new();
        let preview = DecisionPreview {
            irreversible_change_risk: 0.05,
            ..DecisionPreview::neutral()
        };
        let violation = priority.check(&preview);
        assert!(violation.is_some());
        assert_eq!(violation.unwrap().severity, AnchorSeverity::Abort);
    }

    #[test]
    fn low_discount_factor_fails() {
        let config = WellbeingPriorityConfig {
            intergenerational_discount: 0.80,
            ..Default::default()
        };
        let priority = WellbeingPriority::with_config(config);
        let violation = priority.check(&DecisionPreview::neutral());
        assert!(violation.is_some());
    }
}
