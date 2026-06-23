//! SignalAnalysis BC — wavelet-based frequency analysis of time-series data.
//!
//! # Aggregate root
//! [`FrequencySpectrum`] — the result of analyzing a time series.
//!
//! # Port
//! [`WaveletEngine`] trait — the single interface for signal analysis.

use crate::bc::BcError;

// ── Entities ──────────────────────────────────────────────────────────────

/// A discrete time-series signal (e.g. communication frequency samples).
#[derive(Debug, Clone)]
pub struct TimeSeries {
    /// Sample rate in Hz.
    pub sample_rate: f64,
    /// Signal samples.
    pub samples: Vec<f64>,
}

impl TimeSeries {
    /// Create a new time series. Returns error if sample_rate ≤ 0.
    pub fn new(sample_rate: f64, samples: Vec<f64>) -> Result<Self, BcError> {
        if sample_rate <= 0.0 {
            return Err(BcError::InvalidInput {
                field: "sample_rate".into(),
                reason: "must be positive".into(),
            });
        }
        Ok(Self {
            sample_rate,
            samples,
        })
    }

    /// Number of samples.
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Whether the series is empty.
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Duration in seconds.
    pub fn duration_secs(&self) -> f64 {
        self.samples.len() as f64 / self.sample_rate
    }
}

/// A detected peak in the frequency domain.
#[derive(Debug, Clone, Copy)]
pub struct FrequencyPeak {
    /// Frequency in Hz.
    pub frequency_hz: f64,
    /// Power/magnitude at this frequency.
    pub power: f64,
}

/// Signal quality assessment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalQuality {
    /// Signal-to-noise ratio is high; analysis is reliable.
    Good,
    /// SNR is moderate; results should be interpreted with caution.
    Marginal,
    /// SNR is low; results may be unreliable.
    Poor,
}

// ── Aggregate root ────────────────────────────────────────────────────────

/// The frequency spectrum of an analyzed signal — the aggregate root.
///
/// Contains the fundamental frequency, harmonics, and quality assessment.
#[derive(Debug, Clone)]
pub struct FrequencySpectrum {
    /// The dominant (fundamental) frequency in Hz.
    pub fundamental_hz: f64,
    /// All detected peaks, sorted by frequency.
    pub peaks: Vec<FrequencyPeak>,
    /// Signal quality assessment.
    pub quality: SignalQuality,
    /// Number of samples analyzed.
    pub sample_count: usize,
}

impl FrequencySpectrum {
    /// Create a new frequency spectrum.
    ///
    /// # Invariants
    /// - `fundamental_hz` must be ≥ 0
    /// - `peaks` must contain at least one entry (the fundamental)
    pub fn new(
        fundamental_hz: f64,
        peaks: Vec<FrequencyPeak>,
        quality: SignalQuality,
        sample_count: usize,
    ) -> Result<Self, BcError> {
        if fundamental_hz < 0.0 {
            return Err(BcError::InvalidInput {
                field: "fundamental_hz".into(),
                reason: "must be non-negative".into(),
            });
        }
        if peaks.is_empty() {
            return Err(BcError::InvalidInput {
                field: "peaks".into(),
                reason: "must contain at least the fundamental peak".into(),
            });
        }
        Ok(Self {
            fundamental_hz,
            peaks,
            quality,
            sample_count,
        })
    }

    /// Whether the signal quality is sufficient for decision-making.
    pub fn is_reliable(&self) -> bool {
        matches!(self.quality, SignalQuality::Good)
    }
}

// ── Port trait ────────────────────────────────────────────────────────────

/// The single interface for signal analysis.
///
/// Implementations: FFT-based (M0), Morlet CWT (M1+).
pub trait WaveletEngine {
    /// Analyze a time series and return its frequency spectrum.
    fn analyze(&self, signal: &TimeSeries) -> Result<FrequencySpectrum, BcError>;
}

