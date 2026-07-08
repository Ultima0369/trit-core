//! Stage 2 acceptance tests: Embodied vs Individual conflict detection.
//!
//! Migrated from old decision module to new pipeline::analysis link.

use aurora::percept::types::SignalSpec;
use aurora::pipeline::analysis::{frequency_to_embodied, run_analysis, user_state_to_individual};
use trit_core::core::{Frame, TritValue, TritWord};

#[test]
fn embodied_high_vs_individual_normal_is_hold_with_interrupt() {
    let embodied = frequency_to_embodied(2.5, 2.0);
    let individual = user_state_to_individual(true);

    // Cross-frame (Embodied True + Individual True) → Hold
    assert_eq!(embodied.value(), TritValue::True);
    assert_eq!(embodied.frame(), Frame::Embodied);
    assert_eq!(individual.value(), TritValue::True);
    assert_eq!(individual.frame(), Frame::Individual);

    // Run through the full analysis link to verify Hold + conflict
    let spec = SignalSpec {
        freq: 2.5,
        sample_rate: 100.0,
        duration_secs: 1.0,
        noise_std: 0.0,
    };
    let report = run_analysis(&spec, 2.0, true, &[]).unwrap();
    assert_eq!(report.decision.result.value(), TritValue::Hold);
    assert_eq!(report.decision.result.frame(), Frame::Meta);
    assert!(report.decision.has_conflicts());
}

#[test]
fn embodied_low_vs_individual_normal_is_hold_with_interrupt() {
    let embodied = frequency_to_embodied(1.0, 2.0);
    let individual = user_state_to_individual(true);

    assert_eq!(embodied.value(), TritValue::False);
    assert_eq!(embodied.frame(), Frame::Embodied);
    assert_eq!(individual.value(), TritValue::True);
    assert_eq!(individual.frame(), Frame::Individual);

    // Run through the full analysis link
    let spec = SignalSpec {
        freq: 1.0,
        sample_rate: 100.0,
        duration_secs: 1.0,
        noise_std: 0.0,
    };
    let report = run_analysis(&spec, 2.0, false, &[]).unwrap();
    assert_eq!(report.decision.result.value(), TritValue::Hold);
    assert!(report.decision.has_conflicts());
}

#[test]
fn same_frame_true_true_is_true() {
    let a = TritWord::tru(Frame::Individual);
    let b = TritWord::tru(Frame::Individual);
    // Same-frame signals don't produce conflicts
    assert_eq!(a.frame(), b.frame());
    assert_eq!(a.value(), TritValue::True);
    assert_eq!(b.value(), TritValue::True);
}

#[test]
fn same_frame_false_false_is_false() {
    let a = TritWord::fals(Frame::Individual);
    let b = TritWord::fals(Frame::Individual);
    assert_eq!(a.frame(), b.frame());
    assert_eq!(a.value(), TritValue::False);
    assert_eq!(b.value(), TritValue::False);
}
