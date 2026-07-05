use crate::config::ConfigStore;
use crate::percept::{ExternalPercept, PerceptBatch, PerceptError};
use chrono::Utc;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use truncore::{Frame, Phase, TritValue, TritWord};

/// Max tokens requested from the LLM. Decomposition-only output is short.
const MAX_TOKENS: u32 = 1024;

/// Cap upstream error bodies so a verbose provider response can't balloon the
/// error chain or leak excessive detail into logs.
const MAX_ERROR_BODY_LEN: usize = 512;

fn truncate_error_body(body: String) -> String {
    if body.len() <= MAX_ERROR_BODY_LEN {
        body
    } else {
        format!("{}…[truncated]", &body[..MAX_ERROR_BODY_LEN])
    }
}

/// Cloud LLM perception provider.
///
/// Calls Anthropic Messages API or OpenAI Chat Completions API to convert
/// natural language text into structured TritWord signals. The LLM is
/// constrained by a system prompt to output JSON only — it acts as a
/// **棱镜 (prism)**: decomposing raw input into spectral components
/// without interpreting what those components mean.
///
/// ## 流沙 Philosophy
///
/// This provider embodies:
/// - **璇玑**: faithfully rotates — signals are pure decomposition, no meaning attached
/// - **棱镜**: splits into spectral bands — Frame/Value/Phase, each one angle
/// - **微风**: passes through — no summary, no suggestion, no trace left behind
pub struct CloudLLMProvider {
    #[allow(dead_code)]
    config: Arc<ConfigStore>,
    client: reqwest::Client,
    runtime: tokio::runtime::Runtime,
    model: String,
    endpoint: String,
    system_prompt: String,
}

impl CloudLLMProvider {
    /// Create a new CloudLLMProvider.
    ///
    /// `model` determines the API endpoint:
    /// - Models containing "claude" -> Anthropic Messages API
    /// - All others -> OpenAI Chat Completions API
    pub fn new(config: Arc<ConfigStore>, model: &str) -> Result<Self, PerceptError> {
        let api_key = config
            .get_api_key(model)?
            .ok_or_else(|| PerceptError::MissingApiKey(model.to_string()))?;

        let (endpoint, is_anthropic) = if model.contains("claude") {
            ("https://api.anthropic.com/v1/messages".to_string(), true)
        } else {
            (
                "https://api.openai.com/v1/chat/completions".to_string(),
                false,
            )
        };

        let mut headers = reqwest::header::HeaderMap::new();
        if is_anthropic {
            headers.insert(
                "x-api-key",
                api_key.parse().map_err(|e| {
                    PerceptError::ParseError(format!("invalid api key header value: {e}"))
                })?,
            );
            headers.insert(
                "anthropic-version",
                "2023-06-01".parse().map_err(|e| {
                    PerceptError::ParseError(format!("invalid anthropic-version header: {e}"))
                })?,
            );
        } else {
            headers.insert(
                "authorization",
                format!("Bearer {api_key}").parse().map_err(|e| {
                    PerceptError::ParseError(format!("invalid authorization header: {e}"))
                })?,
            );
        }
        headers.insert(
            "content-type",
            "application/json".parse().map_err(|e| {
                PerceptError::ParseError(format!("invalid content-type header: {e}"))
            })?,
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(10)) // ponytail: 10s for cloud LLM; degrade faster on timeout
            .build()
            .map_err(PerceptError::HttpError)?;

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .map_err(|e| PerceptError::ParseError(format!("tokio runtime: {e}")))?;

        Ok(Self {
            config,
            client,
            runtime,
            model: model.to_string(),
            endpoint,
            system_prompt: include_str!("prompts/percept_system.txt").to_string(),
        })
    }

    fn call_api(&self, raw: &str) -> Result<PerceptBatch, PerceptError> {
        if self.model.contains("claude") {
            self.call_anthropic(raw)
        } else {
            self.call_openai(raw)
        }
    }

