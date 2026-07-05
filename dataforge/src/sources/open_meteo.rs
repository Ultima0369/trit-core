//! Open-Meteo Archive API — global temperature anomalies.
//!
//! ponytail: migrated from src-tauri/src/data_sources/climate.rs. Same logic,
//! same stations, same fail-safe semantics. Dataforge version adds RawSignal
//! output for prism consumption.

use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;
use std::time::Duration;

use crate::cache::http_client;
use crate::error::DataforgeError;
use crate::source::DataSource;
use crate::types::{DataCategory, GeoPoint, RawSignal};

const CACHE_TTL: Duration = Duration::from_secs(3600);

/// Open-Meteo temperature anomaly — 5 global reference stations.
pub struct OpenMeteoSource {
    pub http: reqwest::Client,
}

impl OpenMeteoSource {
    pub fn new() -> Self {
        Self {
            http: http_client(),
        }
    }
}

impl Default for OpenMeteoSource {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct ArchiveResponse {
    #[serde(default)]
    daily: Option<ArchiveDaily>,
}

#[derive(Debug, Deserialize)]
struct ArchiveDaily {
    #[serde(default, rename = "temperature_2m_mean")]
    temp_mean: Vec<Option<f64>>,
}

#[async_trait]
impl DataSource for OpenMeteoSource {
    fn name(&self) -> &str {
        "Open-Meteo"
    }

    fn category(&self) -> DataCategory {
        DataCategory::Climate
    }

    async fn fetch(&self) -> Result<Vec<RawSignal>, DataforgeError> {
        let stations = [
            ("Mauna Loa", 19.54, -155.58, 13.0),
            ("Amazon", -3.0, -60.0, 26.0),
            ("Borneo", 1.0, 114.0, 27.0),
            ("Southern Ocean", -60.0, 0.0, -2.0),
            ("Arctic", 80.0, 0.0, -15.0),
        ];

        let now = Utc::now();
        let mut signals = Vec::new();

        for (name, lat, lng, baseline) in stations {
            if let Some(temp) = fetch_station_temp(&self.http, lat, lng).await {
                let anomaly = temp - baseline;
                let raw_content = format!(
                    "station:{name} lat:{lat} lng:{lng} temp_mean_c:{temp:.2} anomaly_c:{anomaly:+.2}"
                );
                let id = RawSignal::compute_id(&format!("open-meteo://station/{name}"), &now);
                signals.push(RawSignal {
                    id,
                    source_url: format!(
                        "https://archive-api.open-meteo.com/v1/archive?latitude={lat}&longitude={lng}"
                    ),
                    source_name: "Open-Meteo".into(),
                    category: DataCategory::Climate,
                    raw_content,
                    captured_at: now,
                    data_period: Some(recent_window_str()),
                    location: Some(GeoPoint { lat, lng }),
                });
            }
        }

        Ok(signals)
    }

    fn fetch_interval(&self) -> Duration {
        CACHE_TTL
    }
}

async fn fetch_station_temp(client: &reqwest::Client, lat: f64, lng: f64) -> Option<f64> {
    let (start, end) = recent_window();
    let url = format!(
        "https://archive-api.open-meteo.com/v1/archive?latitude={lat}&longitude={lng}&start={start}&end={end}&daily=temperature_2m_mean"
    );
    let resp = client.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let parsed: ArchiveResponse = resp.json().await.ok()?;
    let daily = parsed.daily?;
    let temps: Vec<f64> = daily.temp_mean.into_iter().flatten().collect();
    if temps.is_empty() {
        return None;
    }
    Some(temps.iter().sum::<f64>() / temps.len() as f64)
}

fn recent_window() -> (String, String) {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let secs_per_day: u64 = 86_400;
    let today = now / secs_per_day;
    let start = today.saturating_sub(30);
    (ymd(start), ymd(today))
}

fn recent_window_str() -> String {
    let (start, end) = recent_window();
    format!("{start}..{end}")
}

fn ymd(days_since_epoch: u64) -> String {
    let mut days = days_since_epoch as i64;
    let mut year = 1970i64;
    loop {
        let dy = if is_leap(year) { 366 } else { 365 };
        if days < dy {
            break;
        }
        days -= dy;
        year += 1;
    }
    let mdays = if is_leap(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1u64;
    for &dm in &mdays {
        if days < dm {
            break;
        }
        days -= dm;
        month += 1;
    }
    format!("{:04}-{:02}-{:02}", year, month, days + 1)
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ymd_epoch_is_1970_01_01() {
        assert_eq!(ymd(0), "1970-01-01");
    }

    #[test]
    fn ymd_known_dates() {
        assert_eq!(ymd(19_723), "2024-01-01");
        assert_eq!(ymd(19_783), "2024-03-01");
    }

    #[test]
    fn source_metadata() {
        let source = OpenMeteoSource::new();
        assert_eq!(source.name(), "Open-Meteo");
        assert_eq!(source.category(), DataCategory::Climate);
    }
}
