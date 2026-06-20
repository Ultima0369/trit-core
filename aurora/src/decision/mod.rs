//! Decision layer: map wavelet features and user state to Trit-Core trits.

pub mod adapter;
pub mod conflict;

pub use adapter::{embodied_from_frequency, individual_from_user_state};
pub use conflict::detect_conflict;
