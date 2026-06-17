use crate::frame::Frame;
use crate::trit::{TritValue, TritWord};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{debug, info, warn};

/// Frame presence bitmask for O(1) frame lookups during arbitration.
#[derive(Clone, Copy, Debug)]
struct FrameMask(u8);

impl FrameMask {
    const SCIENCE: u8 = 1 << 0;
    const INDIVIDUAL: u8 = 1 << 1;
    const CONSENSUS: u8 = 1 << 2;
    const ABSOLUTE: u8 = 1 << 3;
    const META: u8 = 1 << 4;

    fn from_inputs(inputs: &[TritWord]) -> Self {
        let mut mask = 0u8;
        for t in inputs {
            mask |= match t.frame {
                Frame::Science => Self::SCIENCE,
                Frame::Individual => Self::INDIVIDUAL,
                Frame::Consensus => Self::CONSENSUS,
                Frame::Absolute => Self::ABSOLUTE,
                Frame::Meta => Self::META,
            };
            if mask == 0b11111 {
                break; // all frames seen, early exit
            }
        }
        FrameMask(mask)
    }

    fn has(&self, frame: &Frame) -> bool {
        let bit = match frame {
            Frame::Science => Self::SCIENCE,
            Frame::Individual => Self::INDIVIDUAL,
            Frame::Consensus => Self::CONSENSUS,
            Frame::Absolute => Self::ABSOLUTE,
            Frame::Meta => Self::META,
        };
        (self.0 & bit) != 0
    }

    fn count(&self) -> u32 {
        self.0.count_ones()
    }
}

/// Domain rules for conflict resolution.
/// Each domain defines which frame has priority and whether
/// forced resolution (hard collapse) is allowed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Domain {
    Physical,       // Hard science constraints: Science priority, forced collapse
    Engineering,    // Applied constraints: Science priority, forced collapse
    MedicalEthics,  // Soft constraints: Individual priority, no forced collapse
    ValueJudgment,  // Incommensurable: no priority, must remain Hold
    General,        // Default: attempt negotiation
    Custom(String), // Externally loaded domain rules
}

/// Policy engine that decides how to resolve conflicts.
#[derive(Debug, Clone)]
pub struct ResolutionPolicy {
    pub domain: Domain,
}

impl ResolutionPolicy {
    pub fn new(domain: Domain) -> Self {
        info!(?domain, "ResolutionPolicy created");
        Self { domain }
    }

