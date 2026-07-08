//! Core types — raw observations before any interpretation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A raw, unstructured observation from a public data source.
///
/// This is deliberately NOT a TritWord. It carries the original data
/// exactly as collected, before any Frame/Phase/Value decomposition.
/// Interpretation happens downstream in the prism engine.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RawSignal {
    /// Unique identifier (hash of source URL + timestamp).
    pub id: String,
    /// The URL this data was fetched from.
    pub source_url: String,
    /// Human-readable source name (e.g. "NOAA GML", "Open-Meteo", "GBIF").
    pub source_name: String,
    /// What kind of data this is.
    pub category: DataCategory,
    /// The raw text or numeric content, as returned by the source.
    pub raw_content: String,
    /// When this data was captured (UTC).
    pub captured_at: DateTime<Utc>,
    /// The time period this data covers (e.g. "2026-06 monthly mean").
    pub data_period: Option<String>,
    /// Physical coordinates, if applicable.
    pub location: Option<GeoPoint>,
}

/// Broad category of a raw observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DataCategory {
    /// Temperature, CO2, precipitation, ocean pH, ice coverage.
    Climate,
    /// Biodiversity, species distribution, habitat extent.
    Ecology,
    /// Preprints, journal articles, peer-reviewed findings.
    ScientificResearch,
    /// Armed conflicts, political events, population displacement.
    Geopolitical,
    /// Satellite imagery, remote sensing, Earth observation (MODIS, VIIRS, Sentinel).
    Satellite,
    /// Catch-all for categories not yet modeled.
    Other,
}

impl DataCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            DataCategory::Climate => "climate",
            DataCategory::Ecology => "ecology",
            DataCategory::ScientificResearch => "scientific_research",
            DataCategory::Geopolitical => "geopolitical",
            DataCategory::Satellite => "satellite",
            DataCategory::Other => "other",
        }
    }
}

impl std::fmt::Display for DataCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Geographic coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GeoPoint {
    pub lat: f64,
    pub lng: f64,
}

impl RawSignal {
    /// Generate a stable ID from source URL and capture timestamp.
    pub fn compute_id(source_url: &str, captured_at: &DateTime<Utc>) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        source_url.hash(&mut h);
        captured_at.timestamp_micros().hash(&mut h);
        format!("{:016x}", h.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_signal_id_is_stable() {
        let now = Utc::now();
        let url = "https://example.com/data.json";
        let id1 = RawSignal::compute_id(url, &now);
        let id2 = RawSignal::compute_id(url, &now);
        assert_eq!(id1, id2);
    }

    #[test]
    fn raw_signal_id_differs_by_url() {
        let now = Utc::now();
        let id1 = RawSignal::compute_id("https://a.com", &now);
        let id2 = RawSignal::compute_id("https://b.com", &now);
        assert_ne!(id1, id2);
    }

    #[test]
    fn data_category_display() {
        assert_eq!(DataCategory::Climate.to_string(), "climate");
        assert_eq!(DataCategory::Ecology.to_string(), "ecology");
    }

    #[test]
    fn geo_point_equality() {
        let a = GeoPoint {
            lat: 19.54,
            lng: -155.58,
        };
        let b = GeoPoint {
            lat: 19.54,
            lng: -155.58,
        };
        assert_eq!(a, b);
    }
}
