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

    /// Snapshot all three sensors for display. Emits a reading regardless of
    /// pass/fail so the map popup can show the live value vs threshold.
    /// Fail-closed: an unavailable source records value=NaN, violated=true.
    pub fn snapshot(&self) -> Vec<crate::anchor::SensorReading> {
        use crate::anchor::SensorReading;
        let olr = match self.olr_source.sample() {
            Ok(v) => (v.abs(), v.abs() > self.config.olr_anomaly_max),
            Err(_) => (f64::NAN, true),
        };
        let co2 = match self.co2_source.sample() {
            Ok(v) => (v, v > self.config.co2_ceiling_ppm),
            Err(_) => (f64::NAN, true),
        };
        let imb = match self.imbalance_source.sample() {
            Ok(v) => (v.abs(), v.abs() > self.config.energy_imbalance_max),
            Err(_) => (f64::NAN, true),
        };
        vec![
            SensorReading {
                name: "OLR anomaly",
                value: olr.0,
                threshold: self.config.olr_anomaly_max,
                violated: olr.1,
                unit: "W/m2",
            },
            SensorReading {
                name: "CO2 equivalent",
                value: co2.0,
                threshold: self.config.co2_ceiling_ppm,
                violated: co2.1,
                unit: "ppm",
            },
            SensorReading {
                name: "Energy imbalance",
                value: imb.0,
                threshold: self.config.energy_imbalance_max,
                violated: imb.1,
                unit: "W/m2",
            },
        ]
    }
}

impl AnchorConstraint for ThermalBaseline {
    fn name(&self) -> &'static str {
        "thermal_baseline"
    }

    fn severity(&self) -> AnchorSeverity {
        AnchorSeverity::Abort
    }

    fn check(&self, decision: &DecisionPreview) -> Option<AnchorViolation> {
        // Fail-closed: a sensor error is treated as a violation. This anchor is
        // Abort-severity with veto power — unavailable data must not allow an
        // unsafe decision to proceed. See CLAUDE.md Layer 1.

        // ── Static source checks (global environmental state) ──────
        // These check the ambient environmental thresholds — independent of
        // the specific decision being made.

        match self.olr_source.sample() {
            Ok(olr) if olr.abs() > self.config.olr_anomaly_max => {
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
            Ok(_) => {}
            Err(e) => {
                return Some(AnchorViolation {
                    anchor_name: self.name().to_string(),
                    description: format!("OLR data source unavailable: {e}"),
                    severity: self.severity(),
                    actual_value: f64::NAN,
                    threshold: self.config.olr_anomaly_max,
                });
            }
        }

        match self.co2_source.sample() {
            Ok(co2) if co2 > self.config.co2_ceiling_ppm => {
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
            Ok(_) => {}
            Err(e) => {
                return Some(AnchorViolation {
                    anchor_name: self.name().to_string(),
                    description: format!("CO2 data source unavailable: {e}"),
                    severity: self.severity(),
                    actual_value: f64::NAN,
                    threshold: self.config.co2_ceiling_ppm,
                });
            }
        }

        match self.imbalance_source.sample() {
            Ok(imbalance) if imbalance.abs() > self.config.energy_imbalance_max => {
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
            Ok(_) => {}
            Err(e) => {
                return Some(AnchorViolation {
                    anchor_name: self.name().to_string(),
                    description: format!("Energy imbalance data source unavailable: {e}"),
                    severity: self.severity(),
                    actual_value: f64::NAN,
                    threshold: self.config.energy_imbalance_max,
                });
            }
        }

        // ── Decision-specific check (ponytail audit finding H) ──────
        // Check whether THIS decision's expected carbon footprint would
        // meaningfully worsen the atmospheric state. This makes the anchor
        // responsive to the actual decision being evaluated, not just the
        // global background state.

        if decision.expected_carbon_kg > 1_000_000.0 {
            return Some(AnchorViolation {
                anchor_name: self.name().to_string(),
                description: format!(
                    "Decision carbon footprint {:.0} kg CO2-eq exceeds 1,000-tonne ceiling — \
                     this decision's expected emissions are incompatible with thermal baseline",
                    decision.expected_carbon_kg
                ),
                severity: self.severity(),
                actual_value: decision.expected_carbon_kg,
                threshold: 1_000_000.0,
            });
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

    #[test]
    fn snapshot_safe_has_no_violations() {
        let baseline = ThermalBaseline::safe();
        let readings = baseline.snapshot();
        assert_eq!(readings.len(), 3);
        assert!(
            readings.iter().all(|r| !r.violated),
            "safe baseline should have no violations"
        );
        // CO2 safe value 415 < ceiling 450
        let co2 = &readings[1];
        assert!((co2.value - 415.0).abs() < 1e-9);
        assert!((co2.threshold - 450.0).abs() < 1e-9);
    }

    #[test]
    fn snapshot_exceeded_marks_violations() {
        let baseline = ThermalBaseline::exceeded();
        let readings = baseline.snapshot();
        // exceeded: OLR 3.0 > 2.5, CO2 460 > 450, imbalance 1.5 > 1.0 — all violated
        assert!(
            readings.iter().all(|r| r.violated),
            "exceeded baseline should flag all sensors"
        );
    }
}
