use serde::{Deserialize, Serialize};

use crate::core::sensor::{EnvironmentalContext, SensorSignal};

/// Scenario input for sandbox testing.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScenarioInput {
    pub id: String,
    pub description: String,
    pub domain: String,
    pub signals: Vec<SignalInput>,
    /// Expected high-level behavior: "hold", "commit_true", "commit_false", "negotiate".
    pub expected_behavior: String,
    /// Optional environmental context that persists across the scenario.
    #[serde(default)]
    pub environmental_context: Option<EnvironmentalContext>,
}

/// A single input signal within a scenario.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SignalInput {
    pub frame: String,
    pub value: i8, // 1, 0, -1
    pub phase: f64,
    /// Optional multi-modal sensor signal attached to this frame/value/phase.
    #[serde(default)]
    pub sensor: Option<SensorSignal>,
}
