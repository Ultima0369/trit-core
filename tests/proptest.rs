//! Property-based tests for Trit-Core core invariants.
//!
//! These tests use `proptest` to verify mathematical invariants across the
//! full state space, not just hand-picked examples.

use proptest::prelude::*;
use trit_core::core::frame::Frame;
use trit_core::core::phase::Phase;
use trit_core::core::value::TritValue;
use trit_core::core::word::TritWord;
use trit_core::core::TernaryAlgebra;
use trit_core::meta::{ArbitrationResult, Domain, MetaMonitor, ResolutionPolicy, SafeFallback};

// ============================================================================
// Custom strategies for generating Trit-Core values
// ============================================================================

fn arb_trit_value() -> impl Strategy<Value = TritValue> {
    prop_oneof![
        Just(TritValue::True),
        Just(TritValue::Hold),
        Just(TritValue::False),
        Just(TritValue::Unknown),
    ]
}

fn arb_computable_value() -> impl Strategy<Value = TritValue> {
    prop_oneof![
        Just(TritValue::True),
        Just(TritValue::Hold),
        Just(TritValue::False),
    ]
}

fn arb_frame() -> impl Strategy<Value = Frame> {
    prop_oneof![
        Just(Frame::Science),
        Just(Frame::Individual),
        Just(Frame::Consensus),
        Just(Frame::Meta),
    ]
}

fn arb_phase() -> impl Strategy<Value = Phase> {
    (0.0f64..=1.0).prop_map(|v| Phase::new_clamped((v * 1000.0).round() / 1000.0))
}

fn arb_trit_word() -> impl Strategy<Value = TritWord> {
    (arb_trit_value(), arb_frame(), arb_phase())
        .prop_map(|(value, frame, phase)| TritWord::new(value, phase, frame))
}

fn arb_computable_trit_word() -> impl Strategy<Value = TritWord> {
    (arb_computable_value(), arb_frame(), arb_phase())
        .prop_map(|(value, frame, phase)| TritWord::new(value, phase, frame))
}

// ============================================================================
// Layer 1 — TritValue algebra
// ============================================================================

proptest! {
    #[test]
    fn negation_is_involution(v in arb_trit_value()) {
        assert_eq!(v.negate().negate(), v);
    }

    #[test]
    fn unknown_isolated_from_negation(v in arb_trit_value()) {
        if v == TritValue::Unknown {
            assert_eq!(v.negate(), TritValue::Unknown);
        }
    }

    #[test]
    fn to_i8_maps_true_false_hold(v in arb_trit_value()) {
        match v {
            TritValue::True => assert_eq!(v.to_i8(), 1),
            TritValue::False => assert_eq!(v.to_i8(), -1),
            TritValue::Hold | TritValue::Unknown => assert_eq!(v.to_i8(), 0),
        }
    }
}

// ============================================================================
// Layer 2 — Phase invariants
// ============================================================================

proptest! {
    #[test]
    fn phase_clamped_stays_in_range(v in any::<f64>()) {
        let p = Phase::new_clamped(v);
        assert!(p.inner().is_finite());
        assert!((0.0..=1.0).contains(&p.inner()));
    }

    #[test]
    fn phase_complement_is_bounded(p in arb_phase()) {
        let c = p.complement();
        assert!((0.0..=1.0).contains(&c.inner()));
        assert!((c.inner() - (1.0 - p.inner())).abs() < 1e-9);
    }

    #[test]
    fn phase_mean_is_bounded(a in arb_phase(), b in arb_phase()) {
        let m = Phase::mean(a, b);
        assert!((0.0..=1.0).contains(&m.inner()));
    }
}

// ============================================================================
// Layer 3 — Ternary algebra (HTA)
// ============================================================================

proptest! {
    #[test]
    fn tand_same_frame_commutative(a in arb_trit_word(), b in arb_trit_word()) {
        prop_assume!(a.frame() == b.frame());
        let (r1, _) = TernaryAlgebra::t_and(&a, &b);
        let (r2, _) = TernaryAlgebra::t_and(&b, &a);
        assert_eq!(r1.value(), r2.value());
    }

    #[test]
    fn tand_unknown_propagates(v in arb_trit_value(), frame in arb_frame(), phase in arb_phase()) {
        let unknown = TritWord::unknown(frame);
        let other = TritWord::new(v, phase, frame);
        let (result, _) = TernaryAlgebra::t_and(&unknown, &other);
        assert_eq!(result.value(), TritValue::Unknown);
    }

    #[test]
    fn cross_frame_tand_yields_hold_no_value(
        a in arb_computable_trit_word(),
        b in arb_computable_trit_word()
    ) {
        prop_assume!(a.frame() != b.frame());
        let (result, interrupt) = TernaryAlgebra::t_and(&a, &b);
        assert_eq!(result.value(), TritValue::Hold);
        assert!(interrupt.is_some());
    }

    #[test]
    fn hot_path_matches_safe_path(a in arb_trit_word(), b in arb_trit_word()) {
        prop_assume!(a.frame() == b.frame());
        let (safe, _) = TernaryAlgebra::t_and(&a, &b);
        let hot = TernaryAlgebra::t_and_hot(&a, &b);
        assert_eq!(safe.value(), hot.value());
        assert_eq!(safe.frame(), hot.frame());
    }
}

