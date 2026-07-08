//! Domain classification — which knowledge domain a decision operates in.
//!
//! The `Domain` enum classifies the epistemic context of a ternary decision.
//! Policy logic (how each domain arbitrates conflicts) lives in
//! [`crate::meta::ResolutionPolicy`] — this module is pure classification.
//!
//! Moved from `meta::domain` in the Layer Dependency Cleanup (2026-07-08):
//! `Domain` is a core type consumed by `hold`, `safe_fallback`, and
//! `calibration` — it doesn't belong above `core` in the dependency stack.

use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;

/// Domain rules for conflict resolution.
/// Each domain defines which frame has priority and whether
/// forced resolution (hard collapse) is allowed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Domain {
    /// Hard science constraints: Science priority, forced collapse.
    Physical,
    /// Applied constraints: Science priority, forced collapse.
    Engineering,
    /// Soft constraints: Individual priority, no forced collapse.
    MedicalEthics,
    /// Incommensurable: no priority, must remain Hold.
    ValueJudgment,
    /// Default: attempt negotiation.
    General,
    /// Externally loaded domain rules.
    Custom(String),
    /// Organizational decision: multi-frame negotiation across roles and processes.
    Organizational,
    /// Relational decision: prioritize the relational frame when present.
    Relational,
    /// Cognitive decision: prioritize embodied signals over abstractions.
    Cognitive,
    /// Environmental adaptation: prioritize geo-ecological frame when present.
    Environmental,
    /// Climate science: Instrumental priority, multi-source Science → Hold.
    ///
    /// Climate decisions involve physical measurements (CO2 ppm, temperature
    /// anomalies, ice coverage) that are Instrumental — not Science (theory).
    /// When instrumental measurements conflict with scientific models, the
    /// measurement takes priority. When multiple scientific sources disagree
    /// and no instrumental measurement resolves the conflict → Hold.
    Climate,
}

/// Error returned when a string cannot be parsed as a [`Domain`].
#[derive(Debug, Clone, PartialEq, Error)]
pub enum DomainParseError {
    #[error("unknown domain: '{0}'")]
    Unknown(String),
}

impl FromStr for Domain {
    type Err = DomainParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Physical" => Ok(Domain::Physical),
            "Engineering" => Ok(Domain::Engineering),
            "MedicalEthics" => Ok(Domain::MedicalEthics),
            "ValueJudgment" => Ok(Domain::ValueJudgment),
            "General" => Ok(Domain::General),
            "Organizational" => Ok(Domain::Organizational),
            "Relational" => Ok(Domain::Relational),
            "Cognitive" => Ok(Domain::Cognitive),
            "Environmental" => Ok(Domain::Environmental),
            "Climate" => Ok(Domain::Climate),
            d if d.starts_with("Custom(") => {
                let name = d
                    .strip_prefix("Custom(")
                    .and_then(|s| s.strip_suffix(")"))
                    .unwrap_or("");
                if name.is_empty() {
                    return Err(DomainParseError::Unknown(
                        "Custom domain name cannot be empty".to_string(),
                    ));
                }
                Ok(Domain::Custom(name.to_string()))
            }
            d => Err(DomainParseError::Unknown(d.to_string())),
        }
    }
}

impl std::fmt::Display for Domain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Domain::Physical => write!(f, "Physical"),
            Domain::Engineering => write!(f, "Engineering"),
            Domain::MedicalEthics => write!(f, "MedicalEthics"),
            Domain::ValueJudgment => write!(f, "ValueJudgment"),
            Domain::General => write!(f, "General"),
            Domain::Custom(name) => write!(f, "Custom({})", name),
            Domain::Organizational => write!(f, "Organizational"),
            Domain::Relational => write!(f, "Relational"),
            Domain::Cognitive => write!(f, "Cognitive"),
            Domain::Environmental => write!(f, "Environmental"),
            Domain::Climate => write!(f, "Climate"),
        }
    }
}
