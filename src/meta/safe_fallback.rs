use crate::meta::{ConflictType, Domain, MetaInterrupt};
use crate::trit::{TritValue, TritWord};
use tracing::warn;

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

impl Default for SafeFallback {
    fn default() -> Self {
        Self::new()
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
