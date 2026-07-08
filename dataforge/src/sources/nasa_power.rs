//! NASA POWER (Prediction Of Worldwide Energy Resources) — global gridded climate.
//!
//! Daily climate parameters at 0.5°×0.5° grid resolution. No API key required.
//! Parameters: T2M (temperature 2m), PRECTOTCORR (precipitation corrected),
//! ALLSKY_SFC_SW_DWN (solar radiation), WS2M (wind speed 2m).
//!
//! ponytail: single REST call per grid point, JSON response, no pagination.
//! Five reference grid points cover major climate zones (similar to Open-Meteo stations).

use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

use crate::cache::http_client;
use crate::error::DataforgeError;
use crate::source::DataSource;
use crate::types::{DataCategory, GeoPoint, RawSignal};

const CACHE_TTL: Duration = Duration::from_secs(7200);
const POWER_BASE_URL: &str = "https://power.larc.nasa.gov/api/power/climate/daily";

pub struct NasaPowerSource {
    pub http: reqwest::Client,
}

impl NasaPowerSource {
    pub fn new() -> Self {
        Self {
            http: http_client(),
        }
    }
}

impl Default for NasaPowerSource {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct PowerResponse {
    properties: Option<PowerProperties>,
}

#[derive(Debug, Deserialize)]
struct PowerProperties {
    parameter: Option<HashMap<String, HashMap<String, f64>>>,
}

#[async_trait]
impl DataSource for NasaPowerSource {
    fn name(&self) -> &str {
        "NASA POWER"
    }

    fn category(&self) -> DataCategory {
        DataCategory::Climate
    }

    async fn fetch(&self) -> Result<Vec<RawSignal>, DataforgeError> {
        // Five reference grid points covering major climate zones
        let points = [
            ("Mauna Loa", 19.54, -155.58),
            ("Amazon Basin", -3.0, -60.0),
            ("Borneo", 1.0, 114.0),
            ("Southern Ocean", -60.0, 0.0),
            ("Arctic Ocean", 80.0, 0.0),
        ];

        let (start, end) = recent_7day_window();
        let mut signals = Vec::new();

        for (name, lat, lng) in points {
            match fetch_point(&self.http, name, lat, lng, &start, &end).await {
                Some(signal) => signals.push(signal),
                None => {
                    tracing::debug!(point = name, "NASA POWER fetch returned no data");
                }
            }
        }

        if signals.is_empty() {
            return Err(DataforgeError::EmptyResponse("NASA POWER".into()));
        }

        Ok(signals)
    }

    fn fetch_interval(&self) -> Duration {
        CACHE_TTL
    }
}

async fn fetch_point(
    client: &reqwest::Client,
    name: &str,
    lat: f64,
    lng: f64,
    start: &str,
    end: &str,
) -> Option<RawSignal> {
    let url = format!(
        "{POWER_BASE_URL}?parameters=T2M,PRECTOTCORR,ALLSKY_SFC_SW_DWN,WS2M&\
         community=RE&start={start}&end={end}&latitude={lat}&longitude={lng}&format=JSON"
    );
    let resp = client.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let parsed: PowerResponse = resp.json().await.ok()?;
    let params = parsed.properties?.parameter?;

    // Average each parameter over the 7-day window
    let avg_temp = avg_values(params.get("T2M"));
    let avg_precip = avg_values(params.get("PRECTOTCORR"));
    let avg_solar = avg_values(params.get("ALLSKY_SFC_SW_DWN"));
    let avg_wind = avg_values(params.get("WS2M"));

    let raw_content = format!(
        "station:{name} lat:{lat} lng:{lng} \
         t2m_c:{avg_temp} precip_mm:{avg_precip} \
         solar_w_m2:{avg_solar} wind_m_s:{avg_wind} \
         period:{start}..{end}"
    );
    let now = Utc::now();
    let id = RawSignal::compute_id(&format!("nasa-power://{lat},{lng}"), &now);
    Some(RawSignal {
        id,
        source_url: url.to_string(),
        source_name: "NASA POWER".into(),
        category: DataCategory::Climate,
        raw_content,
        captured_at: now,
        data_period: Some(format!("{start}..{end}")),
        location: Some(GeoPoint { lat, lng }),
    })
}

fn avg_values(values: Option<&HashMap<String, f64>>) -> String {
    match values {
        Some(map) if !map.is_empty() => {
            let avg = map.values().sum::<f64>() / map.len() as f64;
            format!("{avg:.2}")
        }
        _ => "N/A".into(),
    }
}

fn recent_7day_window() -> (String, String) {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let secs_per_day: u64 = 86_400;
    let today = now / secs_per_day;
    let start = today.saturating_sub(7);
    (ymd_power(start), ymd_power(today))
}

fn ymd_power(days_since_epoch: u64) -> String {
    // Same algorithm as open_meteo's ymd but NASA POWER wants YYYYMMDD
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
    format!("{:04}{:02}{:02}", year, month, days + 1)
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ymd_power_epoch_is_19700101() {
        assert_eq!(ymd_power(0), "19700101");
    }

    #[test]
    fn ymd_power_known_dates() {
        assert_eq!(ymd_power(19_723), "20240101");
        assert_eq!(ymd_power(19_783), "20240301");
    }

    #[test]
    fn avg_empty_returns_na() {
        assert_eq!(avg_values(None), "N/A");
        assert_eq!(avg_values(Some(&HashMap::new())), "N/A");
    }

    #[test]
    fn avg_computes_mean() {
        let mut m = HashMap::new();
        m.insert("20260701".into(), 20.0);
        m.insert("20260702".into(), 30.0);
        assert_eq!(avg_values(Some(&m)), "25.00");
    }

    #[test]
    fn source_metadata() {
        let source = NasaPowerSource::new();
        assert_eq!(source.name(), "NASA POWER");
        assert_eq!(source.category(), DataCategory::Climate);
    }
}
