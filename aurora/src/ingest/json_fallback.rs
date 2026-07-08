//! JSON fallback data source — reads contacts from a local JSON file.
//!
//! This is the default data source. Users manually export their
//! communication metadata as JSON and point Aurora at the file.
//! Format: a JSON array of contact objects, each with at minimum
//! `name`, `emails_per_week`, and `relation_label` fields.

use serde::de::DeserializeOwned;
use std::fs;
use std::path::Path;

/// Unified error type for data ingestion.
#[derive(Debug, thiserror::Error)]
pub enum IngestError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
}

/// JSON file-based data source.
///
/// Reads a JSON array of contact objects from a local file.
/// The file is read once at construction time and held in memory.
pub struct JsonFallbackSource {
    raw_json: String,
    contact_count: usize,
}

impl JsonFallbackSource {
    /// Create a new JSON fallback source from a file path.
    ///
    /// Returns `IngestError::Io` if the file cannot be read.
    /// Returns `IngestError::Json` if the file is not valid JSON.
    pub fn new(path: &Path) -> Result<Self, IngestError> {
        let raw_json = fs::read_to_string(path)?;
        // Validate it's a JSON array by parsing to serde_json::Value
        let parsed: serde_json::Value = serde_json::from_str(&raw_json)?;
        let contact_count = parsed.as_array().map(|a| a.len()).unwrap_or(0);
        Ok(Self {
            raw_json,
            contact_count,
        })
    }

    /// Human-readable name of this source.
    pub fn name(&self) -> &str {
        "json_fallback"
    }

    /// Whether this source is currently available.
    pub fn is_available(&self) -> bool {
        true // JSON file was validated at construction time
    }

    /// Load contacts as a deserializable type.
    /// The type parameter allows callers to specify their own contact schema.
    pub fn load<T: DeserializeOwned>(&self) -> Result<Vec<T>, IngestError> {
        let contacts: Vec<T> = serde_json::from_str(&self.raw_json)?;
        Ok(contacts)
    }

    /// Return the number of contacts available.
    pub fn contact_count(&self) -> usize {
        self.contact_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestContact {
        name: String,
        emails_per_week: f64,
    }

    #[test]
    fn loads_valid_json_array() {
        let dir = std::env::temp_dir().join("aurora_test_json_fb");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.json");
        std::fs::write(&path, r#"[{"name":"Test","emails_per_week":5.0}]"#).unwrap();

        let source = JsonFallbackSource::new(&path).unwrap();
        assert_eq!(source.name(), "json_fallback");
        assert!(source.is_available());
        assert_eq!(source.contact_count(), 1);

        let contacts: Vec<TestContact> = source.load().unwrap();
        assert_eq!(contacts[0].name, "Test");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rejects_invalid_json() {
        let dir = std::env::temp_dir().join("aurora_test_json_fb_invalid");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bad.json");
        std::fs::write(&path, "not json").unwrap();

        let result = JsonFallbackSource::new(&path);
        assert!(result.is_err());

        std::fs::remove_dir_all(&dir).ok();
    }
}
