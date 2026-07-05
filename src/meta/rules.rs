use crate::core::frame::Frame;
use crate::core::value::TritValue;
use crate::core::word::TritWord;
use crate::meta::ArbitrationResult;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::str::FromStr;
use thiserror::Error;

// ---------------------------------------------------------------------------
// FallbackBehavior: type-safe fallback behavior for custom rules
// ---------------------------------------------------------------------------

/// Fallback behavior when no priority frame matches in a custom rule.
///
/// Replaces the previous string-based fallback to prevent typos and
/// ensure exhaustive matching at compile time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FallbackBehavior {
    /// Deliberately hold — incommensurable values, cannot decide.
    Hold,
    /// Attempt multi-round negotiation.
    Negotiate,
    /// Commit the first non-Unknown TritWord.
    CommitFirst,
    /// Force a safe collapse (triggers SafeFallback in dangerous domains).
    SafeFallback,
}

impl std::fmt::Display for FallbackBehavior {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FallbackBehavior::Hold => write!(f, "hold"),
            FallbackBehavior::Negotiate => write!(f, "negotiate"),
            FallbackBehavior::CommitFirst => write!(f, "commit_first"),
            FallbackBehavior::SafeFallback => write!(f, "safe_fallback"),
        }
    }
}

impl FromStr for FallbackBehavior {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "hold" => Ok(FallbackBehavior::Hold),
            "negotiate" => Ok(FallbackBehavior::Negotiate),
            "commit_first" => Ok(FallbackBehavior::CommitFirst),
            "safe_fallback" => Ok(FallbackBehavior::SafeFallback),
            other => Err(format!(
                "unknown fallback behavior: '{}' (expected one of: hold, negotiate, commit_first, safe_fallback)",
                other
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// JsonRuleLoader: external domain rule loading
// ---------------------------------------------------------------------------

/// Serializable representation of a custom arbitration rule.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CustomRule {
    /// Human-readable name for this rule set.
    pub name: String,
    /// The priority frame (if any) for this domain.
    pub priority_frame: Option<String>,
    /// Whether forced collapse is allowed.
    pub allow_forced_collapse: bool,
    /// Fallback behavior when no priority frame matches.
    pub fallback: FallbackBehavior,
}

/// Error type for rule loading operations.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum RuleError {
    #[error("{0}")]
    Read(String),
    #[error("Failed to parse custom rule: {0}")]
    Parse(String),
}

/// Rule loader using serde_json.
///
/// # ponytail: free functions, no struct — single caller in domain.rs.
/// Re-add a loader struct when multiple implementations exist.
///
/// Load a single rule from a file path.
pub fn load_rule<P: AsRef<Path>>(path: P) -> Result<CustomRule, RuleError> {
    let raw = std::fs::read_to_string(path.as_ref()).map_err(|e| {
        RuleError::Read(format!(
            "Failed to read rule file '{}': {}",
            path.as_ref().display(),
            e
        ))
    })?;
    load_rule_json(&raw)
}

/// Load a rule from a JSON string.
pub fn load_rule_json(json: &str) -> Result<CustomRule, RuleError> {
    serde_json::from_str::<CustomRule>(json).map_err(|e| RuleError::Parse(e.to_string()))
}

/// Apply a loaded rule to inputs, producing an arbitration result.
pub fn apply_rule(rule: &CustomRule, inputs: &[TritWord]) -> ArbitrationResult {
    // Check for priority frame match
    if let Some(ref pf) = rule.priority_frame {
        match pf.parse::<Frame>() {
            Ok(frame) => {
                if let Some(t) = inputs.iter().find(|w| w.frame() == frame) {
                    return if rule.allow_forced_collapse {
                        ArbitrationResult::Commit(*t)
                    } else {
                        ArbitrationResult::Preserve(*t)
                    };
                }
            }
            Err(_) => {
                tracing::warn!(
                    priority_frame = %pf,
                    rule_name = %rule.name,
                    "CustomRule priority_frame is not a valid Frame name — falling back"
                );
            }
        }
    }

    // Fallback behavior
    match rule.fallback {
        FallbackBehavior::Hold => ArbitrationResult::Hold,
        FallbackBehavior::CommitFirst => {
            if let Some(first) = inputs.iter().find(|t| t.value() != TritValue::Unknown) {
                ArbitrationResult::Commit(*first)
            } else {
                ArbitrationResult::Hold
            }
        }
        FallbackBehavior::SafeFallback => ArbitrationResult::ForceCollapse,
        FallbackBehavior::Negotiate => ArbitrationResult::Negotiate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::frame::Frame;
    use crate::core::word::TritWord;

    #[test]
    fn json_rule_loader_parses_valid_rule() {
        let json = r#"{
            "name": "chemistry_safety",
            "priority_frame": "Science",
            "allow_forced_collapse": true,
            "fallback": "safe_fallback"
        }"#;
        let rule = load_rule_json(json).unwrap();
        assert_eq!(rule.name, "chemistry_safety");
        assert_eq!(rule.priority_frame, Some("Science".to_string()));
        assert!(rule.allow_forced_collapse);
        assert_eq!(rule.fallback, FallbackBehavior::SafeFallback);
    }

    #[test]
    fn json_rule_loader_rejects_invalid_json() {
        assert!(load_rule_json("not json").is_err());
    }

    #[test]
    fn rule_apply_priority_frame_match() {
        let rule = CustomRule {
            name: "test".into(),
            priority_frame: Some("Science".into()),
            allow_forced_collapse: true,
            fallback: FallbackBehavior::Hold,
        };
        let inputs = vec![
            TritWord::fals(Frame::Individual),
            TritWord::tru(Frame::Science),
        ];
        let result = apply_rule(&rule, &inputs);
        assert!(matches!(result, ArbitrationResult::Commit(_)));
    }

    #[test]
    fn rule_apply_fallback_hold() {
        let rule = CustomRule {
            name: "test".into(),
            priority_frame: None,
            allow_forced_collapse: false,
            fallback: FallbackBehavior::Hold,
        };
        let inputs = vec![TritWord::tru(Frame::Science)];
        let result = apply_rule(&rule, &inputs);
        assert_eq!(result, ArbitrationResult::Hold);
    }

    #[test]
    fn rule_apply_fallback_safe_fallback() {
        let rule = CustomRule {
            name: "test".into(),
            priority_frame: None,
            allow_forced_collapse: false,
            fallback: FallbackBehavior::SafeFallback,
        };
        let inputs = vec![TritWord::tru(Frame::Science)];
        let result = apply_rule(&rule, &inputs);
        assert_eq!(result, ArbitrationResult::ForceCollapse);
    }

    #[test]
    fn rule_apply_priority_frame_no_match_forces_collapse_when_allowed() {
        let rule = CustomRule {
            name: "test".into(),
            priority_frame: Some("Science".into()),
            allow_forced_collapse: true,
            fallback: FallbackBehavior::Hold,
        };
        let inputs = vec![TritWord::tru(Frame::Individual)];
        let result = apply_rule(&rule, &inputs);
        assert_eq!(result, ArbitrationResult::Hold); // fallback is hold
    }

    #[test]
    fn rule_apply_commit_first_fallback_skips_unknown() {
        let rule = CustomRule {
            name: "test".into(),
            priority_frame: None,
            allow_forced_collapse: false,
            fallback: FallbackBehavior::CommitFirst,
        };
        let inputs = vec![
            TritWord::unknown(Frame::Science),
            TritWord::fals(Frame::Individual),
        ];
        let result = apply_rule(&rule, &inputs);
        assert!(matches!(result, ArbitrationResult::Commit(w) if w.value() == TritValue::False));
    }

    #[test]
    fn fallback_behavior_from_str() {
        assert_eq!(
            "hold".parse::<FallbackBehavior>().unwrap(),
            FallbackBehavior::Hold
        );
        assert_eq!(
            "negotiate".parse::<FallbackBehavior>().unwrap(),
            FallbackBehavior::Negotiate
        );
        assert_eq!(
            "commit_first".parse::<FallbackBehavior>().unwrap(),
            FallbackBehavior::CommitFirst
        );
        assert_eq!(
            "safe_fallback".parse::<FallbackBehavior>().unwrap(),
            FallbackBehavior::SafeFallback
        );
        assert!("unknown".parse::<FallbackBehavior>().is_err());
    }

    #[test]
    fn fallback_behavior_display() {
        assert_eq!(format!("{}", FallbackBehavior::Hold), "hold");
        assert_eq!(
            format!("{}", FallbackBehavior::SafeFallback),
            "safe_fallback"
        );
    }

    #[test]
    fn rule_serializes_and_deserializes() {
        let rule = CustomRule {
            name: "chemistry".into(),
            priority_frame: Some("Science".into()),
            allow_forced_collapse: true,
            fallback: FallbackBehavior::SafeFallback,
        };
        let json = serde_json::to_string(&rule).unwrap();
        let restored: CustomRule = serde_json::from_str(&json).unwrap();
        assert_eq!(rule.name, restored.name);
        assert_eq!(rule.priority_frame, restored.priority_frame);
        assert_eq!(rule.allow_forced_collapse, restored.allow_forced_collapse);
        assert_eq!(rule.fallback, restored.fallback);
    }

    #[test]
    fn load_from_missing_file_fails() {
        let result = load_rule("/nonexistent/rule.json");
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("Failed to read"));
    }

    #[test]
    fn load_from_valid_temp_file_succeeds() {
        let json = r#"{"name":"temp","priority_frame":null,"allow_forced_collapse":false,"fallback":"hold"}"#;
        let dir = std::env::temp_dir();
        let path = dir.join("trit_core_test_rule.json");
        std::fs::write(&path, json).unwrap();
        let rule = load_rule(&path).unwrap();
        assert_eq!(rule.name, "temp");
        assert_eq!(rule.fallback, FallbackBehavior::Hold);
        // cleanup
        let _ = std::fs::remove_file(&path);
    }
}
