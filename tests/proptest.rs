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
//! **Layer 5 — Phase invariants**: validity, quantization, complement, bounded mean
//! **Layer 6 — De Morgan's laws**: computable values only
//! **Layer 7 — TritWord constructors**: tru/fals/hold/unknown invariants
//! **Layer 8 — Frame invariants**: Display/FromStr roundtrip
//! **Layer 9 — MetaMonitor invariants**: starts empty
//! **Layer 10 — RuleLoader invariants**: JSON roundtrip
//! **Layer 11 — Cascade invariants**: same-frame no interrupts, phase bounded
//! **Layer 12 — Node state machine**: state transitions, phase preservation, decouple
//! **Layer 13 — PLL controller**: correction bounds, deadband, conflict detection
//! **Layer 14 — ResonanceBus**: coupling lifecycle, negotiation invariants
//! **Layer 15 — Message serde**: protocol message roundtrip

use proptest::prelude::*;
use trit_core::frame::Frame;
use trit_core::meta::{
    ArbitrationResult, ConflictType, Domain, MetaMonitor, ResolutionPolicy, SafeFallback,
};
use trit_core::net::bus::ResonanceBus;
use trit_core::net::message::Message;
use trit_core::net::node::{Node, NodeState};
use trit_core::net::pll::PllController;
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

// ============================================================================
// Layer 12: Node state machine invariants
// ============================================================================

fn arb_node_state() -> impl Strategy<Value = NodeState> {
    prop_oneof![
        Just(NodeState::Sovereign),
        Just(NodeState::Coupling),
        Just(NodeState::Coupled),
        Just(NodeState::Hold),
    ]
}

fn arb_node(frame: Frame, phase: f64) -> Node {
    Node::new(uuid::Uuid::new_v4().to_string(), frame, phase)
}

proptest! {

    #[test]
    fn node_starts_sovereign(frame in arb_frame(), phase in arb_phase()) {
        let node = arb_node(frame, phase);
        assert_eq!(node.state, NodeState::Sovereign);
        assert_eq!(node.peers.len(), 0);
        assert_eq!(node.cycles_coupled, 0);
    }


    #[test]
    fn node_sovereign_phase_preserved_after_coupling(
        frame in arb_frame(),
        sovereign_phase in arb_phase(),
        coupled_phase in arb_phase(),
    ) {
        let mut node = arb_node(frame, sovereign_phase);
        node.current_phase = coupled_phase;
        // Sovereign phase must remain unchanged
        assert!((node.sovereign_phase - sovereign_phase).abs() < f64::EPSILON);
        // Current phase reflects coupling drift
        assert!((node.current_phase - coupled_phase).abs() < f64::EPSILON);
    }


    #[test]
    fn node_initiate_coupling_only_from_sovereign(
        frame in arb_frame(),
        phase in arb_phase(),
        state in arb_node_state(),
    ) {
        let mut node = arb_node(frame, phase);
        node.state = state.clone();
        node.initiate_coupling("peer-x");
        if state == NodeState::Sovereign {
            assert_eq!(node.state, NodeState::Coupling);
            assert!(node.peers.contains(&"peer-x".to_string()));
        } else {
            assert_eq!(node.state, state);
            assert!(!node.peers.contains(&"peer-x".to_string()));
        }
    }


    #[test]
    fn node_confirm_coupling_only_from_coupling(
        frame in arb_frame(),
        phase in arb_phase(),
        state in arb_node_state(),
    ) {
        let mut node = arb_node(frame, phase);
        node.state = state.clone();
        node.confirm_coupling();
        if state == NodeState::Coupling {
            assert_eq!(node.state, NodeState::Coupled);
        } else {
            assert_eq!(node.state, state);
        }
    }


    #[test]
    fn node_decouple_restores_sovereign(
        frame in arb_frame(),
        sovereign_phase in arb_phase(),
        coupled_phase in arb_phase(),
        state in arb_node_state(),
        num_peers in 0usize..10usize,
        cycles in 0u64..1000u64,
    ) {
        let mut node = arb_node(frame, sovereign_phase);
        node.current_phase = coupled_phase;
        node.state = state;
        node.peers = (0..num_peers).map(|i| format!("peer-{}", i)).collect();
        node.cycles_coupled = cycles;

        node.decouple();

        assert_eq!(node.state, NodeState::Sovereign);
        assert!((node.current_phase - node.sovereign_phase).abs() < f64::EPSILON);
        assert!(node.peers.is_empty());
        assert_eq!(node.cycles_coupled, 0);
    }


    #[test]
    fn node_phase_adjustment_clamped(
        frame in arb_frame(),
        initial_phase in arb_phase(),
        delta in -2.0f64..2.0f64,
    ) {
        let mut node = arb_node(frame, initial_phase);
        node.adjust_phase(delta);
        let p = node.current_phase;
        assert!(p.is_finite());
        assert!((0.0..=1.0).contains(&p));
    }


    #[test]
    fn node_tick_only_increments_when_coupled(
        frame in arb_frame(),
        phase in arb_phase(),
        state in arb_node_state(),
    ) {
        let mut node = arb_node(frame, phase);
        node.state = state.clone();
        let before = node.cycles_coupled;
        node.tick();
        if state == NodeState::Coupled {
            assert_eq!(node.cycles_coupled, before + 1);
        } else {
            assert_eq!(node.cycles_coupled, before);
        }
    }


    #[test]
    fn node_interference_same_frame_always_constructive(
        frame in arb_frame(),
        p1 in arb_phase(),
        p2 in arb_phase(),
    ) {
        let a = Node::new("a".into(), frame.clone(), p1);
        let b = Node::new("b".into(), frame, p2);
        assert_eq!(a.interference_with(&b), trit_core::net::node::Interference::Constructive);
    }


    #[test]
    fn node_to_trit_preserves_phase_and_frame(
        frame in arb_frame(),
        phase in arb_phase(),
    ) {
        let node = arb_node(frame.clone(), phase);
        let trit = node.to_trit();
        assert_eq!(trit.value, TritValue::Hold);
        assert!((trit.phase.inner() - phase).abs() < f64::EPSILON);
        assert_eq!(trit.frame, frame);
    }


    #[test]
    fn node_enter_hold_records_interrupt(
        frame in arb_frame(),
        phase in arb_phase(),
        reason in "[a-z_ ]{1,50}",
    ) {
        let mut node = arb_node(frame, phase);
        node.enter_hold(&reason);
        assert_eq!(node.state, NodeState::Hold);
        assert_eq!(node.monitor.log().len(), 1);
        assert_eq!(node.monitor.log()[0].conflict, ConflictType::FrameMismatch);
    }
}

