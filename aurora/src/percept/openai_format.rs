//! Shared OpenAI-format response parsing.
//!
//! Extracted from `CloudLLMProvider` during BC Architecture Hardening (2026-07-08).
//! Both `cloud.rs` and `local.rs` parse the same OpenAI-compatible JSON format,
//! so the parsing logic lives here as free functions.

use serde_json::Value;
use trit_core::{Frame, Phase, TritValue, TritWord};

use crate::percept::{PerceptBatch, PerceptError};

/// Parse an OpenAI Chat Completions response into a [`PerceptBatch`].
///
/// Extracts `choices[0].message.content`, parses the inner JSON structure
/// (signals array, confidence, raw_data_layer), and returns a PerceptBatch.
pub fn parse_openai_response(response: &Value) -> Result<PerceptBatch, PerceptError> {
    let text = response["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| PerceptError::ParseError("missing choices[0].message.content".into()))?;
    parse_openai_inner(text)
}

/// Parse the inner JSON content (the text from the LLM response).
///
/// This is the common parser for both OpenAI and Anthropic formats —
/// the wrapper differs but the inner JSON structure is identical.
pub fn parse_openai_inner(text: &str) -> Result<PerceptBatch, PerceptError> {
    let inner: Value =
        serde_json::from_str(text).map_err(|e| PerceptError::ParseError(e.to_string()))?;

    let signals_array = inner["signals"]
        .as_array()
        .ok_or_else(|| PerceptError::ParseError("missing 'signals' array".into()))?;

    let mut signals = Vec::with_capacity(signals_array.len());
    for sig in signals_array {
        if let Some(word) = parse_signal(sig) {
            signals.push(word);
        }
    }

    let confidence = inner["confidence"].as_f64().unwrap_or(0.5).clamp(0.0, 1.0);
    let raw_data_layer = inner["raw_data_layer"].as_str().map(|s| s.to_string());

    Ok(PerceptBatch {
        signals,
        source: "cloud-llm".into(),
        timestamp: chrono::Utc::now(),
        confidence,
        raw_data_layer,
    })
}

/// Parse a single signal object from the LLM's JSON output.
fn parse_signal(sig: &Value) -> Option<TritWord> {
    let frame_str = sig["frame"].as_str()?;
    let frame = match frame_str {
        "Science" => Frame::Science,
        "Individual" => Frame::Individual,
        "Consensus" => Frame::Consensus,
        "Absolute" => Frame::Absolute,
        "FirstPerson" => Frame::FirstPerson,
        "Embodied" => Frame::Embodied,
        "Relational" => Frame::Relational,
        "GeoEco" => Frame::GeoEco,
        "Developmental" => Frame::Developmental,
        "Role" => Frame::Role,
        "Environmental" => Frame::Environmental,
        _ => {
            tracing::warn!(frame = frame_str, "unknown frame from LLM, skipping signal");
            return None;
        }
    };

    let raw_value = sig["value"].as_i64().unwrap_or(0);
    let value = match raw_value {
        1 => TritValue::True,
        -1 => TritValue::False,
        _ => TritValue::Hold,
    };

    let phase_val = sig["phase"].as_f64().unwrap_or(0.5).clamp(0.0, 1.0);
    let phase = Phase::new_clamped(phase_val);

    Some(TritWord::new(value, phase, frame))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_openai_inner_extracts_signals() {
        let text = json!({
            "signals": [
                {"frame": "Science", "value": 1, "phase": 0.8},
                {"frame": "Individual", "value": -1, "phase": 0.3}
            ],
            "confidence": 0.9,
            "raw_data_layer": "test"
        })
        .to_string();

        let batch = parse_openai_inner(&text).unwrap();
        assert_eq!(batch.signals.len(), 2);
        assert!((batch.confidence - 0.9).abs() < f64::EPSILON);
        assert_eq!(batch.raw_data_layer.as_deref(), Some("test"));
        assert_eq!(batch.source, "cloud-llm");
    }

    #[test]
    fn parse_openai_inner_missing_signals_returns_error() {
        let text = json!({"confidence": 0.5}).to_string();
        let err = parse_openai_inner(&text).unwrap_err();
        assert!(matches!(err, PerceptError::ParseError(_)));
    }

    #[test]
    fn parse_openai_inner_malformed_json_returns_error() {
        let err = parse_openai_inner("not json").unwrap_err();
        assert!(matches!(err, PerceptError::ParseError(_)));
    }

    #[test]
    fn parse_openai_inner_empty_signals_is_ok() {
        let text = json!({"signals": [], "confidence": 0.5}).to_string();
        let batch = parse_openai_inner(&text).unwrap();
        assert!(batch.signals.is_empty());
    }

    #[test]
    fn parse_openai_inner_unknown_frame_is_skipped() {
        let text = json!({
            "signals": [
                {"frame": "NonExistent", "value": 1, "phase": 0.5}
            ],
            "confidence": 0.5
        })
        .to_string();
        let batch = parse_openai_inner(&text).unwrap();
        assert!(batch.signals.is_empty());
    }

    #[test]
    fn parse_openai_inner_defaults_missing_fields() {
        let text = json!({"signals": [{"frame": "Science"}]}).to_string();
        let batch = parse_openai_inner(&text).unwrap();
        assert_eq!(batch.signals.len(), 1);
        assert_eq!(batch.signals[0].value(), trit_core::TritValue::Hold);
        assert!((batch.confidence - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_openai_response_extracts_content() {
        let response = json!({
            "choices": [{
                "message": {
                    "content": json!({
                        "signals": [{"frame": "Embodied", "value": 1, "phase": 0.7}],
                        "confidence": 0.85
                    }).to_string()
                }
            }]
        });
        let batch = parse_openai_response(&response).unwrap();
        assert_eq!(batch.signals.len(), 1);
        assert!((batch.confidence - 0.85).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_openai_response_missing_content_returns_error() {
        let response = json!({"choices": []});
        let err = parse_openai_response(&response).unwrap_err();
        assert!(matches!(err, PerceptError::ParseError(_)));
    }
}
