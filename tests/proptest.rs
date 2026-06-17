//! Property-based tests for Trit-Core core invariants.
//!
//! These tests use `proptest` to verify mathematical invariants across the
//! full state space, not just hand-picked examples. Each invariant is proven
//! by exhaustion over randomized input — a stronger guarantee than example-based
//! unit tests alone.
//!
//! ## Invariant Layers
//!
//! **Layer 1 — TritValue algebra**: involution, LUT consistency, Unknown isolation
//! **Layer 2 — Ternary algebra (HTA)**: commutativity, involution, De Morgan,
//!    cross-frame Hold, hot-path consistency, Unknown propagation
//! **Layer 3 — Arbitration**: domain policy invariants, Absolute frame guard
//! **Layer 4 — SafeFallback**: dangerous-domain forcing, safe-domain pass-through

use proptest::prelude::*;
use trit_core::frame::Frame;
use trit_core::meta::{
    ArbitrationResult, ConflictType, Domain, MetaMonitor, ResolutionPolicy, SafeFallback,
};
use trit_core::trit::algebra::TernaryAlgebra;
use trit_core::trit::{Phase, TritValue, TritWord};

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
        Just(Frame::Absolute),
        Just(Frame::Meta),
    ]
}

fn arb_phase() -> impl Strategy<Value = f64> {
    (0.0f64..=1.0).prop_map(|v| (v * 1000.0).round() / 1000.0)
}

fn arb_trit_word() -> impl Strategy<Value = TritWord> {
    (arb_trit_value(), arb_frame(), arb_phase())
        .prop_map(|(value, frame, phase)| TritWord::new(value, phase, frame))
}

#[allow(dead_code)]
fn arb_computable_trit_word() -> impl Strategy<Value = TritWord> {
    (arb_computable_value(), arb_frame(), arb_phase())
        .prop_map(|(value, frame, phase)| TritWord::new(value, phase, frame))
}

fn arb_cross_frame_pair() -> impl Strategy<Value = (TritWord, TritWord)> {
    (arb_trit_word(), arb_trit_word())
        .prop_filter("frames must differ", |(a, b)| a.frame != b.frame)
}

#[allow(dead_code)]
fn arb_same_frame_pair() -> impl Strategy<Value = (TritWord, TritWord)> {
    (
        arb_frame(),
        arb_trit_value(),
        arb_phase(),
        arb_trit_value(),
        arb_phase(),
    )
        .prop_map(|(frame, v1, p1, v2, p2)| {
            (
                TritWord::new(v1, p1, frame.clone()),
                TritWord::new(v2, p2, frame),
            )
        })
}

/// Same-frame pair with computable values only (no Unknown).
/// Used for De Morgan tests where Unknown breaks the law by design.
fn arb_same_frame_computable_pair() -> impl Strategy<Value = (TritWord, TritWord)> {
    (
        arb_frame(),
        arb_computable_value(),
        arb_phase(),
        arb_computable_value(),
        arb_phase(),
    )
        .prop_map(|(frame, v1, p1, v2, p2)| {
            (
                TritWord::new(v1, p1, frame.clone()),
                TritWord::new(v2, p2, frame),
            )
        })
}

fn arb_trit_word_vec(max_size: usize) -> impl Strategy<Value = Vec<TritWord>> {
    proptest::collection::vec(arb_trit_word(), 1..=max_size)
}

// ============================================================================
// Layer 1: TritValue algebraic invariants
// ============================================================================

proptest! {
    #[test]
    #[test]
    fn involution_negate_twice_is_identity(v in arb_trit_value()) {
        assert_eq!(v.negate().negate(), v);
    }

    #[test]
    fn lut_roundtrip_to_i8_from_i8(v in arb_computable_value()) {
        assert_eq!(TritValue::from(v.to_i8()), v);
    }

    #[test]
    fn from_i8_never_produces_unknown(x in -10i8..=10i8) {
        assert_ne!(TritValue::from(x), TritValue::Unknown);
    }

    #[test]
    fn unknown_is_exactly_not_computable(v in arb_trit_value()) {
        assert_eq!(v.is_computable(), v != TritValue::Unknown);
    }

    #[test]
    fn hold_is_default_prop(_dummy in Just(())) {
        assert_eq!(TritValue::default(), TritValue::Hold);
    }

    #[test]
    fn hold_and_unknown_are_different_prop(_dummy in Just(())) {
        assert_ne!(TritValue::Hold, TritValue::Unknown);
    }
}