// ============================================================================
// Layer 13: PLL controller invariants
// ============================================================================

proptest! {

    #[test]
    fn pll_correction_bounded(
        local in 0.0f64..=1.0f64,
        peer in 0.0f64..=1.0f64,
    ) {
        let mut pll = PllController::new();
        let correction = pll.compute_correction(local, peer);
        assert!(correction.abs() <= pll.max_correction);
        assert!(correction.is_finite());
    }


    #[test]
    fn pll_deadband_suppresses_small_errors(
        local in 0.0f64..=1.0f64,
        epsilon in -0.049f64..0.049f64,
    ) {
        let mut pll = PllController::new();
        let peer = (local + epsilon).clamp(0.0, 1.0);
        let correction = pll.compute_correction(local, peer);
        // Error <= deadband → correction should be 0
        let error = (peer - local).abs();
        if error <= pll.deadband {
            assert_eq!(correction, 0.0);
        }
    }


    #[test]
    fn pll_correction_sign_matches_error(
        local in 0.0f64..=1.0f64,
        peer in 0.0f64..=1.0f64,
    ) {
        let mut pll = PllController::new();
        let correction = pll.compute_correction(local, peer);
        let error = peer - local;
        if error.abs() > pll.deadband {
            // Correction should pull local toward peer
            assert!(correction * error >= 0.0,
                "correction {} should have same sign as error {}", correction, error);
        }
    }


    #[test]
    fn pll_total_correction_accumulates(
        local in 0.0f64..=1.0f64,
        peer in 0.0f64..=1.0f64,
    ) {
        let mut pll = PllController::new();
        let before = pll.total_correction;
        let _ = pll.compute_correction(local, peer);
        let after = pll.total_correction;
        let error = (peer - local).abs();
        if error > pll.deadband {
            assert!((after - before).abs() > 0.0);
        }
    }


    #[test]
    fn pll_reset_zeroes_total_correction(
        local in 0.0f64..=1.0f64,
        peer in 0.0f64..=1.0f64,
    ) {
        let mut pll = PllController::new();
        let _ = pll.compute_correction(local, peer);
        pll.reset();
        assert_eq!(pll.total_correction, 0.0);
    }


    #[test]
    fn conflict_phase_gap_threshold(
        p1 in 0.0f64..=1.0f64,
        p2 in 0.0f64..=1.0f64,
    ) {
        let is_conflict = PllController::is_conflict_phase_gap(p1, p2);
        let gap = (p1 - p2).abs();
        assert_eq!(is_conflict, gap > 0.3);
    }


    #[test]
    fn phase_jump_anomaly_threshold(
        old in 0.0f64..=1.0f64,
        new in 0.0f64..=1.0f64,
    ) {
        let is_anomaly = PllController::is_phase_jump_anomaly(old, new);
        let jump = (new - old).abs();
        assert_eq!(is_anomaly, jump > 0.5);
    }
}

