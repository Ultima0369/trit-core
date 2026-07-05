use std::time::Duration;

/// Errors from perception providers.
#[derive(Debug, thiserror::Error)]
pub enum PerceptError {
    #[error("API key not configured for provider '{0}'")]
    MissingApiKey(String),

    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("API returned error {status}: {body}")]
    ApiError { status: u16, body: String },

    #[error("Response parse failed: {0}")]
    ParseError(String),

    #[error("Rate limited, retry after {retry_after:?}")]
    RateLimited { retry_after: Option<Duration> },

    #[error("All perception providers unavailable")]
    AllUnavailable,

    #[error("Configuration error: {0}")]
    ConfigError(#[from] ConfigError),
}

impl PerceptError {
    /// Machine-readable error kind for frontend discrimination.
    pub fn kind(&self) -> &'static str {
        match self {
            PerceptError::MissingApiKey(_) => "missing_api_key",
            PerceptError::HttpError(_) => "http_error",
            PerceptError::ApiError { .. } => "api_error",
            PerceptError::ParseError(_) => "parse_error",
            PerceptError::RateLimited { .. } => "rate_limited",
            PerceptError::AllUnavailable => "all_unavailable",
            PerceptError::ConfigError(_) => "config_error",
        }
    }
}

/// Errors from the encrypted configuration store.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("DPAPI encryption/decryption failed: {0}")]
    Dpapi(String),
}
