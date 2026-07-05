//! GBIF species occurrence records — global biodiversity observations.
//!
//! API: https://api.gbif.org/v1/occurrence/search (public, no key required
//! for basic queries). Returns recent species observations with coordinates,
//! taxonomic metadata, and basis of record.
//!
//! ponytail: GBIF API is rate-limited (no key = lower quota). We fetch a
//! small page of recent records once per hour. Fail-safe on any error.

use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;
use std::time::Duration;

use crate::cache::http_client;
use crate::error::DataforgeError;
use crate::source::DataSource;
use crate::types::{DataCategory, RawSignal};

const CACHE_TTL: Duration = Duration::from_secs(3600);
const GBIF_URL: &str = "https://api.gbif.org/v1/occurrence/search";

/// GBIF occurrence search result.
#[derive(Debug, Deserialize)]
struct GbifResponse {
    #[serde(default)]
    results: Vec<GbifOccurrence>,
}

#[derive(Debug, Deserialize)]
struct GbifOccurrence {
    #[serde(default)]
    species: Option<String>,
    #[serde(default)]
    #[serde(rename = "decimalLatitude")]
    lat: Option<f64>,
    #[serde(default)]
    #[serde(rename = "decimalLongitude")]
    lng: Option<f64>,
    #[serde(default)]
    #[serde(rename = "basisOfRecord")]
    basis: Option<String>,
    #[serde(default)]
    country: Option<String>,
    #[serde(default)]
    #[serde(rename = "eventDate")]
    event_date: Option<String>,
    #[serde(default)]
    #[serde(rename = "taxonomicStatus")]
    #[allow(dead_code)]
    status: Option<String>,
}

pub struct GbifSource {
    pub http: reqwest::Client,
}

impl GbifSource {
    pub fn new() -> Self {
        Self {
            http: http_client(),
        }
    }
}

impl Default for GbifSource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DataSource for GbifSource {
    fn name(&self) -> &str {
        "GBIF"
    }

    fn category(&self) -> DataCategory {
        DataCategory::Ecology
    }

    async fn fetch(&self) -> Result<Vec<RawSignal>, DataforgeError> {
        // Fetch recent observations: limit 50, sorted by last interpreted.
        // Only records with coordinates, accepted taxonomic status.
        let resp = self
            .http
            .get(GBIF_URL)
            .query(&[
                ("limit", "50"),
                ("hasCoordinate", "true"),
                ("taxonomicStatus", "ACCEPTED"),
                ("basisOfRecord", "HUMAN_OBSERVATION"),
            ])
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(DataforgeError::Unavailable(format!(
                "GBIF returned {}",
                resp.status()
            )));
        }

        let body: GbifResponse = resp.json().await?;
        let now = Utc::now();

        let signals: Vec<RawSignal> = body
            .results
            .into_iter()
            .filter_map(|occ| {
                let species = occ.species?;
                let lat = occ.lat?;
                let lng = occ.lng?;
                let basis = occ.basis.unwrap_or_else(|| "unknown".into());
                let country = occ.country.unwrap_or_else(|| "unknown".into());
                let event_date = occ.event_date.unwrap_or_else(|| "unknown".into());

                let raw_content = format!(
                    "species:{species} lat:{lat} lng:{lng} country:{country} \
                     basis_of_record:{basis} event_date:{event_date}"
                );
                let id = RawSignal::compute_id(&format!("gbif://{species}/{lat}/{lng}"), &now);

                Some(RawSignal {
                    id,
                    source_url: GBIF_URL.into(),
                    source_name: "GBIF".into(),
                    category: DataCategory::Ecology,
                    raw_content,
                    captured_at: now,
                    data_period: Some(event_date),
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
    fn source_metadata() {
        let source = GbifSource::new();
        assert_eq!(source.name(), "GBIF");
        assert_eq!(source.category(), DataCategory::Ecology);
    }
}
