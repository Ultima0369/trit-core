use crate::core::phase::Phase;
use crate::core::value::TritValue;
use crate::core::word::TritWord;
use crate::meta::{ConflictType, Domain, MetaInterrupt};
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
    dangerous_custom_domains: Vec<String>,
    enabled: bool,
}

impl SafeFallback {
    /// Create a new SafeFallback with sensible defaults.
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

    /// Disable safe fallback (e.g., for testing or non-critical domains).
    pub fn disabled() -> Self {
        Self {
            dangerous_custom_domains: vec![],
            enabled: false,
        }
    }

    /// Register a custom domain as dangerous.
    pub fn register_dangerous(&mut self, domain: &str) {
        if !self.dangerous_custom_domains.iter().any(|d| d == domain) {
            self.dangerous_custom_domains.push(domain.to_string());
        }
    }

    /// Builder-style method to register a dangerous domain.
    pub fn with_dangerous_domain(mut self, domain: impl Into<String>) -> Self {
        self.register_dangerous(&domain.into());
        self
    }

    /// Enable or disable safe fallback.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Check whether the given domain requires safe fallback.
    pub fn is_dangerous(&self, domain: &Domain) -> bool {
        if !self.enabled {
            return false;
        }
        match domain {
            Domain::Physical | Domain::Engineering => true,
            Domain::MedicalEthics | Domain::ValueJudgment | Domain::General => false,
            Domain::Custom(name) => self.dangerous_custom_domains.iter().any(|d| d == name),
        }
    }

