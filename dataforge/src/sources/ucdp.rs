//! UCDP GED armed conflict events (Uppsala Conflict Data Program).
//!
//! ponytail: migrated from src-tauri/src/data_sources/ucdp.rs. Public API,
//! no key required. Version auto-detection, pagination, fail-safe on errors.

use async_trait::async_trait;
use chrono::{Datelike, Utc};
use serde::Deserialize;
use std::time::Duration;

use crate::cache::http_client;
use crate::error::DataforgeError;
use crate::source::DataSource;
use crate::types::{DataCategory, RawSignal};

const CACHE_TTL: Duration = Duration::from_secs(3600);
const PAGE_SIZE: usize = 500;
const MAX_EVENTS: usize = 500;

pub struct UcdpSource {
    pub http: reqwest::Client,
}

impl UcdpSource {
    pub fn new() -> Self {
        Self {
            http: http_client(),
        }
    }
}

impl Default for UcdpSource {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct UcdpRow {
    latitude: Option<f64>,
    longitude: Option<f64>,
    #[serde(rename = "type_of_violence")]
    type_code: i64,
    #[serde(default)]
    best: Option<i64>,
    country: Option<String>,
    date_start: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UcdpPage {
    #[serde(default, rename = "Result")]
    result: Vec<UcdpRow>,
}

fn violence_type(code: i64) -> &'static str {
    match code {
        1 => "state-based",
        2 => "non-state",
        3 => "one-sided",
        _ => "state-based",
    }
}

#[async_trait]
impl DataSource for UcdpSource {
    fn name(&self) -> &str {
        "UCDP GED"
    }

    fn category(&self) -> DataCategory {
        DataCategory::Geopolitical
    }

    async fn fetch(&self) -> Result<Vec<RawSignal>, DataforgeError> {
        // Version auto-detection: try (this_year.1, last_year.1, 25.1, 24.1)
        let this_year = Utc::now().year();
        let versions = [
            format!("{this_year}.1"),
            format!("{}.1", this_year - 1),
            "25.1".into(),
            "24.1".into(),
        ];

        let client = &self.http;
        let mut all_rows: Vec<UcdpRow> = Vec::new();

        for version in &versions {
            let url = format!(
                "https://ucdpapi.pcr.uu.se/api/gedevents/{version}?pagesize={PAGE_SIZE}&page=1"
            );
            if let Ok(resp) = client.get(&url).send().await {
                if resp.status().is_success() {
                    if let Ok(page) = resp.json::<UcdpPage>().await {
                        all_rows = page.result;
                        break;
                    }
                }
            }
        }

        if all_rows.is_empty() {
            return Ok(Vec::new());
        }

        let now = Utc::now();
        let signals: Vec<RawSignal> = all_rows
            .into_iter()
            .take(MAX_EVENTS)
            .filter_map(|row| {
                let lat = row.latitude?;
                let lng = row.longitude?;
                let deaths = row.best.unwrap_or(0);
                let vtype = violence_type(row.type_code);
                let raw_content = format!(
                    "violence_type:{vtype} deaths:{deaths} country:{} date:{}",
                    row.country.as_deref().unwrap_or("unknown"),
                    row.date_start.as_deref().unwrap_or("unknown")
                );
                let id = RawSignal::compute_id(&format!("ucdp://{lat},{lng}/{deaths}"), &now);
                Some(RawSignal {
                    id,
                    source_url: "https://ucdpapi.pcr.uu.se/api/gedevents".into(),
                    source_name: "UCDP GED".into(),
                    category: DataCategory::Geopolitical,
                    raw_content,
                    captured_at: now,
                    data_period: None,
                    location: Some(crate::types::GeoPoint { lat, lng }),
                })
            })
            .collect();

        Ok(signals)
    }

    fn fetch_interval(&self) -> Duration {
        CACHE_TTL
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn violence_type_mapping() {
        assert_eq!(violence_type(1), "state-based");
        assert_eq!(violence_type(2), "non-state");
        assert_eq!(violence_type(3), "one-sided");
        assert_eq!(violence_type(99), "state-based"); // default
    }

    #[test]
    fn source_metadata() {
        let source = UcdpSource::new();
        assert_eq!(source.name(), "UCDP GED");
        assert_eq!(source.category(), DataCategory::Geopolitical);
    }
}
