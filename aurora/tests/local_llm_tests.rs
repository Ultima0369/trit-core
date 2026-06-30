use aurora::percept::cloud::CloudLLMProvider;
use serde_json::json;

#[test]
fn local_llm_parse_response_same_format_as_cloud() {
    let response = json!({
        "choices": [{
            "message": {
                "content": json!({
                    "signals": [
                        {"frame": "Individual", "value": 0, "phase": 0.5, "reasoning": "uncertain"}
                    ],
                    "confidence": 0.7,
                    "suggested_scenario": "General",
                    "summary": "local model output"
                }).to_string()
            }
        }]
    });

    let batch = CloudLLMProvider::parse_openai_response(&response).unwrap();
    assert_eq!(batch.signals.len(), 1);
    assert_eq!(batch.confidence, 0.7);
}
