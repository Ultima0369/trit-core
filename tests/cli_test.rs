use std::process::Command;

fn run_sandbox(args: &[&str]) -> (bool, String, String) {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--bin", "trit-sandbox", "--"])
        .args(args)
        .output()
        .expect("cargo run should execute");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (output.status.success(), stdout, stderr)
}

#[test]
fn cli_runs_medical_conflict_scenario() {
    let (success, stdout, _stderr) =
        run_sandbox(&["--scenario", "scenarios/medical_conflict_01.json"]);
    assert!(success, "CLI should succeed");
    assert!(
        stdout.contains("final_value"),
        "output should contain final_value"
    );
    assert!(
        stdout.contains("\"final_value_code\": -1"),
        "medical_conflict_01 should commit false"
    );
}

#[test]
fn cli_runs_bridge_safety_scenario() {
    let (success, stdout, _stderr) = run_sandbox(&["--scenario", "scenarios/bridge_safety.json"]);
    assert!(success, "CLI should succeed");
    assert!(
        stdout.contains("\"final_value_code\": -1"),
        "bridge_safety should commit false"
    );
}

#[test]
fn cli_runs_career_value_conflict_scenario() {
    let (success, stdout, _stderr) =
        run_sandbox(&["--scenario", "scenarios/career_value_conflict.json"]);
    assert!(success, "CLI should succeed");
    assert!(
        stdout.contains("\"final_value_code\": 0"),
        "career_value_conflict should hold"
    );
}

#[test]
fn cli_runs_medical_pain_dismissed_scenario() {
    let (success, stdout, _stderr) =
        run_sandbox(&["--scenario", "scenarios/medical_pain_dismissed.json"]);
    assert!(success, "CLI should succeed");
    assert!(
        stdout.contains("\"final_value_code\": -1"),
        "medical_pain_dismissed should preserve individual as false"
    );
    assert!(
        stdout.contains("Individual"),
        "medical_pain_dismissed should resolve to Individual frame"
    );
}

#[test]
fn cli_runs_general_conceptual_spin_scenario() {
    let (success, stdout, _stderr) =
        run_sandbox(&["--scenario", "scenarios/general_conceptual_spin.json"]);
    assert!(success, "CLI should succeed");
    assert!(
        stdout.contains("\"final_value_code\": 0"),
        "general_conceptual_spin should negotiate to hold"
    );
}

#[test]
fn cli_runs_engineering_evacuation_consensus_scenario() {
    let (success, stdout, _stderr) = run_sandbox(&[
        "--scenario",
        "scenarios/engineering_evacuation_consensus.json",
    ]);
    assert!(success, "CLI should succeed");
    assert!(
        stdout.contains("\"final_value_code\": -1"),
        "engineering_evacuation_consensus should commit false"
    );
    assert!(
        stdout.contains("Science"),
        "engineering_evacuation_consensus should resolve to Science frame"
    );
}

#[test]
fn cli_runs_chinese_medical_pain_dismissed_scenario() {
    let (success, stdout, _stderr) =
        run_sandbox(&["--scenario", "scenarios/medical_pain_dismissed.zh.json"]);
    assert!(success, "CLI should succeed for Chinese scenario file");
    assert!(
        stdout.contains("\"final_value_code\": -1"),
        "Chinese medical_pain_dismissed should also preserve individual as false"
    );
}

#[test]
fn cli_runs_value_algorithmic_displacement_scenario() {
    let (success, stdout, _stderr) = run_sandbox(&[
        "--scenario",
        "scenarios/value_algorithmic_displacement.json",
    ]);
    assert!(success, "CLI should succeed");
    assert!(
        stdout.contains("\"final_value_code\": 0"),
        "value_algorithmic_displacement should hold"
    );
    assert!(
        stdout.contains("Meta"),
        "value_algorithmic_displacement should resolve to Meta frame"
    );
}

#[test]
fn cli_runs_general_water_rights_scenario() {
    let (success, stdout, _stderr) =
        run_sandbox(&["--scenario", "scenarios/general_water_rights.json"]);
    assert!(success, "CLI should succeed");
    assert!(
        stdout.contains("\"final_value_code\": 0"),
        "general_water_rights should negotiate to hold"
    );
}

