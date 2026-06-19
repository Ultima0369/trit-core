//! Earth thermal radiation baseline constraint.
//!
//! The Earth's outgoing longwave radiation (OLR) must remain within
//! the range that permits human civilization. This anchor defines
//! threshold bounds based on CERES satellite data.
//!
//! Thresholds (MVP static values):
//! - OLR anomaly: +/- 2.5 W/m2 deviation from 240 W/m2 mean
//! - CO2 equivalent ceiling: 450 ppm (sustained, not transient)
//! - Top-of-atmosphere energy imbalance: within +/- 1.0 W/m2

use std::time::Duration;

use crate::anchor::{
    AnchorConstraint, AnchorSeverity, AnchorViolation, DataSource, DecisionPreview, StaticSource,
};

/// Configurable thermal baseline parameters.
#[derive(Debug, Clone)]
pub struct ThermalBaselineConfig {
    /// Mean OLR in W/m2 (default: 240.0).
    pub olr_mean: f64,
    /// Maximum allowed OLR anomaly in W/m2 (default: 2.5).
    pub olr_anomaly_max: f64,
    /// CO2 equivalent ceiling in ppm (default: 450.0).
    pub co2_ceiling_ppm: f64,
    /// Maximum top-of-atmosphere energy imbalance in W/m2 (default: 1.0).
    pub energy_imbalance_max: f64,
}

impl Default for ThermalBaselineConfig {
    fn default() -> Self {
        ThermalBaselineConfig {
            olr_mean: 240.0,
            olr_anomaly_max: 2.5,
            co2_ceiling_ppm: 450.0,
            energy_imbalance_max: 1.0,
        }
    }
}

/// Thermal baseline anchor constraint.
pub struct ThermalBaseline {
    config: ThermalBaselineConfig,
    /// Source of OLR anomaly data (W/m2 deviation from mean).
    olr_source: Box<dyn DataSource<f64>>,
    /// Source of CO2 equivalent concentration (ppm).
    co2_source: Box<dyn DataSource<f64>>,
    /// Source of energy imbalance data (W/m2 net flux).
    imbalance_source: Box<dyn DataSource<f64>>,
}

impl ThermalBaseline {
    /// Create a thermal baseline with static data sources (MVP).
    pub fn with_static_values(
        olr_anomaly: f64,
        co2_ppm: f64,
        energy_imbalance: f64,
        config: ThermalBaselineConfig,
    ) -> Self {
        ThermalBaseline {
            config,
            olr_source: Box::new(StaticSource::new(olr_anomaly, Duration::from_secs(3600))),
            co2_source: Box::new(StaticSource::new(co2_ppm, Duration::from_secs(3600))),
            imbalance_source: Box::new(StaticSource::new(
                energy_imbalance,
                Duration::from_secs(3600),
            )),
        }
    }

    /// Create with safe (within-bounds) default values.
    pub fn safe() -> Self {
        Self::with_static_values(0.0, 415.0, 0.5, ThermalBaselineConfig::default())
    }

    /// Create with values that trigger Abort (for testing).
    pub fn exceeded() -> Self {
        Self::with_static_values(3.0, 460.0, 1.5, ThermalBaselineConfig::default())
    }
}

impl AnchorConstraint for ThermalBaseline {
    fn name(&self) -> &'static str {
        "thermal_baseline"
    }

    fn severity(&self) -> AnchorSeverity {
        AnchorSeverity::Abort
    }

    fn check(&self, _decision: &DecisionPreview) -> Option<AnchorViolation> {
        // Check OLR anomaly
        if let Ok(olr) = self.olr_source.sample() {
            if olr.abs() > self.config.olr_anomaly_max {
                return Some(AnchorViolation {
                    anchor_name: self.name().to_string(),
                    description: format!(
                        "OLR anomaly {:.1} W/m2 exceeds threshold {:.1} W/m2",
                        olr.abs(),
                        self.config.olr_anomaly_max
                    ),
                    severity: self.severity(),
                    actual_value: olr.abs(),
                    threshold: self.config.olr_anomaly_max,
                });
            }
        }

        // Check CO2
        if let Ok(co2) = self.co2_source.sample() {
            if co2 > self.config.co2_ceiling_ppm {
                return Some(AnchorViolation {
                    anchor_name: self.name().to_string(),
                    description: format!(
                        "CO2 equivalent {:.0} ppm exceeds ceiling {:.0} ppm",
                        co2, self.config.co2_ceiling_ppm
                    ),
                    severity: self.severity(),
                    actual_value: co2,
                    threshold: self.config.co2_ceiling_ppm,
                });
            }
        }

        // Check energy imbalance
        if let Ok(imbalance) = self.imbalance_source.sample() {
            if imbalance.abs() > self.config.energy_imbalance_max {
                return Some(AnchorViolation {
                    anchor_name: self.name().to_string(),
                    description: format!(
                        "Energy imbalance {:.1} W/m2 exceeds threshold {:.1} W/m2",
                        imbalance.abs(),
                        self.config.energy_imbalance_max
                    ),
                    severity: self.severity(),
                    actual_value: imbalance.abs(),
                    threshold: self.config.energy_imbalance_max,
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
    fn safe_baseline_passes() {
        let baseline = ThermalBaseline::safe();
        let result = baseline.check(&DecisionPreview::neutral());
        assert!(result.is_none());
    }

    #[test]
    fn exceeded_baseline_fails() {
        let baseline = ThermalBaseline::exceeded();
        let result = baseline.check(&DecisionPreview::neutral());
        assert!(result.is_some());
        assert_eq!(result.unwrap().severity, AnchorSeverity::Abort);
    }
}
