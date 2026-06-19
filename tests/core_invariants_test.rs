use trit_core::core::frame::Frame;
use trit_core::core::phase::{Commitment, Phase, PhaseError};
use trit_core::core::value::TritValue;
use trit_core::core::word::{TritWord, WordError};

#[test]
fn phase_strict_rejects_nan() {
    assert!(matches!(
        Phase::new(f64::NAN),
        Err(PhaseError::NotFinite(_))
    ));
}

#[test]
fn phase_strict_rejects_out_of_range() {
    assert!(matches!(Phase::new(1.5), Err(PhaseError::OutOfRange(_))));
    assert!(matches!(Phase::new(-0.1), Err(PhaseError::OutOfRange(_))));
}

#[test]
fn phase_clamped_maps_nan_to_neutral() {
    assert_eq!(Phase::new_clamped(f64::NAN).inner(), 0.5);
}

#[test]
fn phase_commitment_neutral_at_half() {
    assert_eq!(Phase::new(0.5).unwrap().commitment(), Commitment::Neutral);
}

#[test]
fn trit_word_absolute_factory_is_hold_neutral() {
    let a = TritWord::absolute();
    assert_eq!(a.value(), TritValue::Hold);
    assert_eq!(a.phase().inner(), 0.5);
    assert_eq!(a.frame(), Frame::Absolute);
}

#[test]
fn trit_word_from_parts_enforces_absolute_invariant() {
    let result = TritWord::from_parts(TritValue::True, Phase::new(0.8).unwrap(), Frame::Absolute);
    assert!(matches!(result, Err(WordError::AbsoluteInvariant)));
}

#[test]
fn trit_word_with_value_preserves_absolute_invariant() {
    let a = TritWord::absolute();
    assert!(matches!(
        a.with_value(TritValue::True),
        Err(WordError::AbsoluteInvariant)
    ));
    let ok = a.with_value(TritValue::Hold).unwrap();
    assert_eq!(ok.value(), TritValue::Hold);
}

#[test]
fn trit_word_try_new_rejects_bad_frame() {
    assert!(TritWord::try_new(TritValue::True, 0.5, "Mythical").is_err());
}

#[test]
fn trit_word_try_new_accepts_valid_input() {
    let w = TritWord::try_new(TritValue::True, 0.8, "Science").unwrap();
    assert_eq!(w.value(), TritValue::True);
    assert_eq!(w.frame(), Frame::Science);
}
