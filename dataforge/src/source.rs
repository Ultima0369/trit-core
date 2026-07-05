//! Data source abstraction — trait for all data collectors.

use async_trait::async_trait;
use std::time::Duration;

use crate::error::DataforgeError;
use crate::types::{DataCategory, RawSignal};

/// A data source capable of fetching raw observations.
///
/// Implementations are stateless (configuration lives in the struct,
/// runtime state like HTTP clients is created once and reused).
/// Each `fetch()` call is independent — no pagination state, no cursors.
#[async_trait]
pub trait DataSource: Send + Sync {
    /// Human-readable name for audit trails.
    fn name(&self) -> &str;

    /// What kind of data this source produces.
    fn category(&self) -> DataCategory;

    /// Fetch a batch of raw signals from this source.
    ///
    /// Returns empty Vec on transient failures — dataforge is fail-safe,
    /// never propagating errors upward. Errors are logged internally.
    async fn fetch(&self) -> Result<Vec<RawSignal>, DataforgeError>;

    /// Recommended minimum interval between fetches.
    ///
    /// Used by SourceRegistry to avoid hammering public APIs.
    /// Sources that update daily should return ~1 hour.
    fn fetch_interval(&self) -> Duration;
}