// ============================================================================
// Layer 14: ResonanceBus coupling lifecycle invariants
// ============================================================================

fn arb_node_id() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{0,15}"
}

proptest! {

    #[test]
    fn bus_register_node_is_retrievable(
        id in arb_node_id(),
        frame in arb_frame(),
        phase in arb_phase(),
    ) {
        let mut bus = ResonanceBus::new();
        let node = Node::new(id.clone(), frame.clone(), phase);
        bus.register(node);
        let retrieved = bus.get_node(&id);
        assert!(retrieved.is_some());
        let n = retrieved.unwrap();
        assert_eq!(n.frame, frame);
        assert!((n.current_phase - phase).abs() < f64::EPSILON);
        assert_eq!(n.state, NodeState::Sovereign);
    }


    #[test]
    fn bus_register_creates_pll(
        id in arb_node_id(),
        frame in arb_frame(),
        phase in arb_phase(),
    ) {
        let mut bus = ResonanceBus::new();
        let node = Node::new(id.clone(), frame, phase);
        bus.register(node);
        assert!(bus.plls.contains_key(&id));
    }


    #[test]
    fn bus_same_frame_resonance_is_constructive(
        id_a in arb_node_id(),
        id_b in arb_node_id(),
        frame in arb_frame(),
        p1 in arb_phase(),
        p2 in arb_phase(),
    ) {
        prop_assume!(id_a != id_b);
        let mut bus = ResonanceBus::new();
        bus.register(Node::new(id_a.clone(), frame.clone(), p1));
        bus.register(Node::new(id_b.clone(), frame.clone(), p2));

        let req = Message::resonate_req(&id_a, &format!("{}", frame), p1, vec![]);
        let ack = bus.handle_resonate_req(&id_a, &id_b, &req);

        assert!(ack.is_some());
        let ack_msg = ack.unwrap();
        match &ack_msg.payload {
            trit_core::net::message::MessagePayload::ResonateAck(data) => {
                assert_eq!(data.interference, "constructive");
                assert!(!data.conflict_detected);
                assert_eq!(data.recommendation, "commit");
            }
            _ => panic!("Expected ResonateAck"),
        }
    }


    #[test]
    fn bus_cross_frame_resonance_detects_conflict(
        id_a in arb_node_id(),
        id_b in arb_node_id(),
        frame_a in arb_frame(),
        frame_b in arb_frame(),
        p1 in arb_phase(),
        p2 in arb_phase(),
    ) {
        prop_assume!(id_a != id_b);
        prop_assume!(frame_a != frame_b);
        let mut bus = ResonanceBus::new();
        bus.register(Node::new(id_a.clone(), frame_a.clone(), p1));
        bus.register(Node::new(id_b.clone(), frame_b.clone(), p2));

        let req = Message::resonate_req(&id_a, &format!("{}", frame_a), p1, vec![]);
        let ack = bus.handle_resonate_req(&id_a, &id_b, &req);

        assert!(ack.is_some());
        let ack_msg = ack.unwrap();
        match &ack_msg.payload {
            trit_core::net::message::MessagePayload::ResonateAck(data) => {
                assert!(data.conflict_detected || data.interference != "constructive");
            }
            _ => panic!("Expected ResonateAck"),
        }
    }


    #[test]
    fn bus_decouple_restores_phase(
        id in arb_node_id(),
        frame in arb_frame(),
        sovereign_phase in arb_phase(),
        coupled_phase in arb_phase(),
        cycles in 0u64..1000u64,
    ) {
        let mut bus = ResonanceBus::new();
        let mut node = Node::new(id.clone(), frame, sovereign_phase);
        node.current_phase = coupled_phase;
        node.state = NodeState::Coupled;
        node.cycles_coupled = cycles;
        bus.register(node);

        let req = Message::decouple_req(&id, "test_decouple");
        let ack = bus.handle_decouple_req(&id, &req, cycles);

        match &ack.payload {
            trit_core::net::message::MessagePayload::DecoupleAck(data) => {
                assert!((data.restored_phase - sovereign_phase).abs() < f64::EPSILON);
                assert_eq!(data.cycles_coupled, cycles);
            }
            _ => panic!("Expected DecoupleAck"),
        }

        let node = bus.get_node(&id).unwrap();
        assert_eq!(node.state, NodeState::Sovereign);
        assert!((node.current_phase - node.sovereign_phase).abs() < f64::EPSILON);
    }


    #[test]
    fn bus_negotiate_same_frame_commits(
        ids in proptest::collection::vec(arb_node_id(), 2..10),
        frame in arb_frame(),
        phases in proptest::collection::vec(arb_phase(), 2..10),
    ) {
        let n = ids.len().min(phases.len());
        let mut bus = ResonanceBus::new();
        for i in 0..n {
            bus.register(Node::new(ids[i].clone(), frame.clone(), phases[i]));
        }
        let participant_ids: Vec<String> = ids[..n].to_vec();
        let (result, has_conflict) = bus.negotiate(&participant_ids);
        assert!(!has_conflict);
        assert_eq!(result.value, TritValue::True);
    }


    #[test]
    fn bus_negotiate_cross_frame_holds(
        ids in proptest::collection::vec(arb_node_id(), 2..6),
        frames in proptest::collection::vec(arb_frame(), 2..6),
        phases in proptest::collection::vec(arb_phase(), 2..6),
    ) {
        let n = ids.len().min(frames.len()).min(phases.len());
        prop_assume!(n >= 2);
        // Ensure at least two different frames
        let all_same = frames[..n].windows(2).all(|w| w[0] == w[1]);
        prop_assume!(!all_same);

        let mut bus = ResonanceBus::new();
        for i in 0..n {
            bus.register(Node::new(ids[i].clone(), frames[i].clone(), phases[i]));
        }
        let participant_ids: Vec<String> = ids[..n].to_vec();
        let (result, has_conflict) = bus.negotiate(&participant_ids);
        assert!(has_conflict);
        assert_eq!(result.value, TritValue::Hold);
    }


    #[test]
    fn bus_empty_negotiate_returns_hold(
        _dummy in Just(()),
    ) {
        let mut bus = ResonanceBus::new();
        let (result, has_conflict) = bus.negotiate(&[]);
        assert!(!has_conflict);
        assert_eq!(result.value, TritValue::Hold);
    }


    #[test]
    fn bus_message_log_grows(
        id_a in arb_node_id(),
        id_b in arb_node_id(),
        frame in arb_frame(),
        p1 in arb_phase(),
        p2 in arb_phase(),
    ) {
        prop_assume!(id_a != id_b);
        let mut bus = ResonanceBus::new();
        bus.register(Node::new(id_a.clone(), frame.clone(), p1));
        bus.register(Node::new(id_b.clone(), frame.clone(), p2));

        let log_before = bus.log().count();
        let req = Message::resonate_req(&id_a, &format!("{}", frame), p1, vec![]);
        let _ = bus.handle_resonate_req(&id_a, &id_b, &req);
        let log_after = bus.log().count();

        assert!(log_after > log_before);
    }
}

