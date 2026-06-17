use criterion::{black_box, criterion_group, criterion_main, Criterion};
use trit_core::frame::Frame;
use trit_core::trit::algebra::TernaryAlgebra;
use trit_core::trit::phase::Phase;
use trit_core::trit::{TritValue, TritWord};

// ---------------------------------------------------------------------------
// Core operation benchmarks
// ---------------------------------------------------------------------------

fn bench_tand_same_frame(c: &mut Criterion) {
    let a = TritWord::tru(Frame::Science);
    let b = TritWord::tru(Frame::Science);
    c.bench_function("tand_same_frame", |b_bench| {
        b_bench.iter(|| TernaryAlgebra::t_and(black_box(&a), black_box(&b)));
    });
}

fn bench_tand_cross_frame(c: &mut Criterion) {
    let a = TritWord::tru(Frame::Science);
    let b = TritWord::fals(Frame::Individual);
    c.bench_function("tand_cross_frame", |b_bench| {
        b_bench.iter(|| TernaryAlgebra::t_and(black_box(&a), black_box(&b)));
    });
}

fn bench_tor_same_frame(c: &mut Criterion) {
    let a = TritWord::tru(Frame::Science);
    let b = TritWord::fals(Frame::Science);
    c.bench_function("tor_same_frame", |b_bench| {
        b_bench.iter(|| TernaryAlgebra::t_or(black_box(&a), black_box(&b)));
    });
}

fn bench_tor_cross_frame(c: &mut Criterion) {
    let a = TritWord::tru(Frame::Science);
    let b = TritWord::fals(Frame::Individual);
    c.bench_function("tor_cross_frame", |b_bench| {
        b_bench.iter(|| TernaryAlgebra::t_or(black_box(&a), black_box(&b)));
    });
}

fn bench_tnot(c: &mut Criterion) {
    let a = TritWord::new(TritValue::True, 0.8, Frame::Science);
    c.bench_function("tnot", |b_bench| {
        b_bench.iter(|| TernaryAlgebra::t_not(black_box(&a)));
    });
}

// ---------------------------------------------------------------------------
// Hot-path benchmarks (no MetaInterrupt allocation)
// ---------------------------------------------------------------------------

fn bench_tand_hot(c: &mut Criterion) {
    let a = TritWord::tru(Frame::Science);
    let b = TritWord::tru(Frame::Science);
    c.bench_function("tand_hot_path", |b_bench| {
        b_bench.iter(|| TernaryAlgebra::t_and_hot(black_box(&a), black_box(&b)));
    });
}

fn bench_tor_hot(c: &mut Criterion) {
    let a = TritWord::tru(Frame::Science);
    let b = TritWord::fals(Frame::Science);
    c.bench_function("tor_hot_path", |b_bench| {
        b_bench.iter(|| TernaryAlgebra::t_or_hot(black_box(&a), black_box(&b)));
    });
}

fn bench_precheck_same_frame(c: &mut Criterion) {
    let a = TritWord::tru(Frame::Science);
    let b = TritWord::fals(Frame::Science);
    c.bench_function("precheck_same_frame", |b_bench| {
        b_bench.iter(|| TernaryAlgebra::precheck_same_frame(black_box(&a), black_box(&b)));
    });
}

// ---------------------------------------------------------------------------
// Cascade benchmarks (measure phase drift and throughput)
// ---------------------------------------------------------------------------

fn bench_cascade_10(c: &mut Criterion) {
    let trits: Vec<TritWord> = (0..10)
        .map(|i| {
            if i % 2 == 0 {
                TritWord::tru(Frame::Science)
            } else {
                TritWord::tru(Frame::Consensus)
            }
        })
        .collect();
    c.bench_function("tand_cascade_10", |b_bench| {
        b_bench.iter(|| {
            let mut current = trits[0].clone();
            for next in &trits[1..] {
                let (res, _) = TernaryAlgebra::t_and(black_box(&current), black_box(next));
                current = res;
            }
            black_box(current);
        });
    });
}

fn bench_cascade_10_hot(c: &mut Criterion) {
    // All same frame — can use hot path
    let trits: Vec<TritWord> = (0..10)
        .map(|_| TritWord::new(TritValue::True, 0.8, Frame::Science))
        .collect();
    c.bench_function("tand_cascade_10_hot", |b_bench| {
        b_bench.iter(|| {
            let mut current = trits[0].clone();
            for next in &trits[1..] {
                current = TernaryAlgebra::t_and_hot(black_box(&current), black_box(next));
            }
            black_box(current);
        });
    });
}

