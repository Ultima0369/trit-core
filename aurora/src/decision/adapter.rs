//! Adapters between raw signals / user input and Trit-Core `TritWord`s.

use truncore::core::{Frame, TritWord};

/// Map a detected communication-rhythm frequency to an Embodied-frame trit.
///
/// The threshold divides "high frequency / active" from "low frequency / quiet".
pub fn embodied_from_frequency(freq: f64, threshold: f64) -> TritWord {
    if freq > threshold {
        TritWord::tru(Frame::Embodied)
    } else if freq < threshold {
        TritWord::fals(Frame::Embodied)
    } else {
        TritWord::hold(Frame::Embodied)
    }
}

/// Map a user's self-reported state to an Individual-frame trit.
pub fn individual_from_user_state(feels_normal: bool) -> TritWord {
    if feels_normal {
        TritWord::tru(Frame::Individual)
    } else {
        TritWord::fals(Frame::Individual)
    }
}
