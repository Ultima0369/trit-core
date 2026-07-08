//! Error types for dataforge operations.

use thiserror::Error;

/// All errors that can occur during data acquisition.
#[derive(Debug, Error)]
pub enum DataforgeError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON deserialization failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("XML parse failed: {0}")]
    Xml(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("cache error: {0}")]
    Cache(String),

    #[error("data source unavailable: {0}")]
    Unavailable(String),

    #[error("rate limited — retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },

    #[error("empty response from {0}")]
    EmptyResponse(String),

    #[error("{0}")]
    Other(String),
}
