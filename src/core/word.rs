use crate::core::frame::{Frame, FrameError};
use crate::core::phase::{Phase, PhaseError};
use crate::core::value::TritValue;
use thiserror::Error;

/// A ternary word: the fundamental unit of computation in Trit-Core.
///
/// Unlike a binary bit (0/1), a trit carries:
/// - `value`: ternary state {True, Hold, False} + out-of-distribution {Unknown}
/// - `phase`: continuous tendency 0.0..1.0 (0.5 = neutral)
/// - `frame`: the decision domain / context this trit belongs to
///
/// # Invariants
///
/// - `phase` is always finite and within `[0.0, 1.0]`.
/// - If `frame == Frame::Absolute`, then `value == TritValue::Hold` and
///   `phase == Phase(0.5)`.
///
/// These invariants are enforced by the constructors. Use the `with_*`
/// methods to transform a word while preserving invariants.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TritWord {
    value: TritValue,
    phase: Phase,
    frame: Frame,
}

/// Error type for [`TritWord`] construction and transformation.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum WordError {
    #[error("invalid phase: {0}")]
    Phase(#[from] PhaseError),
    #[error("invalid frame: {0}")]
    Frame(#[from] FrameError),
    #[error("Frame::Absolute requires TritValue::Hold and Phase(0.5)")]
    AbsoluteInvariant,
}

impl TritWord {
    /// Construct a `TritWord` from already-validated components.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if the `Frame::Absolute` invariant is violated.
    /// In release builds this is a best-effort assertion; prefer
    /// [`TritWord::try_new`] when constructing from untrusted `f64` input.
    pub fn new(value: TritValue, phase: Phase, frame: Frame) -> Self {
        let word = Self {
            value,
            phase,
            frame,
        };
        debug_assert!(
            word.invariant_holds(),
            "TritWord invariant violated: Absolute frame must be Hold + neutral phase"
        );
        word
    }

    /// Construct a `TritWord` from raw `f64` phase, validating everything.
    pub fn try_new(value: TritValue, phase: f64, frame: &str) -> Result<Self, WordError> {
        let frame = frame.parse::<Frame>()?;
        let phase = Phase::new(phase)?;
        Self::from_parts(value, phase, frame)
    }

    /// Construct from validated parts, enforcing the Absolute invariant.
    pub fn from_parts(value: TritValue, phase: Phase, frame: Frame) -> Result<Self, WordError> {
        let word = Self {
            value,
            phase,
            frame,
        };
        if !word.invariant_holds() {
            return Err(WordError::AbsoluteInvariant);
        }
        Ok(word)
    }

    /// Create a neutral hold-state trit.
    pub fn hold(frame: Frame) -> Self {
        // Hold with neutral phase is always valid.
        Self::new(TritValue::Hold, Phase::neutral(), frame)
    }

    /// Create a fully committed true trit.
    pub fn tru(frame: Frame) -> Self {
        Self::new(TritValue::True, Phase::full_true(), frame)
    }

    /// Create a fully committed false trit.
    ///
    /// Named `fals` rather than `false` to avoid collision with the Rust keyword.
    pub fn fals(frame: Frame) -> Self {
        Self::new(TritValue::False, Phase::full_false(), frame)
    }

    /// Create an out-of-distribution trit (unknowable input).
    pub fn unknown(frame: Frame) -> Self {
        Self::new(TritValue::Unknown, Phase::neutral(), frame)
    }

    /// Create the special `Absolute` frame word: always Hold + neutral.
    pub fn absolute() -> Self {
        Self::new(TritValue::Hold, Phase::neutral(), Frame::Absolute)
    }

    /// Read the ternary value.
    #[inline]
    pub fn value(&self) -> TritValue {
        self.value
    }

    /// Read the phase tendency.
    #[inline]
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// Read the frame.
    #[inline]
    pub fn frame(&self) -> Frame {
        self.frame
    }

    /// Return a new word with the given value, preserving frame and phase.
    ///
    /// # Errors
    ///
    /// Returns `WordError::AbsoluteInvariant` if the new value is not `Hold`
    /// when the frame is `Absolute`.
    pub fn with_value(&self, value: TritValue) -> Result<Self, WordError> {
        Self::from_parts(value, self.phase, self.frame)
    }

    /// Return a new word with the given phase, preserving value and frame.
    ///
    /// # Errors
    ///
    /// Returns `WordError::AbsoluteInvariant` if the frame is `Absolute` and
    /// the phase is not neutral.
    pub fn with_phase(&self, phase: Phase) -> Result<Self, WordError> {
        Self::from_parts(self.value, phase, self.frame)
    }

    /// Return a new word with the given frame, preserving value and phase.
    ///
    /// # Errors
    ///
    /// Returns `WordError::AbsoluteInvariant` if the target frame is
    /// `Absolute` but value/phase do not match the required Hold/neutral.
    pub fn with_frame(&self, frame: Frame) -> Result<Self, WordError> {
        Self::from_parts(self.value, self.phase, frame)
    }

    /// Check whether this word's invariants hold.
    #[inline]
    pub fn invariant_holds(&self) -> bool {
        if self.frame == Frame::Absolute {
            self.value == TritValue::Hold && (self.phase.inner() - 0.5).abs() < f64::EPSILON
        } else {
            true
        }
    }
}

/// Type alias kept for backward naming within the 0.2 refactor.
pub type Trit = TritWord;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors_build_valid_words() {
        let h = TritWord::hold(Frame::Science);
        assert_eq!(h.value(), TritValue::Hold);
        assert_eq!(h.frame(), Frame::Science);

        let t = TritWord::tru(Frame::Individual);
        assert_eq!(t.value(), TritValue::True);
        assert_eq!(t.phase().inner(), 1.0);

        let f = TritWord::fals(Frame::Consensus);
        assert_eq!(f.value(), TritValue::False);
        assert_eq!(f.phase().inner(), 0.0);

        let u = TritWord::unknown(Frame::Meta);
        assert_eq!(u.value(), TritValue::Unknown);
    }

    #[test]
    fn absolute_factory_is_hold_neutral() {
        let a = TritWord::absolute();
        assert_eq!(a.value(), TritValue::Hold);
        assert_eq!(a.phase().inner(), 0.5);
        assert_eq!(a.frame(), Frame::Absolute);
    }

    #[test]
    fn try_new_valid() {
        let w = TritWord::try_new(TritValue::True, 0.8, "Science").unwrap();
        assert_eq!(w.value(), TritValue::True);
        assert_eq!(w.frame(), Frame::Science);
    }

    #[test]
    fn try_new_rejects_bad_phase() {
        assert!(TritWord::try_new(TritValue::True, 1.5, "Science").is_err());
        assert!(TritWord::try_new(TritValue::True, -0.1, "Science").is_err());
        assert!(TritWord::try_new(TritValue::True, f64::NAN, "Science").is_err());
        assert!(TritWord::try_new(TritValue::True, f64::INFINITY, "Science").is_err());
    }

    #[test]
    fn try_new_rejects_unknown_frame() {
        assert!(TritWord::try_new(TritValue::True, 0.5, "Bogus").is_err());
        assert!(TritWord::try_new(TritValue::True, 0.5, "").is_err());
    }

    #[test]
    fn new_and_accessors_roundtrip() {
        let word = TritWord::new(
            TritValue::Hold,
            Phase::new(0.25).unwrap(),
            Frame::Individual,
        );
        assert_eq!(word.value(), TritValue::Hold);
        assert_eq!(word.phase().inner(), 0.25);
        assert_eq!(word.frame(), Frame::Individual);
    }

    #[test]
    fn from_parts_enforces_absolute_invariant() {
        let bad = TritWord::from_parts(TritValue::True, Phase::new(0.8).unwrap(), Frame::Absolute);
        assert!(matches!(bad, Err(WordError::AbsoluteInvariant)));
    }

    #[test]
    fn from_parts_succeeds_for_valid_word() {
        let word = TritWord::from_parts(TritValue::True, Phase::new(0.8).unwrap(), Frame::Science)
            .unwrap();
        assert_eq!(word.value(), TritValue::True);
        assert_eq!(word.frame(), Frame::Science);
    }

    #[test]
    fn with_value_preserves_invariant() {
        let a = TritWord::absolute();
        assert!(matches!(
            a.with_value(TritValue::True),
            Err(WordError::AbsoluteInvariant)
        ));
        let ok = a.with_value(TritValue::Hold).unwrap();
        assert_eq!(ok.value(), TritValue::Hold);
    }

    #[test]
    fn with_value_succeeds_for_non_absolute() {
        let h = TritWord::hold(Frame::Science);
        let t = h.with_value(TritValue::True).unwrap();
        assert_eq!(t.value(), TritValue::True);
        assert_eq!(t.frame(), Frame::Science);
    }

    #[test]
    fn with_phase_preserves_invariant() {
        let a = TritWord::absolute();
        let bad_phase = Phase::new(0.8).unwrap();
        assert!(matches!(
            a.with_phase(bad_phase),
            Err(WordError::AbsoluteInvariant)
        ));
    }

    #[test]
    fn with_phase_succeeds_for_non_absolute() {
        let h = TritWord::hold(Frame::Science);
        let p = h.with_phase(Phase::new(0.9).unwrap()).unwrap();
        assert_eq!(p.phase().inner(), 0.9);
    }

    #[test]
    fn with_frame_enforces_absolute_on_target() {
        let s = TritWord::tru(Frame::Science);
        assert!(matches!(
            s.with_frame(Frame::Absolute),
            Err(WordError::AbsoluteInvariant)
        ));

        let a = TritWord::absolute();
        let s2 = a.with_frame(Frame::Science).unwrap();
        assert_eq!(s2.frame(), Frame::Science);
    }

    #[test]
    fn with_frame_succeeds_for_non_absolute_target() {
        let s = TritWord::tru(Frame::Science);
        let i = s.with_frame(Frame::Individual).unwrap();
        assert_eq!(i.frame(), Frame::Individual);
    }

    #[test]
    fn invariant_holds_detects_violation_when_bypassed() {
        // from_parts is the only public way to build a TritWord from raw parts.
        // It returns an error, so no invalid word can be observed.
        let bad = TritWord::from_parts(TritValue::True, Phase::new(0.5).unwrap(), Frame::Absolute);
        assert!(matches!(bad, Err(WordError::AbsoluteInvariant)));
    }

    #[test]
    fn error_display_is_informative() {
        let err = TritWord::try_new(TritValue::True, 1.5, "Science").unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("1.5") || msg.contains("out of range") || msg.contains("Phase"));
    }
}