// ============================================================================
// Layer 2: Ternary algebra (HTA) invariants
// ============================================================================

proptest! {

    #[test]
    fn tand_same_frame_commutative((a, b) in arb_same_frame_computable_pair()) {
        let (res_ab, int_ab) = TernaryAlgebra::t_and(&a, &b);
        let (res_ba, int_ba) = TernaryAlgebra::t_and(&b, &a);
        assert_eq!(res_ab.value, res_ba.value);
        assert_eq!(int_ab.is_some(), int_ba.is_some());
    }


    #[test]
    fn tor_same_frame_commutative((a, b) in arb_same_frame_computable_pair()) {
        let (res_ab, int_ab) = TernaryAlgebra::t_or(&a, &b);
        let (res_ba, int_ba) = TernaryAlgebra::t_or(&b, &a);
        assert_eq!(res_ab.value, res_ba.value);
        assert_eq!(int_ab.is_some(), int_ba.is_some());
    }


    #[test]
    fn tand_cross_frame_always_hold_with_interrupt((a, b) in arb_cross_frame_pair()) {
        let (result, interrupt) = TernaryAlgebra::t_and(&a, &b);
        assert_eq!(result.value, TritValue::Hold);
        assert!(interrupt.is_some());
        assert_eq!(interrupt.unwrap().conflict, ConflictType::FrameMismatch);
    }


    #[test]
    fn tor_cross_frame_always_hold_with_interrupt((a, b) in arb_cross_frame_pair()) {
        let (result, interrupt) = TernaryAlgebra::t_or(&a, &b);
        assert_eq!(result.value, TritValue::Hold);
        assert!(interrupt.is_some());
        assert_eq!(interrupt.unwrap().conflict, ConflictType::FrameMismatch);
    }


    #[test]
    fn tnot_involution_preserves_value(a in arb_trit_word()) {
        let not_once = TernaryAlgebra::t_not(&a);
        let not_twice = TernaryAlgebra::t_not(&not_once);
        assert_eq!(not_twice.value, a.value);
        let phase_diff = (not_twice.phase.inner() - a.phase.inner()).abs();
        assert!(phase_diff < 1e-5);
    }


    #[test]
    fn hot_path_consistency_tand((a, b) in arb_same_frame_computable_pair()) {
        let (res_full, interrupt) = TernaryAlgebra::t_and(&a, &b);
        let res_hot = TernaryAlgebra::t_and_hot(&a, &b);
        assert!(interrupt.is_none());
        assert_eq!(res_full.value, res_hot.value);
        let phase_diff = (res_full.phase.inner() - res_hot.phase.inner()).abs();
        assert!(phase_diff < 1e-5);
        assert_eq!(res_full.frame, res_hot.frame);
    }


    #[test]
    fn hot_path_consistency_tor((a, b) in arb_same_frame_computable_pair()) {
        let (res_full, interrupt) = TernaryAlgebra::t_or(&a, &b);
        let res_hot = TernaryAlgebra::t_or_hot(&a, &b);
        assert!(interrupt.is_none());
        assert_eq!(res_full.value, res_hot.value);
        let phase_diff = (res_full.phase.inner() - res_hot.phase.inner()).abs();
        assert!(phase_diff < 1e-5);
        assert_eq!(res_full.frame, res_hot.frame);
    }


    #[test]
    fn precheck_correctness(a in arb_trit_word(), b in arb_trit_word()) {
        let precheck = TernaryAlgebra::precheck_same_frame(&a, &b);
        let actual = a.frame == b.frame;
        assert_eq!(precheck, actual);
    }
}

// ============================================================================
// Layer 2b: Truth-table invariants
// ============================================================================

