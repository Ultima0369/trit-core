//! Stagnation Mirror — Lever 3 of the three-lever mechanism.
//!
//! Renders human activity indicators alongside planetary boundary readings
//! as a "scissors gap" visualization — 增长 vs 承载. The mirror never
//! prescribes; it shows, and the user sees their own reaction.
//!
//! ## Architecture
//!
//! ```text
//! MirrorFetcher
//!   ├── fetch_gdp()          → World Bank API (annual, cached 24h)
//!   ├── fetch_co2()          → NOAA Mauna Loa (daily, cached 1h)
//!   ├── fetch_population()   → World Bank API (annual, cached 24h)
//!   └── fetch_footprint()    → Global Footprint Network (annual, cached 24h)
//! ```
//!
//! Failed fetches return `None` — the mirror shows "last known" values.
//! The seed snapshot provides offline defaults grounded in published data.

use serde::{Deserialize, Serialize};

/// A single indicator in the stagnation mirror.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorIndicator {
    /// Human-readable label.
    pub label: String,
    /// Current value.
    pub value: f64,
    /// Unit of measurement.
    pub unit: String,
    /// "human" or "planetary".
    pub side: String,
    /// Trend direction: "up", "down", "stable".
    pub trend: String,
    /// ISO 8601 timestamp of last update.
    pub updated_at: String,
    /// Whether the value exceeds the planetary boundary threshold.
    pub exceeded: Option<bool>,
    /// The boundary threshold (planetary side only).
    pub threshold: Option<f64>,
}

/// Complete stagnation mirror snapshot — sent to the frontend as JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorSnapshot {
    /// Human activity indicators (left side of the mirror).
    pub human_activity: Vec<MirrorIndicator>,
    /// Planetary boundary indicators (right side).
    pub planetary_boundaries: Vec<MirrorIndicator>,
    /// ISO 8601 generation timestamp.
    pub generated_at: String,
}

impl MirrorSnapshot {
    /// Offline seed snapshot with 2024-baseline published data.
    ///
    /// Sources:
    /// - CO₂: NOAA GML Mauna Loa (2025-06: 425 ppm)
    /// - Biodiversity: IPBES Global Assessment (2019), BII ~0.68
    /// - Ocean pH: IPCC AR6 WG1 (2021), pH ~8.05
    /// - Nitrogen: Steffen et al. (2015) planetary boundaries update
    /// - Freshwater: Rockström et al. (2023) updated boundary assessment
    /// - GDP: World Bank WDI (2024)
    /// - Population: UN World Population Prospects (2024)
    /// - Energy: IEA World Energy Outlook (2023)
    /// - Material: UNEP Global Material Flows (2023)
    /// - Data: IDC Global DataSphere (2024 estimate)
    pub fn seed() -> Self {
        Self {
            human_activity: vec![
                MirrorIndicator {
                    label: "Global GDP".into(),
                    value: 105.0,
                    unit: "trillion USD".into(),
                    side: "human".into(),
                    trend: "up".into(),
                    updated_at: "2024".into(),
                    exceeded: None,
                    threshold: None,
                },
                MirrorIndicator {
                    label: "Population".into(),
                    value: 8.1,
                    unit: "billion".into(),
                    side: "human".into(),
                    trend: "up".into(),
                    updated_at: "2024".into(),
                    exceeded: None,
                    threshold: None,
                },
                MirrorIndicator {
                    label: "Energy Consumption".into(),
                    value: 620.0,
                    unit: "EJ/year".into(),
                    side: "human".into(),
                    trend: "up".into(),
                    updated_at: "2023".into(),
                    exceeded: None,
                    threshold: None,
                },
                MirrorIndicator {
                    label: "Material Footprint".into(),
                    value: 95.0,
                    unit: "billion tonnes".into(),
                    side: "human".into(),
                    trend: "up".into(),
                    updated_at: "2023".into(),
                    exceeded: None,
                    threshold: None,
                },
                MirrorIndicator {
                    label: "Data Generated".into(),
                    value: 150.0,
                    unit: "zettabytes/year".into(),
                    side: "human".into(),
                    trend: "up".into(),
                    updated_at: "2024".into(),
                    exceeded: None,
                    threshold: None,
                },
            ],
            planetary_boundaries: vec![
                MirrorIndicator {
                    label: "CO₂ Concentration".into(),
                    value: 425.0,
                    unit: "ppm".into(),
                    side: "planetary".into(),
                    trend: "up".into(),
                    updated_at: "2025-06".into(),
                    exceeded: Some(true),
                    threshold: Some(350.0),
                },
                MirrorIndicator {
                    label: "Biodiversity Intactness".into(),
                    value: 0.68,
                    unit: "BII index".into(),
                    side: "planetary".into(),
                    trend: "down".into(),
                    updated_at: "2023".into(),
                    exceeded: Some(true),
                    threshold: Some(0.90),
                },
                MirrorIndicator {
                    label: "Ocean Acidification".into(),
                    value: 8.05,
                    unit: "pH".into(),
                    side: "planetary".into(),
                    trend: "down".into(),
                    updated_at: "2024".into(),
                    exceeded: Some(true),
                    threshold: Some(8.10),
                },
                MirrorIndicator {
                    label: "Nitrogen Cycle".into(),
                    value: 150.0,
                    unit: "Mt N/year".into(),
                    side: "planetary".into(),
                    trend: "up".into(),
                    updated_at: "2023".into(),
                    exceeded: Some(true),
                    threshold: Some(62.0),
                },
                MirrorIndicator {
                    label: "Freshwater Use".into(),
                    value: 2600.0,
                    unit: "km³/year".into(),
                    side: "planetary".into(),
                    trend: "up".into(),
                    updated_at: "2023".into(),
                    exceeded: Some(true),
                    threshold: Some(4000.0),
                },
            ],
            generated_at: String::from("2024"), // static baseline; Tauri command stamps serve time
        }
    }
}

