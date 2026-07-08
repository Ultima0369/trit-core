// Meta-monitor and policy engine for conflict-aware decision arbitration.
//
// Sub-modules:
// - frame_mask: O(1) bitmask for frame presence checks
// - domain: Domain enum, ResolutionPolicy, ArbitrationResult, PolicyError
// - interrupt: MetaInterrupt, ConflictType, MetaMonitor
// - rules: CustomRule, FallbackBehavior, load_rule, load_rule_json, apply_rule
// - safe_fallback: IEC 61508 safety-preserving override

mod domain;
pub(crate) mod frame_mask;
// ponytail: interrupt types moved to core::interrupt. This module is now a re-export shim.
mod interrupt;
mod rules;
mod safe_fallback;

// Re-export public API
// ponytail: Domain moved to core::domain (Layer Dependency Cleanup 2026-07-08).
// Re-export for backward compatibility — new code should import from crate::core::domain.
pub use crate::core::domain::{Domain, DomainParseError};
pub use domain::{ArbitrationResult, PolicyError, ResolutionPolicy};
// ponytail: interrupt types moved to core::interrupt (Layer Dependency Cleanup 2026-07-08).
// Re-export for backward compatibility — new code should import from crate::core::interrupt.
pub use crate::core::interrupt::{
    CognitiveOffload, ConflictType, HoldReason, MetaInterrupt, MetaMonitor, PolicyViolation,
    SourceConflict, MAX_INTERRUPT_LOG,
};
pub use rules::{apply_rule, load_rule_json, CustomRule, FallbackBehavior, RuleError};
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
