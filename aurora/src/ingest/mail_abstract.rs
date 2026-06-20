//! Mail data source abstraction — M1 reserved, M0 stub.
//!
//! This module defines the interface for mail client integration
//! (Apple Mail, Outlook, Thunderbird). In M0, all methods return
//! `IngestError::MailUnavailable`. M1 will add real implementations.

use super::{DataSource, IngestError};
use serde::de::DeserializeOwned;

/// Mail data source — not yet implemented in M0.
///
/// M1 will add platform-specific implementations behind this facade.
/// For now, `is_available()` always returns `false`.
pub struct MailSource {
    platform: String,
}

impl MailSource {
    /// Create a mail source for the current platform.
    /// Always returns unavailable in M0.
    pub fn new() -> Self {
        Self {
            platform: std::env::consts::OS.to_string(),
        }
    }
}

impl Default for MailSource {
    fn default() -> Self {
        Self::new()
    }
}

impl DataSource for MailSource {
    fn name(&self) -> &str {
        "mail"
    }

    fn is_available(&self) -> bool {
        false // M0: mail not yet implemented
    }

    fn load<T: DeserializeOwned>(&self) -> Result<Vec<T>, IngestError> {
        Err(IngestError::MailUnavailable(
            "Mail source not implemented in M0. Use JSON fallback.".into(),
        ))
    }

    fn contact_count(&self) -> usize {
        0
    }
}