// ── M0 implementation (wraps existing wavelet module) ─────────────────────

/// FFT-based wavelet engine — wraps the existing M0 implementation.
pub struct FftWaveletEngine;

impl WaveletEngine for FftWaveletEngine {
    fn analyze(&self, signal: &TimeSeries) -> Result<FrequencySpectrum, BcError> {
        let engine = crate::wavelet::WaveletEngine::new(signal.sample_rate);
        let result = engine
            .analyze(&signal.samples)
            .map_err(|e| BcError::Domain {
                bc: "SignalAnalysis".into(),
                message: e.to_string(),
            })?;

        // Clamp negative fundamental frequencies (can happen with very short signals)
        let fundamental_hz = if result.fundamental_freq < 0.0 {
            0.0
        } else {
            result.fundamental_freq
        };

        let peaks = vec![FrequencyPeak {
            frequency_hz: fundamental_hz,
            power: 1.0,
        }];

        let quality = if signal.samples.len() < 30 {
            SignalQuality::Poor
        } else if signal.samples.len() < 100 {
            SignalQuality::Marginal
        } else {
            SignalQuality::Good
        };

        FrequencySpectrum::new(fundamental_hz, peaks, quality, signal.samples.len())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_series_rejects_zero_sample_rate() {
        let result = TimeSeries::new(0.0, vec![1.0, 2.0]);
        assert!(result.is_err());
    }

    #[test]
    fn time_series_rejects_negative_sample_rate() {
        let result = TimeSeries::new(-1.0, vec![1.0, 2.0]);
        assert!(result.is_err());
    }

    #[test]
    fn time_series_duration_is_correct() {
        let ts = TimeSeries::new(100.0, vec![0.0; 200]).unwrap();
        assert!((ts.duration_secs() - 2.0).abs() < 1e-9);
    }

    #[test]
    fn frequency_spectrum_rejects_empty_peaks() {
        let result = FrequencySpectrum::new(10.0, vec![], SignalQuality::Good, 100);
        assert!(result.is_err());
    }

    #[test]
    fn frequency_spectrum_rejects_negative_fundamental() {
        let peak = FrequencyPeak {
            frequency_hz: 10.0,
            power: 1.0,
        };
        let result = FrequencySpectrum::new(-1.0, vec![peak], SignalQuality::Good, 100);
        assert!(result.is_err());
    }

    #[test]
    fn frequency_spectrum_is_reliable_for_good_quality() {
        let peak = FrequencyPeak {
            frequency_hz: 10.0,
            power: 1.0,
        };
        let spectrum = FrequencySpectrum::new(10.0, vec![peak], SignalQuality::Good, 100).unwrap();
        assert!(spectrum.is_reliable());
    }

    #[test]
    fn fft_engine_detects_sine_wave_fundamental() {
        use crate::wavelet::sine_wave;

        let signal = sine_wave(2.5, 100.0, 1.0, 0.0);
        let ts = TimeSeries::new(100.0, signal).unwrap();
        let engine = FftWaveletEngine;
        let spectrum = engine.analyze(&ts).unwrap();

        // 2.5 Hz input should be detected within ±0.5 Hz
        assert!(
            (spectrum.fundamental_hz - 2.5).abs() < 0.5,
            "expected ~2.5 Hz, got {}",
            spectrum.fundamental_hz
        );
        assert_eq!(spectrum.sample_count, 100);
    }

    #[test]
    fn fft_engine_marks_short_signals_as_poor() {
        use crate::wavelet::sine_wave;

        // 5 samples at 100 Hz = 0.05s duration
        let signal = sine_wave(5.0, 100.0, 0.05, 0.0);
        let ts = TimeSeries::new(100.0, signal).unwrap();
        let engine = FftWaveletEngine;
        let spectrum = engine.analyze(&ts).unwrap();

        // Short signals (< 30 samples) should be marked Poor
        assert_eq!(spectrum.quality, SignalQuality::Poor);
    }
}
