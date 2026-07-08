//! USGS Earthquake Hazards Program — real-time seismic events.
//!
//! Public GeoJSON feed, no API key required. Returns all magnitude ≥1.0
//! earthquakes worldwide in the past hour. Each event has magnitude, depth,
//! coordinates, time, and place description.
//!
//! ponytail: single HTTP GET + GeoJSON parse. The API rate limit is generous
//! (no key = anonymous tier, ~20 requests/minute). We poll every 5 minutes.

use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;
use std::time::Duration;

use crate::cache::http_client;
use crate::error::DataforgeError;
use crate::source::DataSource;
use crate::types::{DataCategory, GeoPoint, RawSignal};

const CACHE_TTL: Duration = Duration::from_secs(300); // 5 minutes
const USGS_URL: &str = "https://earthquake.usgs.gov/earthquakes/feed/v1.0/summary/all_hour.geojson";

pub struct UsgsSource {
    pub http: reqwest::Client,
}

impl UsgsSource {
    pub fn new() -> Self {
        Self {
            http: http_client(),
        }
    }
}

impl Default for UsgsSource {
    fn default() -> Self {
        Self::new()
    }
}

/// GeoJSON FeatureCollection with earthquake properties.
#[derive(Debug, Deserialize)]
struct EqFeed {
    features: Vec<EqFeature>,
}

#[derive(Debug, Deserialize)]
struct EqFeature {
    properties: EqProperties,
    geometry: EqGeometry,
}

#[derive(Debug, Deserialize)]
struct EqProperties {
    mag: Option<f64>,
    place: Option<String>,
    #[serde(rename = "type")]
    #[allow(dead_code)]
    event_type: Option<String>,
    #[allow(dead_code)]
    depth: Option<f64>,
    #[serde(rename = "sig")]
    significance: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct EqGeometry {
    coordinates: Vec<f64>, // [lng, lat, depth_km]
}

#[async_trait]
impl DataSource for UsgsSource {
    fn name(&self) -> &str {
        "USGS"
    }

    fn category(&self) -> DataCategory {
        DataCategory::Other
    }

    async fn fetch(&self) -> Result<Vec<RawSignal>, DataforgeError> {
        let resp = self.http.get(USGS_URL).send().await?;
        if !resp.status().is_success() {
            return Err(DataforgeError::Unavailable(format!(
                "USGS returned {}",
                resp.status()
            )));
        }
        let feed: EqFeed = resp.json().await?;

        if feed.features.is_empty() {
            return Ok(vec![]);
        }

        let now = Utc::now();
        let signals: Vec<RawSignal> = feed
            .features
            .into_iter()
            .filter_map(|f| {
                let props = f.properties;
                let mag = props.mag?;
                // ponytail: only log significant events (mag ≥ 2.5 or significance ≥ 50)
                // to avoid noise from microquakes. The feed already has ≥1.0 filter.
                if mag < 2.5 && props.significance.unwrap_or(0) < 50 {
                    return None;
                }
                let lng = *f.geometry.coordinates.first()?;
                let lat = *f.geometry.coordinates.get(1)?;
                let depth_km = f.geometry.coordinates.get(2).copied().unwrap_or(0.0);
                let place = props.place.unwrap_or_else(|| "unknown".into());

                let raw_content =
                    format!("earthquake_mag:{mag} depth_km:{depth_km:.1} place:{place}");
                let id = RawSignal::compute_id(&format!("usgs://{lat:.4},{lng:.4}/{mag:.1}"), &now);
                Some(RawSignal {
                    id,
                    source_url: USGS_URL.into(),
                    source_name: "USGS".into(),
                    category: DataCategory::Other,
                    raw_content,
                    captured_at: now,
                    data_period: Some("past hour".into()),
                    location: Some(GeoPoint { lat, lng }),
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
    fn source_metadata() {
        let source = UsgsSource::new();
        assert_eq!(source.name(), "USGS");
        assert_eq!(source.category(), DataCategory::Other);
    }
}
