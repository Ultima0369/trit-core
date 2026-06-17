/// Ternary logic value: the four discrete states of a TritWord.
///
/// Three computable states (`True`, `Hold`, `False`) form the core
/// ternary logic (MVL-3). A fourth state (`Unknown`) represents
/// out-of-distribution / unknowable inputs that the system cannot
/// compute on — semantically distinct from `Hold` (chosen suspension
/// of judgment) and used only for input-gating and safety fallback.
///
/// | State     | Ternary role     | Meaning                        |
/// |-----------|------------------|--------------------------------|
/// | `True`    | +1               | Affirmative                    |
/// | `Hold`    | 0 (neutral)      | Suspended judgment (deliberate)|
/// | `False`   | -1               | Negative                       |
/// | `Unknown` | ⊥ (not in MVL-3) | Out-of-distribution, unknowable|
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum TritValue {
    True, // +1
    #[default]
    Hold, // 0 — undetermined / suspended
    False, // -1
    Unknown, // ⊥ — out-of-distribution / unknowable, distinct from Hold
}

impl TritValue {
    /// Internal discriminant for LUT indexing.
    /// LLVM optimizes this to a single register load.
    #[inline]
    fn disc(self) -> usize {
        match self {
            TritValue::True => 0,
            TritValue::Hold => 1,
            TritValue::False => 2,
            TritValue::Unknown => 3,
        }
    }

    /// Branchless negation via lookup table.
    const NEGATE_LUT: [TritValue; 4] = [
        TritValue::False,   // True → False
        TritValue::Hold,    // Hold → Hold
        TritValue::True,    // False → True
        TritValue::Unknown, // Unknown → Unknown
    ];

    /// Branchless to_i8 via lookup table.
    const TO_I8_LUT: [i8; 4] = [1, 0, -1, 0];

    /// Flip true/false; Hold and Unknown remain unchanged.
    #[inline]
    pub fn negate(self) -> Self {
        Self::NEGATE_LUT[self.disc()]
    }

    /// Convert to signed integer for arithmetic use.
    /// Unknown maps to 0 (same as Hold) for arithmetic compatibility.
    #[inline]
    pub fn to_i8(self) -> i8 {
        Self::TO_I8_LUT[self.disc()]
    }

    /// Returns true if this value is computable (not Unknown).
    #[inline]
    pub fn is_computable(self) -> bool {
        self != TritValue::Unknown
    }
}

impl From<i8> for TritValue {
    /// Convert i8 to TritValue. Unknown cannot be created from i8 —
    /// use `TritValue::Unknown` directly when the input is out-of-distribution.
    fn from(v: i8) -> Self {
        match v {
            1 => TritValue::True,
            -1 => TritValue::False,
            _ => TritValue::Hold,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_convert_true_to_i8_positive_one() {
        assert_eq!(TritValue::True.to_i8(), 1);
    }

    #[test]
    fn should_convert_hold_to_i8_zero() {
        assert_eq!(TritValue::Hold.to_i8(), 0);
    }

    #[test]
    fn should_convert_false_to_i8_negative_one() {
        assert_eq!(TritValue::False.to_i8(), -1);
    }

    #[test]
    fn should_create_from_i8_positive_one() {
        assert_eq!(TritValue::from(1), TritValue::True);
    }

    #[test]
    fn should_create_from_i8_negative_one() {
        assert_eq!(TritValue::from(-1), TritValue::False);
    }

    #[test]
    fn should_create_hold_from_any_other_i8() {
        assert_eq!(TritValue::from(0), TritValue::Hold);
        assert_eq!(TritValue::from(2), TritValue::Hold);
        assert_eq!(TritValue::from(-2), TritValue::Hold);
    }

    #[test]
    fn negate_should_flip_true_to_false() {
        assert_eq!(TritValue::True.negate(), TritValue::False);
    }

    #[test]
    fn negate_should_flip_false_to_true() {
        assert_eq!(TritValue::False.negate(), TritValue::True);
    }

    #[test]
    fn negate_should_keep_hold() {
        assert_eq!(TritValue::Hold.negate(), TritValue::Hold);
    }

    #[test]
    fn default_should_be_hold() {
        assert_eq!(TritValue::default(), TritValue::Hold);
    }

    #[test]
    fn unknown_is_not_hold() {
        assert_ne!(TritValue::Unknown, TritValue::Hold);
    }

    #[test]
    fn unknown_is_not_computable() {
        assert!(!TritValue::Unknown.is_computable());
    }

    #[test]
    fn true_is_computable() {
        assert!(TritValue::True.is_computable());
    }

    #[test]
    fn hold_is_computable() {
        // Hold means "undecided" but still computable — the system CAN decide, it just hasn't
        assert!(TritValue::Hold.is_computable());
    }

    #[test]
    fn unknown_to_i8_is_zero() {
        assert_eq!(TritValue::Unknown.to_i8(), 0);
    }

    #[test]
    fn negate_keeps_unknown() {
        assert_eq!(TritValue::Unknown.negate(), TritValue::Unknown);
    }

    #[test]
    fn unknown_cannot_be_created_from_i8() {
        // From<i8> never produces Unknown — you must use it directly
        for v in -10..=10 {
            assert_ne!(TritValue::from(v), TritValue::Unknown);
        }
    }
}