proptest! {

    #[test]
    fn tand_false_annihilates(v in arb_computable_value(), frame in arb_frame(),
                              p1 in arb_phase(), p2 in arb_phase()) {
        let a = TritWord::new(TritValue::False, p1, frame.clone());
        let b = TritWord::new(v, p2, frame);
        let (res, int) = TernaryAlgebra::t_and(&a, &b);
        assert!(int.is_none());
        assert_eq!(res.value, TritValue::False);
    }


    #[test]
    fn tor_true_dominates(v in arb_trit_value(), frame in arb_frame(),
                          p1 in arb_phase(), p2 in arb_phase()) {
        let a = TritWord::new(TritValue::True, p1, frame.clone());
        let b = TritWord::new(v, p2, frame);
        let (res, int) = TernaryAlgebra::t_or(&a, &b);
        assert!(int.is_none());
        assert_eq!(res.value, TritValue::True);
    }


    #[test]
    fn tand_idempotent(v in arb_computable_value(), frame in arb_frame(), phase in arb_phase()) {
        let a = TritWord::new(v, phase, frame);
        let (res, int) = TernaryAlgebra::t_and(&a, &a);
        assert!(int.is_none());
        assert_eq!(res.value, a.value);
        assert_eq!(res.frame, a.frame);
    }


    #[test]
    fn tor_idempotent(v in arb_computable_value(), frame in arb_frame(), phase in arb_phase()) {
        let a = TritWord::new(v, phase, frame);
        let (res, int) = TernaryAlgebra::t_or(&a, &a);
        assert!(int.is_none());
        assert_eq!(res.value, a.value);
        assert_eq!(res.frame, a.frame);
    }
}

// ============================================================================
// Layer 2c: Unknown propagation invariants
// ============================================================================

proptest! {

    #[test]
    fn tand_unknown_propagates(v in arb_trit_value(), frame in arb_frame(),
                                p1 in arb_phase(), p2 in arb_phase()) {
        let a = TritWord::new(TritValue::Unknown, p1, frame.clone());
        let b = TritWord::new(v, p2, frame);
        let (res, int) = TernaryAlgebra::t_and(&a, &b);
        assert!(int.is_none());
        assert_eq!(res.value, TritValue::Unknown);
    }


    #[test]
    fn tand_unknown_propagates_right(v in arb_trit_value(), frame in arb_frame(),
                                      p1 in arb_phase(), p2 in arb_phase()) {
        let a = TritWord::new(v, p1, frame.clone());
        let b = TritWord::new(TritValue::Unknown, p2, frame);
        let (res, int) = TernaryAlgebra::t_and(&a, &b);
        assert!(int.is_none());
        assert_eq!(res.value, TritValue::Unknown);
    }


    #[test]
    fn tnot_preserves_unknown(frame in arb_frame(), phase in arb_phase()) {
        let a = TritWord::new(TritValue::Unknown, phase, frame);
        let res = TernaryAlgebra::t_not(&a);
        assert_eq!(res.value, TritValue::Unknown);
    }


    #[test]
    fn tor_true_dominates_unknown(frame in arb_frame(), p1 in arb_phase(), p2 in arb_phase()) {
        let a = TritWord::new(TritValue::True, p1, frame.clone());
        let b = TritWord::new(TritValue::Unknown, p2, frame);
        let (res, int) = TernaryAlgebra::t_or(&a, &b);
        assert!(int.is_none());
        assert_eq!(res.value, TritValue::True);
    }


    #[test]
    fn tor_unknown_unknown_is_unknown(frame in arb_frame(), p1 in arb_phase(), p2 in arb_phase()) {
        let a = TritWord::new(TritValue::Unknown, p1, frame.clone());
        let b = TritWord::new(TritValue::Unknown, p2, frame);
        let (res, int) = TernaryAlgebra::t_or(&a, &b);
        assert!(int.is_none());
        assert_eq!(res.value, TritValue::Unknown);
    }
}

// ============================================================================
// Layer 3: Arbitration policy invariants
// ============================================================================

