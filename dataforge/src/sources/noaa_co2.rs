//! NOAA GML Mauna Loa monthly mean CO2 (ppm).
//!
//! ponytail: migrated from src-tauri/src/data_sources/climate.rs. Text parsing
//! of fixed-width format at gml.noaa.gov/aftp/products/trends/co2/co2_mm_mlo.txt.

use async_trait::async_trait;
use chrono::Utc;
use std::time::Duration;

use crate::cache::http_client;
use crate::error::DataforgeError;
use crate::source::DataSource;
use crate::types::{DataCategory, RawSignal};

const CACHE_TTL: Duration = Duration::from_secs(3600);
const NOAA_URL: &str = "https://gml.noaa.gov/aftp/products/trends/co2/co2_mm_mlo.txt";

pub struct NoaaCo2Source {
    pub http: reqwest::Client,
}

impl NoaaCo2Source {
    pub fn new() -> Self {
        Self {
            http: http_client(),
        }
    }
}

impl Default for NoaaCo2Source {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DataSource for NoaaCo2Source {
    fn name(&self) -> &str {
        "NOAA GML"
    }

    fn category(&self) -> DataCategory {
        DataCategory::Climate
    }

    async fn fetch(&self) -> Result<Vec<RawSignal>, DataforgeError> {
        let resp = self.http.get(NOAA_URL).send().await?;
        if !resp.status().is_success() {
            return Err(DataforgeError::Unavailable(format!(
                "NOAA GML returned {}",
                resp.status()
            )));
        }
        let text = resp.text().await?;
        let (year, month, ppm) = parse_latest_co2(&text)
            .ok_or_else(|| DataforgeError::EmptyResponse("NOAA GML CO2".into()))?;

        if !(300.0..=600.0).contains(&ppm) {
            return Err(DataforgeError::Other(format!(
                "CO2 value {ppm} ppm outside plausible range [300, 600]"
            )));
        }

        let now = Utc::now();
        let raw_content = format!("co2_ppm:{ppm} year:{year} month:{month:02} station:Mauna Loa");
        let id = RawSignal::compute_id(NOAA_URL, &now);

        Ok(vec![RawSignal {
            id,
            source_url: NOAA_URL.into(),
            source_name: "NOAA GML".into(),
            category: DataCategory::Climate,
            raw_content,
            captured_at: now,
            data_period: Some(format!("{year}-{month:02} monthly mean")),
            location: Some(crate::types::GeoPoint {
                lat: 19.54,
                lng: -155.58,
            }),
        }])
    }

    fn fetch_interval(&self) -> Duration {
        CACHE_TTL
    }
}

/// Parse the latest (year, month, ppm) from NOAA GML text.
///
/// Format: fixed-width columns. # comments. Data rows have 10 columns;
/// column 4 (0-indexed 3) is the monthly mean CO2 ppm.
/// Returns the last valid data row.
pub fn parse_latest_co2(text: &str) -> Option<(i64, i64, f64)> {
    text.lines()
        .rfind(|l| !l.trim_start().starts_with('#') && !l.trim().is_empty())
        .and_then(|l| {
            let parts: Vec<&str> = l.split_whitespace().collect();
            if parts.len() < 5 {
                return None;
            }
            let year = parts[0].parse::<i64>().ok()?;
            let month = parts[1].parse::<i64>().ok()?;
            let ppm = parts[3].parse::<f64>().ok()?;
            Some((year, month, ppm))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_co2_takes_last_data_row() {
        let text = "# header line\n# another comment\n\
                    2026    4   2026.2917      431.12      428.65     23    1.16    0.46\n\
                    2026    5   2026.3750      432.34      429.14     17    0.66    0.31\n";
        let (y, m, ppm) = parse_latest_co2(text).unwrap();
        assert_eq!(y, 2026);
        assert_eq!(m, 5);
        assert!((ppm - 432.34).abs() < 0.01);
    }

    #[test]
    fn parse_co2_returns_none_on_empty() {
        assert_eq!(parse_latest_co2("# only comments\n"), None);
        assert_eq!(parse_latest_co2(""), None);
    }

    #[test]
    fn source_metadata() {
        let source = NoaaCo2Source::new();
        assert_eq!(source.name(), "NOAA GML");
        assert_eq!(source.category(), DataCategory::Climate);
    }
}
