//! Time-series storage for normalized observations.
//!
//! ponytail: in-memory Vec-backed store with basic range queries.
//! Replace with SQLite or Parquet when data volume exceeds ~100k points.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::normalize::NormalizedSignal;

/// A single numeric observation at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimeSeriesPoint {
    /// Parameter name (e.g. "co2_ppm", "t2m_c").
    pub parameter: String,
    /// Numeric value.
    pub value: f64,
    /// When this observation was captured.
    pub timestamp: DateTime<Utc>,
    /// Source name for traceability.
    pub source: String,
    /// Geographic location, if applicable.
    pub location: Option<dataforge::GeoPoint>,
}

/// In-memory time-series store with parameter-grouped queries.
///
/// ponytail: Vec<TimeSeriesPoint> with linear scan for range queries.
/// Indexed by parameter name when query patterns demand it.
/// Replace with columnar storage when >100k points.
pub struct TimeSeriesStore {
    pub(crate) points: Vec<TimeSeriesPoint>,
}

impl TimeSeriesStore {
    pub fn new() -> Self {
        Self { points: Vec::new() }
    }

    /// Insert all numeric values from a normalized signal as time-series points.
    pub fn insert_signal(&mut self, signal: &NormalizedSignal) -> usize {
        let mut count = 0;
        for sv in &signal.values {
            self.points.push(TimeSeriesPoint {
                parameter: sv.name.clone(),
                value: sv.value,
                timestamp: signal.captured_at,
                source: signal.source_name.clone(),
                location: signal.location,
            });
            count += 1;
        }
        count
    }

    /// Insert a batch of normalized signals.
    pub fn insert_batch(&mut self, signals: &[NormalizedSignal]) -> usize {
        signals.iter().map(|s| self.insert_signal(s)).sum()
    }

    /// Total number of stored points.
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Query all points for a specific parameter.
    pub fn query_parameter(&self, parameter: &str) -> Vec<&TimeSeriesPoint> {
        self.points
            .iter()
            .filter(|p| p.parameter == parameter)
            .collect()
    }

    /// Return all stored points.
    pub fn query_all(&self) -> Vec<&TimeSeriesPoint> {
        self.points.iter().collect()
    }

    /// Query points for a parameter within a time range (inclusive).
    pub fn query_range(
        &self,
        parameter: &str,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Vec<&TimeSeriesPoint> {
        self.points
            .iter()
            .filter(|p| p.parameter == parameter && p.timestamp >= from && p.timestamp <= to)
            .collect()
    }

    /// List all unique parameter names in the store.
    pub fn parameters(&self) -> Vec<&str> {
        let mut seen = std::collections::HashSet::new();
        let mut params = Vec::new();
        for p in &self.points {
            if seen.insert(&p.parameter) {
                params.push(p.parameter.as_str());
            }
        }
        params
    }

    /// Average value for a parameter over the entire store.
    pub fn avg(&self, parameter: &str) -> Option<f64> {
        let points = self.query_parameter(parameter);
        if points.is_empty() {
            return None;
        }
        Some(points.iter().map(|p| p.value).sum::<f64>() / points.len() as f64)
    }

    /// Most recent value for a parameter.
    pub fn latest(&self, parameter: &str) -> Option<&TimeSeriesPoint> {
        self.points
            .iter()
            .filter(|p| p.parameter == parameter)
            .max_by_key(|p| p.timestamp)
    }

    /// Export all points to a CSV string.
    ///
    /// Format: `parameter,value,timestamp,source,lat,lng`
    /// Geographic fields are empty when location is None.
    /// ponytail: plain CSV — no schema versioning, no compression.
    /// Good for <100k points. Add Parquet when scale demands it.
    pub fn to_csv(&self) -> String {
        let mut csv = String::from("parameter,value,timestamp,source,lat,lng\n");
        for p in &self.points {
            let (lat, lng) = match &p.location {
                Some(loc) => (loc.lat.to_string(), loc.lng.to_string()),
                None => (String::new(), String::new()),
            };
            csv.push_str(&format!(
                "{},{},{},{},{},{}\n",
                p.parameter,
                p.value,
                p.timestamp.format("%Y-%m-%dT%H:%M:%SZ"),
                p.source,
                lat,
                lng,
            ));
        }
        csv
    }

    /// Import points from a CSV string (header expected).
    ///
    /// Non-compliant lines are skipped with a tracing::debug.
    pub fn from_csv(&mut self, csv: &str) -> usize {
        let mut imported = 0;
        for line in csv.lines().skip(1) {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 4 {
                continue;
            }
            let value: f64 = match parts[1].parse() {
                Ok(v) => v,
                Err(_) => continue,
            };
            let timestamp = match chrono::DateTime::parse_from_rfc3339(parts[2])
                .map(|d| d.with_timezone(&chrono::Utc))
            {
                Ok(t) => t,
                Err(_) => continue,
            };
            let lat: Option<f64> = parts.get(4).and_then(|s| s.parse().ok());
            let lng: Option<f64> = parts.get(5).and_then(|s| s.parse().ok());
            let location = match (lat, lng) {
                (Some(lat), Some(lng)) => Some(dataforge::GeoPoint { lat, lng }),
                _ => None,
            };

            self.points.push(TimeSeriesPoint {
                parameter: parts[0].to_string(),
                value,
                timestamp,
                source: parts[3].to_string(),
                location,
            });
            imported += 1;
        }
        imported
    }

    /// Export all points as a JSON string.
    ///
    /// ponytail: uses serde_json on the already-Serialize TimeSeriesPoint.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.points)
    }

