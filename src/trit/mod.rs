pub mod algebra;
pub mod phase;
pub mod value;

pub use phase::Phase;
pub use value::TritValue;

use crate::frame::Frame;

/// A ternary word: the fundamental unit of computation in Trit-Core.
///
/// Unlike a binary bit (0/1), a trit carries:
/// - `value`: ternary state {True, Hold, False} + out-of-distribution {Unknown}
/// - `phase`: continuous tendency 0.0..1.0 (0.5 = neutral)
/// - `frame`: the decision domain / context this trit belongs to
///
/// The core ternary logic (MVL-3) operates on True/Hold/False.
/// `Unknown` (⊥) is a meta-state for inputs the system cannot compute on —
/// it propagates through TAND/TOR and triggers SafeFallback in dangerous domains.
#[derive(Clone, Debug, PartialEq)]
pub struct TritWord {
    pub value: TritValue,
    pub phase: Phase,
    pub frame: Frame,
}

impl TritWord {
    pub fn new(value: TritValue, phase: f64, frame: Frame) -> Self {
        Self {
            value,
            phase: Phase::new(phase),
            frame,
        }
    }

    /// Create a neutral hold-state trit
    pub fn hold(frame: Frame) -> Self {
        Self::new(TritValue::Hold, 0.5, frame)
    }

    /// Create a fully committed true trit
    pub fn tru(frame: Frame) -> Self {
        Self::new(TritValue::True, 1.0, frame)
    }

    /// Create a fully committed false trit
    pub fn fals(frame: Frame) -> Self {
        Self::new(TritValue::False, 0.0, frame)
    }

    /// Create an out-of-distribution trit (unknowable input).
    pub fn unknown(frame: Frame) -> Self {
        Self::new(TritValue::Unknown, 0.5, frame)
    }
}

pub type Trit = TritWord;
