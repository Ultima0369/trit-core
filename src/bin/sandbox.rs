use std::fs;
use trit_core::frame::Frame;
use trit_core::meta::{Domain, MetaInterrupt, MetaMonitor, ResolutionPolicy};
use trit_core::sandbox::{SandboxOutput, ScenarioInput};
use trit_core::trit::algebra::TernaryAlgebra;
use trit_core::trit::{TritValue, TritWord};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 || args[1] != "--scenario" {
        eprintln!("Usage: trit-sandbox --scenario <path.json>");
        std::process::exit(1);
    }

    let path = &args[2];
    let raw = fs::read_to_string(path).expect("Failed to read scenario file");
    let scenario: ScenarioInput = serde_json::from_str(&raw).expect("Invalid JSON");

    let policy = match scenario.domain.as_str() {
        "Physical" => ResolutionPolicy::new(Domain::Physical),
        "Engineering" => ResolutionPolicy::new(Domain::Engineering),
        "MedicalEthics" => ResolutionPolicy::new(Domain::MedicalEthics),
        "ValueJudgment" => ResolutionPolicy::new(Domain::ValueJudgment),
        _ => ResolutionPolicy::new(Domain::General),
    };

    let mut monitor = MetaMonitor::new(policy.clone());

    let trits: Vec<TritWord> = scenario
        .signals
        .iter()
        .map(|s| {
            let frame = match s.frame.as_str() {
                "Science" => Frame::Science,
                "Individual" => Frame::Individual,
                "Consensus" => Frame::Consensus,
                "Absolute" => Frame::Absolute,
                _ => Frame::Meta,
            };
            let val = match s.value {
                1 => TritValue::True,
                -1 => TritValue::False,
                _ => TritValue::Hold,
            };
            TritWord::new(val, s.phase, frame)
        })
        .collect();

    // Aggregate via TAND cascade
    let mut current = trits[0].clone();
    let mut interrupts: Vec<MetaInterrupt> = vec![];

    for next in &trits[1..] {
        let (result, maybe_int) = TernaryAlgebra::t_and(&current, next);
        if let Some(int) = maybe_int {
            monitor.record(int.clone());
            interrupts.push(int);
        }
        current = result;
    }

    // Policy arbitration if still in conflict
    let policy_result = policy.arbitrate(&trits);
    let final_word = match &policy_result {
        trit_core::meta::ArbitrationResult::Commit(w) => w.clone(),
        trit_core::meta::ArbitrationResult::Preserve(w) => w.clone(),
        _ => current.clone(),
    };

    let output = SandboxOutput {
        scenario_id: scenario.id.clone(),
        final_value: final_word.value.to_i8(),
        final_frame: format!("{}", final_word.frame),
        final_phase: final_word.phase.inner(),
        interrupts: interrupts
            .iter()
            .map(|i| format!("{:?}: {}", i.conflict, i.reason))
            .collect(),
        policy_action: format!("{:?}", policy_result),
    };

    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
