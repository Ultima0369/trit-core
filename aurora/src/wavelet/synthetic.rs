//! Synthetic signal generation for testing and calibration.

/// Generate a discrete sine wave with a given frequency, sampling rate,
/// duration, and optional additive noise.
///
/// # Arguments
/// * `freq` - signal frequency in Hz
/// * `sample_rate` - samples per second
/// * `duration_secs` - total signal duration in seconds
/// * `noise_std` - standard deviation of deterministic pseudo-noise (0.0 for clean signal).
///   The "noise" uses `sin(i * 0.37123948234)` — it is deterministic and periodic, not random.
///   This keeps tests reproducible. For stochastic noise, use a proper PRNG seeded from
///   the test RNG or system entropy.
pub fn sine_wave(freq: f64, sample_rate: f64, duration_secs: f64, noise_std: f64) -> Vec<f64> {
    // Guard against negative/non-finite args: a negative product would cast to
    // a huge usize and OOM on with_capacity. Non-positive or non-finite → empty.
    let n = (sample_rate * duration_secs).ceil();
    if !n.is_finite() || n <= 0.0 {
        return Vec::new();
    }
    let n = n as usize;
    let mut signal = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f64 / sample_rate;
        let sample = (2.0 * std::f64::consts::PI * freq * t).sin();
        let noise = if noise_std > 0.0 {
            // Deterministic pseudo-noise keeps tests reproducible.
            let x = (i as f64 * 0.371_239_482_34).sin();
            x * noise_std
        } else {
            0.0
        };
        signal.push(sample + noise);
    }
    signal
}
