use aurora::percept::types::SignalSpec;
use aurora::percept::{ExternalPercept, FFTProvider};

#[test]
fn fft_provider_never_fails() {
    let spec = SignalSpec {
        freq: 2.0,
        sample_rate: 100.0,
        duration_secs: 1.0,
        noise_std: 0.0,
    };
    let provider = FFTProvider::new(spec);
    let batch = provider.perceive("any text").unwrap();
    assert!(batch.confidence >= 0.0);
}

#[test]
fn fft_provider_priority_is_lowest() {
    let spec = SignalSpec {
        freq: 2.0,
        sample_rate: 100.0,
        duration_secs: 1.0,
        noise_std: 0.0,
    };
    let provider = FFTProvider::new(spec);
    assert!(provider.priority() >= 2);
}

#[test]
fn fft_provider_always_available() {
    let spec = SignalSpec {
        freq: 2.0,
        sample_rate: 100.0,
        duration_secs: 1.0,
        noise_std: 0.0,
    };
    let provider = FFTProvider::new(spec);
    assert!(provider.available());
}

#[test]
fn fft_provider_has_meaningful_name() {
    let spec = SignalSpec {
        freq: 2.0,
        sample_rate: 100.0,
        duration_secs: 1.0,
        noise_std: 0.0,
    };
    let provider = FFTProvider::new(spec);
    assert!(!provider.provider_name().is_empty());
}
