//! NASA GIBS (Global Imagery Browse Services) satellite imagery metadata.
//!
//! GIBS provides daily satellite imagery from MODIS, VIIRS, and other instruments.
//! We use the WMTS GetCapabilities endpoint to retrieve available layers and dates.
//! No API key required — GIBS is open to the public.
//!
//! ponytail: single HTTP GET to WMTS GetCapabilities XML, parses Layer/Title/Identifier
//! and the most recent AvailableTime. This gives us a daily "what's new" feed of
//! satellite data without needing to download actual imagery tiles.

use async_trait::async_trait;
use chrono::Utc;
use std::time::Duration;

use crate::cache::http_client;
use crate::error::DataforgeError;
use crate::source::DataSource;
use crate::types::{DataCategory, RawSignal};

const CACHE_TTL: Duration = Duration::from_secs(3600);
const GIBS_WMTS_URL: &str =
    "https://gibs.earthdata.nasa.gov/wmts/epsg4326/best/1.0.0/WMTSCapabilities.xml";

pub struct GibsSource {
    pub http: reqwest::Client,
}

impl GibsSource {
    pub fn new() -> Self {
        Self {
            http: http_client(),
        }
    }
}

impl Default for GibsSource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DataSource for GibsSource {
    fn name(&self) -> &str {
        "NASA GIBS"
    }

    fn category(&self) -> DataCategory {
        DataCategory::Satellite
    }

    async fn fetch(&self) -> Result<Vec<RawSignal>, DataforgeError> {
        let resp = self.http.get(GIBS_WMTS_URL).send().await?;
        if !resp.status().is_success() {
            return Err(DataforgeError::Unavailable(format!(
                "NASA GIBS returned {}",
                resp.status()
            )));
        }
        let xml = resp.text().await?;
        let layers = parse_wmts_layers(&xml);

        if layers.is_empty() {
            return Err(DataforgeError::EmptyResponse("NASA GIBS WMTS".into()));
        }

        let now = Utc::now();
        let signals: Vec<RawSignal> = layers
            .into_iter()
            .map(|l| {
                let id =
                    RawSignal::compute_id(&format!("{}/{}", GIBS_WMTS_URL, l.identifier), &now);
                RawSignal {
                    id,
                    source_url: GIBS_WMTS_URL.into(),
                    source_name: "NASA GIBS".into(),
                    category: DataCategory::Satellite,
                    raw_content: format!(
                        "satellite_layer:{} title:{} latest:{}",
                        l.identifier, l.title, l.latest_time
                    ),
                    captured_at: now,
                    data_period: Some(l.latest_time),
                    location: None,
                }
            })
            .collect();

        Ok(signals)
    }

    fn fetch_interval(&self) -> Duration {
        CACHE_TTL
    }
}

/// Parsed WMTS layer metadata.
struct WmtsLayer {
    identifier: String,
    title: String,
    latest_time: String,
}

/// Parse WMTS GetCapabilities XML for layer identifiers, titles, and latest times.
///
/// ponytail: simple string scanning, no XML parser dependency. GIBS WMTS XML is
/// machine-generated and structurally stable. Add quick-xml if parsing breaks.
fn parse_wmts_layers(xml: &str) -> Vec<WmtsLayer> {
    let mut layers = Vec::new();

    // Split on <Layer> boundaries. First chunk is pre-layer preamble.
    for chunk in xml.split("<Layer>").skip(1) {
        // End of this layer element
        let layer_end = chunk.find("</Layer>").unwrap_or(chunk.len());
        let layer_xml = &chunk[..layer_end];

        let identifier = tag_content(layer_xml, "ows:Identifier");
        let title = tag_content(layer_xml, "ows:Title");
        // GIBS WMTS uses <wmts:Dimension><ows:Value> for available times;
        // we take the last <ows:Value> under <Dimension> as latest.
        let latest_time = last_value_in_dimension(layer_xml);

        if let (Some(id), Some(title)) = (identifier, title) {
            layers.push(WmtsLayer {
                identifier: id,
                title,
                latest_time: latest_time.unwrap_or_else(|| "unknown".into()),
            });
        }
    }

    layers
}

/// Extract content of a simple XML tag: `<tag>content</tag>`.
fn tag_content(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)?;
    Some(xml[start..start + end].trim().to_string())
}

/// Extract the last `<ows:Value>` inside the last `<Dimension>` in this layer.
fn last_value_in_dimension(xml: &str) -> Option<String> {
    let dim_start = xml.rfind("<Dimension>")?;
    let dim_end = xml[dim_start..].find("</Dimension>")?;
    let dim_xml = &xml[dim_start..dim_start + dim_end];
    dim_xml
        .split("<ows:Value>")
        .last()?
        .split("</ows:Value>")
        .next()
        .map(|s| s.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_layer() {
        let xml = r#"<Layer>
            <ows:Title>MODIS Aqua Corrected Reflectance</ows:Title>
            <ows:Identifier>MODIS_Aqua_CorrectedReflectance_TrueColor</ows:Identifier>
            <Dimension>
                <ows:Value>2026-07-07</ows:Value>
                <ows:Value>2026-07-08</ows:Value>
            </Dimension>
        </Layer>"#;
        let layers = parse_wmts_layers(xml);
        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0].title, "MODIS Aqua Corrected Reflectance");
        assert_eq!(
            layers[0].identifier,
            "MODIS_Aqua_CorrectedReflectance_TrueColor"
        );
        assert_eq!(layers[0].latest_time, "2026-07-08");
    }

    #[test]
    fn parse_multiple_layers() {
        let xml = r#"<?xml version="1.0"?>
        <Capabilities>
            <Layer>
                <ows:Title>MODIS Terra</ows:Title>
                <ows:Identifier>MODIS_Terra</ows:Identifier>
                <Dimension><ows:Value>2026-07-07</ows:Value></Dimension>
            </Layer>
            <Layer>
                <ows:Title>VIIRS SNPP</ows:Title>
                <ows:Identifier>VIIRS_SNPP</ows:Identifier>
                <Dimension><ows:Value>2026-07-08</ows:Value></Dimension>
            </Layer>
        </Capabilities>"#;
        let layers = parse_wmts_layers(xml);
        assert_eq!(layers.len(), 2);
    }

    #[test]
    fn parse_empty_xml_returns_empty() {
        assert_eq!(parse_wmts_layers("<Capabilities></Capabilities>").len(), 0);
        assert_eq!(parse_wmts_layers("").len(), 0);
    }

    #[test]
    fn source_metadata() {
        let source = GibsSource::new();
        assert_eq!(source.name(), "NASA GIBS");
    }
}
