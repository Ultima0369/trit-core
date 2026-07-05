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

    /// Snapshot all three sensors for display. Emits a reading regardless of
    /// pass/fail so the map popup can show the live value vs threshold.
    /// Fail-closed: an unavailable source records value=NaN, violated=true.
    pub fn snapshot(&self) -> Vec<crate::anchor::SensorReading> {
        use crate::anchor::SensorReading;
        let bii = match self.bii_source.sample() {
            Ok(v) => (v, v < self.config.bii_min),
            Err(_) => (f64::NAN, true),
        };
        let sink = match self.sink_source.sample() {
            Ok(v) => (v, v < self.config.sink_capacity_min),
            Err(_) => (f64::NAN, true),
        };
        let ph = match self.ph_source.sample() {
            Ok(v) => (v, v < self.config.ocean_ph_min),
            Err(_) => (f64::NAN, true),
        };
        vec![
            SensorReading {
                name: "Biodiversity Intactness",
                value: bii.0,
                threshold: self.config.bii_min,
                violated: bii.1,
                unit: "index",
            },
            SensorReading {
                name: "Carbon sink capacity",
                value: sink.0,
                threshold: self.config.sink_capacity_min,
                violated: sink.1,
                unit: "fraction",
            },
            SensorReading {
                name: "Ocean surface pH",
                value: ph.0,
                threshold: self.config.ocean_ph_min,
                violated: ph.1,
                unit: "pH",
            },
        ]
    }
}

impl AnchorConstraint for EcologicalBase {
    fn name(&self) -> &'static str {
        "ecological_base"
    }

    fn severity(&self) -> AnchorSeverity {
        AnchorSeverity::Abort
    }

    fn check(&self, decision: &DecisionPreview) -> Option<AnchorViolation> {
        // Fail-closed: a sensor error is treated as a violation. These anchors
        // are Abort-severity with veto power — unavailable data must not allow
        // an unsafe decision to proceed. See CLAUDE.md Layer 1.

        // ── Static source checks (global environmental state) ──────

        match self.bii_source.sample() {
            Ok(bii) if bii < self.config.bii_min => {
                return Some(AnchorViolation {
                    anchor_name: self.name().to_string(),
                    description: format!("BII {:.2} below minimum {:.2}", bii, self.config.bii_min),
                    severity: self.severity(),
                    actual_value: bii,
                    threshold: self.config.bii_min,
                });
            }
            Ok(_) => {}
            Err(e) => {
                return Some(AnchorViolation {
                    anchor_name: self.name().to_string(),
                    description: format!("BII data source unavailable: {e}"),
                    severity: self.severity(),
                    actual_value: f64::NAN,
                    threshold: self.config.bii_min,
                });
            }
        }

        match self.sink_source.sample() {
            Ok(sink) if sink < self.config.sink_capacity_min => {
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
            Ok(_) => {}
            Err(e) => {
                return Some(AnchorViolation {
                    anchor_name: self.name().to_string(),
                    description: format!("Carbon sink data source unavailable: {e}"),
                    severity: self.severity(),
                    actual_value: f64::NAN,
                    threshold: self.config.sink_capacity_min,
                });
            }
        }

        match self.ph_source.sample() {
            Ok(ph) if ph < self.config.ocean_ph_min => {
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
            Ok(_) => {}
            Err(e) => {
                return Some(AnchorViolation {
                    anchor_name: self.name().to_string(),
                    description: format!("Ocean pH data source unavailable: {e}"),
                    severity: self.severity(),
                    actual_value: f64::NAN,
                    threshold: self.config.ocean_ph_min,
                });
            }
        }

        // ── Decision-specific check (ponytail audit H) ────────────
        // Check whether the decision's ecosystem impact zone combined with
        // high irreversible change risk triggers a veto.

        if decision.irreversible_change_risk > 0.8 {
            return Some(AnchorViolation {
                anchor_name: self.name().to_string(),
                description: format!(
                    "Decision irreversible change risk {:.0}% exceeds ecological ceiling — \
                     species extinction-level risk is not acceptable regardless of other metrics",
                    decision.irreversible_change_risk * 100.0
                ),
                severity: self.severity(),
                actual_value: decision.irreversible_change_risk,
                threshold: 0.8,
            });
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anchor::AnchorError;
    use std::time::Duration;

    /// A data source that always errors — proves fail-closed behavior.
    struct FailingSource;
    impl DataSource<f64> for FailingSource {
        fn sample(&self) -> Result<f64, AnchorError> {
            Err(AnchorError::Unavailable("sensor offline".to_string()))
        }
        fn resolution(&self) -> Duration {
            Duration::from_secs(1)
        }
    }

    fn ecological_with_failing_sources() -> EcologicalBase {
        EcologicalBase {
            config: EcologicalBaseConfig::default(),
            bii_source: Box::new(FailingSource),
            sink_source: Box::new(FailingSource),
            ph_source: Box::new(FailingSource),
        }
    }

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

    #[test]
    fn unavailable_sensor_fails_closed() {
        // Abort-severity anchor: a sensor error must produce a violation,
        // not silently pass. Regression guard for the fail-open pattern.
        let eco = ecological_with_failing_sources();
        let violation = eco
            .check(&DecisionPreview::neutral())
            .expect("sensor failure must fail closed, not pass");
        assert_eq!(violation.severity, AnchorSeverity::Abort);
        assert!(violation.description.contains("unavailable"));
        assert!(violation.actual_value.is_nan());
    }

    #[test]
    fn snapshot_safe_has_no_violations() {
        let eco = EcologicalBase::safe();
        let readings = eco.snapshot();
        assert_eq!(readings.len(), 3);
        assert!(
            readings.iter().all(|r| !r.violated),
            "safe ecology should have no violations"
        );
        // BII safe value 0.85 > threshold 0.75
        let bii = &readings[0];
        assert!((bii.value - 0.85).abs() < 1e-9);
        assert!((bii.threshold - 0.75).abs() < 1e-9);
    }

    #[test]
    fn snapshot_degraded_marks_violations() {
        let eco = EcologicalBase::degraded();
        let readings = eco.snapshot();
        // degraded: BII 0.60 < 0.75, sink 0.40 < 0.50, pH 7.90 < 7.95 — all violated
        assert!(
            readings.iter().all(|r| r.violated),
            "degraded ecology should flag all sensors"
        );
    }

    #[test]
    fn snapshot_failing_source_records_nan_and_violated() {
        let eco = ecological_with_failing_sources();
        let readings = eco.snapshot();
        assert!(readings.iter().all(|r| r.violated));
        assert!(readings.iter().all(|r| r.value.is_nan()));
    }
}
