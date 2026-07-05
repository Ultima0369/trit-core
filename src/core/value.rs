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
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, Deserialize, Serialize)]
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
    const fn disc(self) -> usize {
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

    /// Branchless discriminant LUT.
    const DISC_LUT: [u8; 4] = [0, 1, 2, 3];

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

    /// Returns a unique numeric discriminant for all four states:
    /// True→0, Hold→1, False→2, Unknown→3.
    /// Unlike `to_i8()`, this preserves the Hold/Unknown distinction.
    #[inline]
    pub fn discriminant(self) -> u8 {
        Self::DISC_LUT[self.disc()]
    }
}

impl From<i8> for TritValue {
    /// Convert i8 to TritValue. Unknown cannot be created from i8 —
    /// use `TritValue::Unknown` directly when the input is out-of-distribution.
    ///
    /// Values outside {-1, 0, 1} are silently clamped to `Hold`. This is
    /// intentional for wire-format compatibility (many protocols use i8),
    /// but callers parsing untrusted input should prefer
    /// [`TritValue::from_i8_strict`] which rejects out-of-range values.
    fn from(v: i8) -> Self {
        match v {
            1 => TritValue::True,
            -1 => TritValue::False,
            0 => TritValue::Hold,
            other => {
                // ponytail: silent clamp for wire-format compat; from_i8_strict for validation
                tracing::debug!(
                    value = other,
                    "TritValue::from<i8> clamping out-of-range value to Hold"
                );
                TritValue::Hold
            }
        }
    }
}

impl TritValue {
    /// Strict conversion from i8: only accepts -1, 0, 1.
    /// Unlike `From<i8>`, this rejects out-of-range values instead of
    /// silently collapsing to Hold.
    pub fn from_i8_strict(v: i8) -> Result<Self, &'static str> {
        match v {
            1 => Ok(TritValue::True),
            0 => Ok(TritValue::Hold),
            -1 => Ok(TritValue::False),
            _ => Err("TritValue can only be created from -1, 0, or 1"),
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

    #[test]
    fn from_i8_strict_rejects_out_of_range() {
        assert!(TritValue::from_i8_strict(2).is_err());
        assert!(TritValue::from_i8_strict(-2).is_err());
        assert!(TritValue::from_i8_strict(127).is_err());
    }

    #[test]
    fn from_i8_strict_accepts_valid() {
        assert_eq!(TritValue::from_i8_strict(1).unwrap(), TritValue::True);
        assert_eq!(TritValue::from_i8_strict(0).unwrap(), TritValue::Hold);
        assert_eq!(TritValue::from_i8_strict(-1).unwrap(), TritValue::False);
    }

    #[test]
    fn from_i8_strict_error_message() {
        let err = TritValue::from_i8_strict(2).unwrap_err();
        assert!(err.contains("-1, 0, or 1"));
    }

    #[test]
    fn false_is_computable() {
        assert!(TritValue::False.is_computable());
    }

    #[test]
    fn debug_format_contains_variant_name() {
        let s = format!("{:?}", TritValue::Hold);
        assert!(s.contains("Hold"));
    }

    #[test]
    fn discriminant_distinguishes_all_four_states() {
        assert_eq!(TritValue::True.discriminant(), 0);
        assert_eq!(TritValue::Hold.discriminant(), 1);
        assert_eq!(TritValue::False.discriminant(), 2);
        assert_eq!(TritValue::Unknown.discriminant(), 3);
    }

    #[test]
    fn discriminant_preserves_hold_unknown_distinction() {
        // to_i8() conflates Hold and Unknown (both → 0)
        // discriminant() must preserve the distinction
        assert_eq!(TritValue::Hold.to_i8(), 0);
        assert_eq!(TritValue::Unknown.to_i8(), 0);
        assert_ne!(
            TritValue::Hold.discriminant(),
            TritValue::Unknown.discriminant()
        );
    }
}
