// Meta-monitor and policy engine for conflict-aware decision arbitration.
//
// Sub-modules:
// - frame_mask: O(1) bitmask for frame presence checks
// - domain: Domain enum, ResolutionPolicy, ArbitrationResult, PolicyError
// - interrupt: MetaInterrupt, ConflictType, MetaMonitor
// - rules: CustomRule, JsonRuleLoader, RuleError
// - safe_fallback: IEC 61508 safety-preserving override

mod domain;
pub(crate) mod frame_mask;
mod interrupt;
mod rules;
mod safe_fallback;

// Re-export public API
pub use domain::{ArbitrationResult, Domain, DomainParseError, PolicyError, ResolutionPolicy};
pub use interrupt::{ConflictType, MetaInterrupt, MetaMonitor, PolicyViolation, MAX_INTERRUPT_LOG};
pub use rules::{CustomRule, FallbackBehavior, JsonRuleLoader, RuleError};
pub use safe_fallback::SafeFallback;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::frame::Frame;
    use crate::core::word::TritWord;

    #[test]
    fn custom_domain_arbitrates_as_negotiate_by_default() {
        let policy = ResolutionPolicy::new(Domain::Custom("chemistry".into()));
        let inputs = vec![
            TritWord::tru(Frame::Science),
            TritWord::fals(Frame::Individual),
        ];
        let result = policy.arbitrate(&inputs).unwrap();
        assert_eq!(result, ArbitrationResult::Negotiate);
    }

    #[test]
    fn custom_domain_not_equal_to_general() {
        assert_ne!(Domain::Custom("chemistry".into()), Domain::General);
    }
}
