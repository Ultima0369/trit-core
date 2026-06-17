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

impl From<i8> for TritValue {
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
}
