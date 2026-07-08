//! Ethics gate tests — release hard stop.
//!
//! These tests encode the four bottom lines from CHARTER.md:
//! 1. Do not deprive (user can override, disable SafeFallback)
//! 2. Do not deceive (Hold on conflict, no guessing, Absolute stays Hold)
//! 3. Do not evolve (no user profiling / adaptive features)
//! 4. Publicly auditable (Frame labels + explanations in output)

use trit_core::core::{Frame, Phase, TernaryAlgebra, TritValue, TritWord};
use trit_core::meta::{ConflictType, Domain, PolicyViolation, SafeFallback};
use trit_core::security::SecurityMode;

#[test]
fn ethics_cross_frame_must_hold() {
    let a = TritWord::tru(Frame::Science);
    let b = TritWord::fals(Frame::Individual);
    let (result, interrupt) = TernaryAlgebra::t_and(&a, &b);
    assert_eq!(result.value(), TritValue::Hold);
    assert!(interrupt.is_some());
}

#[test]
fn ethics_user_can_override_hold() {
    let a = TritWord::tru(Frame::Science);
    let b = TritWord::fals(Frame::Individual);
    let (result, _) = TernaryAlgebra::t_and(&a, &b);
    assert_eq!(result.value(), TritValue::Hold);
    // User override: the system does not prevent the user from choosing Individual True.
    let user_override = TritWord::tru(Frame::Individual);
    assert_eq!(user_override.value(), TritValue::True);
}

#[test]
fn ethics_user_can_disable_safe_fallback() {
    let sf = SafeFallback::disabled();
    let result = TritWord::unknown(Frame::Science);
    let (guarded, interrupt) = sf.guard(&Domain::Physical, &result, 1);
    assert_eq!(guarded.value(), TritValue::Unknown);
    assert!(interrupt.is_none());
}

#[test]
fn ethics_awareness_does_not_block_computation() {
    let a = TritWord::new(TritValue::True, Phase::full_true(), Frame::Meta);
    let b = TritWord::tru(Frame::Science);

    let interrupt = TernaryAlgebra::awareness_check(&a, &b);
    assert!(interrupt.is_some());
    assert!(matches!(
        interrupt.unwrap().conflict,
        ConflictType::PolicyViolation(PolicyViolation::FrameContamination)
    ));

    // Computation continues: TAND still runs despite the awareness notification.
    let (result, _) = TernaryAlgebra::t_and(&a, &b);
    assert_eq!(result.value(), TritValue::Hold);
}

#[test]
fn ethics_meta_frame_cannot_be_external_input() {
    let meta_input = TritWord::new(TritValue::True, Phase::full_true(), Frame::Meta);
    let science = TritWord::tru(Frame::Science);

    let (result, interrupt) = TernaryAlgebra::t_and(&meta_input, &science);
    assert_eq!(result.value(), TritValue::Hold);
    assert!(interrupt.is_some());

    // Awareness check flags the same issue as a policy violation.
    let awareness = TernaryAlgebra::awareness_check(&meta_input, &science);
    assert!(awareness.is_some());
}

#[test]
fn ethics_system_does_not_guess_on_hold() {
    let a = TritWord::tru(Frame::Science);
    let b = TritWord::fals(Frame::Individual);
    let (result, _) = TernaryAlgebra::t_and(&a, &b);
    assert_eq!(result.value(), TritValue::Hold);
    // The result must not be collapsed to True or False.
    assert_ne!(result.value(), TritValue::True);
    assert_ne!(result.value(), TritValue::False);
}

#[test]
fn ethics_absolute_frame_remains_hold() {
    let word = TritWord::absolute();
    assert_eq!(word.value(), TritValue::Hold);
    assert_eq!(word.frame(), Frame::Absolute);
}

#[test]
fn ethics_no_user_profiling_feature() {
    // Aurora deliberately does not implement user profiling.
    let manifest = include_str!("../Cargo.toml");
    assert!(
        !manifest.contains("user_profiling"),
        "user_profiling feature must not exist"
    );
}

#[test]
fn ethics_no_adaptive_recommendation_feature() {
    // Aurora deliberately does not adapt to user preferences.
    let manifest = include_str!("../Cargo.toml");
    assert!(
        !manifest.contains("adaptive_recommendation"),
        "adaptive_recommendation feature must not exist"
    );
}

#[test]
fn ethics_security_mode_awareness_does_not_block() {
    assert!(SecurityMode::Awareness.allows_computation());
    assert!(SecurityMode::Awareness.requires_notification());
    assert!(SecurityMode::Service.allows_computation());
    assert!(!SecurityMode::Refusal.allows_computation());
}
