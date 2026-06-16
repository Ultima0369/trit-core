use criterion::{black_box, criterion_group, criterion_main, Criterion};
use trit_core::frame::Frame;
use trit_core::trit::algebra::TernaryAlgebra;
use trit_core::trit::{TritValue, TritWord};

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

fn bench_tnot(c: &mut Criterion) {
    let a = TritWord::new(TritValue::True, 0.8, Frame::Science);
    c.bench_function("tnot", |b_bench| {
        b_bench.iter(|| TernaryAlgebra::t_not(black_box(&a)));
    });
}

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

criterion_group!(
    benches,
    bench_tand_same_frame,
    bench_tand_cross_frame,
    bench_tnot,
    bench_cascade_10
);
criterion_main!(benches);
