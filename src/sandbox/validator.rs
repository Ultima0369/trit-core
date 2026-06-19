use crate::sandbox::error::SandboxError;
use crate::sandbox::output::SandboxOutput;

/// Validate that a sandbox output matches its declared expected behavior.
#[derive(Debug, Clone, Default)]
pub struct ScenarioValidator;

impl ScenarioValidator {
    /// Check whether the output satisfies the expected behavior string.
    ///
    /// Supported expected behaviors:
    /// - `"hold"` → final value code must be 0.
    /// - `"commit_true"` → final value code must be 1.
    /// - `"commit_false"` → final value code must be -1.
    /// - `"negotiate"` → policy action must contain `Negotiate`.
    pub fn validate(output: &SandboxOutput, expected_behavior: &str) -> Result<(), SandboxError> {
        match expected_behavior {
            "hold" => {
                if !output.is_hold() {
                    return Err(SandboxError::ExpectedBehaviorMismatch {
                        expected: expected_behavior.into(),
                        got: output.final_value.clone(),
                    });
                }
            }
            "commit_true" => {
                if !output.is_commit_true() {
                    return Err(SandboxError::ExpectedBehaviorMismatch {
                        expected: expected_behavior.into(),
                        got: output.final_value.clone(),
                    });
                }
            }
            "commit_false" => {
                if !output.is_commit_false() {
                    return Err(SandboxError::ExpectedBehaviorMismatch {
                        expected: expected_behavior.into(),
                        got: output.final_value.clone(),
                    });
                }
            }
            "negotiate" => {
                if !output.policy_action.contains("Negotiate") {
                    return Err(SandboxError::ExpectedBehaviorMismatch {
                        expected: expected_behavior.into(),
                        got: output.policy_action.clone(),
                    });
                }
            }
            other => {
                return Err(SandboxError::InvalidScenario(format!(
                    "unknown expected_behavior: '{}'",
                    other
                )))
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output(value_code: i8, policy_action: &str) -> SandboxOutput {
        SandboxOutput {
            scenario_id: "x".into(),
            final_value: match value_code {
                1 => "True".into(),
                -1 => "False".into(),
                _ => "Hold".into(),
            },
            final_value_code: value_code,
            final_frame: "Meta".into(),
            final_phase_raw: 0.5,
            interrupts: vec![],
            policy_action: policy_action.into(),
            reflexive_alert: None,
            attention_cmd: None,
            receiver_estimate: None,
            hold_state: None,
        }
    }

    #[test]
    fn validate_hold_ok() {
        ScenarioValidator::validate(&output(0, "Hold"), "hold").unwrap();
    }

    #[test]
    fn validate_commit_true_ok() {
        ScenarioValidator::validate(&output(1, "Commit"), "commit_true").unwrap();
    }

    #[test]
    fn validate_commit_false_ok() {
        ScenarioValidator::validate(&output(-1, "Commit"), "commit_false").unwrap();
    }

    #[test]
    fn validate_negotiate_ok() {
        ScenarioValidator::validate(&output(0, "Negotiate"), "negotiate").unwrap();
    }

    #[test]
    fn validate_mismatch() {
        assert!(ScenarioValidator::validate(&output(1, "Commit"), "hold").is_err());
    }
}