    /// Given conflicting inputs, return the arbitration result.
    /// Uses FrameMask for O(1) frame presence checks.
    #[tracing::instrument(skip_all, fields(domain = ?self.domain))]
    pub fn arbitrate(&self, inputs: &[TritWord]) -> ArbitrationResult {
        debug!(input_count = inputs.len(), "arbitration started");
        let mask = FrameMask::from_inputs(inputs);
        let result = match self.domain {
            Domain::Physical | Domain::Engineering => {
                if mask.has(&Frame::Science) {
                    let t = inputs.iter().find(|t| t.frame == Frame::Science).unwrap();
                    ArbitrationResult::Commit(t.clone())
                } else {
                    ArbitrationResult::ForceCollapse
                }
            }
            Domain::MedicalEthics => {
                if mask.has(&Frame::Individual) {
                    let t = inputs
                        .iter()
                        .find(|t| t.frame == Frame::Individual)
                        .unwrap();
                    ArbitrationResult::Preserve(t.clone())
                } else {
                    ArbitrationResult::Negotiate
                }
            }
            Domain::ValueJudgment => ArbitrationResult::Hold,
            Domain::Custom(ref name) => {
                info!(custom_domain = %name, "custom domain arbitration: defaulting to Negotiate");
                ArbitrationResult::Negotiate
            }
            Domain::General => {
                if mask.count() == 1 {
                    // All same frame (single bit set)
                    ArbitrationResult::Commit(inputs[0].clone())
                } else {
                    ArbitrationResult::Negotiate
                }
            }
        };
        info!(?result, "arbitration completed");
        result
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ArbitrationResult {
    Commit(TritWord),
    Preserve(TritWord),
    ForceCollapse,
    Hold,
    Negotiate,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MetaInterrupt {
    pub conflict: ConflictType,
    pub reason: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl MetaInterrupt {
    pub fn new(conflict: ConflictType, reason: String) -> Self {
        Self {
            conflict,
            reason,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Create a FrameMismatch interrupt from a frame pair.
    /// Uses pre-sized String allocation instead of `format!()`.
    #[inline]
    pub fn with_frames(op: &'static str, frame_a: Frame, frame_b: Frame) -> Self {
        let reason = Self::build_frame_mismatch_reason(op, &frame_a, &frame_b);
        Self {
            conflict: ConflictType::FrameMismatch,
            reason,
            timestamp: chrono::Utc::now(),
        }
    }

    fn build_frame_mismatch_reason(op: &str, a: &Frame, b: &Frame) -> String {
        // Maximum: "TAND conflict: Consensus vs Individual" ≈ 40 bytes
        let mut reason = String::with_capacity(48);
        reason.push_str(op);
        reason.push_str(" conflict: ");
        use std::fmt::Write;
        let _ = write!(reason, "{}", a);
        reason.push_str(" vs ");
        let _ = write!(reason, "{}", b);
        reason
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConflictType {
    FrameMismatch,
    OutOfScope,
    PhaseDrift,
    PolicyViolation,
}

#[derive(Debug, Clone)]
pub struct MetaMonitor {
    #[allow(dead_code)]
    policy: ResolutionPolicy,
    log: Vec<MetaInterrupt>,
}

impl MetaMonitor {
    pub fn new(policy: ResolutionPolicy) -> Self {
        Self {
            policy,
            log: vec![],
        }
    }

    pub fn record(&mut self, interrupt: MetaInterrupt) {
        self.log.push(interrupt);
    }

    pub fn log(&self) -> &[MetaInterrupt] {
        &self.log
    }

    pub fn inspect(&self, word: &TritWord) -> Option<MetaInterrupt> {
        if word.frame == Frame::Absolute && word.value != TritValue::Hold {
            return Some(MetaInterrupt::new(
                ConflictType::PolicyViolation,
                "Absolute frame must remain Hold".to_string(),
            ));
        }
        None
    }
}

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
            if let Ok(frame) = pf.parse::<Frame>() {
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

// ---------------------------------------------------------------------------
// SafeFallback: forced-False safety mechanism for dangerous domains
// ---------------------------------------------------------------------------

/// SafeFallback provides a safety-preserving override when the system
/// cannot reach a confident decision in a dangerous context (e.g.,
/// chemical synthesis, gene editing, structural safety).
///
/// Unlike normal `Hold` (which means "undecided, suspend judgment"),
/// SafeFallback forces `False` for safety-critical scenarios where
/// indecision could result in harm.
///
/// ## Design rationale (per IEC 61508 / ISO 26262 principles):
/// - In dangerous domains, "I don't know" must default to "don't do it."
/// - `Hold` = "not ready to decide, gather more data" (non-dangerous).
/// - `SafeFallback::force_false()` = "cannot decide, but failure means harm → block."
///
/// ## Built-in dangerous domains
/// `Physical` and `Engineering` are always dangerous (science overrides opinion
/// when the physical world doesn't negotiate). `MedicalEthics` is omitted
/// because patient autonomy (`Individual` frame) IS the safe default.
#[derive(Debug, Clone)]
pub struct SafeFallback {
    /// Additional custom domains that require safe fallback behavior.
    pub dangerous_custom_domains: Vec<String>,
    /// Whether SafeFallback is active.
    pub enabled: bool,
}

impl SafeFallback {
    /// Create a new SafeFallback with sensible defaults.
    /// Physical/Engineering are inherently dangerous — Earth doesn't negotiate.
    pub fn new() -> Self {
        Self {
            dangerous_custom_domains: vec![
                "chemistry".to_string(),
                "genetics".to_string(),
                "structural".to_string(),
                "nuclear".to_string(),
                "pharmaceutical".to_string(),
            ],
            enabled: true,
        }
    }

    /// Register a custom domain as dangerous.
    pub fn register_dangerous(&mut self, domain: &str) {
        if !self.dangerous_custom_domains.iter().any(|d| d == domain) {
            self.dangerous_custom_domains.push(domain.to_string());
        }
    }

    /// Check whether the given domain requires safe fallback.
    /// `Physical` and `Engineering` are always dangerous.
    pub fn is_dangerous(&self, domain: &Domain) -> bool {
        if !self.enabled {
            return false;
        }
        match domain {
            // Earth doesn't negotiate: Physical + Engineering always dangerous
            Domain::Physical | Domain::Engineering => true,
            // MedicalEthics: patient autonomy (Individual frame) is the safe fallback
            Domain::MedicalEthics | Domain::ValueJudgment | Domain::General => false,
            Domain::Custom(name) => self.dangerous_custom_domains.iter().any(|d| d == name),
        }
    }

    /// Apply safe fallback: if the result is Hold or Unknown and the
    /// domain is dangerous, force False with an interrupt.
    pub fn guard(
        &self,
        domain: &Domain,
        result: &TritWord,
        interrupt_count: usize,
    ) -> (TritWord, Option<MetaInterrupt>) {
        if !self.is_dangerous(domain) {
            return (result.clone(), None);
        }

        if result.value == TritValue::Hold || result.value == TritValue::Unknown {
            if interrupt_count > 0 {
                let domain_name = domain_label(domain);
                let interrupt = MetaInterrupt::new(
                    ConflictType::OutOfScope,
                    format!(
                        "SafeFallback: forcing False in dangerous domain '{}' — {} interrupts detected",
                        domain_name, interrupt_count
                    ),
                );
                warn!(
                    domain = domain_name,
                    interrupt_count = interrupt_count,
                    "SafeFallback triggered"
                );
                (
                    TritWord::new(TritValue::False, result.phase.inner(), result.frame.clone()),
                    Some(interrupt),
                )
            } else {
                (result.clone(), None)
            }
        } else {
            (result.clone(), None)
        }
    }
}

/// Human-readable label for a domain (used in SafeFallback messages).
fn domain_label(domain: &Domain) -> &str {
    match domain {
        Domain::Physical => "Physical",
        Domain::Engineering => "Engineering",
        Domain::MedicalEthics => "MedicalEthics",
        Domain::ValueJudgment => "ValueJudgment",
        Domain::General => "General",
        Domain::Custom(name) => name.as_str(),
    }
}

impl Default for SafeFallback {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Domain::Custom tests
    // -----------------------------------------------------------------------

    #[test]
    fn custom_domain_arbitrates_as_negotiate_by_default() {
        let policy = ResolutionPolicy::new(Domain::Custom("chemistry".into()));
        let inputs = vec![
            TritWord::tru(Frame::Science),
            TritWord::fals(Frame::Individual),
        ];
        let result = policy.arbitrate(&inputs);
        assert_eq!(result, ArbitrationResult::Negotiate);
    }

    #[test]
    fn custom_domain_not_equal_to_general() {
        assert_ne!(Domain::Custom("chemistry".into()), Domain::General);
    }

    // -----------------------------------------------------------------------
    // RuleLoader tests
    // -----------------------------------------------------------------------

    #[test]
    fn json_rule_loader_parses_valid_rule() {
        let json = r#"{
            "name": "chemistry_safety",
            "priority_frame": "Science",
            "allow_forced_collapse": true,
            "fallback": "safe_fallback"
        }"#;
        let rule = JsonRuleLoader::load_json(json).unwrap();
        assert_eq!(rule.name, "chemistry_safety");
        assert_eq!(rule.priority_frame, Some("Science".to_string()));
        assert!(rule.allow_forced_collapse);
        assert_eq!(rule.fallback, "safe_fallback");
    }

    #[test]
    fn json_rule_loader_rejects_invalid_json() {
        assert!(JsonRuleLoader::load_json("not json").is_err());
    }

    #[test]
    fn rule_apply_priority_frame_match() {
        let rule = CustomRule {
            name: "test".into(),
            priority_frame: Some("Science".into()),
            allow_forced_collapse: true,
            fallback: "hold".into(),
        };
        let inputs = vec![
            TritWord::fals(Frame::Individual),
            TritWord::tru(Frame::Science),
        ];
        let result = JsonRuleLoader::apply(&rule, &inputs);
        assert!(matches!(result, ArbitrationResult::Commit(_)));
    }

    #[test]
    fn rule_apply_fallback_hold() {
        let rule = CustomRule {
            name: "test".into(),
            priority_frame: None,
            allow_forced_collapse: false,
            fallback: "hold".into(),
        };
        let inputs = vec![TritWord::tru(Frame::Science)];
        let result = JsonRuleLoader::apply(&rule, &inputs);
        assert_eq!(result, ArbitrationResult::Hold);
    }

    #[test]
    fn rule_apply_fallback_safe_fallback() {
        let rule = CustomRule {
            name: "test".into(),
            priority_frame: None,
            allow_forced_collapse: false,
            fallback: "safe_fallback".into(),
        };
        let inputs = vec![TritWord::tru(Frame::Science)];
        let result = JsonRuleLoader::apply(&rule, &inputs);
        assert_eq!(result, ArbitrationResult::ForceCollapse);
    }

    // -----------------------------------------------------------------------
    // SafeFallback tests
    // -----------------------------------------------------------------------

    #[test]
    fn safe_fallback_defaults_to_enabled() {
        let sf = SafeFallback::new();
        assert!(sf.enabled);
    }

    #[test]
    fn safe_fallback_physical_is_always_dangerous() {
        let sf = SafeFallback::new();
        assert!(sf.is_dangerous(&Domain::Physical));
    }

    #[test]
    fn safe_fallback_engineering_is_always_dangerous() {
        let sf = SafeFallback::new();
        assert!(sf.is_dangerous(&Domain::Engineering));
    }

    #[test]
    fn safe_fallback_medical_ethics_is_not_dangerous() {
        let sf = SafeFallback::new();
        // Patient autonomy (Individual frame) IS the safe default
        assert!(!sf.is_dangerous(&Domain::MedicalEthics));
    }

    #[test]
    fn safe_fallback_custom_dangerous_domains() {
        let sf = SafeFallback::new();
        assert!(sf.is_dangerous(&Domain::Custom("chemistry".into())));
        assert!(sf.is_dangerous(&Domain::Custom("genetics".into())));
        assert!(sf.is_dangerous(&Domain::Custom("nuclear".into())));
    }

    #[test]
    fn safe_fallback_custom_non_dangerous() {
        let sf = SafeFallback::new();
        assert!(!sf.is_dangerous(&Domain::Custom("literature".into())));
        assert!(!sf.is_dangerous(&Domain::Custom("music".into())));
    }

    #[test]
    fn safe_fallback_can_register_new_dangerous_domain() {
        let mut sf = SafeFallback::new();
        sf.register_dangerous("biohacking");
        assert!(sf.is_dangerous(&Domain::Custom("biohacking".into())));
    }

    #[test]
    fn safe_fallback_guard_forces_false_on_hold_with_interrupts() {
        let sf = SafeFallback::new();
        let result = TritWord::new(TritValue::Hold, 0.5, Frame::Meta);
        let (guarded, interrupt) = sf.guard(&Domain::Custom("chemistry".into()), &result, 3);
        assert_eq!(guarded.value, TritValue::False);
        assert!(interrupt.is_some());
    }

    #[test]
    fn safe_fallback_guard_forces_false_on_physical_domain() {
        let sf = SafeFallback::new();
        let result = TritWord::new(TritValue::Hold, 0.5, Frame::Meta);
        // Physical domain: Hold + interrupts → forced False
        let (guarded, interrupt) = sf.guard(&Domain::Physical, &result, 2);
        assert_eq!(guarded.value, TritValue::False);
        assert!(interrupt.is_some());
    }

    #[test]
    fn safe_fallback_guard_forces_false_on_engineering_domain() {
        let sf = SafeFallback::new();
        let result = TritWord::new(TritValue::Unknown, 0.5, Frame::Meta);
        let (guarded, interrupt) = sf.guard(&Domain::Engineering, &result, 1);
        assert_eq!(guarded.value, TritValue::False);
        assert!(interrupt.is_some());
    }

    #[test]
    fn safe_fallback_guard_forces_false_on_unknown() {
        let sf = SafeFallback::new();
        let result = TritWord::new(TritValue::Unknown, 0.5, Frame::Meta);
        let (guarded, interrupt) = sf.guard(&Domain::Custom("genetics".into()), &result, 1);
        assert_eq!(guarded.value, TritValue::False);
        assert!(interrupt.is_some());
    }

    #[test]
    fn safe_fallback_guard_passes_through_true() {
        let sf = SafeFallback::new();
        let result = TritWord::new(TritValue::True, 0.8, Frame::Science);
        let (guarded, interrupt) = sf.guard(&Domain::Custom("chemistry".into()), &result, 3);
        assert_eq!(guarded.value, TritValue::True);
        assert!(interrupt.is_none());
    }

    #[test]
    fn safe_fallback_guard_no_interrupts_no_force() {
        let sf = SafeFallback::new();
        let result = TritWord::new(TritValue::Hold, 0.5, Frame::Meta);
        // No interrupts → not dangerous enough to force
        let (guarded, interrupt) = sf.guard(&Domain::Custom("chemistry".into()), &result, 0);
        assert_eq!(guarded.value, TritValue::Hold);
        assert!(interrupt.is_none());
    }

    #[test]
    fn safe_fallback_medical_ethics_passes_through() {
        let sf = SafeFallback::new();
        let result = TritWord::new(TritValue::Hold, 0.5, Frame::Meta);
        // MedicalEthics: patient autonomy is the safe default, no forced False
        let (guarded, interrupt) = sf.guard(&Domain::MedicalEthics, &result, 5);
        assert_eq!(guarded.value, TritValue::Hold);
        assert!(interrupt.is_none());
    }

    #[test]
    fn safe_fallback_disabled_passes_through_everything() {
        let mut sf = SafeFallback::new();
        sf.enabled = false;
        let result = TritWord::new(TritValue::Hold, 0.5, Frame::Meta);
        let (guarded, interrupt) = sf.guard(&Domain::Physical, &result, 5);
        assert_eq!(guarded.value, TritValue::Hold);
        assert!(interrupt.is_none());
    }
}
