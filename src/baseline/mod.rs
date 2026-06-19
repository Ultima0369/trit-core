// Binary baseline comparator for M2 validation.
// Implements simple majority-rule logic (no Hold state) to demonstrate
// where binary systems fail to preserve conflicts that Trit-Core detects.

use crate::core::value::TritValue;
use crate::core::word::TritWord;

#[cfg(test)]
use crate::core::frame::Frame;

/// Result of a binary baseline evaluation.
#[derive(Debug, Clone, PartialEq)]
pub struct BinaryResult {
    /// The collapsed binary value (True or False, never Hold).
    pub value: TritValue,
    /// Whether the binary result conflicts with the ternary result.
    pub conflicts_with_ternary: bool,
    /// Explanation of the binary decision logic.
    pub reasoning: String,
}

/// Binary baseline comparator using simple majority voting.
///
/// Unlike Trit-Core's ternary logic which preserves Hold states when
/// domains conflict, the binary baseline always collapses to True or False.
pub struct BinaryBaseline;

impl BinaryBaseline {
    /// Evaluate a set of signals using binary majority rule.
    ///
    /// Counts True vs False votes. Hold signals are treated as abstentions
    /// (not counted). In case of a tie, defaults to False (conservative).
    pub fn evaluate(signals: &[TritWord]) -> BinaryResult {
        let trues: usize = signals
            .iter()
            .filter(|t| t.value() == TritValue::True)
            .count();
        let falses: usize = signals
            .iter()
            .filter(|t| t.value() == TritValue::False)
            .count();
        let holds: usize = signals
            .iter()
            .filter(|t| t.value() == TritValue::Hold)
            .count();

        let value = if trues > falses {
            TritValue::True
        } else {
            TritValue::False // tie goes to False (conservative)
        };

        let reasoning = format!(
            "Binary majority: {} True, {} False, {} Hold (abstained). {} wins.",
            trues,
            falses,
            holds,
            if value == TritValue::True {
                "True"
            } else {
                "False"
            }
        );

        BinaryResult {
            value,
            conflicts_with_ternary: false, // set by caller
            reasoning,
        }
    }

    /// Compare binary baseline against ternary (Trit-Core) result.
    /// Returns true if the binary result would have "smoothed" or "overridden"
    /// a conflict that the ternary system correctly identified.
    pub fn compare(ternary: &TritWord, binary: &BinaryResult) -> BinaryResult {
        let conflicts = ternary.value() == TritValue::Hold
            || (ternary.value() != binary.value && ternary.value() != TritValue::Hold);

        BinaryResult {
            value: binary.value,
            conflicts_with_ternary: conflicts,
            reasoning: if conflicts {
                format!(
                    "{} [CONFLICT: ternary={:?}, binary would override]",
                    binary.reasoning,
                    ternary.value()
                )
            } else {
                format!(
                    "{} [AGREE: ternary={:?}]",
                    binary.reasoning,
                    ternary.value()
                )
            },
        }
    }

    /// Check if a set of signals has cross-frame conflicts that binary
    /// would ignore but ternary would detect.
    pub fn has_hidden_conflict(signals: &[TritWord]) -> bool {
        if signals.is_empty() {
            return false;
        }
        let first_frame = signals[0].frame();
        signals.iter().any(|t| t.frame() != first_frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_majority_true_wins() {
        let signals = vec![
            TritWord::tru(Frame::Science),
            TritWord::tru(Frame::Science),
            TritWord::fals(Frame::Science),
        ];
        let result = BinaryBaseline::evaluate(&signals);
        assert_eq!(result.value, TritValue::True);
    }

    #[test]
    fn binary_majority_false_wins() {
        let signals = vec![
            TritWord::tru(Frame::Science),
            TritWord::fals(Frame::Science),
            TritWord::fals(Frame::Science),
        ];
        let result = BinaryBaseline::evaluate(&signals);
        assert_eq!(result.value, TritValue::False);
    }

    #[test]
    fn binary_tie_defaults_false() {
        let signals = vec![
            TritWord::tru(Frame::Science),
            TritWord::fals(Frame::Science),
        ];
        let result = BinaryBaseline::evaluate(&signals);
        assert_eq!(result.value, TritValue::False);
    }

    #[test]
    fn binary_ignores_cross_frame_conflict() {
        // Binary doesn't care about frame mismatches
        let signals = vec![
            TritWord::tru(Frame::Science),
            TritWord::fals(Frame::Individual),
        ];
        let result = BinaryBaseline::evaluate(&signals);
        // Tie (1 True, 1 False) → False
        assert_eq!(result.value, TritValue::False);
        // But there IS a hidden cross-frame conflict
        assert!(BinaryBaseline::has_hidden_conflict(&signals));
    }

    #[test]
    fn binary_vs_ternary_conflict_detected() {
        let signals = vec![
            TritWord::tru(Frame::Science),
            TritWord::fals(Frame::Individual),
        ];
        let binary = BinaryBaseline::evaluate(&signals);
        let ternary = TritWord::hold(Frame::Meta);
        let comparison = BinaryBaseline::compare(&ternary, &binary);
        assert!(comparison.conflicts_with_ternary);
    }

    #[test]
    fn binary_all_hold_defaults_false() {
        let signals = vec![
            TritWord::hold(Frame::Science),
            TritWord::hold(Frame::Science),
        ];
        let result = BinaryBaseline::evaluate(&signals);
        assert_eq!(result.value, TritValue::False);
        assert!(result.reasoning.contains("0 True, 0 False"));
    }

    #[test]
    fn binary_ignores_unknown() {
        // Unknown values are not counted as True, False, or Hold.
        let signals = vec![
            TritWord::unknown(Frame::Science),
            TritWord::tru(Frame::Science),
            TritWord::fals(Frame::Science),
        ];
        let result = BinaryBaseline::evaluate(&signals);
        // Tie (1 True, 1 False) → False
        assert_eq!(result.value, TritValue::False);
    }

    #[test]
    fn compare_agrees_when_values_match() {
        let binary = BinaryBaseline::evaluate(&[TritWord::tru(Frame::Science)]);
        let ternary = TritWord::tru(Frame::Science);
        let comparison = BinaryBaseline::compare(&ternary, &binary);
        assert!(!comparison.conflicts_with_ternary);
        assert!(comparison.reasoning.contains("AGREE"));
    }

    #[test]
    fn empty_signals_have_no_hidden_conflict() {
        assert!(!BinaryBaseline::has_hidden_conflict(&[]));
    }

    #[test]
    fn single_signal_has_no_hidden_conflict() {
        let signals = vec![TritWord::tru(Frame::Science)];
        assert!(!BinaryBaseline::has_hidden_conflict(&signals));
    }

    #[test]
    fn reasoning_includes_counts() {
        let signals = vec![
            TritWord::tru(Frame::Science),
            TritWord::hold(Frame::Science),
        ];
        let result = BinaryBaseline::evaluate(&signals);
        assert!(result.reasoning.contains("1 True"));
        assert!(result.reasoning.contains("0 False"));
        assert!(result.reasoning.contains("1 Hold"));
    }
}
