//! NOAA Tides & Currents — sea level observations at reference stations.
//!
//! Public REST API, no key required. Returns hourly water levels at NOAA
//! tide gauge stations. We poll 5 reference stations spanning major ocean basins.
//!
//! ponytail: single HTTP GET per station, JSON response. Parameters:
//! date=latest, datum=MSL (Mean Sea Level), units=metric, time_zone=gmt.

use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;
use std::time::Duration;

use crate::cache::http_client;
use crate::error::DataforgeError;
use crate::source::DataSource;
use crate::types::{DataCategory, GeoPoint, RawSignal};

const CACHE_TTL: Duration = Duration::from_secs(7200);

pub struct NoaaTidesSource {
    pub http: reqwest::Client,
}

impl NoaaTidesSource {
    pub fn new() -> Self {
        Self {
            http: http_client(),
        }
    }
}

impl Default for NoaaTidesSource {
    fn default() -> Self {
        Self::new()
    }
}

/// NOAA Tides & Currents API JSON response.
#[derive(Debug, Deserialize)]
struct TidesResponse {
    #[serde(default)]
    data: Vec<TideDatum>,
}

#[derive(Debug, Deserialize)]
struct TideDatum {
    #[serde(rename = "v")]
    water_level_m: Option<String>,
    #[serde(rename = "t")]
    time_gmt: Option<String>,
}

#[async_trait]
impl DataSource for NoaaTidesSource {
    fn name(&self) -> &str {
        "NOAA Tides"
    }

    fn category(&self) -> DataCategory {
        DataCategory::Ecology
    }

    async fn fetch(&self) -> Result<Vec<RawSignal>, DataforgeError> {
        // Five NOAA tide gauge reference stations
        let stations = [
            ("Honolulu", "1612340", 21.31, -157.86),
            ("San Francisco", "9414290", 37.81, -122.47),
            ("New York", "8518750", 40.70, -74.01),
            ("Tokyo", "000107", 35.68, 139.76),
            ("Sydney", "000059", -33.86, 151.21),
        ];

        let now = Utc::now();
        let mut signals = Vec::new();

        for (name, station_id, lat, lng) in stations {
            let url = format!(
                "https://api.tidesandcurrents.noaa.gov/api/prod/datagetter?\
                 station={station_id}&product=water_level&datum=MSL&units=metric&\
                 time_zone=gmt&application=trit-core&format=json&date=latest"
            );
            match fetch_station(&self.http, &url, name, station_id, lat, lng, &now).await {
                Some(signal) => signals.push(signal),
                None => {
                    tracing::debug!(station = name, "NOAA Tides fetch returned no data");
                }
            }
        }

        if signals.is_empty() {
            return Err(DataforgeError::EmptyResponse("NOAA Tides".into()));
        }

        Ok(signals)
    }

    fn fetch_interval(&self) -> Duration {
        CACHE_TTL
    }
}

async fn fetch_station(
    client: &reqwest::Client,
    url: &str,
    name: &str,
    station_id: &str,
    lat: f64,
    lng: f64,
    now: &chrono::DateTime<Utc>,
) -> Option<RawSignal> {
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let parsed: TidesResponse = resp.json().await.ok()?;
    let data = parsed.data;
    if data.is_empty() {
        return None;
    }

    // Take the most recent reading
    let latest = &data[data.len() - 1];
    let water_level = latest
        .water_level_m
        .as_ref()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.0);
    let time_str = latest.time_gmt.clone().unwrap_or_else(|| "unknown".into());

    let raw_content = format!(
        "station:{name} station_id:{station_id} water_level_m:{water_level:.3} time:{time_str}"
    );
    let id = RawSignal::compute_id(&format!("noaa-tides://{station_id}"), now);
    Some(RawSignal {
        id,
        source_url: url.to_string(),
        source_name: "NOAA Tides".into(),
        category: DataCategory::Ecology,
        raw_content,
        captured_at: *now,
        data_period: Some(time_str),
        location: Some(GeoPoint { lat, lng }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_metadata() {
        let source = NoaaTidesSource::new();
        assert_eq!(source.name(), "NOAA Tides");
        assert_eq!(source.category(), DataCategory::Ecology);
    }
}
