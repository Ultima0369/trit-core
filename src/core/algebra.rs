// ===== 核心提醒 - 来自与一个清醒心智的对话 =====
// 1. Hold 不是失败，是有意的悬置。
// 2. 跨帧冲突不该被抹平，而应被可审计地记录。
// 3. 自知是知人的前提：先知道自己如何陷入，再推测他人。
// 4. “追光”追不上，所以要学会“停一下”而不是更快。
// 5. 真正的逻辑是经得起生死检验的因果推断，不是书斋里的口才。
// ================================================

use crate::core::frame::Frame;
use crate::core::phase::{Phase, PhaseError};
use crate::core::value::TritValue;
use crate::core::word::TritWord;
use crate::meta::MetaInterrupt;
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
        a.frame() == b.frame()
    }

    /// Shared cross-frame conflict handler — used by both TAND and TOR.
    /// Uses `MetaInterrupt::with_frames` to avoid `format!()` overhead.
    fn cross_frame_conflict(
        op_name: &'static str,
        a: &TritWord,
        b: &TritWord,
    ) -> (TritWord, Option<MetaInterrupt>) {
        let hold = TritWord::hold(Frame::Meta);
        let interrupt = MetaInterrupt::with_frames(op_name, a.frame(), b.frame());
        if tracing::enabled!(tracing::Level::WARN) {
            warn!(op = op_name, a = %a.frame(), b = %b.frame(), "cross-frame conflict detected");
        }
        (hold, Some(interrupt))
    }

    /// TAND: harmonic conjunction.
    /// - Same frame: standard ternary logic with phase averaging (hot path).
    /// - Different frame: produces Hold + triggers MetaInterrupt (cold path).
    #[tracing::instrument(skip_all, fields(op = "t_and"))]
    pub fn t_and(a: &TritWord, b: &TritWord) -> (TritWord, Option<MetaInterrupt>) {
        trace!(
            a_frame = %a.frame(),
            a_value = ?a.value(),
            a_phase = a.phase().inner(),
            b_frame = %b.frame(),
            b_value = ?b.value(),
            b_phase = b.phase().inner(),
            "entering TAND"
        );

        if a.frame() != b.frame() {
            return Self::cross_frame_conflict("TAND", a, b);
        }

        let val = match (a.value(), b.value()) {
            (TritValue::True, TritValue::True) => TritValue::True,
            (TritValue::Unknown, _) | (_, TritValue::Unknown) => TritValue::Unknown,
            (TritValue::False, _) | (_, TritValue::False) => TritValue::False,
            _ => TritValue::Hold,
        };

        let phase = Phase::mean(a.phase(), b.phase());
        debug!(result_value = ?val, result_phase = phase.inner(), "TAND same-frame computed");
        (TritWord::new(val, phase, a.frame()), None)
    }

    /// TAND hot path: same-frame only, no MetaInterrupt allocation.
    ///
    /// # Panics
    ///
    /// Panics if frames differ. This is the unchecked fast path; callers **must**
    /// precheck with [`Self::precheck_same_frame`]. The assertion is active in all
    /// build modes (not `debug_assert`) to prevent silent wrong results in release.
    #[inline]
    pub fn t_and_hot(a: &TritWord, b: &TritWord) -> TritWord {
        assert_eq!(a.frame(), b.frame(), "t_and_hot requires same frame");

        let val = match (a.value(), b.value()) {
            (TritValue::True, TritValue::True) => TritValue::True,
            (TritValue::Unknown, _) | (_, TritValue::Unknown) => TritValue::Unknown,
            (TritValue::False, _) | (_, TritValue::False) => TritValue::False,
            _ => TritValue::Hold,
        };

        let phase = Phase::mean(a.phase(), b.phase());
        TritWord::new(val, phase, a.frame())
    }

    /// TOR: harmonic disjunction.
    #[tracing::instrument(skip_all, fields(op = "t_or"))]
    pub fn t_or(a: &TritWord, b: &TritWord) -> (TritWord, Option<MetaInterrupt>) {
        trace!(a_frame = %a.frame(), b_frame = %b.frame(), "entering TOR");

        if a.frame() != b.frame() {
            return Self::cross_frame_conflict("TOR", a, b);
        }

        let val = match (a.value(), b.value()) {
            (TritValue::True, _) | (_, TritValue::True) => TritValue::True,
            (TritValue::Unknown, TritValue::Unknown) => TritValue::Unknown,
            (TritValue::False, TritValue::False) => TritValue::False,
            _ => TritValue::Hold,
        };

        let phase = Phase::mean(a.phase(), b.phase());
        (TritWord::new(val, phase, a.frame()), None)
    }

    /// TOR hot path: same-frame only, no MetaInterrupt allocation.
    ///
    /// # Panics
    ///
    /// Panics if frames differ. See [`Self::t_and_hot`] for rationale.
    #[inline]
    pub fn t_or_hot(a: &TritWord, b: &TritWord) -> TritWord {
        assert_eq!(a.frame(), b.frame(), "t_or_hot requires same frame");

        let val = match (a.value(), b.value()) {
            (TritValue::True, _) | (_, TritValue::True) => TritValue::True,
            (TritValue::Unknown, TritValue::Unknown) => TritValue::Unknown,
            (TritValue::False, TritValue::False) => TritValue::False,
            _ => TritValue::Hold,
        };

        let phase = Phase::mean(a.phase(), b.phase());
        TritWord::new(val, phase, a.frame())
    }

    /// TNOT: phase-flipped negation.
    #[tracing::instrument(skip_all)]
    pub fn t_not(a: &TritWord) -> TritWord {
        let val = a.value().negate();
        let phase = a.phase().complement();
        TritWord::new(val, phase, a.frame())
    }

    /// THOLD: force into Hold state (meta-monitor instruction).
    pub fn t_hold(a: &TritWord) -> TritWord {
        TritWord::new(TritValue::Hold, Phase::neutral(), a.frame())
    }

    /// TSENSE: create a Hold from raw sensor input.
    ///
    /// Returns `Err` if `phase` is not finite and in `[0.0, 1.0]`. For a
    /// non-failing variant that silently clamps invalid inputs, use
    /// [`t_sense_clamped`](Self::t_sense_clamped).
    pub fn t_sense(phase: f64, frame: Frame) -> Result<TritWord, PhaseError> {
        Ok(TritWord::new(TritValue::Hold, Phase::new(phase)?, frame))
    }

    /// TSENSE with silent clamping: create a Hold from raw sensor input.
    ///
    /// Out-of-range or non-finite phase values are clamped to `[0.0, 1.0]`
    /// (NaN/Infinity maps to neutral 0.5). This is useful for untrusted
    /// external sensors where graceful degradation is preferred over hard
    /// failure.
    pub fn t_sense_clamped(phase: f64, frame: Frame) -> TritWord {
        TritWord::new(TritValue::Hold, Phase::new_clamped(phase), frame)
    }

    /// Batch TAND: harmonic conjunction over N inputs.
    ///
    /// Unlike sequential left-fold via `t_and`, this computes Phase as the
    /// arithmetic mean of all input phases, avoiding left-fold bias:
    /// `mean(mean(a,b),c) != (a+b+c)/3` but this method always computes
    /// `(a+b+c)/3` for equal-weight semantics.
    ///
    /// TritValue is computed by cumulatively applying the TAND truth table.
    /// Cross-frame conflicts produce Hold + MetaInterrupt per pair.
    pub fn t_and_n(inputs: &[TritWord]) -> (TritWord, Vec<MetaInterrupt>) {
        if inputs.is_empty() {
            return (TritWord::hold(Frame::Meta), vec![]);
        }
        if inputs.len() == 1 {
            return (inputs[0], vec![]);
        }

        let mut interrupts = vec![];
        let first_frame = inputs[0].frame();

        // Check all same frame
        for word in &inputs[1..] {
            if word.frame() != first_frame {
                let interrupt = MetaInterrupt::with_frames("TAND_N", first_frame, word.frame());
                interrupts.push(interrupt);
            }
        }

        if !interrupts.is_empty() {
            // Cross-frame conflict: return Hold in Meta frame with all interrupts
            return (TritWord::hold(Frame::Meta), interrupts);
        }

        // Same frame: compute value via cumulative TAND semantics
        let mut value = inputs[0].value();
        for word in &inputs[1..] {
            value = match (value, word.value()) {
                (TritValue::True, TritValue::True) => TritValue::True,
                (TritValue::Unknown, _) | (_, TritValue::Unknown) => TritValue::Unknown,
                (TritValue::False, _) | (_, TritValue::False) => TritValue::False,
                _ => TritValue::Hold,
            };
        }

        // Batch Phase: arithmetic mean of all phases (equal weight)
        let phase_sum: f64 = inputs.iter().map(|w| w.phase().inner()).sum();
        let phase = Phase::new_clamped(phase_sum / inputs.len() as f64).quantize(1e-6);

        (TritWord::new(value, phase, first_frame), interrupts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(result.value(), TritValue::False);
        assert_eq!(result.frame(), Frame::Science);
    }

    #[test]
    fn tor_hot_same_frame_returns_no_interrupt() {
        let a = TritWord::tru(Frame::Science);
        let b = TritWord::fals(Frame::Science);
        let result = TernaryAlgebra::t_or_hot(&a, &b);
        assert_eq!(result.value(), TritValue::True);
    }

    #[test]
    fn tor_hot_false_false_is_false() {
        let a = TritWord::fals(Frame::Science);
        let b = TritWord::fals(Frame::Science);
        let result = TernaryAlgebra::t_or_hot(&a, &b);
        assert_eq!(result.value(), TritValue::False);
    }

    #[test]
    fn tor_hot_unknown_unknown_is_unknown() {
        let a = TritWord::unknown(Frame::Science);
        let b = TritWord::unknown(Frame::Science);
        let result = TernaryAlgebra::t_or_hot(&a, &b);
        assert_eq!(result.value(), TritValue::Unknown);
    }

    #[test]
    fn tor_hot_hold_propagates() {
        let a = TritWord::hold(Frame::Science);
        let b = TritWord::fals(Frame::Science);
        let result = TernaryAlgebra::t_or_hot(&a, &b);
        assert_eq!(result.value(), TritValue::Hold);
    }

    #[test]
    #[should_panic(expected = "t_and_hot requires same frame")]
    fn tand_hot_different_frame_panics() {
        let a = TritWord::tru(Frame::Science);
        let b = TritWord::tru(Frame::Individual);
        TernaryAlgebra::t_and_hot(&a, &b);
    }

    #[test]
    fn tand_with_unknown_propagates_unknown() {
        let a = TritWord::unknown(Frame::Science);
        let b = TritWord::tru(Frame::Science);
        let (res, int) = TernaryAlgebra::t_and(&a, &b);
        assert_eq!(res.value(), TritValue::Unknown);
        assert!(int.is_none());
    }

    #[test]
    fn tor_unknown_unknown_is_unknown() {
        let a = TritWord::unknown(Frame::Science);
        let b = TritWord::unknown(Frame::Science);
        let (res, int) = TernaryAlgebra::t_or(&a, &b);
        assert_eq!(res.value(), TritValue::Unknown);
        assert!(int.is_none());
    }

    #[test]
    fn tor_true_dominates_unknown() {
        let a = TritWord::new(TritValue::True, Phase::new(0.7).unwrap(), Frame::Science);
        let b = TritWord::unknown(Frame::Science);
        let (res, int) = TernaryAlgebra::t_or(&a, &b);
        assert_eq!(res.value(), TritValue::True);
        assert!(int.is_none());
    }

    #[test]
    fn tnot_unknown_remains_unknown() {
        let a = TritWord::unknown(Frame::Science);
        let res = TernaryAlgebra::t_not(&a);
        assert_eq!(res.value(), TritValue::Unknown);
    }

    #[test]
    fn tand_cross_frame_returns_hold_and_interrupt() {
        let a = TritWord::tru(Frame::Science);
        let b = TritWord::fals(Frame::Individual);
        let (res, int) = TernaryAlgebra::t_and(&a, &b);
        assert_eq!(res.value(), TritValue::Hold);
        assert_eq!(res.frame(), Frame::Meta);
        assert!(int.is_some());
    }

    #[test]
    fn t_sense_accepts_valid_phase() {
        let word = TernaryAlgebra::t_sense(0.75, Frame::Science).unwrap();
        assert_eq!(word.value(), TritValue::Hold);
        assert_eq!(word.phase().inner(), 0.75);
        assert_eq!(word.frame(), Frame::Science);
    }

    #[test]
    fn t_sense_rejects_nan_phase() {
        assert!(TernaryAlgebra::t_sense(f64::NAN, Frame::Science).is_err());
    }

    #[test]
    fn t_sense_rejects_infinite_phase() {
        assert!(TernaryAlgebra::t_sense(f64::INFINITY, Frame::Science).is_err());
        assert!(TernaryAlgebra::t_sense(f64::NEG_INFINITY, Frame::Science).is_err());
    }

    #[test]
    fn t_sense_rejects_out_of_range_phase() {
        assert!(TernaryAlgebra::t_sense(-0.1, Frame::Science).is_err());
        assert!(TernaryAlgebra::t_sense(1.1, Frame::Science).is_err());
    }

    #[test]
    fn t_sense_clamped_maps_invalid_to_neutral() {
        let word = TernaryAlgebra::t_sense_clamped(f64::NAN, Frame::Science);
        assert_eq!(word.value(), TritValue::Hold);
        assert_eq!(word.phase().inner(), 0.5);
    }

    #[test]
    fn t_sense_clamped_preserves_valid_phase() {
        let word = TernaryAlgebra::t_sense_clamped(0.25, Frame::Individual);
        assert_eq!(word.phase().inner(), 0.25);
    }

    #[test]
    fn tand_truth_table_same_frame() {
        // Exhaustive 4x4 truth table for TAND with same frame.
        let values = [
            TritValue::True,
            TritValue::False,
            TritValue::Hold,
            TritValue::Unknown,
        ];
        let expected = [
            // True, False, Hold, Unknown
            [
                TritValue::True,
                TritValue::False,
                TritValue::Hold,
                TritValue::Unknown,
            ], // True
            [
                TritValue::False,
                TritValue::False,
                TritValue::False,
                TritValue::Unknown,
            ], // False
            [
                TritValue::Hold,
                TritValue::False,
                TritValue::Hold,
                TritValue::Unknown,
            ], // Hold
            [
                TritValue::Unknown,
                TritValue::Unknown,
                TritValue::Unknown,
                TritValue::Unknown,
            ], // Unknown
        ];
        for (i, &a) in values.iter().enumerate() {
            for (j, &b) in values.iter().enumerate() {
                let left = TritWord::new(a, Phase::new(0.8).unwrap(), Frame::Science);
                let right = TritWord::new(b, Phase::new(0.6).unwrap(), Frame::Science);
                let (res, int) = TernaryAlgebra::t_and(&left, &right);
                assert_eq!(
                    res.value(),
                    expected[i][j],
                    "TAND({:?}, {:?}) should be {:?}",
                    a,
                    b,
                    expected[i][j]
                );
                assert!(int.is_none());
            }
        }
    }

    #[test]
    fn tor_truth_table_same_frame() {
        // Exhaustive 4x4 truth table for TOR with same frame.
        let values = [
            TritValue::True,
            TritValue::False,
            TritValue::Hold,
            TritValue::Unknown,
        ];
        let expected = [
            // True, False, Hold, Unknown
            [
                TritValue::True,
                TritValue::True,
                TritValue::True,
                TritValue::True,
            ], // True
            [
                TritValue::True,
                TritValue::False,
                TritValue::Hold,
                TritValue::Hold,
            ], // False
            [
                TritValue::True,
                TritValue::Hold,
                TritValue::Hold,
                TritValue::Hold,
            ], // Hold
            [
                TritValue::True,
                TritValue::Hold,
                TritValue::Hold,
                TritValue::Unknown,
            ], // Unknown
        ];
        for (i, &a) in values.iter().enumerate() {
            for (j, &b) in values.iter().enumerate() {
                let left = TritWord::new(a, Phase::new(0.8).unwrap(), Frame::Science);
                let right = TritWord::new(b, Phase::new(0.6).unwrap(), Frame::Science);
                let (res, int) = TernaryAlgebra::t_or(&left, &right);
                assert_eq!(
                    res.value(),
                    expected[i][j],
                    "TOR({:?}, {:?}) should be {:?}",
                    a,
                    b,
                    expected[i][j]
                );
                assert!(int.is_none());
            }
        }
    }

    #[test]
    fn tnot_truth_table() {
        let cases = [
            (TritValue::True, TritValue::False),
            (TritValue::False, TritValue::True),
            (TritValue::Hold, TritValue::Hold),
            (TritValue::Unknown, TritValue::Unknown),
        ];
        for (input, expected) in cases {
            let word = TritWord::new(input, Phase::new(0.7).unwrap(), Frame::Science);
            let res = TernaryAlgebra::t_not(&word);
            assert_eq!(
                res.value(),
                expected,
                "TNOT({:?}) should be {:?}",
                input,
                expected
            );
            assert_eq!(res.frame(), Frame::Science);
        }
    }

    #[test]
    fn t_hold_preserves_frame() {
        let word = TritWord::tru(Frame::Individual);
        let held = TernaryAlgebra::t_hold(&word);
        assert_eq!(held.value(), TritValue::Hold);
        assert_eq!(held.frame(), Frame::Individual);
        assert_eq!(held.phase().inner(), 0.5);
    }

    #[test]
    #[should_panic(expected = "t_or_hot requires same frame")]
    fn tor_hot_different_frame_panics() {
        let a = TritWord::tru(Frame::Science);
        let b = TritWord::tru(Frame::Individual);
        TernaryAlgebra::t_or_hot(&a, &b);
    }

    #[test]
    fn tor_cross_frame_returns_hold_and_interrupt() {
        let a = TritWord::tru(Frame::Science);
        let b = TritWord::fals(Frame::Individual);
        let (res, int) = TernaryAlgebra::t_or(&a, &b);
        assert_eq!(res.value(), TritValue::Hold);
        assert_eq!(res.frame(), Frame::Meta);
        assert!(int.is_some());
    }

    #[test]
    fn tand_phase_averages() {
        let a = TritWord::new(TritValue::True, Phase::new(0.8).unwrap(), Frame::Science);
        let b = TritWord::new(TritValue::True, Phase::new(0.6).unwrap(), Frame::Science);
        let (res, _) = TernaryAlgebra::t_and(&a, &b);
        // (0.8 + 0.6) / 2 = 0.7, not near any anchor.
        assert_float_eq!(res.phase().inner(), 0.7);
    }

    // --- t_and_n batch tests ---

    #[test]
    fn t_and_n_single_input_is_identity() {
        let a = TritWord::tru(Frame::Science);
        let (res, ints) = TernaryAlgebra::t_and_n(&[a]);
        assert_eq!(res.value(), TritValue::True);
        assert!(ints.is_empty());
    }

    #[test]
    fn t_and_n_empty_returns_hold_meta() {
        let (res, ints) = TernaryAlgebra::t_and_n(&[]);
        assert_eq!(res.value(), TritValue::Hold);
        assert_eq!(res.frame(), Frame::Meta);
        assert!(ints.is_empty());
    }

    #[test]
    fn t_and_n_same_frame_matches_tand_value() {
        let a = TritWord::tru(Frame::Science);
        let b = TritWord::fals(Frame::Science);
        let (batch, batch_ints) = TernaryAlgebra::t_and_n(&[a, b]);
        let (pair, pair_int) = TernaryAlgebra::t_and(&a, &b);
        assert_eq!(batch.value(), pair.value());
        assert!(batch_ints.is_empty());
        assert!(pair_int.is_none());
    }

    #[test]
    fn t_and_n_avoids_left_fold_phase_bias() {
        let a = TritWord::new(TritValue::True, Phase::new(0.4).unwrap(), Frame::Science);
        let b = TritWord::new(TritValue::True, Phase::new(0.6).unwrap(), Frame::Science);
        let c = TritWord::new(TritValue::True, Phase::new(0.8).unwrap(), Frame::Science);
        let (result, _) = TernaryAlgebra::t_and_n(&[a, b, c]);
        // Batch mean: (0.4 + 0.6 + 0.8) / 3 = 0.6
        assert!((result.phase().inner() - 0.6).abs() < 1e-9);
    }

    #[test]
    fn t_and_n_cross_frame_produces_interrupts() {
        let a = TritWord::tru(Frame::Science);
        let b = TritWord::fals(Frame::Individual);
        let (res, ints) = TernaryAlgebra::t_and_n(&[a, b]);
        assert_eq!(res.value(), TritValue::Hold);
        assert_eq!(res.frame(), Frame::Meta);
        assert!(!ints.is_empty());
    }

    #[test]
    fn t_and_n_unknown_propagates() {
        let a = TritWord::unknown(Frame::Science);
        let b = TritWord::tru(Frame::Science);
        let (res, _) = TernaryAlgebra::t_and_n(&[a, b]);
        assert_eq!(res.value(), TritValue::Unknown);
    }
}