// ============================================================================
// Layer 4 — Arbitration
// ============================================================================

proptest! {
    #[test]
    fn value_judgment_arbitration_always_hold(a in arb_trit_word(), b in arb_trit_word()) {
        let policy = ResolutionPolicy::new(Domain::ValueJudgment);
        let result = policy.arbitrate(&[a, b]).unwrap();
        prop_assert_eq!(result, ArbitrationResult::Hold);
    }

    #[test]
    fn general_same_frame_commits(a in arb_trit_word(), b in arb_trit_word()) {
        prop_assume!(a.frame() == b.frame());
        let policy = ResolutionPolicy::new(Domain::General);
        let result = policy.arbitrate(&[a, b]).unwrap();
        prop_assert!(matches!(result, ArbitrationResult::Commit(_)));
    }

    #[test]
    fn general_mixed_frames_negotiates(
        a in arb_computable_trit_word(),
        b in arb_computable_trit_word()
    ) {
        prop_assume!(a.frame() != b.frame());
        let policy = ResolutionPolicy::new(Domain::General);
        let result = policy.arbitrate(&[a, b]).unwrap();
        prop_assert_eq!(result, ArbitrationResult::Negotiate);
    }
}

// ============================================================================
// Layer 5 — SafeFallback
// ============================================================================

proptest! {
    #[test]
    fn safe_fallback_dangerous_unknown_forces_false(frame in arb_frame(), phase in arb_phase()) {
        let sf = SafeFallback::new();
        let word = TritWord::new(TritValue::Unknown, phase, frame);
        let (guarded, interrupt) = sf.guard(&Domain::Physical, &word, 1);
        assert_eq!(guarded.value(), TritValue::False);
        assert!(interrupt.is_some());
    }

    #[test]
    fn safe_fallback_non_dangerous_passes_through(
        v in arb_trit_value(),
        frame in arb_frame(),
        phase in arb_phase()
    ) {
        let sf = SafeFallback::new();
        let word = TritWord::new(v, phase, frame);
        let (guarded, interrupt) = sf.guard(&Domain::MedicalEthics, &word, 100);
        assert_eq!(guarded.value(), word.value());
        assert!(interrupt.is_none());
    }
}

// ============================================================================
// Layer 6 — MetaMonitor
// ============================================================================

#[test]
fn meta_monitor_starts_empty() {
    let monitor = MetaMonitor::new();
    assert_eq!(monitor.log().count(), 0);
}

// ============================================================================
// Layer 7 — Batch TAND (t_and_n)
// ============================================================================

proptest! {
    #[test]
    fn t_and_n_matches_tand_value_for_two_inputs(
        a in arb_computable_trit_word(),
        b in arb_computable_trit_word()
    ) {
        prop_assume!(a.frame() == b.frame());
        let (batch, _) = TernaryAlgebra::t_and_n(&[a, b]);
        let (pair, _) = TernaryAlgebra::t_and(&a, &b);
        prop_assert_eq!(batch.value(), pair.value());
    }

    #[test]
    fn t_and_n_phase_is_global_mean(a in arb_phase(), b in arb_phase(), c in arb_phase()) {
        let a_word = TritWord::new(TritValue::True, a, Frame::Science);
        let b_word = TritWord::new(TritValue::True, b, Frame::Science);
        let c_word = TritWord::new(TritValue::True, c, Frame::Science);
        let (result, _) = TernaryAlgebra::t_and_n(&[a_word, b_word, c_word]);
        let expected_phase = (a.inner() + b.inner() + c.inner()) / 3.0;
        prop_assert!((result.phase().inner() - expected_phase).abs() < 1e-9);
    }

    #[test]
    fn t_and_n_cross_frame_yields_hold(
        a in arb_computable_trit_word(),
        b in arb_computable_trit_word()
    ) {
        prop_assume!(a.frame() != b.frame());
        let (result, interrupts) = TernaryAlgebra::t_and_n(&[a, b]);
        prop_assert_eq!(result.value(), TritValue::Hold);
        prop_assert!(!interrupts.is_empty());
    }
}
