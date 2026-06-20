//! Cross-frame conflict detection using Trit-Core TAND.

use truncore::core::{TernaryAlgebra, TritWord};
use truncore::meta::MetaInterrupt;

/// Detect conflict between an embodied signal and an individual self-report.
///
/// Cross-frame inputs (e.g. Embodied vs Individual) produce Hold + MetaInterrupt.
/// Same-frame inputs are resolved by standard ternary logic.
pub fn detect_conflict(
    embodied: &TritWord,
    individual: &TritWord,
) -> (TritWord, Option<MetaInterrupt>) {
    TernaryAlgebra::t_and(embodied, individual)
}