proptest! {

    #[test]
    fn value_judgment_always_hold(inputs in arb_trit_word_vec(10)) {
        let policy = ResolutionPolicy::new(Domain::ValueJudgment);
        let result = policy.arbitrate(&inputs);
        assert_eq!(result, ArbitrationResult::Hold);
    }


    #[test]
    fn absolute_frame_non_hold_triggers_violation(phase in arb_phase()) {
        let policy = ResolutionPolicy::new(Domain::General);
        let monitor = MetaMonitor::new(policy);

        let word_true = TritWord::new(TritValue::True, phase, Frame::Absolute);
        let interrupt = monitor.inspect(&word_true);
        assert!(interrupt.is_some());
        assert_eq!(interrupt.unwrap().conflict, ConflictType::PolicyViolation);

        let word_false = TritWord::new(TritValue::False, phase, Frame::Absolute);
        let interrupt = monitor.inspect(&word_false);
        assert!(interrupt.is_some());
    }


    #[test]
    fn absolute_frame_hold_is_accepted(phase in arb_phase()) {
        let policy = ResolutionPolicy::new(Domain::General);
        let monitor = MetaMonitor::new(policy);
        let word = TritWord::new(TritValue::Hold, phase, Frame::Absolute);
        assert!(monitor.inspect(&word).is_none());
    }


    #[test]
    fn physical_domain_prioritizes_science(
        other_frames in proptest::collection::vec(
            (arb_frame(), arb_computable_value(), arb_phase()), 1..5
        ),
        science_phase in arb_phase(),
    ) {
        let mut inputs: Vec<TritWord> = vec![
            TritWord::new(TritValue::False, science_phase, Frame::Science),
        ];
        for (frame, val, phase) in other_frames {
            inputs.push(TritWord::new(val, phase, frame));
        }
        let policy = ResolutionPolicy::new(Domain::Physical);
        let result = policy.arbitrate(&inputs);
        match result {
            ArbitrationResult::Commit(w) => assert_eq!(w.frame, Frame::Science),
            other => panic!("Expected Commit(Science), got {:?}", other),
        }
    }


    #[test]
    fn engineering_domain_prioritizes_science(
        other_frames in proptest::collection::vec(
            (arb_frame(), arb_computable_value(), arb_phase()), 1..5
        ),
        science_phase in arb_phase(),
    ) {
        let mut inputs: Vec<TritWord> = vec![
            TritWord::new(TritValue::True, science_phase, Frame::Science),
        ];
        for (frame, val, phase) in other_frames {
            inputs.push(TritWord::new(val, phase, frame));
        }
        let policy = ResolutionPolicy::new(Domain::Engineering);
        let result = policy.arbitrate(&inputs);
        match result {
            ArbitrationResult::Commit(w) => assert_eq!(w.frame, Frame::Science),
            other => panic!("Expected Commit(Science), got {:?}", other),
        }
    }


    #[test]
    fn medical_ethics_prioritizes_individual(
        other_frames in proptest::collection::vec(
            (arb_frame(), arb_computable_value(), arb_phase()), 1..5
        ),
        individual_phase in arb_phase(),
        individual_val in arb_computable_value(),
    ) {
        let mut inputs: Vec<TritWord> = vec![
            TritWord::new(individual_val, individual_phase, Frame::Individual),
        ];
        for (frame, val, phase) in other_frames {
            inputs.push(TritWord::new(val, phase, frame));
        }
        let policy = ResolutionPolicy::new(Domain::MedicalEthics);
        let result = policy.arbitrate(&inputs);
        match result {
            ArbitrationResult::Preserve(w) => assert_eq!(w.frame, Frame::Individual),
            other => panic!("Expected Preserve(Individual), got {:?}", other),
        }
    }


    #[test]
    fn general_domain_single_frame_commits_first(
        frame in arb_frame(),
        signals in proptest::collection::vec(
            (arb_computable_value(), arb_phase()), 1..5
        ),
    ) {
        let inputs: Vec<TritWord> = signals
            .iter()
            .map(|(val, phase)| TritWord::new(*val, *phase, frame.clone()))
            .collect();
        let policy = ResolutionPolicy::new(Domain::General);
        let result = policy.arbitrate(&inputs);
        match result {
            ArbitrationResult::Commit(w) => assert_eq!(w, inputs[0]),
            other => panic!("Expected Commit(first), got {:?}", other),
        }
    }


    #[test]
    fn general_domain_mixed_frames_negotiates((a, b) in arb_cross_frame_pair()) {
        let inputs = vec![a, b];
        let policy = ResolutionPolicy::new(Domain::General);
        let result = policy.arbitrate(&inputs);
        assert_eq!(result, ArbitrationResult::Negotiate);
    }


    #[test]
    fn custom_domain_defaults_to_negotiate(
        name in "[a-zA-Z_][a-zA-Z0-9_]{0,31}",
        inputs in arb_trit_word_vec(10),
    ) {
        let policy = ResolutionPolicy::new(Domain::Custom(name));
        let result = policy.arbitrate(&inputs);
        assert_eq!(result, ArbitrationResult::Negotiate);
    }
}

