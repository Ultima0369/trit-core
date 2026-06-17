// Meta-monitor and policy engine for conflict-aware decision arbitration.
//
// Sub-modules:
// - frame_mask: O(1) bitmask for frame presence checks
// - domain: Domain enum, ResolutionPolicy, ArbitrationResult
// - interrupt: MetaInterrupt, ConflictType, MetaMonitor
// - rules: CustomRule, RuleLoader trait, JsonRuleLoader
// - safe_fallback: IEC 61508 safety-preserving override

mod domain;
pub(crate) mod frame_mask;
mod interrupt;
mod rules;
mod safe_fallback;

// Re-export public API
pub use domain::{ArbitrationResult, Domain, ResolutionPolicy};
pub use interrupt::{ConflictType, MetaInterrupt, MetaMonitor};
pub use rules::{CustomRule, JsonRuleLoader, RuleLoader};
pub use safe_fallback::SafeFallback;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::Frame;
    use crate::trit::{TritValue, TritWord};

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
        let (guarded, interrupt) = sf.guard(&Domain::Custom("chemistry".into()), &result, 0);
        assert_eq!(guarded.value, TritValue::Hold);
        assert!(interrupt.is_none());
    }

    #[test]
    fn safe_fallback_medical_ethics_passes_through() {
        let sf = SafeFallback::new();
        let result = TritWord::new(TritValue::Hold, 0.5, Frame::Meta);
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
