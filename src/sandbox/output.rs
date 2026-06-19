use crate::core::hold::HoldState;
use crate::core::phase::Phase;
use crate::core::value::TritValue;
use crate::knowledge::ReceiverEstimate;
use serde::{Deserialize, Deserializer, Serialize};

/// Sandbox output record.
///
/// All fields are validated on deserialization: `final_phase_raw` must be
/// finite and within `[0.0, 1.0]`, and `final_value_code` must be one
/// of `{1, 0, -1}`.
///
/// Use [`final_phase()`](SandboxOutput::final_phase) and
/// [`final_trit_value()`](SandboxOutput::final_trit_value) for typed access.
#[derive(Debug, Clone, Serialize)]
pub struct SandboxOutput {
    pub scenario_id: String,
    /// The final decision state as a human-readable string.
    /// One of: "True", "Hold", "False", "Unknown".
    pub final_value: String,
    /// Numeric representation of the final value (1=True, 0=Hold/Unknown, -1=False).
    pub final_value_code: i8,
    pub final_frame: String,
    /// The final phase tendency. Always finite and in `[0.0, 1.0]`.
    #[serde(rename = "final_phase")]
    pub final_phase_raw: f64,
    pub interrupts: Vec<String>,
    pub policy_action: String,
    /// Optional alert from the reflexive guard.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reflexive_alert: Option<String>,
    /// Optional attention scheduler command.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attention_cmd: Option<String>,
    /// Optional receiver-state estimate from self-knowledge.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receiver_estimate: Option<ReceiverEstimate>,
    /// Optional Hold state metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hold_state: Option<HoldState>,
}

impl SandboxOutput {
    /// Returns true if the final value represents a committed True decision.
    pub fn is_commit_true(&self) -> bool {
        self.final_value_code == 1
    }

    /// Returns true if the final value represents a committed False decision.
    pub fn is_commit_false(&self) -> bool {
        self.final_value_code == -1
    }

    /// Returns true if the final value represents Hold (including Unknown).
    pub fn is_hold(&self) -> bool {
        self.final_value_code == 0
    }

    /// The final phase as a typed [`Phase`] value.
    pub fn final_phase(&self) -> Phase {
        Phase::new_clamped(self.final_phase_raw)
    }

    /// The final trit value as a typed [`TritValue`].
    pub fn final_trit_value(&self) -> TritValue {
        TritValue::from(self.final_value_code)
    }
}

/// Custom deserialization with validation for `final_phase` and `final_value_code`.
impl<'de> Deserialize<'de> for SandboxOutput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct SandboxOutputRaw {
            scenario_id: String,
            final_value: String,
            final_value_code: i8,
            final_frame: String,
            #[serde(rename = "final_phase")]
            final_phase_raw: f64,
            interrupts: Vec<String>,
            policy_action: String,
            #[serde(default)]
            reflexive_alert: Option<String>,
            #[serde(default)]
            attention_cmd: Option<String>,
            #[serde(default)]
            receiver_estimate: Option<ReceiverEstimate>,
            #[serde(default)]
            hold_state: Option<HoldState>,
        }

        let raw = SandboxOutputRaw::deserialize(deserializer)?;

        // Validate final_phase_raw: must be finite and in [0.0, 1.0]
        if raw.final_phase_raw.is_nan() || raw.final_phase_raw.is_infinite() {
            return Err(serde::de::Error::custom(format!(
                "final_phase_raw must be finite, got: {}",
                raw.final_phase_raw
            )));
        }
        if !(0.0..=1.0).contains(&raw.final_phase_raw) {
            return Err(serde::de::Error::custom(format!(
                "final_phase_raw must be in [0.0, 1.0], got: {}",
                raw.final_phase_raw
            )));
        }

        // Validate final_value_code: must be 1, 0, or -1
        if !matches!(raw.final_value_code, -1..=1) {
            return Err(serde::de::Error::custom(format!(
                "final_value_code must be -1, 0, or 1, got: {}",
                raw.final_value_code
            )));
        }

        Ok(SandboxOutput {
            scenario_id: raw.scenario_id,
            final_value: raw.final_value,
            final_value_code: raw.final_value_code,
            final_frame: raw.final_frame,
            final_phase_raw: raw.final_phase_raw,
            interrupts: raw.interrupts,
            policy_action: raw.policy_action,
            reflexive_alert: raw.reflexive_alert,
            attention_cmd: raw.attention_cmd,
            receiver_estimate: raw.receiver_estimate,
            hold_state: raw.hold_state,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_valid_output() {
        let json = r#"{
            "scenario_id": "test",
            "final_value": "True",
            "final_value_code": 1,
            "final_frame": "Science",
            "final_phase": 0.8,
            "interrupts": [],
            "policy_action": "Commit"
        }"#;
        let output: SandboxOutput = serde_json::from_str(json).unwrap();
        assert_eq!(output.final_value_code, 1);
        assert!((output.final_phase_raw - 0.8).abs() < f64::EPSILON);
        assert!((output.final_phase().inner() - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn deserialize_rejects_nan_phase() {
        let json = r#"{
            "scenario_id": "test",
            "final_value": "Hold",
            "final_value_code": 0,
            "final_frame": "Meta",
            "final_phase": NaN,
            "interrupts": [],
            "policy_action": "Hold"
        }"#;
        // NaN is not valid JSON, but let's test with a post-deserialization check
        // by constructing manually
        let result = serde_json::from_str::<SandboxOutput>(json);
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_rejects_out_of_range_phase() {
        let json = r#"{
            "scenario_id": "test",
            "final_value": "Hold",
            "final_value_code": 0,
            "final_frame": "Meta",
            "final_phase": 1.5,
            "interrupts": [],
            "policy_action": "Hold"
        }"#;
        let result = serde_json::from_str::<SandboxOutput>(json);
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_rejects_invalid_value_code() {
        let json = r#"{
            "scenario_id": "test",
            "final_value": "Hold",
            "final_value_code": 5,
            "final_frame": "Meta",
            "final_phase": 0.5,
            "interrupts": [],
            "policy_action": "Hold"
        }"#;
        let result = serde_json::from_str::<SandboxOutput>(json);
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_rejects_negative_phase() {
        let json = r#"{
            "scenario_id": "test",
            "final_value": "Hold",
            "final_value_code": 0,
            "final_frame": "Meta",
            "final_phase": -0.1,
            "interrupts": [],
            "policy_action": "Hold"
        }"#;
        let result = serde_json::from_str::<SandboxOutput>(json);
        assert!(result.is_err());
    }
}
