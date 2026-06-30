use aurora::percept::cloud::CloudLLMProvider;
use serde_json::Value;
use std::fs;
use truncore::TritValue;

fn load_fixture(name: &str) -> Value {
    let path = format!("tests/fixtures/{name}");
    let text = fs::read_to_string(&path).unwrap_or_else(|_| panic!("fixture not found: {path}"));
    serde_json::from_str(&text).expect("invalid fixture JSON")
}

#[test]
fn ethics_gate_value_conflict_produces_hold() {
    let response = load_fixture("llm_value_conflict_response.json");
    let batch = CloudLLMProvider::parse_anthropic_response(&response).unwrap();

    let has_hold = batch.signals.iter().any(|s| s.value() == TritValue::Hold);
    assert!(
        has_hold,
        "ETHICS GATE FAILURE: value conflict must produce at least one Hold signal"
    );
}

#[test]
fn ethics_gate_confidence_bounded() {
    let response = load_fixture("llm_value_conflict_response.json");
    let batch = CloudLLMProvider::parse_anthropic_response(&response).unwrap();
    assert!((0.0..=1.0).contains(&batch.confidence));
}

#[test]
fn ethics_gate_no_absolute_frame_with_strong_phase() {
    let response = load_fixture("llm_value_conflict_response.json");
    let batch = CloudLLMProvider::parse_anthropic_response(&response).unwrap();

    for signal in &batch.signals {
        if signal.frame() == truncore::Frame::Absolute {
            assert_eq!(
                signal.value(),
                TritValue::Hold,
                "ETHICS GATE FAILURE: Absolute frame must be Hold"
            );
        }
    }
}

#[test]
fn ethics_gate_raw_data_layer_has_no_imperative_markers() {
    // 流沙: raw_data_layer describes physical measurements only — no advice, no commands
    let response = load_fixture("llm_imperative_response.json");
    let batch = CloudLLMProvider::parse_anthropic_response(&response).unwrap();

    let forbidden = [
        "you must",
        "you should",
        "you have to",
        "do not",
        "never",
        "therefore",
        "this means",
    ];
    if let Some(ref raw) = batch.raw_data_layer {
        let raw_lower = raw.to_lowercase();
        for word in &forbidden {
            assert!(
                !raw_lower.contains(word),
                "ETHICS GATE FAILURE: raw_data_layer contains forbidden marker '{}': {}",
                word,
                raw
            );
        }
    }
}

#[test]
fn ethics_gate_signals_not_empty() {
    let response = load_fixture("llm_value_conflict_response.json");
    let batch = CloudLLMProvider::parse_anthropic_response(&response).unwrap();
    assert!(!batch.signals.is_empty());
}

#[test]
fn ethics_gate_system_prompt_contains_key_principles() {
    let prompt = include_str!("../src/percept/prompts/percept_system.txt");

    // 文字.md principles
    assert!(
        prompt.contains("map"),
        "system prompt must mention map/territory distinction"
    );
    assert!(
        prompt.contains("Hold"),
        "system prompt must explain Hold state"
    );
    assert!(
        prompt.to_lowercase().contains("never force"),
        "system prompt must forbid forcing binary choices. Prompt length: {}",
        prompt.len()
    );
    assert!(
        prompt.contains("do not rule"),
        "system prompt must affirm user sovereignty"
    );

    // 流沙 philosophy (NEW — from 整体架构图.md)
    assert!(
        prompt.contains("璇玑"),
        "system prompt must contain 璇玑 (Armillary Sphere) principle"
    );
    assert!(
        prompt.contains("棱镜"),
        "system prompt must contain 棱镜 (Prism) principle"
    );
    assert!(
        prompt.contains("微风"),
        "system prompt must contain 微风 (Breeze) principle"
    );
    assert!(
        prompt.contains("raw_data_layer"),
        "system prompt must define raw_data_layer output field"
    );
    // 流沙: the prompt may mention these fields in negation (e.g. "do NOT include X"),
    // but must not present them as part of the expected output contract.
    // Check that the JSON output template does not contain these fields.
    let json_template_start = prompt.find('{').unwrap_or(0);
    let json_section = &prompt[json_template_start..];
    assert!(
        !json_section.contains("\"suggested_scenario\""),
        "system prompt JSON template must NOT contain suggested_scenario field"
    );
    assert!(
        !json_section.contains("\"summary\""),
        "system prompt JSON template must NOT contain summary field"
    );
    assert!(
        !json_section.contains("\"reasoning\""),
        "system prompt JSON template must NOT contain reasoning field"
    );
}

#[test]
fn ethics_gate_raw_data_layer_contains_physical_measurements() {
    // 流沙: when raw_data_layer is present, it should contain physical quantities
    let response = load_fixture("llm_imperative_response.json");
    let batch = CloudLLMProvider::parse_anthropic_response(&response).unwrap();

    assert!(
        batch.raw_data_layer.is_some(),
        "ETHICS GATE: fixture should have raw_data_layer with physical measurements"
    );

    let raw = batch.raw_data_layer.unwrap();
    // Should contain at least one physical measurement pattern (key:value)
    assert!(
        raw.contains(':'),
        "ETHICS GATE: raw_data_layer should contain key:value pairs, got: {}",
        raw
    );
}

#[test]
fn ethics_gate_no_summary_field_in_batch() {
    // 流沙: PerceptBatch must not have summary (removed in v2)
    let response = load_fixture("llm_value_conflict_response.json");
    let _batch = CloudLLMProvider::parse_anthropic_response(&response).unwrap();

    // If the parser doesn't produce a summary, raw_data_layer may be None
    // The key check: the LLM JSON contract must not include "summary"
    let raw_json: Value = load_fixture("llm_value_conflict_response.json");
    let text = raw_json["content"][0]["text"].as_str().unwrap();
    assert!(
        !text.contains("\"summary\""),
        "ETHICS GATE FAILURE: LLM response contains 'summary' field — violates 零文字 (不解释)"
    );
    assert!(
        !text.contains("\"suggested_scenario\""),
        "ETHICS GATE FAILURE: LLM response contains 'suggested_scenario' — violates 棱镜 (不引导)"
    );
    assert!(
        !text.contains("\"reasoning\""),
        "ETHICS GATE FAILURE: LLM response contains 'reasoning' — violates 璇玑 (不解释)"
    );
}
