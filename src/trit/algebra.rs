use crate::frame::Frame;
use crate::meta::{ConflictType, MetaInterrupt};
use crate::trit::{Phase, TritValue, TritWord};
use tracing::{debug, trace, warn};

/// Harmonic Ternary Algebra (HTA): the core logic engine.
pub struct TernaryAlgebra;

impl TernaryAlgebra {
    /// TAND: harmonic conjunction.
    /// - Same frame: standard ternary logic with phase averaging.
    /// - Different frame: produces Hold + triggers MetaInterrupt.
    #[tracing::instrument(skip_all, fields(op = "t_and"))]
    pub fn t_and(a: &TritWord, b: &TritWord) -> (TritWord, Option<MetaInterrupt>) {
        trace!(a_frame = %a.frame, a_value = ?a.value, a_phase = a.phase.inner(),
               b_frame = %b.frame, b_value = ?b.value, b_phase = b.phase.inner(),
               "entering TAND");

        if a.frame != b.frame {
            let hold = TritWord::new(TritValue::Hold, 0.5, Frame::Meta);
            let interrupt = MetaInterrupt::new(
                ConflictType::FrameMismatch,
                format!("TAND conflict: {} vs {}", a.frame, b.frame),
            );
            warn!(reason = %interrupt.reason, "cross-frame conflict detected");
            return (hold, Some(interrupt));
        }

        let val = match (a.value, b.value) {
            (TritValue::True, TritValue::True) => TritValue::True,
            (TritValue::False, _) | (_, TritValue::False) => TritValue::False,
            _ => TritValue::Hold,
        };

        let phase = Phase::mean(a.phase, b.phase);
        debug!(result_value = ?val, result_phase = phase.inner(), "TAND same-frame computed");
        (TritWord::new(val, phase.inner(), a.frame.clone()), None)
    }

    /// TOR: harmonic disjunction.
    #[tracing::instrument(skip_all, fields(op = "t_or"))]
    pub fn t_or(a: &TritWord, b: &TritWord) -> (TritWord, Option<MetaInterrupt>) {
        trace!(a_frame = %a.frame, b_frame = %b.frame, "entering TOR");

        if a.frame != b.frame {
            let hold = TritWord::new(TritValue::Hold, 0.5, Frame::Meta);
            let interrupt = MetaInterrupt::new(
                ConflictType::FrameMismatch,
                format!("TOR conflict: {} vs {}", a.frame, b.frame),
            );
            warn!(reason = %interrupt.reason, "cross-frame conflict detected");
            return (hold, Some(interrupt));
        }

        let val = match (a.value, b.value) {
            (TritValue::True, _) | (_, TritValue::True) => TritValue::True,
            (TritValue::False, TritValue::False) => TritValue::False,
            _ => TritValue::Hold,
        };

        let phase = Phase::mean(a.phase, b.phase);
        (TritWord::new(val, phase.inner(), a.frame.clone()), None)
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