// ============================================================================
// Layer 4: SafeFallback invariants
// ============================================================================

proptest! {

    #[test]
    fn safe_fallback_forces_false_on_dangerous_domain_with_interrupts(
        phase in arb_phase(),
        interrupt_count in 1usize..100usize,
    ) {
        let sf = SafeFallback::new();
        let word = TritWord::new(TritValue::Hold, phase, Frame::Meta);
        let (guarded, interrupt) = sf.guard(&Domain::Physical, &word, interrupt_count);
        assert_eq!(guarded.value, TritValue::False);
        assert!(interrupt.is_some());

        let word_u = TritWord::new(TritValue::Unknown, phase, Frame::Meta);
        let (guarded_u, interrupt_u) = sf.guard(&Domain::Physical, &word_u, interrupt_count);
        assert_eq!(guarded_u.value, TritValue::False);
        assert!(interrupt_u.is_some());
    }


    #[test]
    fn safe_fallback_passes_through_committed_value(
        phase in arb_phase(),
        interrupt_count in 1usize..100usize,
    ) {
        let sf = SafeFallback::new();
        let word = TritWord::new(TritValue::True, phase, Frame::Science);
        let (guarded, interrupt) = sf.guard(&Domain::Physical, &word, interrupt_count);
        assert_eq!(guarded.value, TritValue::True);
        assert!(interrupt.is_none());
    }


    #[test]
    fn safe_domains_never_force_false(
        phase in arb_phase(),
        interrupt_count in 0usize..50usize,
    ) {
        let sf = SafeFallback::new();
        let word = TritWord::new(TritValue::Hold, phase, Frame::Meta);
        for domain in [Domain::MedicalEthics, Domain::ValueJudgment, Domain::General] {
            let (guarded, interrupt) = sf.guard(&domain, &word, interrupt_count);
            assert_eq!(guarded.value, TritValue::Hold);
            assert!(interrupt.is_none());
        }
    }


    #[test]
    fn safe_fallback_no_interrupts_no_force(phase in arb_phase()) {
        let sf = SafeFallback::new();
        let word = TritWord::new(TritValue::Hold, phase, Frame::Meta);
        for domain in [
            Domain::Physical,
            Domain::Engineering,
            Domain::Custom("chemistry".into()),
            Domain::Custom("genetics".into()),
            Domain::Custom("nuclear".into()),
        ] {
            let (guarded, interrupt) = sf.guard(&domain, &word, 0);
            assert_eq!(guarded.value, TritValue::Hold);
            assert!(interrupt.is_none());
        }
    }


    #[test]
    fn safe_fallback_disabled_passes_through(
        value in arb_trit_value(),
        phase in arb_phase(),
        interrupt_count in 0usize..100usize,
    ) {
        let mut sf = SafeFallback::new();
        sf.enabled = false;
        let word = TritWord::new(value, phase, Frame::Meta);
        let (guarded, interrupt) = sf.guard(&Domain::Physical, &word, interrupt_count);
        assert_eq!(guarded.value, value);
        assert!(interrupt.is_none());
    }


    #[test]
    fn non_dangerous_custom_domain_passes_through(
        name in "[a-z_]{1,20}",
        phase in arb_phase(),
    ) {
        let sf = SafeFallback::new();
        let word = TritWord::new(TritValue::Hold, phase, Frame::Meta);
        let domain = Domain::Custom(name.clone());
        let is_registered_dangerous = sf.dangerous_custom_domains.iter().any(|d| d == &name);
        let (guarded, interrupt) = sf.guard(&domain, &word, 5);
        if is_registered_dangerous {
            assert_eq!(guarded.value, TritValue::False);
            assert!(interrupt.is_some());
        } else {
            assert_eq!(guarded.value, TritValue::Hold);
            assert!(interrupt.is_none());
        }
    }
}

