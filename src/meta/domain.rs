use crate::core::frame::Frame;
use crate::core::word::TritWord;
use crate::meta::frame_mask::FrameMask;
use crate::meta::rules::{CustomRule, JsonRuleLoader, RuleLoader};
use std::str::FromStr;
use thiserror::Error;
use tracing::{debug, info};

/// Domain rules for conflict resolution.
/// Each domain defines which frame has priority and whether
/// forced resolution (hard collapse) is allowed.
#[derive(Clone, Debug, PartialEq, Eq)]
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
}

/// Error type for policy arbitration failures.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum PolicyError {
    #[error("no inputs provided for arbitration")]
    EmptyInputs,
    #[error("custom rule failed to apply: {0}")]
    CustomRule(String),
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
        }
    }
}

/// Policy engine that decides how to resolve conflicts.
#[derive(Debug, Clone)]
pub struct ResolutionPolicy {
    pub domain: Domain,
    /// Optional externally loaded rule for `Domain::Custom`.
    pub custom_rule: Option<CustomRule>,
}

impl ResolutionPolicy {
    pub fn new(domain: Domain) -> Self {
        debug!(?domain, "ResolutionPolicy created");
        Self {
            domain,
            custom_rule: None,
        }
    }

    /// Attach a custom rule to this policy.
    pub fn with_custom_rule(mut self, rule: CustomRule) -> Self {
        self.custom_rule = Some(rule);
        self
    }

    /// Given conflicting inputs, return the arbitration result.
    /// Uses FrameMask for O(1) frame presence checks.
    ///
    /// # Errors
    ///
    /// Returns `PolicyError::EmptyInputs` if `inputs` is empty.
    #[tracing::instrument(skip_all, fields(domain = ?self.domain))]
    pub fn arbitrate(&self, inputs: &[TritWord]) -> Result<ArbitrationResult, PolicyError> {
        debug!(input_count = inputs.len(), "arbitration started");
        if inputs.is_empty() {
            return Err(PolicyError::EmptyInputs);
        }
        let mask = FrameMask::from_inputs(inputs);

        // Mind-engineering default: first-person experience is preserved when
        // it conflicts with statistical/scientific frames. This implements the
        // principle that lived experience should not be silently overridden by
        // population aggregates.
        if mask.has(&Frame::FirstPerson) && mask.has(&Frame::Science) {
            return Ok(Self::preserve_frame(inputs, Frame::FirstPerson));
        }

        let result = match &self.domain {
            Domain::Physical | Domain::Engineering => {
                self.arbitrate_physical_engineering(inputs, &mask)
            }
            Domain::MedicalEthics => self.arbitrate_medical_ethics(inputs, &mask),
            Domain::ValueJudgment => ArbitrationResult::Hold,
            Domain::Custom(name) => self.arbitrate_custom(name, inputs, &mask),
            Domain::General => self.arbitrate_general(inputs, &mask),
        };
        info!(?result, "arbitration completed");
        Ok(result)
    }

    /// Physical/Engineering: Science frame priority, force collapse when absent.
    fn arbitrate_physical_engineering(
        &self,
        inputs: &[TritWord],
        mask: &FrameMask,
    ) -> ArbitrationResult {
        if mask.has(&Frame::Science) {
            Self::commit_frame(inputs, Frame::Science)
        } else {
            ArbitrationResult::ForceCollapse
        }
    }

    /// MedicalEthics: Individual frame priority, negotiate when absent.
    fn arbitrate_medical_ethics(&self, inputs: &[TritWord], mask: &FrameMask) -> ArbitrationResult {
        if mask.has(&Frame::Individual) {
            Self::preserve_frame(inputs, Frame::Individual)
        } else {
            ArbitrationResult::Negotiate
        }
    }

    /// Custom domain: delegate to loaded rule, or default to Negotiate.
    fn arbitrate_custom(
        &self,
        name: &str,
        inputs: &[TritWord],
        _mask: &FrameMask,
    ) -> ArbitrationResult {
        if let Some(ref rule) = self.custom_rule {
            info!(custom_domain = %name, rule = %rule.name, "custom domain arbitration using loaded rule");
            return JsonRuleLoader::apply(rule, inputs);
        }
        info!(custom_domain = %name, "custom domain arbitration: no rule loaded, defaulting to Negotiate");
        ArbitrationResult::Negotiate
    }

