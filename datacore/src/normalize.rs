//! Signal normalization — parse RawSignal raw_content into structured numeric values.
//!
//! ponytail: simple keyword-based extraction from the `raw_content` field.
//! Data sources already format their content with `key:value` conventions
//! (e.g. "co2_ppm:432.34", "anomaly_c:+1.23", "t2m_c:25.50").
//! No ML needed — just regex-less key:value scanning.

use chrono::{DateTime, Utc};
use dataforge::{DataCategory, RawSignal};
use serde::{Deserialize, Serialize};

/// A parsed numeric observation extracted from a RawSignal.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizedSignal {
    /// Original signal ID (passthrough from RawSignal).
    pub signal_id: String,
    /// Source name (passthrough).
    pub source_name: String,
    /// Data category (passthrough).
    pub category: DataCategory,
    /// When the data was captured.
    pub captured_at: DateTime<Utc>,
    /// Geographic location, if available.
    pub location: Option<dataforge::GeoPoint>,
    /// Parsed numeric values, keyed by parameter name.
    pub values: Vec<SignalValue>,
}

/// A single numeric measurement extracted from raw_content.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SignalValue {
    /// Parameter name (e.g. "co2_ppm", "t2m_c", "precip_mm").
    pub name: String,
    /// Numeric value.
    pub value: f64,
    /// Unit hint (e.g. "ppm", "celsius", "mm").
    pub unit: String,
}

/// Normalizes RawSignals into structured numeric observations.
///
/// ponytail: scans raw_content for `key:value` pairs. Keys are alphanumeric+underscore,
/// values are f64-parseable. Everything else is skipped.
pub struct SignalNormalizer;

impl SignalNormalizer {
    pub fn new() -> Self {
        Self
    }

    /// Normalize a single RawSignal into a NormalizedSignal.
    ///
    /// Returns None if no numeric values could be extracted.
    pub fn normalize(&self, raw: &RawSignal) -> Option<NormalizedSignal> {
        let values = extract_values(&raw.raw_content);
        if values.is_empty() {
            return None;
        }
        Some(NormalizedSignal {
            signal_id: raw.id.clone(),
            source_name: raw.source_name.clone(),
            category: raw.category,
            captured_at: raw.captured_at,
            location: raw.location,
            values,
        })
    }

    /// Normalize a batch of signals, discarding those with no extractable values.
    pub fn normalize_batch(&self, signals: &[RawSignal]) -> Vec<NormalizedSignal> {
        signals.iter().filter_map(|s| self.normalize(s)).collect()
    }
}

impl Default for SignalNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract key:value pairs from raw_content text.
///
/// Scans for patterns like `co2_ppm:432.34` or `anomaly_c:+1.23`.
/// Keys: `[a-zA-Z_][a-zA-Z0-9_]*`. Values: f64-parseable tokens after `:`.
fn extract_values(raw: &str) -> Vec<SignalValue> {
    let mut results = Vec::new();
    let bytes = raw.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Find start of a key (alphabetic or underscore)
        if !bytes[i].is_ascii_alphabetic() && bytes[i] != b'_' {
            i += 1;
            continue;
        }

        let key_start = i;
        while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
            i += 1;
        }
        let key = &raw[key_start..i];

        // Skip whitespace before colon
        while i < len && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= len || bytes[i] != b':' {
            continue;
        }
        i += 1; // skip ':'

        // Skip whitespace after colon
        while i < len && bytes[i].is_ascii_whitespace() {
            i += 1;
        }

        // Read the value token (may start with + or - or digit)
        let val_start = i;
        if i < len && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        while i < len && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
            i += 1;
        }
        let val_str = &raw[val_start..i];

        if let Ok(value) = val_str.parse::<f64>() {
            let unit = infer_unit(key);
            // Skip metadata keys that aren't measurements
            if !is_metadata_key(key) {
                results.push(SignalValue {
                    name: key.to_string(),
                    value,
                    unit,
                });
            }
        }
    }

    results
}

/// Known unit hints for common parameter names.
fn infer_unit(key: &str) -> String {
    match key {
        k if k.contains("ppm") || k.contains("co2") => "ppm".into(),
        k if k.contains("temp") || k.contains("t2m") => "celsius".into(),
        k if k.contains("precip") || k.contains("pr") => "mm".into(),
        k if k.contains("solar") || k.contains("sw_dwn") => "w/m2".into(),
        k if k.contains("wind") || k.contains("ws") => "m/s".into(),
        k if k.contains("anomaly") => "delta".into(),
        _ => "unknown".into(),
    }
}

/// Keys that are metadata, not measurements.
fn is_metadata_key(key: &str) -> bool {
    matches!(
        key,
        "lat"
            | "lng"
            | "year"
            | "month"
            | "station"
            | "period"
            | "latest"
            | "title"
            | "identifier"
            | "satellite_layer"
            | "day"
            | "station_id"
            | "time"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_co2() {
        let raw = "co2_ppm:432.34 year:2026 month:05 station:Mauna Loa";
        let values = extract_values(raw);
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].name, "co2_ppm");
        assert!((values[0].value - 432.34).abs() < 0.01);
        assert_eq!(values[0].unit, "ppm");
    }

    #[test]
    fn extract_multiple_climate_params() {
        let raw = "t2m_c:25.50 precip_mm:3.20 solar_w_m2:180.50 wind_m_s:4.10";
        let values = extract_values(raw);
        assert_eq!(values.len(), 4);
        assert_eq!(values[0].name, "t2m_c");
        assert_eq!(values[0].unit, "celsius");
        assert_eq!(values[1].name, "precip_mm");
        assert_eq!(values[1].unit, "mm");
    }

    #[test]
    fn extract_anomaly_with_sign() {
        let raw = "station:Mauna Loa anomaly_c:+1.23";
        let values = extract_values(raw);
        assert_eq!(values.len(), 1);
        assert!((values[0].value - 1.23).abs() < 0.01);
        assert_eq!(values[0].unit, "delta");
    }

    #[test]
    fn extract_satellite_layer() {
        let raw = "satellite_layer:MODIS_Aqua title:Corrected Reflectance latest:2026-07-08";
        let values = extract_values(raw);
        // No numeric values in this content — satellite metadata only
        assert_eq!(values.len(), 0);
    }

    #[test]
    fn empty_content_returns_empty() {
        assert_eq!(extract_values("").len(), 0);
        assert_eq!(extract_values("no numbers here").len(), 0);
    }

    #[test]
    fn metadata_keys_are_skipped() {
        let raw = "lat:19.54 lng:-155.58 station:Mauna Loa co2_ppm:432.34";
        let values = extract_values(raw);
        // lat, lng, station are metadata; only co2_ppm is a measurement
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].name, "co2_ppm");
    }
}