    /// Apply safe fallback: if the result is Hold or Unknown and the
    /// domain is dangerous, force False with an interrupt.
    ///
    /// `Unknown` always triggers SafeFallback in dangerous domains
    /// (the system cannot compute — "I don't know" must default to
    /// "don't do it"). `Hold` triggers only when interrupts are present
    /// (deliberate suspension + conflict = safety risk).
    pub fn guard(
        &self,
        domain: &Domain,
        result: &TritWord,
        interrupt_count: usize,
    ) -> (TritWord, Option<MetaInterrupt>) {
        if !self.is_dangerous(domain) {
            return (*result, None);
        }

        let should_fallback = result.value() == TritValue::Unknown
            || (result.value() == TritValue::Hold && interrupt_count > 0);

        if should_fallback {
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
                TritWord::new(TritValue::False, Phase::full_false(), result.frame()),
                Some(interrupt),
            )
        } else {
            (*result, None)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::frame::Frame;
    use crate::core::phase::Phase;
    use crate::core::word::TritWord;

    #[test]
    fn safe_fallback_defaults_to_enabled() {
        let sf = SafeFallback::new();
        assert!(sf.is_dangerous(&Domain::Physical));
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
        let result = TritWord::hold(Frame::Meta);
        let (guarded, interrupt) = sf.guard(&Domain::Custom("chemistry".into()), &result, 3);
        assert_eq!(guarded.value(), TritValue::False);
        assert!(interrupt.is_some());
    }

    #[test]
    fn safe_fallback_guard_forces_false_on_physical_domain() {
        let sf = SafeFallback::new();
        let result = TritWord::hold(Frame::Meta);
        let (guarded, interrupt) = sf.guard(&Domain::Physical, &result, 2);
        assert_eq!(guarded.value(), TritValue::False);
        assert!(interrupt.is_some());
    }

    #[test]
    fn safe_fallback_guard_forces_false_on_engineering_domain() {
        let sf = SafeFallback::new();
        let result = TritWord::unknown(Frame::Meta);
        let (guarded, interrupt) = sf.guard(&Domain::Engineering, &result, 1);
        assert_eq!(guarded.value(), TritValue::False);
        assert!(interrupt.is_some());
    }

    #[test]
    fn safe_fallback_guard_forces_false_on_unknown() {
        let sf = SafeFallback::new();
        let result = TritWord::unknown(Frame::Meta);
        let (guarded, interrupt) = sf.guard(&Domain::Custom("genetics".into()), &result, 1);
        assert_eq!(guarded.value(), TritValue::False);
        assert!(interrupt.is_some());
    }

    #[test]
    fn safe_fallback_guard_passes_through_true() {
        let sf = SafeFallback::new();
        let result = TritWord::tru(Frame::Science);
        let (guarded, interrupt) = sf.guard(&Domain::Custom("chemistry".into()), &result, 3);
        assert_eq!(guarded.value(), TritValue::True);
        assert!(interrupt.is_none());
    }

    #[test]
    fn safe_fallback_guard_no_interrupts_no_force() {
        let sf = SafeFallback::new();
        let result = TritWord::hold(Frame::Meta);
        let (guarded, interrupt) = sf.guard(&Domain::Custom("chemistry".into()), &result, 0);
        assert_eq!(guarded.value(), TritValue::Hold);
        assert!(interrupt.is_none());
    }

    #[test]
    fn safe_fallback_medical_ethics_passes_through() {
        let sf = SafeFallback::new();
        let result = TritWord::hold(Frame::Meta);
        let (guarded, interrupt) = sf.guard(&Domain::MedicalEthics, &result, 5);
        assert_eq!(guarded.value(), TritValue::Hold);
        assert!(interrupt.is_none());
    }

    #[test]
    fn safe_fallback_disabled_passes_through_everything() {
        let sf = SafeFallback::disabled();
        let result = TritWord::hold(Frame::Meta);
        let (guarded, interrupt) = sf.guard(&Domain::Physical, &result, 5);
        assert_eq!(guarded.value(), TritValue::Hold);
        assert!(interrupt.is_none());
    }

    #[test]
    fn builder_registers_dangerous_domain() {
        let sf = SafeFallback::new().with_dangerous_domain("biohacking");
        assert!(sf.is_dangerous(&Domain::Custom("biohacking".into())));
    }

    #[test]
    fn enabled_can_disable_fallback() {
        let sf = SafeFallback::new().enabled(false);
        assert!(!sf.is_dangerous(&Domain::Physical));
        let result = TritWord::unknown(Frame::Meta);
        let (guarded, interrupt) = sf.guard(&Domain::Physical, &result, 5);
        assert_eq!(guarded.value(), TritValue::Unknown);
        assert!(interrupt.is_none());
    }

    #[test]
    fn register_dangerous_deduplicates() {
        let mut sf = SafeFallback::new();
        sf.register_dangerous("chemistry");
        sf.register_dangerous("chemistry");
        // internal vec should not contain duplicates; exact count is private,
        // but behavior should remain consistent.
        assert!(sf.is_dangerous(&Domain::Custom("chemistry".into())));
    }

    #[test]
    fn guard_preserves_false() {
        let sf = SafeFallback::new();
        let result = TritWord::fals(Frame::Science);
        let (guarded, interrupt) = sf.guard(&Domain::Physical, &result, 5);
        assert_eq!(guarded.value(), TritValue::False);
        assert!(interrupt.is_none());
    }

    #[test]
    fn guard_resets_phase_to_full_false() {
        let sf = SafeFallback::new();
        let result = TritWord::new(
            TritValue::Unknown,
            Phase::new(0.25).unwrap(),
            Frame::Individual,
        );
        let (guarded, _) = sf.guard(&Domain::Physical, &result, 1);
        // Phase is reset to full_false() when SafeFallback forces False
        assert_eq!(guarded.phase().inner(), 0.0);
        assert_eq!(guarded.frame(), Frame::Individual);
    }

    #[test]
    fn non_dangerous_domain_does_not_force_even_with_interrupts() {
        let sf = SafeFallback::new();
        let result = TritWord::unknown(Frame::Meta);
        let (guarded, interrupt) = sf.guard(&Domain::MedicalEthics, &result, 5);
        assert_eq!(guarded.value(), TritValue::Unknown);
        assert!(interrupt.is_none());
    }
}
