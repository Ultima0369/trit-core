//! Wavelet analysis module for Aurora.
//!
//! The M0 implementation uses FFT-based fundamental-frequency detection as a
//! fast, verifiable baseline. A full Morlet CWT scalogram will be layered in
//! once the core pipeline is proven.

pub mod detect;
pub mod synthetic;

pub use detect::{FeatureType, WaveletEngine, WaveletError, WaveletFeature, WaveletResult};
pub use synthetic::sine_wave;
