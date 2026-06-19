//! Error path tests: verify that all SandboxError variants are correctly
//! triggered and produce informative error messages.

use trit_core::sandbox::input::{ScenarioInput, SignalInput};
use trit_core::sandbox::{
    validate_domain, validate_scenario, SandboxOutput, MAX_SIGNALS, MAX_STRING_LEN,
};

fn signal(frame: &str, value: i8, phase: f64) -> SignalInput {
    SignalInput {
        frame: frame.into(),
        value,
        phase,
        sensor: None,
    }
}

fn scenario(id: &str, domain: &str, signals: Vec<SignalInput>) -> ScenarioInput {
    ScenarioInput {
        id: id.into(),
        description: "test".into(),
        domain: domain.into(),
        signals,
        expected_behavior: "hold".into(),
        environmental_context: None,
    }
}

// --- InvalidFrame ---

#[test]
fn error_invalid_frame_unknown() {
    let s = scenario("x", "General", vec![signal("Bogus", 1, 0.5)]);
    let err = validate_scenario(&s).unwrap_err();
    let msg = format!("{}", err);
    assert!(msg.contains("unknown frame"), "got: {}", msg);
}

// --- InvalidPhase ---

#[test]
fn error_invalid_phase_nan() {
    let s = scenario("x", "General", vec![signal("Science", 1, f64::NAN)]);
    let err = validate_scenario(&s).unwrap_err();
    let msg = format!("{}", err);
    assert!(msg.contains("phase"), "got: {}", msg);
}

#[test]
fn error_invalid_phase_infinity() {
    let s = scenario("x", "General", vec![signal("Science", 1, f64::INFINITY)]);
    let err = validate_scenario(&s).unwrap_err();
    let msg = format!("{}", err);
    assert!(msg.contains("phase"), "got: {}", msg);
}

#[test]
fn error_invalid_phase_out_of_range() {
    let s = scenario("x", "General", vec![signal("Science", 1, 1.5)]);
    let err = validate_scenario(&s).unwrap_err();
    let msg = format!("{}", err);
    assert!(msg.contains("phase"), "got: {}", msg);
}

#[test]
fn error_invalid_phase_negative() {
    let s = scenario("x", "General", vec![signal("Science", 1, -0.1)]);
    let err = validate_scenario(&s).unwrap_err();
    let msg = format!("{}", err);
    assert!(msg.contains("phase"), "got: {}", msg);
}

// --- InvalidValue ---

#[test]
fn error_invalid_value_out_of_range() {
    let s = scenario("x", "General", vec![signal("Science", 2, 0.5)]);
    let err = validate_scenario(&s).unwrap_err();
    let msg = format!("{}", err);
    assert!(msg.contains("value"), "got: {}", msg);
}

#[test]
fn error_invalid_value_large() {
    let s = scenario("x", "General", vec![signal("Science", 127, 0.5)]);
    let err = validate_scenario(&s).unwrap_err();
    let msg = format!("{}", err);
    assert!(msg.contains("value"), "got: {}", msg);
}

// --- InvalidDomain ---

#[test]
fn error_invalid_domain_unknown() {
    let err = validate_domain("Mystic").unwrap_err();
    let msg = format!("{}", err);
    assert!(msg.contains("unknown domain"), "got: {}", msg);
}

#[test]
fn error_invalid_domain_empty() {
    let err = validate_domain("").unwrap_err();
    let msg = format!("{}", err);
    assert!(msg.contains("unknown domain"), "got: {}", msg);
}

// --- InvalidScenario ---

#[test]
fn error_scenario_empty_signals() {
    let s = scenario("x", "General", vec![]);
    let err = validate_scenario(&s).unwrap_err();
    let msg = format!("{}", err);
    assert!(msg.contains("At least one signal"), "got: {}", msg);
}

#[test]
fn error_scenario_too_many_signals() {
    let signals: Vec<_> = (0..=MAX_SIGNALS)
        .map(|_| signal("Science", 1, 0.5))
        .collect();
    let s = scenario("x", "General", signals);
    let err = validate_scenario(&s).unwrap_err();
    let msg = format!("{}", err);
    assert!(msg.contains("Too many signals"), "got: {}", msg);
}

#[test]
fn error_scenario_id_too_long() {
    let s = scenario(
        &"x".repeat(MAX_STRING_LEN + 1),
        "General",
        vec![signal("Science", 1, 0.5)],
    );
    let err = validate_scenario(&s).unwrap_err();
    let msg = format!("{}", err);
    assert!(msg.contains("id too long"), "got: {}", msg);
}

// --- SandboxOutput deserialization validation ---

#[test]
fn error_output_rejects_out_of_range_phase() {
    let json = r#"{
        "scenario_id": "test",
        "final_value": "Hold",
        "final_value_code": 0,
        "final_frame": "Meta",
        "final_phase": 1.5,
        "interrupts": [],
        "policy_action": "Hold"
    }"#;
    let result = serde_json::from_str::<SandboxOutput>(json);
    assert!(result.is_err());
}

#[test]
fn error_output_rejects_negative_phase() {
    let json = r#"{
        "scenario_id": "test",
        "final_value": "Hold",
        "final_value_code": 0,
        "final_frame": "Meta",
        "final_phase": -0.1,
        "interrupts": [],
        "policy_action": "Hold"
    }"#;
    let result = serde_json::from_str::<SandboxOutput>(json);
    assert!(result.is_err());
}

#[test]
fn error_output_rejects_invalid_value_code() {
    let json = r#"{
        "scenario_id": "test",
        "final_value": "Hold",
        "final_value_code": 5,
        "final_frame": "Meta",
        "final_phase": 0.5,
        "interrupts": [],
        "policy_action": "Hold"
    }"#;
    let result = serde_json::from_str::<SandboxOutput>(json);
    assert!(result.is_err());
}

#[test]
fn output_accepts_valid_deserialization() {
    let json = r#"{
        "scenario_id": "test",
        "final_value": "True",
        "final_value_code": 1,
        "final_frame": "Science",
        "final_phase": 0.8,
        "interrupts": [],
        "policy_action": "Commit"
    }"#;
    let result: SandboxOutput = serde_json::from_str(json).unwrap();
    assert_eq!(result.final_value_code, 1);
    assert!((result.final_phase_raw - 0.8).abs() < f64::EPSILON);
}
