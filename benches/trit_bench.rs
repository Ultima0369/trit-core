use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::Arc;
use tokio::sync::Mutex;
use trit_core::frame::Frame;
use trit_core::meta::{Domain, ResolutionPolicy, SafeFallback};
use trit_core::net::bus::ResonanceBus;
use trit_core::net::frame_codec;
use trit_core::net::message::Message;
use trit_core::net::node::Node;
use trit_core::sandbox::{SandboxOutput, ScenarioInput};
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
// End-to-end pipeline benchmarks (Phase B)
// ---------------------------------------------------------------------------

fn bench_full_pipeline_medical(c: &mut Criterion) {
    let json = r#"{
        "id": "bench-medical",
        "description": "Patient treatment conflict: chemotherapy vs palliative care",
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
            let mut trits = Vec::with_capacity(input.signals.len());
            for s in &input.signals {
                let frame = match s.frame.as_str() {
                    "Science" => Frame::Science,
                    "Individual" => Frame::Individual,
                    "Consensus" => Frame::Consensus,
                    "Absolute" => Frame::Absolute,
                    _ => Frame::Meta,
                };
                let value = TritValue::from(s.value);
                trits.push(TritWord::new(value, s.phase, frame));
            }
            // TAND cascade
            let mut current = trits[0].clone();
            let mut interrupts = Vec::new();
            for next in &trits[1..] {
                let (res, int) = TernaryAlgebra::t_and(&current, next);
                current = res;
                if let Some(i) = int {
                    interrupts.push(i);
                }
            }
            // Arbitration
            let domain = Domain::General; // simplified for bench
            let policy = ResolutionPolicy::new(domain);
            let _action = policy.arbitrate(&trits);
            // SafeFallback
            let sf = SafeFallback::new();
            let (_final_word, _interrupt) =
                sf.guard(&Domain::MedicalEthics, &current, interrupts.len());
            // Output construction
            let output = SandboxOutput {
                scenario_id: input.id.clone(),
                final_value: format!("{:?}", current.value),
                final_value_code: current.value.to_i8(),
                final_frame: format!("{}", current.frame),
                final_phase: current.phase.inner(),
                interrupts: interrupts.iter().map(|i| i.reason.clone()).collect(),
                policy_action: "Hold".to_string(),
            };
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
            let mut trits = Vec::new();
            for s in &input.signals {
                let frame = match s.frame.as_str() {
                    "Science" => Frame::Science,
                    "Individual" => Frame::Individual,
                    _ => Frame::Meta,
                };
                trits.push(TritWord::new(TritValue::from(s.value), s.phase, frame));
            }
            let mut current = trits[0].clone();
            let mut interrupts = Vec::new();
            for next in &trits[1..] {
                let (res, int) = TernaryAlgebra::t_and(&current, next);
                current = res;
                if let Some(i) = int {
                    interrupts.push(i);
                }
            }
            let policy = ResolutionPolicy::new(Domain::Physical);
            let _action = policy.arbitrate(&trits);
            black_box((current, interrupts.len()));
        });
    });
}

// ---------------------------------------------------------------------------
// TCP frame roundtrip benchmark
// ---------------------------------------------------------------------------

fn bench_tcp_frame_roundtrip(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let msg = Message::resonate_req("node1", "Science", 0.8, vec![]);

    c.bench_function("tcp_frame_roundtrip", |b| {
        b.iter(|| {
            rt.block_on(async {
                let json = serde_json::to_vec(black_box(&msg)).unwrap();
                let mut buf = Vec::new();
                frame_codec::write_frame(&mut buf, black_box(&json))
                    .await
                    .unwrap();
                let mut cursor = std::io::Cursor::new(buf);
                let read_back = frame_codec::read_frame(&mut cursor).await.unwrap();
                let _msg: Message = serde_json::from_slice(&read_back).unwrap();
                black_box(_msg)
            });
        });
    });
}

fn bench_tcp_frame_serialize_only(c: &mut Criterion) {
    let msg = Message::resonate_req("node1", "Science", 0.8, vec![]);

    c.bench_function("tcp_frame_serialize_only", |b| {
        b.iter(|| {
            let json = serde_json::to_vec(black_box(&msg)).unwrap();
            black_box(json);
        });
    });
}

fn bench_tcp_frame_deserialize_only(c: &mut Criterion) {
    let msg = Message::resonate_req("node1", "Science", 0.8, vec![]);
    let json = serde_json::to_vec(&msg).unwrap();

    c.bench_function("tcp_frame_deserialize_only", |b| {
        b.iter(|| {
            let _msg: Message = serde_json::from_slice(black_box(&json)).unwrap();
            black_box(_msg);
        });
    });
}

