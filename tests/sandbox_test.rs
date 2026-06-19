use std::collections::HashMap;
use std::fs;
use trit_core::sandbox::{SandboxPipeline, ScenarioInput, ScenarioValidator};

/// Discover all scenario JSON files in the `scenarios/` directory.
fn scenario_files() -> Vec<String> {
    let mut files = vec![];
    for entry in fs::read_dir("scenarios").expect("scenarios/ directory should exist") {
        let entry = entry.expect("valid directory entry");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            files.push(path.to_string_lossy().to_string());
        }
    }
    files.sort();
    files
}

#[test]
fn all_scenarios_match_expected_behavior() {
    let mut failures = vec![];

    for path in scenario_files() {
        let raw = fs::read_to_string(&path).expect("scenario file should be readable");
        let scenario: ScenarioInput =
            serde_json::from_str(&raw).expect("scenario file should be valid JSON");

        // Skip scenarios without an expected_behavior assertion.
        if scenario.expected_behavior.is_empty() {
            continue;
        }

        let mut pipeline = SandboxPipeline::default();
        let output = match pipeline.run(&scenario) {
            Ok(o) => o,
            Err(e) => {
                failures.push(format!("{}: pipeline failed: {}", path, e));
                continue;
            }
        };

        if let Err(e) = ScenarioValidator::validate(&output, &scenario.expected_behavior) {
            failures.push(format!(
                "{} (id={}): expected '{}', got value_code={} action={}: {}",
                path,
                scenario.id,
                scenario.expected_behavior,
                output.final_value_code,
                output.policy_action,
                e
            ));
        }
    }

    if !failures.is_empty() {
        panic!(
            "{} scenario(s) failed expected_behavior validation:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }
}

#[test]
fn diagnostics_shape_matches_expected_fields() {
    let raw = fs::read_to_string("scenarios/medical_conflict_01.json")
        .expect("scenario file should be readable");
    let scenario: ScenarioInput =
        serde_json::from_str(&raw).expect("scenario file should be valid JSON");

    let mut pipeline = SandboxPipeline::default();
    let (output, diagnostics) = pipeline
        .run_with_diagnostics(&scenario)
        .expect("pipeline should succeed");

    // Basic shape assertions.
    assert_eq!(diagnostics.signal_count, scenario.signals.len());
    assert_eq!(
        diagnostics.interrupt_count,
        diagnostics.interrupt_types.len()
    );
    assert!(!diagnostics.policy_action.is_empty());
    assert!(diagnostics.elapsed_ns > 0);

    // Serialization round-trip: every public field should appear in JSON.
    let json = serde_json::to_value(&diagnostics).expect("diagnostics should serialize");
    assert!(json.get("elapsed_ns").is_some());
    assert!(json.get("signal_count").is_some());
    assert!(json.get("frame_distribution").is_some());
    assert!(json.get("interrupt_count").is_some());
    assert!(json.get("interrupt_types").is_some());
    assert!(json.get("policy_action").is_some());
    assert!(json.get("safe_fallback_triggered").is_some());
    assert!(json.get("stage_timings_ns").is_some());

    // SafeFallback did not trigger for this MedicalEthics scenario.
    assert!(!diagnostics.safe_fallback_triggered);

    // Policy action recorded in diagnostics should match the output action.
    assert!(diagnostics.policy_action.contains(&output.policy_action));

    // Stage timings should cover all pipeline stages.
    let stage_keys: HashMap<String, serde_json::Value> =
        serde_json::from_value(json["stage_timings_ns"].clone()).unwrap();
    for stage in [
        "validate",
        "build_policy",
        "build_trits",
        "t_and_n",
        "arbitrate",
        "reflexive_guard",
        "safe_fallback",
        "attention",
        "self_knowledge",
        "build_output",
    ] {
        assert!(
            stage_keys.contains_key(stage),
            "stage_timings_ns should contain {}",
            stage
        );
    }
}