fn bench_cascade_100_hot(c: &mut Criterion) {
    let trits: Vec<TritWord> = (0..100)
        .map(|i| TritWord::new(TritValue::True, 0.5 + (i as f64 % 0.5), Frame::Science))
        .collect();
    c.bench_function("tand_cascade_100_hot", |b_bench| {
        b_bench.iter(|| {
            let mut current = trits[0].clone();
            for next in &trits[1..] {
                current = TernaryAlgebra::t_and_hot(black_box(&current), black_box(next));
            }
            black_box(current);
        });
    });
}

// ---------------------------------------------------------------------------
// Cross-domain throughput benchmark
// Measures TAND across different domain frame pairs
// ---------------------------------------------------------------------------

fn bench_cross_domain_throughput(c: &mut Criterion) {
    // Pre-allocate trits across all domain combinations
    let science_trits: Vec<TritWord> = (0..100)
        .map(|i| TritWord::new(TritValue::True, 0.5 + i as f64 * 0.005, Frame::Science))
        .collect();
    let individual_trits: Vec<TritWord> = (0..100)
        .map(|i| TritWord::new(TritValue::False, 0.5 - i as f64 * 0.005, Frame::Individual))
        .collect();
    let consensus_trits: Vec<TritWord> = (0..100)
        .map(|_i| TritWord::new(TritValue::Hold, 0.5, Frame::Consensus))
        .collect();

    c.bench_function("cross_domain_tand_100pairs", |b_bench| {
        let mut sci_iter = science_trits.iter().cycle();
        let mut ind_iter = individual_trits.iter().cycle();
        let mut con_iter = consensus_trits.iter().cycle();
        b_bench.iter(|| {
            for _ in 0..100 {
                let a = sci_iter.next().unwrap();
                let b = ind_iter.next().unwrap();
                black_box(TernaryAlgebra::t_and(black_box(a), black_box(b)));
                let c = con_iter.next().unwrap();
                black_box(TernaryAlgebra::t_and(black_box(a), black_box(c)));
            }
        });
    });
}

fn bench_cross_domain_hot_vs_cold(c: &mut Criterion) {
    let same = [TritWord::tru(Frame::Science), TritWord::tru(Frame::Science)];
    let cross = [
        TritWord::tru(Frame::Science),
        TritWord::fals(Frame::Individual),
    ];

    c.bench_function("hot_path_same_frame_pair", |b_bench| {
        b_bench.iter(|| TernaryAlgebra::t_and_hot(black_box(&same[0]), black_box(&same[1])));
    });

    c.bench_function("cold_path_cross_frame_pair", |b_bench| {
        b_bench.iter(|| TernaryAlgebra::t_and(black_box(&cross[0]), black_box(&cross[1])));
    });
}

// ---------------------------------------------------------------------------
// Phase quantization precision benchmark
// ---------------------------------------------------------------------------

fn bench_phase_quantize(c: &mut Criterion) {
    let near_neutral = Phase::new(0.5000000001);
    let near_zero = Phase::new(0.0000000001);
    let near_one = Phase::new(0.9999999999);
    let normal = Phase::new(0.73);

    c.bench_function("phase_quantize_near_neutral", |b_bench| {
        b_bench.iter(|| black_box(near_neutral).quantize(black_box(1e-6)))
    });

    c.bench_function("phase_quantize_near_zero", |b_bench| {
        b_bench.iter(|| black_box(near_zero).quantize(black_box(1e-6)))
    });

    c.bench_function("phase_quantize_near_one", |b_bench| {
        b_bench.iter(|| black_box(near_one).quantize(black_box(1e-6)))
    });

    c.bench_function("phase_quantize_noop", |b_bench| {
        b_bench.iter(|| black_box(normal).quantize(black_box(1e-6)))
    });
}

// ---------------------------------------------------------------------------
// Grouping
// ---------------------------------------------------------------------------

criterion_group!(
    core_ops,
    bench_tand_same_frame,
    bench_tand_cross_frame,
    bench_tor_same_frame,
    bench_tor_cross_frame,
    bench_tnot,
);

criterion_group!(
    hot_path,
    bench_tand_hot,
    bench_tor_hot,
    bench_precheck_same_frame,
);

criterion_group!(
    cascades,
    bench_cascade_10,
    bench_cascade_10_hot,
    bench_cascade_100_hot,
);

criterion_group!(
    cross_domain,
    bench_cross_domain_throughput,
    bench_cross_domain_hot_vs_cold,
);

criterion_group!(phase_precision, bench_phase_quantize);

criterion_main!(core_ops, hot_path, cascades, cross_domain, phase_precision);
