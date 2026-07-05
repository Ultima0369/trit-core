//! Flourishing desiderata pool.
//!
//! Non-survival desiderata that accumulate over time but cannot be
//! traded against survival motives. These represent what a system
//! (or agent) can aspire to once survival is secured.
//!
//! Dimensions: Autonomy, Creativity, Connection, Transcendence

use crate::anchor::{AnchorConstraint, AnchorSeverity, AnchorViolation, DecisionPreview};

/// Flourishing dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlourishingDimension {
    Autonomy,
    Creativity,
    Connection,
    Transcendence,
}

/// A single flourishing indicator with current level and aspiration.
#[derive(Debug, Clone, PartialEq)]
pub struct FlourishingIndicator {
    pub dimension: FlourishingDimension,
    /// Current level in [0.0, 1.0].
    pub current_level: f64,
    /// Aspiration level in [0.0, 1.0].
    pub aspiration: f64,
    /// Whether this indicator is currently satisfied (current >= aspiration).
    pub satisfied: bool,
}

impl FlourishingIndicator {
    pub fn new(dimension: FlourishingDimension, current: f64, aspiration: f64) -> Self {
        let current_level = current.clamp(0.0, 1.0);
        let aspiration = aspiration.clamp(0.0, 1.0);
        FlourishingIndicator {
            dimension,
            current_level,
            aspiration,
            // Compute from the clamped value, consistent with `update()`.
            satisfied: current_level >= aspiration,
        }
    }
}

/// Pool of flourishing indicators.
///
/// These accumulate over time. No flourishing indicator can be
/// traded against a survival motive — the survival layer always
/// takes priority.
pub struct FlourishingPool {
    pub indicators: Vec<FlourishingIndicator>,
}

impl FlourishingPool {
    pub fn new() -> Self {
        FlourishingPool {
            indicators: vec![
                FlourishingIndicator::new(FlourishingDimension::Autonomy, 0.7, 0.7),
                FlourishingIndicator::new(FlourishingDimension::Creativity, 0.6, 0.6),
                FlourishingIndicator::new(FlourishingDimension::Connection, 0.7, 0.7),
                FlourishingIndicator::new(FlourishingDimension::Transcendence, 0.5, 0.5),
            ],
        }
    }

    /// Returns true if all indicators are satisfied.
    pub fn all_satisfied(&self) -> bool {
        self.indicators.iter().all(|i| i.satisfied)
    }

    /// Returns the fraction of satisfied indicators.
    pub fn satisfaction_fraction(&self) -> f64 {
        if self.indicators.is_empty() {
            return 1.0;
        }
        let satisfied = self.indicators.iter().filter(|i| i.satisfied).count();
        satisfied as f64 / self.indicators.len() as f64
    }

    /// Update the level of a flourishing dimension.
    pub fn update(&mut self, dim: FlourishingDimension, new_level: f64) {
        if let Some(indicator) = self.indicators.iter_mut().find(|i| i.dimension == dim) {
            indicator.current_level = new_level.clamp(0.0, 1.0);
            indicator.satisfied = indicator.current_level >= indicator.aspiration;
        }
    }
}

impl Default for FlourishingPool {
    fn default() -> Self {
        Self::new()
    }
}

impl AnchorConstraint for FlourishingPool {
    fn name(&self) -> &'static str {
        "flourishing_pool"
    }

    fn severity(&self) -> AnchorSeverity {
        // Flourishing violations are DowngradeToHold, not Abort.
        // A system can operate with unmet flourishing needs,
        // but should signal that it is doing so.
        AnchorSeverity::DowngradeToHold
    }

    /// Check flourishing satisfaction.
    ///
    /// Note (ponytail audit H): this check uses the pool's internal indicator
    /// state only — DecisionPreview has no flourishing-specific fields.
    /// When a flourishing-impact model is added to DecisionPreview (e.g.,
    /// `autonomy_impact: f64`), this check should incorporate those fields.
    fn check(&self, _decision: &DecisionPreview) -> Option<AnchorViolation> {
        let fraction = self.satisfaction_fraction();
        if fraction < 0.25 {
            return Some(AnchorViolation {
                anchor_name: self.name().to_string(),
                description: format!(
                    "Flourishing satisfaction fraction {:.2} below 0.25 threshold",
                    fraction
                ),
                severity: self.severity(),
                actual_value: fraction,
                threshold: 0.25,
            });
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_pool_has_four_indicators() {
        let pool = FlourishingPool::new();
        assert_eq!(pool.indicators.len(), 4);
    }

    #[test]
    fn satisfaction_fraction_is_correct() {
        let mut pool = FlourishingPool::new();
        assert_eq!(pool.satisfaction_fraction(), 1.0); // default is all-satisfied
                                                       // Make some unsatisfied
        pool.indicators[0].current_level = 0.5;
        pool.indicators[0].aspiration = 0.9;
        pool.indicators[0].satisfied = false;
        assert!(pool.satisfaction_fraction() < 1.0);
    }

    #[test]
    fn low_satisfaction_triggers_downgrade() {
        let mut pool = FlourishingPool::new();
        for ind in &mut pool.indicators {
            ind.current_level = 0.1;
            ind.satisfied = false;
        }
        let violation = pool.check(&DecisionPreview::neutral());
        assert!(violation.is_some());
        assert_eq!(violation.unwrap().severity, AnchorSeverity::DowngradeToHold);
    }
}