    /// General: commit when single frame, negotiate when mixed.
    fn arbitrate_general(&self, inputs: &[TritWord], mask: &FrameMask) -> ArbitrationResult {
        if mask.count() == 1 {
            ArbitrationResult::Commit(inputs[0])
        } else {
            ArbitrationResult::Negotiate
        }
    }

    /// Find and commit the input with the given frame.
    fn commit_frame(inputs: &[TritWord], frame: Frame) -> ArbitrationResult {
        inputs
            .iter()
            .find(|t| t.frame() == frame)
            .cloned()
            .map(ArbitrationResult::Commit)
            .unwrap_or(ArbitrationResult::Hold)
    }

    /// Find and preserve the input with the given frame.
    fn preserve_frame(inputs: &[TritWord], frame: Frame) -> ArbitrationResult {
        inputs
            .iter()
            .find(|t| t.frame() == frame)
            .cloned()
            .map(ArbitrationResult::Preserve)
            .unwrap_or(ArbitrationResult::Hold)
    }
}

impl Default for ResolutionPolicy {
    fn default() -> Self {
        Self::new(Domain::General)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ArbitrationResult {
    /// Commit to a specific TritWord as the final decision.
    Commit(TritWord),
    /// Preserve a TritWord (MedicalEthics: Individual frame).
    Preserve(TritWord),
    /// Force a safe collapse. When this is returned from arbitrate(),
    /// the caller should invoke SafeFallback::guard() to determine the
    /// final value — in dangerous domains (Physical, Engineering,
    /// chemistry, genetics, etc.) this will force False when interrupts
    /// are present, implementing IEC 61508 fail-safe semantics.
    ForceCollapse,
    /// Deliberately hold — incommensurable values, cannot decide.
    Hold,
    /// Attempt multi-round negotiation (General domain with mixed frames).
    Negotiate,
}

impl std::fmt::Display for ArbitrationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArbitrationResult::Commit(w) => {
                write!(
                    f,
                    "Commit({:?}, phase={:.3}, {})",
                    w.value(),
                    w.phase().inner(),
                    w.frame()
                )
            }
            ArbitrationResult::Preserve(w) => {
                write!(
                    f,
                    "Preserve({:?}, phase={:.3}, {})",
                    w.value(),
                    w.phase().inner(),
                    w.frame()
                )
            }
            ArbitrationResult::ForceCollapse => write!(f, "ForceCollapse"),
            ArbitrationResult::Hold => write!(f, "Hold"),
            ArbitrationResult::Negotiate => write!(f, "Negotiate"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::frame::Frame;
    use crate::core::word::TritWord;
    use crate::meta::rules::FallbackBehavior;

    #[test]
    fn physical_commits_science() {
        let policy = ResolutionPolicy::new(Domain::Physical);
        let inputs = vec![
            TritWord::tru(Frame::Consensus),
            TritWord::fals(Frame::Science),
        ];
        let result = policy.arbitrate(&inputs).unwrap();
        assert!(matches!(result, ArbitrationResult::Commit(_)));
    }

    #[test]
    fn physical_without_science_forces_collapse() {
        let policy = ResolutionPolicy::new(Domain::Physical);
        let inputs = vec![
            TritWord::tru(Frame::Individual),
            TritWord::fals(Frame::Consensus),
        ];
        let result = policy.arbitrate(&inputs).unwrap();
        assert_eq!(result, ArbitrationResult::ForceCollapse);
    }

    #[test]
    fn medical_ethics_preserves_individual() {
        let policy = ResolutionPolicy::new(Domain::MedicalEthics);
        let inputs = vec![
            TritWord::tru(Frame::Science),
            TritWord::fals(Frame::Individual),
        ];
        let result = policy.arbitrate(&inputs).unwrap();
        assert!(matches!(result, ArbitrationResult::Preserve(_)));
    }

    #[test]
    fn value_judgment_always_hold() {
        let policy = ResolutionPolicy::new(Domain::ValueJudgment);
        let inputs = vec![
            TritWord::tru(Frame::Science),
            TritWord::fals(Frame::Individual),
        ];
        let result = policy.arbitrate(&inputs).unwrap();
        assert_eq!(result, ArbitrationResult::Hold);
    }

    #[test]
    fn general_same_frame_commits_first() {
        let policy = ResolutionPolicy::new(Domain::General);
        let inputs = vec![
            TritWord::tru(Frame::Science),
            TritWord::fals(Frame::Science),
        ];
        let result = policy.arbitrate(&inputs).unwrap();
        assert!(matches!(result, ArbitrationResult::Commit(_)));
    }

    #[test]
    fn general_mixed_frames_negotiates() {
        let policy = ResolutionPolicy::new(Domain::General);
        let inputs = vec![
            TritWord::tru(Frame::Science),
            TritWord::fals(Frame::Individual),
        ];
        let result = policy.arbitrate(&inputs).unwrap();
        assert_eq!(result, ArbitrationResult::Negotiate);
    }

    #[test]
    fn custom_without_rule_defaults_to_negotiate() {
        let policy = ResolutionPolicy::new(Domain::Custom("chemistry".into()));
        let inputs = vec![
            TritWord::tru(Frame::Science),
            TritWord::fals(Frame::Individual),
        ];
        let result = policy.arbitrate(&inputs).unwrap();
        assert_eq!(result, ArbitrationResult::Negotiate);
    }

    #[test]
    fn custom_with_priority_frame_rule() {
        let rule = CustomRule {
            name: "chem".into(),
            priority_frame: Some("Science".into()),
            allow_forced_collapse: true,
            fallback: FallbackBehavior::Hold,
        };
        let policy =
            ResolutionPolicy::new(Domain::Custom("chemistry".into())).with_custom_rule(rule);
        let inputs = vec![
            TritWord::fals(Frame::Individual),
            TritWord::tru(Frame::Science),
        ];
        let result = policy.arbitrate(&inputs).unwrap();
        assert!(matches!(result, ArbitrationResult::Commit(_)));
    }

    #[test]
    fn arbitrate_rejects_empty_inputs() {
        let policy = ResolutionPolicy::new(Domain::Physical);
        assert!(matches!(
            policy.arbitrate(&[]).unwrap_err(),
            PolicyError::EmptyInputs
        ));
    }

    #[test]
    fn engineering_behaves_like_physical() {
        let policy = ResolutionPolicy::new(Domain::Engineering);
        let inputs = vec![
            TritWord::tru(Frame::Consensus),
            TritWord::fals(Frame::Science),
        ];
        let result = policy.arbitrate(&inputs).unwrap();
        assert!(matches!(result, ArbitrationResult::Commit(_)));
    }

    #[test]
    fn medical_ethics_without_individual_negotiates() {
        let policy = ResolutionPolicy::new(Domain::MedicalEthics);
        let inputs = vec![
            TritWord::tru(Frame::Science),
            TritWord::fals(Frame::Consensus),
        ];
        let result = policy.arbitrate(&inputs).unwrap();
        assert_eq!(result, ArbitrationResult::Negotiate);
    }

    #[test]
    fn custom_rule_fallback_hold() {
        let rule = CustomRule {
            name: "lit".into(),
            priority_frame: None,
            allow_forced_collapse: false,
            fallback: FallbackBehavior::Hold,
        };
        let policy =
            ResolutionPolicy::new(Domain::Custom("literature".into())).with_custom_rule(rule);
        let inputs = vec![
            TritWord::tru(Frame::Science),
            TritWord::fals(Frame::Individual),
        ];
        let result = policy.arbitrate(&inputs).unwrap();
        assert_eq!(result, ArbitrationResult::Hold);
    }

    #[test]
    fn custom_rule_fallback_safe_fallback() {
        let rule = CustomRule {
            name: "chem".into(),
            priority_frame: None,
            allow_forced_collapse: true,
            fallback: FallbackBehavior::SafeFallback,
        };
        let policy =
            ResolutionPolicy::new(Domain::Custom("chemistry".into())).with_custom_rule(rule);
        let inputs = vec![
            TritWord::tru(Frame::Science),
            TritWord::fals(Frame::Individual),
        ];
        let result = policy.arbitrate(&inputs).unwrap();
        assert_eq!(result, ArbitrationResult::ForceCollapse);
    }

    #[test]
    fn policy_error_display_is_informative() {
        let err = PolicyError::EmptyInputs;
        let msg = format!("{}", err);
        assert!(msg.contains("no inputs"));
    }

    #[test]
    fn default_policy_is_general() {
        let policy: ResolutionPolicy = Default::default();
        assert_eq!(policy.domain, Domain::General);
        assert!(policy.custom_rule.is_none());
    }
}