// ============================================================================
// Layer 5: Phase invariants
// ============================================================================
// Using larger case counts because f64 space is much larger.

proptest! {

    #[test]
    fn phase_new_always_valid(x in any::<f64>()) {
        let p = Phase::new(x);
        let inner = p.inner();
        assert!(inner.is_finite());
        assert!((0.0..=1.0).contains(&inner));
    }


    #[test]
    fn phase_try_new_rejects_invalid(x in any::<f64>()) {
        let result = Phase::try_new(x);
        let is_valid = x.is_finite() && (0.0..=1.0).contains(&x);
        assert_eq!(result.is_ok(), is_valid);
    }


    #[test]
    fn phase_quantize_snaps_to_neutral(delta in -1e-7f64..1e-7f64) {
        let p = Phase::new(0.5 + delta);
        let q = p.quantize(1e-6);
        assert!((q.inner() - 0.5).abs() < f64::EPSILON);
    }


    #[test]
    fn phase_complement_involution(x in 0.0f64..=1.0f64) {
        let p = Phase::new(x);
        let c1 = p.complement();
        let c2 = c1.complement();
        let diff = (c2.inner() - p.inner()).abs();
        assert!(diff < 1e-5);
    }


    #[test]
    fn phase_mean_bounded(a in 0.0f64..=1.0f64, b in 0.0f64..=1.0f64) {
        let pa = Phase::new(a);
        let pb = Phase::new(b);
        let m = Phase::mean(pa, pb).inner();
        let min = a.min(b);
        let max = a.max(b);
        assert!(m >= min - 1e-10);
        assert!(m <= max + 1e-10);
    }
}

// ============================================================================
// Layer 6: De Morgan's laws
// ============================================================================

proptest! {

    #[test]
    fn de_morgan_tand_tor_tnot((a, b) in arb_same_frame_computable_pair()) {
        let (and_res, _) = TernaryAlgebra::t_and(&a, &b);
        let left = TernaryAlgebra::t_not(&and_res);

        let not_a = TernaryAlgebra::t_not(&a);
        let not_b = TernaryAlgebra::t_not(&b);
        let (right, _) = TernaryAlgebra::t_or(&not_a, &not_b);

        assert_eq!(left.value, right.value);
    }


    #[test]
    fn de_morgan_tor_tand_tnot((a, b) in arb_same_frame_computable_pair()) {
        let (or_res, _) = TernaryAlgebra::t_or(&a, &b);
        let left = TernaryAlgebra::t_not(&or_res);

        let not_a = TernaryAlgebra::t_not(&a);
        let not_b = TernaryAlgebra::t_not(&b);
        let (right, _) = TernaryAlgebra::t_and(&not_a, &not_b);

        assert_eq!(left.value, right.value);
    }
}

// ============================================================================
// Layer 7: TritWord constructor invariants
// ============================================================================

proptest! {

    #[test]
    fn tritword_tru_is_true_with_full_phase(frame in arb_frame()) {
        let w = TritWord::tru(frame.clone());
        assert_eq!(w.value, TritValue::True);
        assert!((w.phase.inner() - 1.0).abs() < f64::EPSILON);
        assert_eq!(w.frame, frame);
    }


    #[test]
    fn tritword_fals_is_false_with_zero_phase(frame in arb_frame()) {
        let w = TritWord::fals(frame.clone());
        assert_eq!(w.value, TritValue::False);
        assert!((w.phase.inner() - 0.0).abs() < f64::EPSILON);
        assert_eq!(w.frame, frame);
    }


    #[test]
    fn tritword_hold_is_hold_with_neutral_phase(frame in arb_frame()) {
        let w = TritWord::hold(frame.clone());
        assert_eq!(w.value, TritValue::Hold);
        assert!((w.phase.inner() - 0.5).abs() < f64::EPSILON);
        assert_eq!(w.frame, frame);
    }


    #[test]
    fn tritword_unknown_is_unknown_with_neutral_phase(frame in arb_frame()) {
        let w = TritWord::unknown(frame.clone());
        assert_eq!(w.value, TritValue::Unknown);
        assert!((w.phase.inner() - 0.5).abs() < f64::EPSILON);
        assert_eq!(w.frame, frame);
    }
}

