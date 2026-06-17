use crate::frame::Frame;
use crate::meta::MetaInterrupt;
use crate::trit::{Phase, TritValue, TritWord};
use tracing::{debug, trace, warn};

/// Harmonic Ternary Algebra (HTA): the core logic engine.
///
/// ## Hot vs Cold Path
///
/// Same-frame operations (hot path) skip MetaMonitor entirely — they account
/// for ~80% of typical decisions. Cross-frame operations (cold path) generate
/// MetaInterrupt events and trigger policy arbitration.
pub struct TernaryAlgebra;

impl TernaryAlgebra {
    /// Precheck: returns true if both trits share the same frame.
    /// Callers can use this to decide whether to take the hot path.
    #[inline]
    pub fn precheck_same_frame(a: &TritWord, b: &TritWord) -> bool {
        a.frame == b.frame
    }

    /// Shared cross-frame conflict handler — used by both TAND and TOR.
    /// Uses `MetaInterrupt::with_frames` to avoid `format!()` overhead.
    fn cross_frame_conflict(
        op_name: &'static str,
        a: &TritWord,
        b: &TritWord,
    ) -> (TritWord, Option<MetaInterrupt>) {
        let hold = TritWord::new(TritValue::Hold, 0.5, Frame::Meta);
        let interrupt = MetaInterrupt::with_frames(op_name, a.frame.clone(), b.frame.clone());
        if tracing::enabled!(tracing::Level::WARN) {
            warn!(op = op_name, a = %a.frame, b = %b.frame, "cross-frame conflict detected");
        }
        (hold, Some(interrupt))
    }

    /// TAND: harmonic conjunction.
    /// - Same frame: standard ternary logic with phase averaging (hot path).
    /// - Different frame: produces Hold + triggers MetaInterrupt (cold path).
    #[tracing::instrument(skip_all, fields(op = "t_and"))]
    pub fn t_and(a: &TritWord, b: &TritWord) -> (TritWord, Option<MetaInterrupt>) {
        trace!(a_frame = %a.frame, a_value = ?a.value, a_phase = a.phase.inner(),
               b_frame = %b.frame, b_value = ?b.value, b_phase = b.phase.inner(),
               "entering TAND");

        if a.frame != b.frame {
            return Self::cross_frame_conflict("TAND", a, b);
        }

        let val = match (a.value, b.value) {
            (TritValue::True, TritValue::True) => TritValue::True,
            (TritValue::Unknown, _) | (_, TritValue::Unknown) => TritValue::Unknown,
            (TritValue::False, _) | (_, TritValue::False) => TritValue::False,
            _ => TritValue::Hold,
        };

        let phase = Phase::mean(a.phase, b.phase);
        debug!(result_value = ?val, result_phase = phase.inner(), "TAND same-frame computed");
        (TritWord::new(val, phase.inner(), a.frame.clone()), None)
    }

    /// TAND hot path: same-frame only, no MetaInterrupt allocation.
    /// Panics if frames differ — callers must precheck with [`precheck_same_frame`].
    #[inline]
    pub fn t_and_hot(a: &TritWord, b: &TritWord) -> TritWord {
        debug_assert_eq!(a.frame, b.frame, "t_and_hot requires same frame");

        let val = match (a.value, b.value) {
            (TritValue::True, TritValue::True) => TritValue::True,
            (TritValue::Unknown, _) | (_, TritValue::Unknown) => TritValue::Unknown,
            (TritValue::False, _) | (_, TritValue::False) => TritValue::False,
            _ => TritValue::Hold,
        };

        let phase = Phase::mean(a.phase, b.phase);
        TritWord::new(val, phase.inner(), a.frame.clone())
    }

    /// TOR: harmonic disjunction.
    #[tracing::instrument(skip_all, fields(op = "t_or"))]
    pub fn t_or(a: &TritWord, b: &TritWord) -> (TritWord, Option<MetaInterrupt>) {
        trace!(a_frame = %a.frame, b_frame = %b.frame, "entering TOR");

        if a.frame != b.frame {
            return Self::cross_frame_conflict("TOR", a, b);
        }

        let val = match (a.value, b.value) {
            (TritValue::True, _) | (_, TritValue::True) => TritValue::True,
            (TritValue::Unknown, TritValue::Unknown) => TritValue::Unknown,
            (TritValue::False, TritValue::False) => TritValue::False,
            _ => TritValue::Hold,
        };

        let phase = Phase::mean(a.phase, b.phase);
        (TritWord::new(val, phase.inner(), a.frame.clone()), None)
    }

