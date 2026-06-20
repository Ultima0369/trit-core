//! Data ingestion layer — abstract over mail, JSON, and future sources.
//!
//! The [`DataSource`] trait is the single abstraction boundary between
//! data acquisition and the decision pipeline. Every data source
//! (Apple Mail, JSON fallback, calendar, etc.) implements this trait.
//!
//! [`IngestManager`] selects the best available source at startup:
//! mail first, JSON fallback second. This is the "mail采集抽象层"
//! required by M0.

use serde::de::DeserializeOwned;

pub mod json_fallback;
pub mod mail_abstract;

/// Unified error type for data ingestion.
#[derive(Debug, thiserror::Error)]
pub enum IngestError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("mail source unavailable: {0}")]
    MailUnavailable(String),
    #[error("no data source available")]
    NoSourceAvailable,
}

/// Abstract data source — the single trait boundary for all ingestion.
///
/// Implementations: `JsonFallbackSource` (M0), `MailSource` (M1).
pub trait DataSource {
    /// Human-readable name of this source (e.g. "json_fallback", "apple_mail").
    fn name(&self) -> &str;

    /// Whether this source is currently available.
    fn is_available(&self) -> bool;

    /// Load contacts as a deserializable type.
    /// The type parameter allows callers to specify their own contact schema.
    fn load<T: DeserializeOwned>(&self) -> Result<Vec<T>, IngestError>;

    /// Return the number of contacts available.
    fn contact_count(&self) -> usize;
}

/// Manages data source selection with fallback logic.
///
/// Priority order:
/// 1. Mail source (if available on this platform)
/// 2. JSON fallback (always available if file exists)
///
/// Uses an enum internally to avoid trait object limitations
/// (the `load<T>` generic method makes `DataSource` not dyn-safe).
pub struct IngestManager {
    inner: SourceKind,
}

enum SourceKind {
    Json(json_fallback::JsonFallbackSource),
    Mail(mail_abstract::MailSource),
}

impl IngestManager {
    /// Create an IngestManager with JSON fallback only (M0 default).
    pub fn with_json_fallback(path: &std::path::Path) -> Result<Self, IngestError> {
        let source = json_fallback::JsonFallbackSource::new(path)?;
        Ok(Self {
            inner: SourceKind::Json(source),
        })
    }

    /// Return the name of the active data source.
    pub fn source_name(&self) -> &str {
        match &self.inner {
            SourceKind::Json(s) => s.name(),
            SourceKind::Mail(s) => s.name(),
        }
    }

    /// Whether the active source is available.
    pub fn is_available(&self) -> bool {
        match &self.inner {
            SourceKind::Json(s) => s.is_available(),
            SourceKind::Mail(s) => s.is_available(),
        }
    }

    /// Load contacts from the active source.
    pub fn load<T: DeserializeOwned>(&self) -> Result<Vec<T>, IngestError> {
        match &self.inner {
            SourceKind::Json(s) => s.load(),
            SourceKind::Mail(s) => s.load(),
        }
    }

    /// Number of contacts in the active source.
    pub fn contact_count(&self) -> usize {
        match &self.inner {
            SourceKind::Json(s) => s.contact_count(),
            SourceKind::Mail(s) => s.contact_count(),
        }
    }
}
