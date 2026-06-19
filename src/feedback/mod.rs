//! Output and feedback loop layer.
//!
//! Closes the loop: every decision output is tested against reality
//! (or its best available proxy). Consequences are classified and
//! deviations/errors trigger correction, not just recording.
//!
//! ## Mathematical Foundation
//!
//! The feedback loop implements a corrective control law:
//!
//!   delta = ||f(o) - f(c)||_2
//!
//! where f maps to the feature space. Correction triggers when:
//!
//!   delta > tau_correction

pub mod consequence_review;
pub mod correction;
pub mod experience_recorder;
pub mod practice_test;

use crate::anchor::AnchorViolation;
use crate::hook::ScenarioType;

/// Result of testing a decision against observed consequences.
#[derive(Debug, Clone, PartialEq)]
pub enum PracticeTestResult {
    /// Consequences matched expectations.
    Matched { confidence: f64 },
    /// Consequences deviated from expectations — correction suggested.
    Deviated {
        /// The magnitude of deviation (L2 norm difference).
        delta: f64,
        /// Hint for how to correct.
        correction: CorrectionHint,
    },
    /// Consequences were erroneous — immediate correction required.
    Erroneous {
        /// Human-readable reason.
        reason: String,
        /// Severity of the correction needed.
        severity: CorrectionSeverity,
    },
}

/// Hint for correcting a deviation.
#[derive(Debug, Clone, PartialEq)]
pub enum CorrectionHint {
    /// Adjust module parameters toward the given direction.
    AdjustParameters { module_id: String, delta: f64 },
    /// Re-mount a different set of modules.
    RemountModules { recommended_scenario: ScenarioType },
    /// Re-enter the decision pipeline with additional input.
    ReEnterPipeline { additional_input: String },
    /// Escalate to external oversight.
    EscalateToOversight { reason: String },
}

/// Severity of a correction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CorrectionSeverity {
    /// Minor adjustment — proceed normally.
    Minor,
    /// Significant deviation — review required.
    Significant,
    /// Critical error — immediate intervention required.
    Critical,
}

/// Signal sent from Layer 5 back to Layer 2 (Hook Manager).
#[derive(Debug, Clone, PartialEq)]
pub struct FeedbackSignal {
    /// The practice test result.
    pub test_result: PracticeTestResult,
    /// ID of the decision that was tested.
    pub source_decision_id: String,
    /// Recommended scenario type for re-processing, if applicable.
    pub recommended_scenario: Option<ScenarioType>,
    /// Any anchor violations discovered during testing.
    pub anchor_violations: Vec<AnchorViolation>,
}

/// Abstraction over consequence prediction.
///
/// In MVP, implemented by `StaticRuleModel` using hand-coded consequence rules.
/// In future, can be replaced with realistic simulators or real-world outcome data.
pub trait ProxyEnvironment: Send + Sync {
    /// Predict the expected consequence of a decision.
    /// Returns None if the decision falls outside the proxy's modeling range.
    fn predict(&self, decision: &crate::sandbox::SandboxOutput) -> Option<ConsequencePrediction>;

    /// The confidence of this proxy's predictions, in [0.0, 1.0].
    fn confidence(&self) -> f64;

    /// Human-readable name of the proxy.
    fn name(&self) -> &'static str;
}

/// A predicted consequence from a proxy environment.
#[derive(Debug, Clone, PartialEq)]
pub struct ConsequencePrediction {
    /// Predicted energy impact in joules.
    pub energy_joules: f64,
    /// Predicted carbon impact in kg.
    pub carbon_kg: f64,
    /// Predicted affected population.
    pub affected_population: Option<u64>,
    /// Predicted irreversible change risk [0.0, 1.0].
    pub irreversible_risk: f64,
    /// Confidence of this specific prediction [0.0, 1.0].
    pub confidence: f64,
}

/// MVP implementation: static rule-based consequence model.
///
/// Uses hand-coded rules mapping decision patterns to expected consequences.
pub struct StaticRuleModel {
    #[allow(dead_code)]
    name: String,
    confidence: f64,
}

impl StaticRuleModel {
    pub fn new() -> Self {
        StaticRuleModel {
            name: "StaticRuleModel".to_string(),
            confidence: 0.6,
        }
    }
}

impl Default for StaticRuleModel {
    fn default() -> Self {
        Self::new()
    }
}

impl ProxyEnvironment for StaticRuleModel {
    fn predict(&self, decision: &crate::sandbox::SandboxOutput) -> Option<ConsequencePrediction> {
        // Simple heuristic rules:
        // - True decisions: assume moderate energy/carbon impact
        // - Hold decisions: assume near-zero impact (no action taken)
        // - False decisions: assume negative energy impact (prevention)
        let (energy, carbon) = match decision.final_value_code {
            1 => (1000.0, 0.1),    // True: moderate impact
            0 => (0.0, 0.0),       // Hold: no action
            -1 => (-500.0, -0.05), // False: prevention
            _ => return None,
        };

        Some(ConsequencePrediction {
            energy_joules: energy,
            carbon_kg: carbon,
            affected_population: None,
            irreversible_risk: if decision.final_value_code == 1 {
                0.01
            } else {
                0.0
            },
            confidence: self.confidence,
        })
    }

    fn confidence(&self) -> f64 {
        self.confidence
    }

    fn name(&self) -> &'static str {
        "StaticRuleModel"
    }
}
