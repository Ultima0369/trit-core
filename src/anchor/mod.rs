//! Steady anchor layer: non-negotiable constraints with veto power.
//!
//! This layer defines the baselines that the system cannot violate
//! regardless of scenario, frame, or domain. Each anchor constraint
//! is queried before every decision; any violation forces Hold + alert.
//!
//! ## Mathematical Foundation
//!
//! Each anchor constraint C_i is a predicate on the decision preview space D:
//!
//!   C_i: D -> { pass, violation }
//!
//! The anchor layer produces a conjunctive report:
//!
//!   AnchorReport = AND_{i=1..5} C_i(d)
//!
//! If any C_i returns `Abort`, the entire decision is rejected.
//! If any C_i returns `DowngradeToHold`, the ternary result is overridden to Hold.

pub mod ecological_base;
pub mod flourishing_pool;
pub mod survival_motives;
pub mod thermal_baseline;
pub mod wellbeing_priority;

use serde::{Deserialize, Serialize};

use crate::core::frame::Frame;
use crate::core::value::TritValue;

// ── Core types ──────────────────────────────────────────────────

/// Severity of an anchor violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub enum AnchorSeverity {
    /// Direct rejection — no frame or domain can override.
    Abort,
    /// Downgrade to Hold — continue gathering variables.
    DowngradeToHold,
}

/// Ecosystem zone for ecological impact assessment.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum EcosystemZone {
    Terrestrial,
    Marine,
    Freshwater,
    Atmospheric,
    Polar,
    Coastal,
}

/// What the anchor layer inspects before a decision is finalized.
///
/// Contains ONLY observable expected consequences, not internal engine state.
/// This is the "decision preview" that each anchor constraint evaluates.
#[derive(Debug, Clone, PartialEq)]
pub struct DecisionPreview {
    /// Estimated energy consumption in joules.
    pub expected_energy_joules: f64,
    /// Estimated carbon equivalent emission in kg.
    pub expected_carbon_kg: f64,
    /// Population potentially affected, if estimable.
    pub affected_population: Option<u64>,
    /// Risk of irreversible change, in [0.0, 1.0].
    /// 1.0 = certainly irreversible (e.g., species extinction).
    pub irreversible_change_risk: f64,
    /// Ecosystem zone impacted, if applicable.
    pub ecosystem_impact_zone: Option<EcosystemZone>,
    /// The frame of the proposed decision.
    pub frame: Frame,
    /// The proposed trit value.
    pub trit_value: TritValue,
}

impl DecisionPreview {
    /// Create a neutral preview (all zeros, neutral frame, Hold value).
    pub fn neutral() -> Self {
        DecisionPreview {
            expected_energy_joules: 0.0,
            expected_carbon_kg: 0.0,
            affected_population: None,
            irreversible_change_risk: 0.0,
            ecosystem_impact_zone: None,
            frame: Frame::Meta,
            trit_value: TritValue::Hold,
        }
    }
}

/// A single anchor violation.
#[derive(Debug, Clone, PartialEq)]
pub struct AnchorViolation {
    /// Name of the anchor that was violated.
    pub anchor_name: String,
    /// Human-readable description of the violation.
    pub description: String,
    /// Severity of the violation.
    pub severity: AnchorSeverity,
    /// The value that triggered the violation.
    pub actual_value: f64,
    /// The threshold that was exceeded.
    pub threshold: f64,
}

/// Aggregated result of all anchor checks.
#[derive(Debug, Clone, PartialEq)]
pub struct AnchorReport {
    /// All violations found.
    pub violations: Vec<AnchorViolation>,
    /// The highest severity violation found (or None if clean).
    pub overall_severity: Option<AnchorSeverity>,
}

impl AnchorReport {
    /// Create an empty (clean) report.
    pub fn clean() -> Self {
        AnchorReport {
            violations: Vec::new(),
            overall_severity: None,
        }
    }

    /// Returns true if any Abort-level violation exists.
    pub fn has_abort(&self) -> bool {
        self.overall_severity == Some(AnchorSeverity::Abort)
    }

