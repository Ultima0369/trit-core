use crate::config::ConfigStore;
use crate::percept::{ExternalPercept, PerceptBatch, PerceptError};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;

/// Local LLM perception provider.
///
/// Communicates with a local inference server (ollama, llama.cpp, etc.)
/// via HTTP on localhost. Uses the same JSON output contract as
/// CloudLLMProvider — the local model is expected to follow the same
/// system prompt and return structured TritWord signals.
///
/// No API key is needed (localhost trust boundary).
pub struct LocalLLMProvider {
    config: Arc<ConfigStore>,
    client: reqwest::Client,
    runtime: tokio::runtime::Runtime,
    endpoint: String,
}

impl LocalLLMProvider {
    /// Create a new LocalLLMProvider.
    ///
    /// Reads the endpoint URL from `ConfigStore::local_model_path()`.
    /// Defaults to `http://localhost:11434` (ollama default) if not configured.
    pub fn new(config: Arc<ConfigStore>) -> Result<Self, PerceptError> {
        let endpoint = config
            .local_model_path()?
            .unwrap_or_else(|| "http://localhost:11434".to_string());

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15)) // ponytail: 15s for local LLM; longer than cloud since local may be CPU-bound
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
            endpoint,
        })
    }

    fn call_local(&self, raw: &str) -> Result<PerceptBatch, PerceptError> {
        let system_prompt = include_str!("prompts/percept_system.txt");

        let body = serde_json::json!({
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": raw}
            ],
            "stream": false
        });

        let url = format!(
            "{}/v1/chat/completions",
            self.endpoint.trim_end_matches('/')
        );

        let response = self
            .runtime
            .block_on(async { self.client.post(&url).json(&body).send().await })
            .map_err(PerceptError::HttpError)?;

        let status = response.status();
        if !status.is_success() {
            let body = self
                .runtime
                .block_on(async { response.text().await })
                .unwrap_or_default();
            return Err(PerceptError::ApiError {
                status: status.as_u16(),
                body,
            });
        }

        let json: Value = self
            .runtime
            .block_on(async { response.json().await })
            .map_err(PerceptError::HttpError)?;
        crate::percept::openai_format::parse_openai_response(&json)
    }
}

impl ExternalPercept for LocalLLMProvider {
    fn perceive(&self, raw: &str) -> Result<PerceptBatch, PerceptError> {
        self.call_local(raw)
    }

    fn provider_name(&self) -> &str {
        "local-llm"
    }

    fn priority(&self) -> u8 {
        1
    }

    fn available(&self) -> bool {
        self.config.local_model_path().ok().flatten().is_some()
    }
}
