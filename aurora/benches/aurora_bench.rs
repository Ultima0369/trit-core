//! Aurora benchmarks — wavelet analysis, decision pipeline, attention scheduling.
//!
//! Run with: `cargo bench -p aurora`

use aurora::bc::signal_analysis::FftWaveletEngine;
use aurora::percept::types::SignalSpec;
use aurora::pipeline::analysis::run_analysis;
use aurora::wavelet::synthetic::sine_wave;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_sine_wave(c: &mut Criterion) {
    c.bench_function("sine_wave_1024", |b| {
        b.iter(|| {
            sine_wave(
                black_box(10.0),
                black_box(1000.0),
                black_box(1.0),
                black_box(0.0),
            )
        })
    });
}

fn bench_fft_analysis(c: &mut Criterion) {
    let signal = sine_wave(10.0, 1000.0, 1.0, 0.0);
    let ts = aurora::bc::signal_analysis::TimeSeries::new(1000.0, signal).unwrap();
    let engine = FftWaveletEngine;
    c.bench_function("fft_analyze_1024", |b| {
        b.iter(|| engine.analyze(black_box(&ts)))
    });
}

fn bench_full_pipeline(c: &mut Criterion) {
    let spec = SignalSpec {
        freq: 10.0,
        sample_rate: 1000.0,
        duration_secs: 1.0,
        noise_std: 0.0,
    };
    c.bench_function("run_analysis_medical", |b| {
        b.iter(|| {
            run_analysis(
                black_box(&spec),
                black_box(0.5),
                black_box(true),
                black_box(&[]),
            )
        })
    });
}

criterion_group!(
    benches,
    bench_sine_wave,
    bench_fft_analysis,
    bench_full_pipeline
);
criterion_main!(benches);
