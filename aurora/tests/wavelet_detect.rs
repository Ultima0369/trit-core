//! Stage 1 acceptance tests: synthetic data → fundamental frequency.

use aurora::wavelet::{sine_wave, WaveletEngine};

#[test]
fn detects_2hz_sine_wave() {
    let sample_rate = 100.0;
    let signal = sine_wave(2.0, sample_rate, 10.0, 0.0);
    let engine = WaveletEngine::new(sample_rate);
    let result = engine.analyze(&signal).unwrap();
    let err = (result.fundamental_freq - 2.0).abs();
    assert!(
        err < 0.1,
        "expected ~2.0 Hz, got {} Hz (error {})",
        result.fundamental_freq,
        err
    );
}

#[test]
fn detects_2hz_with_noise() {
    let sample_rate = 100.0;
    let signal = sine_wave(2.0, sample_rate, 10.0, 0.1);
    let engine = WaveletEngine::new(sample_rate);
    let result = engine.analyze(&signal).unwrap();
    let err = (result.fundamental_freq - 2.0).abs();
    assert!(
        err < 0.1,
        "expected ~2.0 Hz with noise, got {} Hz (error {})",
        result.fundamental_freq,
        err
    );
}

#[test]
fn rejects_empty_signal() {
    let engine = WaveletEngine::new(100.0);
    assert!(engine.analyze(&[]).is_err());
}

#[test]
fn rejects_invalid_sample_rate() {
    let signal = sine_wave(2.0, 100.0, 1.0, 0.0);
    let engine = WaveletEngine::new(0.0);
    assert!(engine.analyze(&signal).is_err());
}
