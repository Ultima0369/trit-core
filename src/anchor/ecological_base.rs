//! Ecological foundation state monitoring.
//!
//! Thresholds (MVP static values):
//! - Biodiversity Intactness Index (BII): must stay above 0.75 globally
//! - Carbon sink capacity: oceanic + terrestrial sinks >= 50% of pre-industrial
//! - Ocean acidification: surface pH must not drop below 7.95 (pre-industrial: 8.17)

use std::time::Duration;

use crate::anchor::{
    AnchorConstraint, AnchorSeverity, AnchorViolation, DataSource, DecisionPreview, StaticSource,
};

#[derive(Debug, Clone)]
pub struct EcologicalBaseConfig {
    /// Minimum BII threshold (default: 0.75).
    pub bii_min: f64,
    /// Minimum carbon sink capacity fraction (default: 0.50).
    pub sink_capacity_min: f64,
    /// Minimum ocean surface pH (default: 7.95).
    pub ocean_ph_min: f64,
}

impl Default for EcologicalBaseConfig {
    fn default() -> Self {
        EcologicalBaseConfig {
            bii_min: 0.75,
            sink_capacity_min: 0.50,
            ocean_ph_min: 7.95,
        }
    }
}

pub struct EcologicalBase {
    config: EcologicalBaseConfig,
    bii_source: Box<dyn DataSource<f64>>,
    sink_source: Box<dyn DataSource<f64>>,
    ph_source: Box<dyn DataSource<f64>>,
}

impl EcologicalBase {
    pub fn with_static_values(
        bii: f64,
        sink_capacity: f64,
        ocean_ph: f64,
        config: EcologicalBaseConfig,
    ) -> Self {
        EcologicalBase {
            config,
            bii_source: Box::new(StaticSource::new(bii, Duration::from_secs(86400))),
            sink_source: Box::new(StaticSource::new(sink_capacity, Duration::from_secs(86400))),
            ph_source: Box::new(StaticSource::new(ocean_ph, Duration::from_secs(86400))),
        }
    }

    pub fn safe() -> Self {
        Self::with_static_values(0.85, 0.70, 8.05, EcologicalBaseConfig::default())
    }

    pub fn degraded() -> Self {
        Self::with_static_values(0.60, 0.40, 7.90, EcologicalBaseConfig::default())
    }
}

impl AnchorConstraint for EcologicalBase {
    fn name(&self) -> &'static str {
        "ecological_base"
    }

    fn severity(&self) -> AnchorSeverity {
        AnchorSeverity::Abort
    }

    fn check(&self, _decision: &DecisionPreview) -> Option<AnchorViolation> {
        if let Ok(bii) = self.bii_source.sample() {
            if bii < self.config.bii_min {
                return Some(AnchorViolation {
                    anchor_name: self.name().to_string(),
                    description: format!("BII {:.2} below minimum {:.2}", bii, self.config.bii_min),
                    severity: self.severity(),
                    actual_value: bii,
                    threshold: self.config.bii_min,
                });
            }
        }

        if let Ok(sink) = self.sink_source.sample() {
            if sink < self.config.sink_capacity_min {
                return Some(AnchorViolation {
                    anchor_name: self.name().to_string(),
                    description: format!(
                        "Carbon sink capacity {:.2} below minimum {:.2}",
                        sink, self.config.sink_capacity_min
                    ),
                    severity: self.severity(),
                    actual_value: sink,
                    threshold: self.config.sink_capacity_min,
                });
            }
        }

        if let Ok(ph) = self.ph_source.sample() {
            if ph < self.config.ocean_ph_min {
                return Some(AnchorViolation {
                    anchor_name: self.name().to_string(),
                    description: format!(
                        "Ocean pH {:.2} below minimum {:.2}",
                        ph, self.config.ocean_ph_min
                    ),
                    severity: self.severity(),
                    actual_value: ph,
                    threshold: self.config.ocean_ph_min,
                });
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_ecology_passes() {
        let eco = EcologicalBase::safe();
        assert!(eco.check(&DecisionPreview::neutral()).is_none());
    }

    #[test]
    fn degraded_ecology_fails() {
        let eco = EcologicalBase::degraded();
        assert!(eco.check(&DecisionPreview::neutral()).is_some());
    }
}