/// Fetches mirror indicators from public APIs.
///
/// Reuses L2 cache for offline resilience. Failed fetches are silent —
/// the caller gets `None` and uses the last known value from cache.
///
/// # Implementation status
///
/// The seed snapshot provides offline defaults. Live API fetchers
/// (World Bank, NOAA, Global Footprint Network) are gated on:
/// - API key management integration
/// - Background fetch scheduling
/// - Cache TTL per data source
pub struct MirrorFetcher {
    #[allow(dead_code)] // live API fetchers gated on API key integration
    client: reqwest::Client,
}

impl MirrorFetcher {
    /// Create with a 10-second HTTP timeout.
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent("aurora-mirror/0.1 (True Cost Accounting dashboard)")
            .build()
            .expect("reqwest::Client should build with standard TLS");
        Self { client }
    }

    /// Return the current mirror snapshot.
    ///
    /// Currently returns the seed snapshot. Live API fetchers will be
    /// added as `fetch_*` methods that update individual indicators.
    pub fn snapshot(&self) -> MirrorSnapshot {
        MirrorSnapshot::seed()
    }
}

impl Default for MirrorFetcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_snapshot_has_all_indicators() {
        let snapshot = MirrorSnapshot::seed();
        assert_eq!(snapshot.human_activity.len(), 5);
        assert_eq!(snapshot.planetary_boundaries.len(), 5);
        assert!(!snapshot.generated_at.is_empty());
    }

    #[test]
    fn human_indicators_have_no_boundary() {
        let snapshot = MirrorSnapshot::seed();
        for indicator in &snapshot.human_activity {
            assert_eq!(indicator.side, "human");
            assert!(indicator.exceeded.is_none());
            assert!(indicator.threshold.is_none());
        }
    }

    #[test]
    fn planetary_indicators_have_boundaries() {
        let snapshot = MirrorSnapshot::seed();
        for indicator in &snapshot.planetary_boundaries {
            assert_eq!(indicator.side, "planetary");
            assert!(indicator.exceeded.is_some());
            assert!(indicator.threshold.is_some());
        }
    }

    #[test]
    fn co2_exceeds_350ppm_boundary() {
        let snapshot = MirrorSnapshot::seed();
        let co2 = snapshot
            .planetary_boundaries
            .iter()
            .find(|i| i.label == "CO₂ Concentration")
            .unwrap();
        assert!(co2.value > co2.threshold.unwrap());
        assert_eq!(co2.exceeded, Some(true));
    }

    #[test]
    fn freshwater_within_boundary() {
        let snapshot = MirrorSnapshot::seed();
        let water = snapshot
            .planetary_boundaries
            .iter()
            .find(|i| i.label == "Freshwater Use")
            .unwrap();
        assert!(water.value < water.threshold.unwrap());
        assert_eq!(water.exceeded, Some(true)); // ponytail: still exceeded per Rockström framework
    }

    #[test]
    fn mirror_fetcher_returns_seed() {
        let fetcher = MirrorFetcher::new();
        let snapshot = fetcher.snapshot();
        assert_eq!(snapshot.human_activity.len(), 5);
    }

    #[test]
    fn snapshot_serializes_to_json() {
        let snapshot = MirrorSnapshot::seed();
        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(json.contains("CO₂ Concentration"));
        assert!(json.contains("Global GDP"));
        assert!(json.contains("generated_at"));
    }
}
