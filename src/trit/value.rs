/// Ternary value: the three discrete states.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum TritValue {
    True, // +1
    #[default]
    Hold, // 0 — undetermined / suspended
    False, // -1
}

impl TritValue {
    /// Flip true/false; Hold remains Hold.
    pub fn negate(self) -> Self {
        match self {
            TritValue::True => TritValue::False,
            TritValue::False => TritValue::True,
            TritValue::Hold => TritValue::Hold,
        }
    }

    /// Convert to signed integer for arithmetic use.
    pub fn to_i8(self) -> i8 {
        match self {
            TritValue::True => 1,
            TritValue::Hold => 0,
            TritValue::False => -1,
        }
    }
}