    /// Returns true if any violation exists (Abort or DowngradeToHold).
    pub fn has_violations(&self) -> bool {
        !self.violations.is_empty()
    }

    /// Returns true if the decision should be downgraded to Hold
    /// (DowngradeToHold violations exist but no Abort).
    pub fn should_downgrade_to_hold(&self) -> bool {
        self.overall_severity == Some(AnchorSeverity::DowngradeToHold)
    }
}

/// A single sensor's current reading for display/diagnostic purposes.
///
/// Unlike `AnchorViolation` (only produced on breach), `SensorReading` is
/// emitted regardless of pass/fail — the map popup needs to show the live
/// value vs threshold even when the anchor is healthy.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SensorReading {
    /// Sensor name, e.g. "OLR anomaly", "CO2", "BII".
    pub name: &'static str,
    /// Current sampled value (f64::NAN if the source is unavailable).
    pub value: f64,
    /// Threshold the value is checked against.
    pub threshold: f64,
    /// True if this sensor breached its threshold (or source unavailable).
    pub violated: bool,
    /// Unit suffix, e.g. "W/m2", "ppm", "pH".
    pub unit: &'static str,
}

// ── Trait ───────────────────────────────────────────────────────

/// Error type for anchor data source operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum AnchorError {
    #[error("data source unavailable: {0}")]
    Unavailable(String),
    #[error("value out of bounds: {0}")]
    OutOfBounds(String),
    #[error("configuration error: {0}")]
    Config(String),
}

use std::time::Duration;

/// Abstraction over data sources.
///
/// In MVP, implemented by `StaticSource<T>` backed by config values.
/// In future, can be replaced with real sensor streams.
pub trait DataSource<T>: Send + Sync {
    /// Sample the current value from this data source.
    fn sample(&self) -> Result<T, AnchorError>;
    /// The temporal resolution of this data source.
    fn resolution(&self) -> Duration;
}

/// A data source that returns a constant value.
/// Used in MVP when real sensors are not available.
pub struct StaticSource<T: Clone + Send + Sync> {
    value: T,
    resolution: Duration,
}

impl<T: Clone + Send + Sync> StaticSource<T> {
    pub fn new(value: T, resolution: Duration) -> Self {
        StaticSource { value, resolution }
    }
}

impl<T: Clone + Send + Sync> DataSource<T> for StaticSource<T> {
    fn sample(&self) -> Result<T, AnchorError> {
        Ok(self.value.clone())
    }

    fn resolution(&self) -> Duration {
        self.resolution
    }
}

// ── The AnchorConstraint trait ──────────────────────────────────

/// Each anchor constraint must implement this trait.
///
/// The `check` method inspects a `DecisionPreview` and returns
/// `Some(AnchorViolation)` if the constraint is violated, `None` if it passes.
pub trait AnchorConstraint: Send + Sync {
    /// Human-readable name of this constraint.
    fn name(&self) -> &'static str;
    /// The severity of violations from this constraint.
    fn severity(&self) -> AnchorSeverity;
    /// Check whether the given decision preview violates this constraint.
    fn check(&self, decision: &DecisionPreview) -> Option<AnchorViolation>;
}

// ── Report aggregation ──────────────────────────────────────────

/// Run all anchor constraints against a decision preview and produce a report.
pub fn check_all(
    constraints: &[Box<dyn AnchorConstraint>],
    decision: &DecisionPreview,
) -> AnchorReport {
    let mut report = AnchorReport::clean();

    for constraint in constraints {
        if let Some(violation) = constraint.check(decision) {
            report.overall_severity = Some(match report.overall_severity {
                None => violation.severity,
                Some(prev) if violation.severity > prev => violation.severity,
                Some(prev) => prev,
            });
            report.violations.push(violation);
        }
    }

    report
}