// ---------------------------------------------------------------------------
// Concurrent ResonanceBus stress test
// ---------------------------------------------------------------------------

fn bench_concurrent_bus_register_100_nodes(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let frames = [
        Frame::Science,
        Frame::Individual,
        Frame::Consensus,
        Frame::Meta,
    ];

    c.bench_function("concurrent_bus_register_100_nodes", |b| {
        b.iter(|| {
            rt.block_on(async {
                let bus = Arc::new(Mutex::new(ResonanceBus::new()));
                for i in 0..100u32 {
                    let mut b = bus.lock().await;
                    b.register(Node::new(
                        format!("node-{}", i),
                        frames[i as usize % 4].clone(),
                        (i as f64 % 100.0) / 100.0,
                    ));
                }
                black_box(bus);
            });
        });
    });
}

fn bench_concurrent_bus_10_resonates(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("concurrent_bus_10_resonates", |b| {
        b.iter(|| {
            rt.block_on(async {
                let bus = Arc::new(Mutex::new(ResonanceBus::new()));
                {
                    let mut b = bus.lock().await;
                    b.register(Node::new("a".into(), Frame::Science, 0.7));
                    b.register(Node::new("b".into(), Frame::Science, 0.8));
                }
                let mut handles = Vec::new();
                for _ in 0..10 {
                    let bus_clone = bus.clone();
                    let handle = tokio::spawn(async move {
                        let req = Message::resonate_req("a", "Science", 0.7, vec![]);
                        let mut b = bus_clone.lock().await;
                        black_box(b.handle_resonate_req("a", "b", &req));
                    });
                    handles.push(handle);
                }
                for h in handles {
                    let _ = h.await;
                }
                black_box(bus);
            });
        });
    });
}

// ---------------------------------------------------------------------------
// JSON serde isolation benchmarks
// ---------------------------------------------------------------------------

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
        final_phase: 0.5,
        interrupts: vec!["FrameMismatch".into(), "PhaseDrift".into()],
        policy_action: "Hold".into(),
    };

    c.bench_function("json_ser_output", |b| {
        b.iter(|| {
            let _json = serde_json::to_string(black_box(&output)).unwrap();
        });
    });
}

fn bench_json_serde_message_roundtrip(c: &mut Criterion) {
    let msg = Message::resonate_req("bench-node", "Science", 0.75, vec![0.5, 0.6]);

    c.bench_function("json_message_roundtrip", |b| {
        b.iter(|| {
            let json = serde_json::to_vec(black_box(&msg)).unwrap();
            let _msg: Message = serde_json::from_slice(&json).unwrap();
            black_box(_msg);
        });
    });
}

fn bench_json_serde_negotiate_large(c: &mut Criterion) {
    let participants: Vec<String> = (0..10).map(|i| format!("node-{}", i)).collect();
    let frames: Vec<String> = (0..10)
        .map(|i| {
            match i % 4 {
                0 => "Science",
                1 => "Individual",
                2 => "Consensus",
                _ => "Meta",
            }
            .to_string()
        })
        .collect();
    let phases: Vec<f64> = (0..10).map(|i| i as f64 / 10.0).collect();
    let msg = Message::negotiate("bench", participants, frames, phases, "hold");

    c.bench_function("json_message_negotiate_10", |b| {
        b.iter(|| {
            let json = serde_json::to_vec(black_box(&msg)).unwrap();
            let _msg: Message = serde_json::from_slice(&json).unwrap();
            black_box((_msg, json.len()));
        });
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

criterion_group!(
    pipeline,
    bench_full_pipeline_medical,
    bench_full_pipeline_physical,
);

criterion_group!(
    tcp_roundtrip,
    bench_tcp_frame_roundtrip,
    bench_tcp_frame_serialize_only,
    bench_tcp_frame_deserialize_only,
);

criterion_group!(
    concurrent_bus,
    bench_concurrent_bus_register_100_nodes,
    bench_concurrent_bus_10_resonates,
);

criterion_group!(
    json_serde,
    bench_json_serde_scenario_deser,
    bench_json_serde_output_ser,
    bench_json_serde_message_roundtrip,
    bench_json_serde_negotiate_large,
);

criterion_main!(
    core_ops,
    hot_path,
    cascades,
    cross_domain,
    phase_precision,
    pipeline,
    tcp_roundtrip,
    concurrent_bus,
    json_serde,
);
