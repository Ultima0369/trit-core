//! NSIDC Sea Ice Index — daily Arctic and Antarctic sea ice extent.
//!
//! Public CSV files from NOAA/NSIDC, no authentication required.
//! Arctic: N_seaice_extent_daily_v3.0.csv (million km²)
//! Antarctic: S_seaice_extent_daily_v3.0.csv
//!
//! ponytail: GET the CSV, parse the last data row. CSV format:
//! Year, Month, Day, Extent, Missing, Source

use async_trait::async_trait;
use chrono::Utc;
use std::time::Duration;

use crate::cache::http_client;
use crate::error::DataforgeError;
use crate::source::DataSource;
use crate::types::{DataCategory, GeoPoint, RawSignal};

const CACHE_TTL: Duration = Duration::from_secs(7200);
const NSIDC_ARCTIC_URL: &str =
    "https://noaadata.apps.nsidc.org/NOAA/G02135/north/daily/data/N_seaice_extent_daily_v3.0.csv";
const NSIDC_ANTARCTIC_URL: &str =
    "https://noaadata.apps.nsidc.org/NOAA/G02135/south/daily/data/S_seaice_extent_daily_v3.0.csv";

pub struct NsidcSource {
    pub http: reqwest::Client,
}

impl NsidcSource {
    pub fn new() -> Self {
        Self {
            http: http_client(),
        }
    }
}

impl Default for NsidcSource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DataSource for NsidcSource {
    fn name(&self) -> &str {
        "NSIDC"
    }

    fn category(&self) -> DataCategory {
        DataCategory::Climate
    }

    async fn fetch(&self) -> Result<Vec<RawSignal>, DataforgeError> {
        let now = Utc::now();
        let mut signals = Vec::new();

        for (url, label, lat, lng) in [
            (NSIDC_ARCTIC_URL, "Arctic", 90.0, 0.0),
            (NSIDC_ANTARCTIC_URL, "Antarctic", -90.0, 0.0),
        ] {
            if let Some(signal) = fetch_sea_ice(&self.http, url, label, lat, lng, &now).await {
                signals.push(signal);
            }
        }

        if signals.is_empty() {
            return Err(DataforgeError::EmptyResponse("NSIDC Sea Ice".into()));
        }

        Ok(signals)
    }

    fn fetch_interval(&self) -> Duration {
        CACHE_TTL
    }
}

async fn fetch_sea_ice(
    client: &reqwest::Client,
    url: &str,
    label: &str,
    lat: f64,
    lng: f64,
    now: &chrono::DateTime<Utc>,
) -> Option<RawSignal> {
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let text = resp.text().await.ok()?;
    // Parse last non-empty data row: Year,Month,Day,Extent,...
    let last_line = text.lines().rev().find(|l| {
        let trimmed = l.trim();
        !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with("Year")
    })?;

    let parts: Vec<&str> = last_line.split(',').collect();
    if parts.len() < 4 {
        return None;
    }
    let year = parts[0].trim().parse::<i64>().ok()?;
    let month = parts[1].trim().parse::<i64>().ok()?;
    let day = parts[2].trim().parse::<i64>().ok()?;
    let extent = parts[3].trim().parse::<f64>().ok()?;

    if !(0.0..=25.0).contains(&extent) {
        return None; // implausible value for million km²
    }

    let raw_content =
        format!("region:{label} sea_ice_extent_mkm2:{extent:.3} date:{year}-{month:02}-{day:02}");
    let id = RawSignal::compute_id(&format!("nsidc://{label}"), now);
    Some(RawSignal {
        id,
        source_url: url.into(),
        source_name: "NSIDC".into(),
        category: DataCategory::Climate,
        raw_content,
        captured_at: *now,
        data_period: Some(format!("{year}-{month:02}-{day:02}")),
        location: Some(GeoPoint { lat, lng }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_metadata() {
        let source = NsidcSource::new();
        assert_eq!(source.name(), "NSIDC");
        assert_eq!(source.category(), DataCategory::Climate);
    }
}
