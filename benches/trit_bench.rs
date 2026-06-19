use criterion::{black_box, criterion_group, criterion_main, Criterion};
use trit_core::core::frame::Frame;
use trit_core::core::phase::Phase;
use trit_core::core::value::TritValue;
use trit_core::core::word::TritWord;
use trit_core::core::TernaryAlgebra;
use trit_core::sandbox::{SandboxOutput, SandboxPipeline, ScenarioInput};

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

fn bench_tnot(c: &mut Criterion) {
    let a = TritWord::new(TritValue::True, Phase::new(0.8).unwrap(), Frame::Science);
    c.bench_function("tnot", |b_bench| {
        b_bench.iter(|| TernaryAlgebra::t_not(black_box(&a)));
    });
}

fn bench_tand_hot(c: &mut Criterion) {
    let a = TritWord::tru(Frame::Science);
    let b = TritWord::tru(Frame::Science);
    c.bench_function("tand_hot_path", |b_bench| {
        b_bench.iter(|| TernaryAlgebra::t_and_hot(black_box(&a), black_box(&b)));
    });
}

fn bench_precheck_same_frame(c: &mut Criterion) {
    let a = TritWord::tru(Frame::Science);
    let b = TritWord::fals(Frame::Science);
    c.bench_function("precheck_same_frame", |b_bench| {
        b_bench.iter(|| TernaryAlgebra::precheck_same_frame(black_box(&a), black_box(&b)));
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
            let mut current = trits[0];
            for next in &trits[1..] {
                let (res, _) = TernaryAlgebra::t_and(black_box(&current), black_box(next));
                current = res;
            }
            black_box(current);
        });
    });
}

fn bench_cascade_10_hot(c: &mut Criterion) {
    let trits: Vec<TritWord> = (0..10)
        .map(|_| TritWord::new(TritValue::True, Phase::new(0.8).unwrap(), Frame::Science))
        .collect();
    c.bench_function("tand_cascade_10_hot", |b_bench| {
        b_bench.iter(|| {
            let mut current = trits[0];
            for next in &trits[1..] {
                current = TernaryAlgebra::t_and_hot(black_box(&current), black_box(next));
            }
            black_box(current);
        });
    });
}

fn bench_phase_quantize(c: &mut Criterion) {
    let near_neutral = Phase::new(0.5000000001).unwrap();
    let near_zero = Phase::new(0.0000000001).unwrap();
    let near_one = Phase::new(0.9999999999).unwrap();
    let normal = Phase::new(0.73).unwrap();

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

fn bench_full_pipeline_medical(c: &mut Criterion) {
    let json = r#"{
        "id": "bench-medical",
        "description": "Patient treatment conflict",
        "domain": "MedicalEthics",
        "signals": [
            {"frame": "Science", "value": 1, "phase": 0.85},
            {"frame": "Individual", "value": -1, "phase": 0.75}
        ],
        "expected_behavior": "hold"
    }"#;
    c.bench_function("full_pipeline_medical_ethics", |b| {
        b.iter(|| {
            let input: ScenarioInput = serde_json::from_str(black_box(json)).unwrap();
            let mut pipeline = SandboxPipeline::default();
            let output = pipeline.run(black_box(&input)).unwrap();
            black_box(output);
        });
    });
}

fn bench_full_pipeline_physical(c: &mut Criterion) {
    let json = r#"{
        "id": "bench-physical",
        "description": "Bridge safety overload detection",
        "domain": "Physical",
        "signals": [
            {"frame": "Science", "value": -1, "phase": 0.95},
            {"frame": "Individual", "value": 1, "phase": 0.55}
        ],
        "expected_behavior": "commit_false"
    }"#;
    c.bench_function("full_pipeline_physical", |b| {
        b.iter(|| {
            let input: ScenarioInput = serde_json::from_str(black_box(json)).unwrap();
            let mut pipeline = SandboxPipeline::default();
            let output = pipeline.run(black_box(&input)).unwrap();
            black_box(output);
        });
    });
}

fn bench_json_serde_scenario_deser(c: &mut Criterion) {
    let json = r#"{
        "id": "bench-ser",
        "description": "JSON deserialization benchmark",
        "domain": "MedicalEthics",
        "signals": [
            {"frame": "Science", "value": 1, "phase": 0.85},
            {"frame": "Individual", "value": -1, "phase": 0.75}
        ],
        "expected_behavior": "hold"
    }"#;

    c.bench_function("json_deser_scenario", |b| {
        b.iter(|| {
            let _input: ScenarioInput = serde_json::from_str(black_box(json)).unwrap();
        });
    });
}

fn bench_json_serde_output_ser(c: &mut Criterion) {
    let output = SandboxOutput {
        scenario_id: "bench".into(),
        final_value: "Hold".into(),
        final_value_code: 0,
        final_frame: "Meta".into(),
        final_phase_raw: 0.5,
        interrupts: vec!["FrameMismatch".into(), "PhaseDrift".into()],
        policy_action: "Hold".into(),
        reflexive_alert: None,
        attention_cmd: None,
        receiver_estimate: None,
        hold_state: None,
    };

    c.bench_function("json_ser_output", |b| {
        b.iter(|| {
            let _json = serde_json::to_string(black_box(&output)).unwrap();
        });
    });
}

criterion_group!(
    core_ops,
    bench_tand_same_frame,
    bench_tand_cross_frame,
    bench_tor_same_frame,
    bench_tnot,
);

criterion_group!(hot_path, bench_tand_hot, bench_precheck_same_frame);

criterion_group!(cascades, bench_cascade_10, bench_cascade_10_hot);

criterion_group!(phase_precision, bench_phase_quantize);

criterion_group!(
    pipeline,
    bench_full_pipeline_medical,
    bench_full_pipeline_physical
);

criterion_group!(
    json_serde,
    bench_json_serde_scenario_deser,
    bench_json_serde_output_ser
);

criterion_main!(
    core_ops,
    hot_path,
    cascades,
    phase_precision,
    pipeline,
    json_serde
);
