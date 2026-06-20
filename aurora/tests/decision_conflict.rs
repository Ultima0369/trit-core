//! Stage 2 acceptance tests: Embodied vs Individual conflict detection.

use aurora::decision::{detect_conflict, embodied_from_frequency, individual_from_user_state};
use truncore::core::{Frame, TritValue, TritWord};

#[test]
fn embodied_high_vs_individual_normal_is_hold_with_interrupt() {
    let embodied = embodied_from_frequency(2.5, 2.0);
    let individual = individual_from_user_state(true);
    let (result, interrupt) = detect_conflict(&embodied, &individual);
    assert_eq!(result.value(), TritValue::Hold);
    assert_eq!(result.frame(), Frame::Meta);
    assert!(interrupt.is_some());
}

#[test]
fn embodied_low_vs_individual_normal_is_hold_with_interrupt() {
    let embodied = embodied_from_frequency(1.0, 2.0);
    let individual = individual_from_user_state(true);
    let (result, interrupt) = detect_conflict(&embodied, &individual);
    assert_eq!(result.value(), TritValue::Hold);
    assert!(interrupt.is_some());
}

#[test]
fn same_frame_true_true_is_true() {
    let a = TritWord::tru(Frame::Individual);
    let b = TritWord::tru(Frame::Individual);
    let (result, interrupt) = detect_conflict(&a, &b);
    assert_eq!(result.value(), TritValue::True);
    assert!(interrupt.is_none());
}

#[test]
fn same_frame_false_false_is_false() {
    let a = TritWord::fals(Frame::Individual);
    let b = TritWord::fals(Frame::Individual);
    let (result, interrupt) = detect_conflict(&a, &b);
    assert_eq!(result.value(), TritValue::False);
    assert!(interrupt.is_none());
}