#[test]
fn cli_runs_engineering_dam_breach_risk_scenario() {
    let (success, stdout, _stderr) =
        run_sandbox(&["--scenario", "scenarios/engineering_dam_breach_risk.json"]);
    assert!(success, "CLI should succeed");
    assert!(
        stdout.contains("\"final_value_code\": -1"),
        "engineering_dam_breach_risk should commit false"
    );
    assert!(
        stdout.contains("Science"),
        "engineering_dam_breach_risk should resolve to Science frame"
    );
}

#[test]
fn cli_validate_only_reports_validation_success() {
    let (success, stdout, _stderr) = run_sandbox(&[
        "--scenario",
        "scenarios/medical_conflict_01.json",
        "--validate-only",
    ]);
    assert!(
        success,
        "--validate-only should succeed for a valid scenario"
    );
    assert!(
        stdout.contains("ValidateOnly"),
        "validate-only output should indicate validation mode"
    );
    assert!(
        stdout.contains("\"final_value_code\": 0"),
        "validate-only output should use Hold placeholder"
    );
}

#[test]
fn cli_dry_run_skips_arbitration() {
    let (success, stdout, _stderr) = run_sandbox(&[
        "--scenario",
        "scenarios/medical_conflict_01.json",
        "--dry-run",
    ]);
    assert!(success, "--dry-run should succeed");
    assert!(
        stdout.contains("DryRun"),
        "dry-run output should indicate DryRun policy action"
    );
    assert!(
        stdout.contains("\"final_value_code\": 0"),
        "dry-run on cross-frame input should produce Hold"
    );
}

#[test]
fn cli_rejects_path_traversal() {
    let (success, _stdout, stderr) = run_sandbox(&["--scenario", "../README.md"]);
    assert!(!success, "path traversal should fail");
    assert!(
        stderr.contains("Security error") || stderr.contains("path traversal"),
        "error should mention security or path traversal: {stderr}"
    );
}

#[test]
fn cli_runs_first_person_attention_scenario() {
    let (success, stdout, _stderr) =
        run_sandbox(&["--scenario", "scenarios/first_person_attention.json"]);
    assert!(success, "CLI should succeed for FirstPerson scenario");
    assert!(
        stdout.contains("\"final_value_code\": 0"),
        "FirstPerson vs Science in General domain should negotiate to hold"
    );
    assert!(
        !stdout.contains("receiver_estimate"),
        "receiver_estimate should not appear without --self-knowledge"
    );
}

#[test]
fn cli_self_knowledge_includes_receiver_estimate() {
    let (success, stdout, _stderr) = run_sandbox(&[
        "--scenario",
        "scenarios/first_person_attention.json",
        "--self-knowledge",
    ]);
    assert!(success, "CLI should succeed with --self-knowledge");
    assert!(
        stdout.contains("receiver_estimate"),
        "output should contain receiver_estimate when self-knowledge is enabled"
    );
}

#[test]
fn cli_reflexive_guard_overrides_forced_collapse() {
    let (success, stdout, _stderr) = run_sandbox(&[
        "--scenario",
        "scenarios/mind_reflexive_trigger.json",
        "--reflexive",
    ]);
    assert!(success, "CLI should succeed with --reflexive");
    assert!(
        stdout.contains("\"final_value_code\": 0"),
        "reflexive guard should override forced collapse to Hold"
    );
    assert!(
        stdout.contains("reflexive_alert"),
        "output should contain a reflexive alert"
    );
}

#[test]
fn cli_trace_phase_includes_diagnostics() {
    let (success, _stdout, stderr) = run_sandbox(&[
        "--scenario",
        "scenarios/first_person_attention.json",
        "--trace-phase",
        "--diagnostic",
    ]);
    assert!(
        success,
        "CLI should succeed with --trace-phase --diagnostic"
    );
    assert!(
        stderr.contains("phase_trace"),
        "diagnostic report should contain phase_trace"
    );
}

#[test]
fn cli_hold_final_succeeds() {
    let (success, stdout, _stderr) = run_sandbox(&[
        "--scenario",
        "scenarios/first_person_attention.json",
        "--hold-final",
    ]);
    assert!(success, "CLI should succeed with --hold-final");
    assert!(
        stdout.contains("hold_state"),
        "output should contain hold_state for a Hold result"
    );
}

#[test]
fn cli_rejects_unknown_argument() {
    let (success, _stdout, stderr) = run_sandbox(&[
        "--scenario",
        "scenarios/medical_conflict_01.json",
        "--unknown-flag",
    ]);
    assert!(!success, "unknown argument should fail");
    assert!(
        stderr.contains("unknown argument"),
        "error should mention unknown argument: {stderr}"
    );
}
