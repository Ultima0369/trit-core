//! Fundamental-frequency detection.
//!
//! Current M0 implementation uses FFT peak detection with parabolic
//! interpolation as a fast, verifiable baseline. This satisfies the stage-1
//! acceptance test: "input 2Hz sine wave → output 2.0 ± 0.1Hz".

use num_complex::Complex;
use rustfft::{Fft, FftPlanner};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WaveletError {
    #[error("empty signal")]
    EmptySignal,
    #[error("sample rate must be positive")]
    InvalidSampleRate,
}

#[derive(Debug, Clone)]
pub struct WaveletResult {
    pub fundamental_freq: f64,
    pub peak_magnitude: f64,
}

#[derive(Debug, Clone)]
pub struct WaveletFeature {
    pub feature_type: FeatureType,
    pub value: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FeatureType {
    FundamentalFreq,
}

pub struct WaveletEngine {
    pub sampling_rate: f64,
}

impl WaveletEngine {
    pub fn new(sampling_rate: f64) -> Self {
        Self { sampling_rate }
    }

    pub fn analyze(&self, signal: &[f64]) -> Result<WaveletResult, WaveletError> {
        if signal.is_empty() {
            return Err(WaveletError::EmptySignal);
        }
        if self.sampling_rate <= 0.0 {
            return Err(WaveletError::InvalidSampleRate);
        }

        let n = signal.len().next_power_of_two();
        let mut buffer: Vec<Complex<f64>> = signal
            .iter()
            .copied()
            .map(|x| Complex::new(x, 0.0))
            .collect();
        buffer.resize(n, Complex::new(0.0, 0.0));

        // Hann window to reduce spectral leakage.
        let denominator = (signal.len() as f64 - 1.0).max(1.0);
        for (i, z) in buffer.iter_mut().enumerate() {
            let hann = 0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / denominator).cos();
            z.re *= hann;
        }

        let mut planner = FftPlanner::new();
        let fft: Arc<dyn Fft<f64>> = planner.plan_fft_forward(n);
        fft.process(&mut buffer);

        // Find the positive-frequency peak, excluding DC.
        let half = n / 2;
        let mut peak_idx = 1usize;
        let mut peak_mag = buffer[1].norm();
        for (i, z) in buffer.iter().enumerate().take(half).skip(2) {
            let mag = z.norm();
            if mag > peak_mag {
                peak_mag = mag;
                peak_idx = i;
            }
        }

        // Parabolic interpolation for sub-bin resolution.
        let alpha = buffer[peak_idx - 1].norm();
        let beta = buffer[peak_idx].norm();
        let gamma = buffer[peak_idx + 1].norm();
        let denom = alpha - 2.0 * beta + gamma;
        let p = if denom.abs() < f64::EPSILON {
            0.0
        } else {
            0.5 * (alpha - gamma) / denom
        };
        let refined_idx = peak_idx as f64 + p;

        let freq_resolution = self.sampling_rate / n as f64;
        let fundamental_freq = refined_idx * freq_resolution;

        Ok(WaveletResult {
            fundamental_freq,
            peak_magnitude: beta,
        })
    }

    pub fn extract_features(
        &self,
        result: &WaveletResult,
    ) -> Result<Vec<WaveletFeature>, WaveletError> {
        Ok(vec![WaveletFeature {
            feature_type: FeatureType::FundamentalFreq,
            value: result.fundamental_freq,
            confidence: 1.0,
        }])
    }
}