    /// TOR hot path: same-frame only, no MetaInterrupt allocation.
    #[inline]
    pub fn t_or_hot(a: &TritWord, b: &TritWord) -> TritWord {
        debug_assert_eq!(a.frame, b.frame, "t_or_hot requires same frame");

        let val = match (a.value, b.value) {
            (TritValue::True, _) | (_, TritValue::True) => TritValue::True,
            (TritValue::Unknown, TritValue::Unknown) => TritValue::Unknown,
            (TritValue::False, TritValue::False) => TritValue::False,
            _ => TritValue::Hold,
        };

        let phase = Phase::mean(a.phase, b.phase);
        TritWord::new(val, phase.inner(), a.frame.clone())
    }

    /// TNOT: phase-flipped negation.
    #[tracing::instrument(skip_all)]
    pub fn t_not(a: &TritWord) -> TritWord {
        let val = a.value.negate();
        let phase = a.phase.complement();
        TritWord::new(val, phase.inner(), a.frame.clone())
    }

    /// THOLD: force into Hold state (meta-monitor instruction).
    pub fn t_hold(a: &TritWord) -> TritWord {
        TritWord::new(TritValue::Hold, 0.5, a.frame.clone())
    }

    /// TSENSE: create a Hold from raw sensor input.
    pub fn t_sense(phase: f64, frame: Frame) -> TritWord {
        TritWord::new(TritValue::Hold, phase, frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Hot path tests
    // -----------------------------------------------------------------------

    #[test]
    fn precheck_same_frame_returns_true() {
        let a = TritWord::tru(Frame::Science);
        let b = TritWord::fals(Frame::Science);
        assert!(TernaryAlgebra::precheck_same_frame(&a, &b));
    }

    #[test]
    fn precheck_different_frame_returns_false() {
        let a = TritWord::tru(Frame::Science);
        let b = TritWord::fals(Frame::Individual);
        assert!(!TernaryAlgebra::precheck_same_frame(&a, &b));
    }

    #[test]
    fn tand_hot_same_frame_returns_no_interrupt() {
        let a = TritWord::tru(Frame::Science);
        let b = TritWord::fals(Frame::Science);
        let result = TernaryAlgebra::t_and_hot(&a, &b);
        assert_eq!(result.value, TritValue::False);
        assert_eq!(result.frame, Frame::Science);
    }

    #[test]
    fn tor_hot_same_frame_returns_no_interrupt() {
        let a = TritWord::tru(Frame::Science);
        let b = TritWord::fals(Frame::Science);
        let result = TernaryAlgebra::t_or_hot(&a, &b);
        assert_eq!(result.value, TritValue::True);
    }

    #[test]
    #[should_panic(expected = "t_and_hot requires same frame")]
    fn tand_hot_different_frame_panics_in_debug() {
        let a = TritWord::tru(Frame::Science);
        let b = TritWord::tru(Frame::Individual);
        TernaryAlgebra::t_and_hot(&a, &b);
    }

    // -----------------------------------------------------------------------
    // Unknown propagation tests
    // -----------------------------------------------------------------------

    #[test]
    fn tand_with_unknown_propagates_unknown() {
        let a = TritWord::new(TritValue::Unknown, 0.5, Frame::Science);
        let b = TritWord::tru(Frame::Science);
        let (res, int) = TernaryAlgebra::t_and(&a, &b);
        assert_eq!(res.value, TritValue::Unknown);
        assert!(int.is_none());
    }

    #[test]
    fn tor_unknown_unknown_is_unknown() {
        let a = TritWord::new(TritValue::Unknown, 0.5, Frame::Science);
        let b = TritWord::new(TritValue::Unknown, 0.5, Frame::Science);
        let (res, int) = TernaryAlgebra::t_or(&a, &b);
        assert_eq!(res.value, TritValue::Unknown);
        assert!(int.is_none());
    }

    #[test]
    fn tor_true_dominates_unknown() {
        // True OR Unknown → True (True dominates OR)
        let a = TritWord::new(TritValue::True, 0.7, Frame::Science);
        let b = TritWord::new(TritValue::Unknown, 0.5, Frame::Science);
        let (res, int) = TernaryAlgebra::t_or(&a, &b);
        assert_eq!(res.value, TritValue::True);
        assert!(int.is_none());
    }

    #[test]
    fn tnot_unknown_remains_unknown() {
        let a = TritWord::new(TritValue::Unknown, 0.5, Frame::Science);
        let res = TernaryAlgebra::t_not(&a);
        assert_eq!(res.value, TritValue::Unknown);
    }
}
