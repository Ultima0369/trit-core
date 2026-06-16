use serde::{Deserialize, Serialize};

/// Scenario input for sandbox testing.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScenarioInput {
    pub id: String,
    pub description: String,
    pub domain: String,
    pub signals: Vec<SignalInput>,
    pub expected_behavior: String, // "hold", "commit_true", "commit_false", "negotiate"
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SignalInput {
    pub frame: String,
    pub value: i8, // 1, 0, -1
    pub phase: f64,
}

/// Sandbox output record.
#[derive(Debug, Clone, Serialize)]
pub struct SandboxOutput {
    pub scenario_id: String,
    pub final_value: i8,
    pub final_frame: String,
    pub final_phase: f64,
    pub interrupts: Vec<String>,
    pub policy_action: String,
}