    /// Import points from a JSON string (array of TimeSeriesPoint).
    ///
    /// Returns the number of points imported. Invalid entries are skipped.
    pub fn from_json(&mut self, json: &str) -> usize {
        let parsed: Vec<TimeSeriesPoint> = match serde_json::from_str(json) {
            Ok(v) => v,
            Err(_) => return 0,
        };
        let count = parsed.len();
        self.points.extend(parsed);
        count
    }

    /// Export all points grouped by parameter as a JSON value.
    ///
    /// Returns `{ "parameter_name": [TimeSeriesPoint, ...], ... }`.
    /// Useful for dashboards that render per-parameter charts.
    pub fn to_json_grouped(&self) -> serde_json::Value {
        let mut map = serde_json::Map::new();
        for param in self.parameters() {
            let points: Vec<&TimeSeriesPoint> = self.query_parameter(param);
            if let Ok(json) = serde_json::to_value(points) {
                map.insert(param.to_string(), json);
            }
        }
        serde_json::Value::Object(map)
    }
}

impl Default for TimeSeriesStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalize::{NormalizedSignal, SignalValue};

    fn make_signal(
        id: &str,
        source: &str,
        timestamp: DateTime<Utc>,
        values: Vec<(&str, f64)>,
    ) -> NormalizedSignal {
        NormalizedSignal {
            signal_id: id.into(),
            source_name: source.into(),
            category: dataforge::DataCategory::Climate,
            captured_at: timestamp,
            location: None,
            values: values
                .into_iter()
                .map(|(name, value)| SignalValue {
                    name: name.into(),
                    value,
                    unit: "test".into(),
                })
                .collect(),
        }
    }

    #[test]
    fn insert_and_query() {
        let mut store = TimeSeriesStore::new();
        let t = Utc::now();
        let sig = make_signal("s1", "TestSrc", t, vec![("co2_ppm", 432.34)]);
        let count = store.insert_signal(&sig);
        assert_eq!(count, 1);
        assert_eq!(store.len(), 1);
        let results = store.query_parameter("co2_ppm");
        assert_eq!(results.len(), 1);
        assert!((results[0].value - 432.34).abs() < 0.01);
    }

    #[test]
    fn query_range_filters_by_time() {
        let mut store = TimeSeriesStore::new();
        let t1 = "2026-01-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let t2 = "2026-07-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let t3 = "2026-12-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap();

        store.insert_signal(&make_signal("s1", "S", t1, vec![("v", 1.0)]));
        store.insert_signal(&make_signal("s2", "S", t2, vec![("v", 2.0)]));
        store.insert_signal(&make_signal("s3", "S", t3, vec![("v", 3.0)]));

        let range = store.query_range(
            "v",
            "2026-03-01T00:00:00Z".parse().unwrap(),
            "2026-10-01T00:00:00Z".parse().unwrap(),
        );
        assert_eq!(range.len(), 1);
        assert!((range[0].value - 2.0).abs() < 0.01);
    }

    #[test]
    fn avg_and_latest() {
        let mut store = TimeSeriesStore::new();
        let t1 = "2026-01-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let t2 = "2026-01-02T00:00:00Z".parse::<DateTime<Utc>>().unwrap();

        store.insert_signal(&make_signal("s1", "S", t1, vec![("v", 10.0)]));
        store.insert_signal(&make_signal("s2", "S", t2, vec![("v", 20.0)]));

        assert_eq!(store.avg("v"), Some(15.0));
        assert!((store.latest("v").unwrap().value - 20.0).abs() < 0.01);
    }

    #[test]
    fn parameters_lists_unique() {
        let mut store = TimeSeriesStore::new();
        let t = Utc::now();
        store.insert_signal(&make_signal("s1", "S", t, vec![("a", 1.0), ("b", 2.0)]));
        store.insert_signal(&make_signal("s2", "S", t, vec![("a", 3.0), ("c", 4.0)]));

        let mut params = store.parameters();
        params.sort();
        assert_eq!(params, vec!["a", "b", "c"]);
    }

    #[test]
    fn empty_store_returns_none() {
        let store = TimeSeriesStore::new();
        assert!(store.is_empty());
        assert_eq!(store.avg("x"), None);
        assert_eq!(store.latest("x"), None);
    }

    #[test]
    fn csv_roundtrip_preserves_data() {
        let mut store = TimeSeriesStore::new();
        let t = Utc::now();
        let sig = make_signal("s1", "TestSrc", t, vec![("co2_ppm", 432.34)]);
        store.insert_signal(&sig);

        let csv = store.to_csv();
        assert!(csv.starts_with("parameter,value,timestamp,source,lat,lng"));
        assert!(csv.contains("co2_ppm"));

        // Import into a fresh store
        let mut store2 = TimeSeriesStore::new();
        let imported = store2.from_csv(&csv);
        assert_eq!(imported, 1);
        assert_eq!(store2.len(), 1);
        let points = store2.query_parameter("co2_ppm");
        assert_eq!(points.len(), 1);
        assert!((points[0].value - 432.34).abs() < 0.01);
        assert_eq!(points[0].source, "TestSrc");
    }

    #[test]
    fn csv_roundtrip_with_location() {
        let mut store = TimeSeriesStore::new();
        let t = Utc::now();
        let sig = NormalizedSignal {
            signal_id: "loc1".into(),
            source_name: "LocSrc".into(),
            category: dataforge::DataCategory::Climate,
            captured_at: t,
            location: Some(dataforge::GeoPoint {
                lat: 35.0,
                lng: 139.0,
            }),
            values: vec![SignalValue {
                name: "temp".into(),
                value: 25.0,
                unit: "celsius".into(),
            }],
        };
        store.insert_signal(&sig);

        let csv = store.to_csv();
        let mut store2 = TimeSeriesStore::new();
        store2.from_csv(&csv);
        let points = store2.query_parameter("temp");
        assert_eq!(points[0].location.unwrap().lat, 35.0);
    }

    #[test]
    fn csv_import_skips_bad_lines() {
        let csv = "parameter,value,timestamp,source,lat,lng\n\
                   valid,42.0,2026-01-01T00:00:00Z,test,,\n\
                   bad_no_value,notanumber,2026-01-01T00:00:00Z,test,,\n\
                   also_valid,43.0,2026-01-02T00:00:00Z,test,,\n";
        let mut store = TimeSeriesStore::new();
        let imported = store.from_csv(csv);
        assert_eq!(imported, 2); // bad line skipped
    }

    #[test]
    fn to_json_produces_valid_json() {
        let mut store = TimeSeriesStore::new();
        let t = Utc::now();
        let sig = make_signal("s1", "TestSrc", t, vec![("co2_ppm", 432.34)]);
        store.insert_signal(&sig);

        let json_str = store.to_json().unwrap();
        assert!(json_str.contains("co2_ppm"));
        assert!(json_str.contains("432.34"));
        // Must be valid JSON
        let _parsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("should be valid JSON");
    }

    #[test]
    fn to_json_grouped_by_parameter() {
        let mut store = TimeSeriesStore::new();
        let t = Utc::now();
        let sig1 = make_signal("s1", "Src", t, vec![("a", 1.0), ("b", 2.0)]);
        let sig2 = make_signal("s2", "Src", t, vec![("a", 3.0)]);
        store.insert_signal(&sig1);
        store.insert_signal(&sig2);

        let grouped = store.to_json_grouped();
        let obj = grouped.as_object().unwrap();
        assert!(obj.contains_key("a"));
        assert!(obj.contains_key("b"));
        assert_eq!(obj["a"].as_array().unwrap().len(), 2);
        assert_eq!(obj["b"].as_array().unwrap().len(), 1);
    }
}
