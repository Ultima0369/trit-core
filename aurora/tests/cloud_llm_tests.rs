use aurora::percept::cloud::CloudLLMProvider;
use serde_json::json;
use truncore::TritValue;

#[test]
fn parse_valid_anthropic_response() {
    let response = json!({
        "content": [{
            "text": json!({
                "signals": [
                    {"frame": "Science", "value": 1, "phase": 0.8, "reasoning": "clear evidence"},
                    {"frame": "Individual", "value": 0, "phase": 0.5, "reasoning": "uncertain"}
                ],
                "confidence": 0.85,
                "suggested_scenario": "General",
                "summary": "Mixed input detected"
            }).to_string()
        }]
    });

    let batch = CloudLLMProvider::parse_anthropic_response(&response).unwrap();
    assert_eq!(batch.signals.len(), 2);
    assert_eq!(batch.confidence, 0.85);
}

#[test]
fn parse_valid_openai_response() {
    let response = json!({
        "choices": [{
            "message": {
                "content": json!({
                    "signals": [
                        {"frame": "Consensus", "value": -1, "phase": 0.9, "reasoning": "social rejection"}
                    ],
                    "confidence": 0.95,
                    "suggested_scenario": "ValueConflict",
                    "summary": "Norm violation"
                }).to_string()
            }
        }]
    });

    let batch = CloudLLMProvider::parse_openai_response(&response).unwrap();
    assert_eq!(batch.signals.len(), 1);
    assert_eq!(batch.confidence, 0.95);
}

#[test]
fn parse_response_with_hold_signal() {
    let response = json!({
        "content": [{
            "text": json!({
                "signals": [
                    {"frame": "Absolute", "value": 0, "phase": 0.5, "reasoning": "cannot resolve"}
                ],
                "confidence": 0.6,
                "suggested_scenario": "ValueConflict",
                "summary": "Unresolvable tension"
            }).to_string()
        }]
    });

    let batch = CloudLLMProvider::parse_anthropic_response(&response).unwrap();
    assert_eq!(batch.signals[0].value(), TritValue::Hold);
}

#[test]
fn parse_malformed_json_returns_parse_error() {
    let response = json!({
        "content": [{"text": "not valid json at all"}]
    });
    let result = CloudLLMProvider::parse_anthropic_response(&response);
    assert!(result.is_err());
}

#[test]
fn parse_missing_signals_field_returns_error() {
    let response = json!({
        "content": [{
            "text": json!({"confidence": 0.5, "summary": "no signals"}).to_string()
        }]
    });
    let result = CloudLLMProvider::parse_anthropic_response(&response);
    assert!(result.is_err());
}

#[test]
fn parse_empty_signals_array_is_valid() {
    let response = json!({
        "content": [{
            "text": json!({
                "signals": [],
                "confidence": 1.0,
                "suggested_scenario": "General",
                "summary": "nothing to report"
            }).to_string()
        }]
    });
    let batch = CloudLLMProvider::parse_anthropic_response(&response).unwrap();
    assert!(batch.signals.is_empty());
}

#[test]
fn parse_invalid_frame_name_skips_signal() {
    let response = json!({
        "content": [{
            "text": json!({
                "signals": [
                    {"frame": "NotARealFrame", "value": 1, "phase": 0.5, "reasoning": "bad"},
                    {"frame": "Science", "value": 1, "phase": 0.8, "reasoning": "good"}
                ],
                "confidence": 0.7,
                "summary": "one bad frame skipped"
            }).to_string()
        }]
    });
    let batch = CloudLLMProvider::parse_anthropic_response(&response).unwrap();
    assert_eq!(batch.signals.len(), 1);
}

#[test]
fn parse_invalid_value_does_not_panic() {
    let response = json!({
        "content": [{
            "text": json!({
                "signals": [
                    {"frame": "Science", "value": 999, "phase": 0.5, "reasoning": "out of range"}
                ],
                "confidence": 0.5,
                "summary": "bad value"
            }).to_string()
        }]
    });
    let result = CloudLLMProvider::parse_anthropic_response(&response);
    let _ = result; // no panic
}