    fn call_anthropic(&self, raw: &str) -> Result<PerceptBatch, PerceptError> {
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": MAX_TOKENS,
            "system": self.system_prompt,
            "messages": [{"role": "user", "content": raw}]
        });

        let response = self
            .runtime
            .block_on(async { self.client.post(&self.endpoint).json(&body).send().await })
            .map_err(PerceptError::HttpError)?;

        let status = response.status();
        if !status.is_success() {
            let body = self
                .runtime
                .block_on(async { response.text().await })
                .unwrap_or_default();
            if status.as_u16() == 429 {
                return Err(PerceptError::RateLimited {
                    retry_after: Some(Duration::from_secs(30)),
                });
            }
            return Err(PerceptError::ApiError {
                status: status.as_u16(),
                body: truncate_error_body(body),
            });
        }

        let json: Value = self
            .runtime
            .block_on(async { response.json().await })
            .map_err(PerceptError::HttpError)?;
        Self::parse_anthropic_response(&json)
    }

    fn call_openai(&self, raw: &str) -> Result<PerceptBatch, PerceptError> {
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": MAX_TOKENS,
            "messages": [
                {"role": "system", "content": self.system_prompt},
                {"role": "user", "content": raw}
            ]
        });

        let response = self
            .runtime
            .block_on(async { self.client.post(&self.endpoint).json(&body).send().await })
            .map_err(PerceptError::HttpError)?;

        let status = response.status();
        if !status.is_success() {
            let body = self
                .runtime
                .block_on(async { response.text().await })
                .unwrap_or_default();
            if status.as_u16() == 429 {
                return Err(PerceptError::RateLimited {
                    retry_after: Some(Duration::from_secs(30)),
                });
            }
            return Err(PerceptError::ApiError {
                status: status.as_u16(),
                body: truncate_error_body(body),
            });
        }

        let json: Value = self
            .runtime
            .block_on(async { response.json().await })
            .map_err(PerceptError::HttpError)?;
        Self::parse_openai_response(&json)
    }

    /// Parse an Anthropic Messages API response into a PerceptBatch.
    pub fn parse_anthropic_response(response: &Value) -> Result<PerceptBatch, PerceptError> {
        let text = response["content"][0]["text"]
            .as_str()
            .ok_or_else(|| PerceptError::ParseError("missing content[0].text".into()))?;
        Self::parse_inner_json(text)
    }

    /// Parse an OpenAI Chat Completions response into a PerceptBatch.
    pub fn parse_openai_response(response: &Value) -> Result<PerceptBatch, PerceptError> {
        let text = response["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| PerceptError::ParseError("missing choices[0].message.content".into()))?;
        Self::parse_inner_json(text)
    }

    fn parse_inner_json(text: &str) -> Result<PerceptBatch, PerceptError> {
        let inner: Value =
            serde_json::from_str(text).map_err(|e| PerceptError::ParseError(e.to_string()))?;

        let signals_array = inner["signals"]
            .as_array()
            .ok_or_else(|| PerceptError::ParseError("missing 'signals' array".into()))?;

        let mut signals = Vec::with_capacity(signals_array.len());
        for sig in signals_array {
            if let Some(word) = Self::parse_signal(sig) {
                signals.push(word);
            }
        }

        let confidence = inner["confidence"].as_f64().unwrap_or(0.5).clamp(0.0, 1.0);

        // 流沙: raw_data_layer describes the territory (physical measurements),
        // never the map (interpretations). No summary, no suggested_scenario.
        let raw_data_layer = inner["raw_data_layer"].as_str().map(|s| s.to_string());

        Ok(PerceptBatch {
            signals,
            source: "cloud-llm".into(),
            timestamp: Utc::now(),
            confidence,
            raw_data_layer,
        })
    }

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
            // Meta is system-internal — LLM should not produce it
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
}

impl ExternalPercept for CloudLLMProvider {
    fn perceive(&self, raw: &str) -> Result<PerceptBatch, PerceptError> {
        self.call_api(raw)
    }

    fn provider_name(&self) -> &str {
        &self.model
    }

    fn priority(&self) -> u8 {
        0
    }

    fn available(&self) -> bool {
        self.config
            .get_api_key(&self.model)
            .ok()
            .flatten()
            .is_some()
    }
}
