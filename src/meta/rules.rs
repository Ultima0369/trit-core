use crate::trit::TritWord;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::meta::ArbitrationResult;

// ---------------------------------------------------------------------------
// RuleLoader: external domain rule loading
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
    pub fallback: String, // "hold", "negotiate", "commit_first", "safe_fallback"
}

/// Trait for loading external domain rules from configuration files.
///
/// Implementations can load from JSON, YAML, or other formats.
/// The default `load_json` implementation is provided for convenience.
pub trait RuleLoader {
    type Error: std::fmt::Display;

    /// Load a single rule from a file path.
    fn load<P: AsRef<Path>>(path: P) -> Result<CustomRule, Self::Error>;

    /// Load a rule from a JSON string.
    fn load_json(json: &str) -> Result<CustomRule, Self::Error>;

    /// Apply a loaded rule to inputs, producing an arbitration result.
    fn apply(rule: &CustomRule, inputs: &[TritWord]) -> ArbitrationResult {
        // Check for priority frame match
        if let Some(ref pf) = rule.priority_frame {
            if let Ok(frame) = pf.parse::<crate::Frame>() {
                if let Some(t) = inputs.iter().find(|w| w.frame == frame) {
                    return if rule.allow_forced_collapse {
                        ArbitrationResult::Commit(t.clone())
                    } else {
                        ArbitrationResult::Preserve(t.clone())
                    };
                }
            }
        }

        // Fallback behavior
        match rule.fallback.as_str() {
            "hold" => ArbitrationResult::Hold,
            "commit_first" => {
                if let Some(first) = inputs.first() {
                    ArbitrationResult::Commit(first.clone())
                } else {
                    ArbitrationResult::Hold
                }
            }
            "safe_fallback" => ArbitrationResult::ForceCollapse,
            _ => ArbitrationResult::Negotiate,
        }
    }
}

/// Default RuleLoader implementation using serde_json.
pub struct JsonRuleLoader;

impl RuleLoader for JsonRuleLoader {
    type Error = String;

    fn load<P: AsRef<Path>>(path: P) -> Result<CustomRule, Self::Error> {
        let raw = std::fs::read_to_string(path.as_ref()).map_err(|e| {
            format!(
                "Failed to read rule file '{}': {}",
                path.as_ref().display(),
                e
            )
        })?;
        Self::load_json(&raw)
    }

    fn load_json(json: &str) -> Result<CustomRule, Self::Error> {
        serde_json::from_str::<CustomRule>(json)
            .map_err(|e| format!("Failed to parse custom rule: {}", e))
    }
}