/// Build a [`DecisionPreview`] from a scenario input and proposed final word.
///
/// Moved from `src/sandbox/pipeline.rs` — this is anchor-layer logic,
/// not pipeline logic. In MVP, environmental impact is inferred
/// heuristically from the scenario's `EnvironmentalContext`.
pub fn build_decision_preview(
    scenario: &crate::sandbox::ScenarioInput,
    final_word: &crate::core::word::TritWord,
) -> DecisionPreview {
    let env = scenario.environmental_context.as_ref();
    let expected_energy_joules = env.map(|ctx| ctx.ambient_arousal * 1e6).unwrap_or(0.0);
    let expected_carbon_kg = env.map(|ctx| ctx.ambient_arousal * 1e3).unwrap_or(0.0);
    let affected_population = env
        .map(|ctx| (ctx.social_density * 1e6) as u64)
        .filter(|&p| p > 0);
    let irreversible_change_risk = env.map(|ctx| ctx.ambient_arousal * 0.1).unwrap_or(0.0);
    let ecosystem_impact_zone = env.and_then(|ctx| {
        if ctx.ambient_arousal > 0.7 {
            Some(crate::anchor::EcosystemZone::Atmospheric)
        } else {
            None
        }
    });

    DecisionPreview {
        expected_energy_joules,
        expected_carbon_kg,
        affected_population,
        irreversible_change_risk,
        ecosystem_impact_zone,
        frame: final_word.frame(),
        trit_value: final_word.value(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_report_is_clean() {
        let report = AnchorReport::clean();
        assert!(!report.has_violations());
        assert!(!report.has_abort());
    }

    #[test]
    fn report_with_only_downgrade() {
        let report = AnchorReport {
            violations: vec![AnchorViolation {
                anchor_name: "test".into(),
                description: "test".into(),
                severity: AnchorSeverity::DowngradeToHold,
                actual_value: 0.5,
                threshold: 0.3,
            }],
            overall_severity: Some(AnchorSeverity::DowngradeToHold),
        };
        assert!(report.has_violations());
        assert!(!report.has_abort());
        assert!(report.should_downgrade_to_hold());
    }

    #[test]
    fn report_with_abort() {
        let report = AnchorReport {
            violations: vec![AnchorViolation {
                anchor_name: "thermal".into(),
                description: "OLR exceeded".into(),
                severity: AnchorSeverity::Abort,
                actual_value: 3.0,
                threshold: 2.5,
            }],
            overall_severity: Some(AnchorSeverity::Abort),
        };
        assert!(report.has_violations());
        assert!(report.has_abort());
        assert!(!report.should_downgrade_to_hold());
    }

    #[test]
    fn static_source_returns_constant() {
        let source = StaticSource::new(42.0, Duration::from_secs(1));
        assert_eq!(source.sample().unwrap(), 42.0);
        assert_eq!(source.sample().unwrap(), 42.0);
    }

    #[test]
    fn decision_preview_neutral_is_safe() {
        let preview = DecisionPreview::neutral();
        assert_eq!(preview.expected_energy_joules, 0.0);
        assert_eq!(preview.expected_carbon_kg, 0.0);
        assert_eq!(preview.irreversible_change_risk, 0.0);
        assert_eq!(preview.trit_value, TritValue::Hold);
    }

    #[test]
    fn check_all_runs_all_constraints() {
        struct PassConstraint;
        impl AnchorConstraint for PassConstraint {
            fn name(&self) -> &'static str {
                "pass"
            }
            fn severity(&self) -> AnchorSeverity {
                AnchorSeverity::DowngradeToHold
            }
            fn check(&self, _d: &DecisionPreview) -> Option<AnchorViolation> {
                None
            }
        }

        struct FailConstraint;
        impl AnchorConstraint for FailConstraint {
            fn name(&self) -> &'static str {
                "fail"
            }
            fn severity(&self) -> AnchorSeverity {
                AnchorSeverity::Abort
            }
            fn check(&self, _d: &DecisionPreview) -> Option<AnchorViolation> {
                Some(AnchorViolation {
                    anchor_name: "fail".into(),
                    description: "always fails".into(),
                    severity: AnchorSeverity::Abort,
                    actual_value: 1.0,
                    threshold: 0.0,
                })
            }
        }

        let constraints: Vec<Box<dyn AnchorConstraint>> =
            vec![Box::new(PassConstraint), Box::new(FailConstraint)];

        let report = check_all(&constraints, &DecisionPreview::neutral());
        assert!(report.has_abort());
        assert_eq!(report.violations.len(), 1);
    }
}