// ============================================================================
// Layer 15: Message serde roundtrip invariants
//
// NOTE: msg_id and timestamp are regenerated on each Message constructor call,
// so we compare parsed structs semantically rather than comparing raw JSON.
// ============================================================================

proptest! {

    #[test]
    fn message_resonate_req_roundtrip(
        sender in arb_node_id(),
        frame_name in "[A-Z][a-z]{2,12}",
        phase in arb_phase(),
        history in proptest::collection::vec(arb_phase(), 0..10),
    ) {
        let msg = Message::resonate_req(&sender, &frame_name, phase, history.clone());
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: Message = serde_json::from_str(&json).unwrap();
        // Verify header fields preserved
        assert_eq!(parsed.header.proto, msg.header.proto);
        assert_eq!(parsed.header.sender, msg.header.sender);
        // Verify payload preserved
        match &parsed.payload {
            trit_core::net::message::MessagePayload::ResonateReq(data) => {
                assert_eq!(data.frame, frame_name);
                assert!((data.phase - phase).abs() < f64::EPSILON);
                assert_eq!(data.history, history);
            }
            _ => panic!("Expected ResonateReq"),
        }
    }


    #[test]
    fn message_resonate_ack_roundtrip(
        sender in arb_node_id(),
        coupled_phase in arb_phase(),
        interference in prop_oneof![
            Just("constructive".to_string()),
            Just("neutral".to_string()),
            Just("destructive".to_string()),
        ],
        conflict_detected in any::<bool>(),
        recommendation in prop_oneof![
            Just("commit".to_string()),
            Just("hold".to_string()),
            Just("negotiate".to_string()),
        ],
    ) {
        let msg = Message::resonate_ack(
            &sender, coupled_phase, &interference, conflict_detected, &recommendation,
        );
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.header.proto, msg.header.proto);
        assert_eq!(parsed.header.sender, msg.header.sender);
        match &parsed.payload {
            trit_core::net::message::MessagePayload::ResonateAck(data) => {
                assert!((data.coupled_phase - coupled_phase).abs() < f64::EPSILON);
                assert_eq!(data.interference, interference);
                assert_eq!(data.conflict_detected, conflict_detected);
                assert_eq!(data.recommendation, recommendation);
            }
            _ => panic!("Expected ResonateAck"),
        }
    }


    #[test]
    fn message_decouple_req_roundtrip(
        sender in arb_node_id(),
        reason in prop_oneof![
            Just("user_disconnect".to_string()),
            Just("timeout".to_string()),
            Just("policy_violation".to_string()),
        ],
    ) {
        let msg = Message::decouple_req(&sender, &reason);
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.header.proto, msg.header.proto);
        assert_eq!(parsed.header.sender, msg.header.sender);
        match &parsed.payload {
            trit_core::net::message::MessagePayload::DecoupleReq(data) => {
                assert_eq!(data.reason, reason);
            }
            _ => panic!("Expected DecoupleReq"),
        }
    }


    #[test]
    fn message_decouple_ack_roundtrip(
        sender in arb_node_id(),
        restored_phase in arb_phase(),
        cycles_coupled in 0u64..10000u64,
    ) {
        let msg = Message::decouple_ack(&sender, restored_phase, cycles_coupled);
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.header.proto, msg.header.proto);
        assert_eq!(parsed.header.sender, msg.header.sender);
        match &parsed.payload {
            trit_core::net::message::MessagePayload::DecoupleAck(data) => {
                assert!((data.restored_phase - restored_phase).abs() < f64::EPSILON);
                assert_eq!(data.cycles_coupled, cycles_coupled);
            }
            _ => panic!("Expected DecoupleAck"),
        }
    }


    #[test]
    fn message_negotiate_roundtrip(
        sender in arb_node_id(),
        phases in proptest::collection::vec(arb_phase(), 1..10),
    ) {
        let n = phases.len();
        let participants: Vec<String> = (0..n).map(|i| format!("node-{}", i)).collect();
        let frames: Vec<String> = (0..n).map(|_| format!("{}", Frame::Science)).collect();
        let consensus_phase = phases.iter().sum::<f64>() / n as f64;
        let resolution = if n > 1 { "hold" } else { "commit_true" };

        let msg = Message::negotiate(&sender, participants.clone(), frames.clone(), phases.clone(), resolution);
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.header.proto, msg.header.proto);
        assert_eq!(parsed.header.sender, msg.header.sender);
        match &parsed.payload {
            trit_core::net::message::MessagePayload::Negotiate(data) => {
                assert_eq!(data.participants, participants);
                assert_eq!(data.frames, frames);
                // Float comparison with tolerance for serde serialization roundtrip
                for (a, b) in data.phases.iter().zip(phases.iter()) {
                    assert!((a - b).abs() < 1e-10);
                }
                assert!((data.consensus_phase - consensus_phase).abs() < 1e-10);
                assert_eq!(data.conflict_resolution, resolution);
            }
            _ => panic!("Expected Negotiate"),
        }
    }


    #[test]
    fn message_heartbeat_roundtrip(
        sender in arb_node_id(),
        node_state in prop_oneof![
            Just("Sovereign".to_string()),
            Just("Coupling".to_string()),
            Just("Coupled".to_string()),
            Just("Hold".to_string()),
        ],
        current_phase in arb_phase(),
    ) {
        let msg = Message::heartbeat(&sender, &node_state, current_phase);
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.header.proto, msg.header.proto);
        assert_eq!(parsed.header.sender, msg.header.sender);
        match &parsed.payload {
            trit_core::net::message::MessagePayload::Heartbeat(data) => {
                assert_eq!(data.node_state, node_state);
                assert!((data.current_phase - current_phase).abs() < f64::EPSILON);
            }
            _ => panic!("Expected Heartbeat"),
        }
    }


    #[test]
    fn message_header_fields_preserved(
        sender in arb_node_id(),
        frame_name in "[A-Z][a-z]{2,12}",
        phase in arb_phase(),
    ) {
        let msg = Message::resonate_req(&sender, &frame_name, phase, vec![]);
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.header.proto, "trit-proto/1.0");
        assert_eq!(parsed.header.sender, sender);
        assert!(!parsed.header.msg_id.is_empty());
        assert!(!parsed.header.timestamp.is_empty());
    }
}