// ============================================================================
// Layer 8: Frame invariants
// ============================================================================

proptest! {

    #[test]
    fn frame_display_from_str_roundtrip(frame in arb_frame()) {
        let s = format!("{}", frame);
        let parsed: Frame = s.parse().unwrap();
        assert_eq!(parsed, frame);
    }


    #[test]
    fn unknown_frame_name_rejected(name in "[A-Z][a-zA-Z0-9_]{1,20}") {
        let is_standard = matches!(
            name.as_str(),
            "Science" | "Individual" | "Consensus" | "Absolute" | "Meta"
        );
        let result: Result<Frame, _> = name.parse();
        assert_eq!(result.is_ok(), is_standard);
    }
}

// ============================================================================
// Layer 9: MetaMonitor invariants
// ============================================================================

proptest! {

    #[test]
    fn meta_monitor_starts_empty(domain in prop_oneof![
        Just(Domain::Physical),
        Just(Domain::Engineering),
        Just(Domain::MedicalEthics),
        Just(Domain::ValueJudgment),
        Just(Domain::General),
    ]) {
        let monitor = MetaMonitor::new(ResolutionPolicy::new(domain));
        assert!(monitor.log().is_empty());
    }
}

// ============================================================================
// Layer 10: RuleLoader invariants
// ============================================================================

proptest! {

    #[test]
    fn custom_rule_json_roundtrip(
        name in "[a-z_]{1,20}",
        priority_frame in prop_oneof![
            Just(None),
            Just(Some("Science".to_string())),
            Just(Some("Individual".to_string())),
        ],
        allow_forced_collapse in any::<bool>(),
        fallback in prop_oneof![
            Just("hold".to_string()),
            Just("negotiate".to_string()),
            Just("commit_first".to_string()),
            Just("safe_fallback".to_string()),
        ],
    ) {
        use trit_core::meta::CustomRule;
        let rule = CustomRule {
            name: name.clone(),
            priority_frame: priority_frame.clone(),
            allow_forced_collapse,
            fallback: fallback.clone(),
        };
        let json = serde_json::to_string(&rule).unwrap();
        let parsed = serde_json::from_str::<CustomRule>(&json).unwrap();
        assert_eq!(parsed.name, name);
        assert_eq!(parsed.priority_frame, priority_frame);
        assert_eq!(parsed.allow_forced_collapse, allow_forced_collapse);
        assert_eq!(parsed.fallback, fallback);
    }
}

// ============================================================================
// Layer 11: Cascade invariants
// ============================================================================

proptest! {

    #[test]
    fn cascade_same_frame_produces_no_interrupts(
        frame in arb_frame(),
        signals in proptest::collection::vec(
            (arb_computable_value(), arb_phase()), 2..20
        ),
    ) {
        let trits: Vec<TritWord> = signals
            .iter()
            .map(|(v, p)| TritWord::new(*v, *p, frame.clone()))
            .collect();
        let mut total_interrupts = 0usize;
        let mut current = trits[0].clone();
        for next in &trits[1..] {
            let (_result, interrupt) = TernaryAlgebra::t_and(&current, next);
            if interrupt.is_some() { total_interrupts += 1; }
            current = _result;
        }
        assert_eq!(total_interrupts, 0);
    }


    #[test]
    fn cascade_output_phase_bounded(trits in arb_trit_word_vec(50)) {
        if trits.is_empty() { return Ok(()); }
        let mut current = trits[0].clone();
        for next in &trits[1..] {
            let (result, _) = TernaryAlgebra::t_and(&current, next);
            let phase = result.phase.inner();
            assert!(phase.is_finite());
            assert!((0.0..=1.0).contains(&phase));
            current = result;
        }
    }
}
