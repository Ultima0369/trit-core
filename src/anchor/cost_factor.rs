//! True Cost Accounting factor library.
//!
//! Maps environmental and social impacts to monetary metadata using
//! the True Cost Accounting (TCA) methodology developed by the
//! True Price Foundation and related research communities.
//!
//! ## Design
//!
//! Each lookup method answers one question:
//! "What is the externalized cost of this impact, in USD?"
//!
//! Factors are tiered:
//! - **Global default** — the baseline conversion factor
//! - **Regional adjustment** — multiplier for geographic context
//! - **Sector adjustment** — multiplier for industry context
//! - **Confidence** — 0.0–1.0, lower when data is sparse

use serde::{Deserialize, Serialize};

/// Monetary cost metadata for a single impact dimension.
///
/// All monetary values are in 2025 USD. Regional and sector adjustments
/// are multipliers applied to the global baseline.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CostMetadata {
    /// Human-readable impact name, e.g. "CO₂ emissions".
    pub impact_name: String,
    /// Global baseline cost per unit (2025 USD).
    pub global_cost_per_unit: f64,
    /// Unit of measurement, e.g. "tonne", "m³", "hectare".
    pub unit: String,
    /// Regional adjustment multiplier (1.0 = no adjustment).
    pub regional_multiplier: f64,
    /// Sector adjustment multiplier (1.0 = no adjustment).
    pub sector_multiplier: f64,
    /// Confidence in this factor [0.0, 1.0].
    pub confidence: f64,
    /// Data source citation.
    pub source: String,
}

impl CostMetadata {
    /// Effective cost per unit after applying regional and sector adjustments.
    pub fn effective_cost(&self) -> f64 {
        self.global_cost_per_unit * self.regional_multiplier * self.sector_multiplier
    }
}

/// Geographic region for cost factor adjustment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize, Serialize)]
pub enum Region {
    #[default]
    Global,
    /// Sub-Saharan Africa
    SSA,
    /// South Asia
    SouthAsia,
    /// East Asia & Pacific
    EastAsiaPacific,
    /// Europe & Central Asia
    EuropeCentralAsia,
    /// Latin America & Caribbean
    LatinAmericaCaribbean,
    /// Middle East & North Africa
    MENA,
    /// North America
    NorthAmerica,
}

/// Economic sector for cost factor adjustment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize, Serialize)]
pub enum Sector {
    #[default]
    Generic,
    Agriculture,
    Energy,
    Manufacturing,
    Transportation,
    Construction,
    Finance,
    Healthcare,
    Technology,
}

/// JSON-backed factor loader.
///
/// Loads factor data from a JSON string via [`load_from_str`].
/// File I/O is handled by [`crate::sandbox::load_factors_from_file`].
///
/// Lookup methods return [`CostMetadata`] for a given [`Region`]/[`Sector`]
/// pair, or `None` if the impact is not in the loaded data.
pub struct JsonFactorLoader {
    factors: Vec<FactorEntry>,
}

/// Error type for factor loading failures.
#[derive(Debug, thiserror::Error)]
pub enum FactorError {
    #[error("failed to read factor data: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse factor JSON: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("unknown impact: '{0}'")]
    UnknownImpact(String),
}

#[derive(Debug, Clone, Deserialize)]
struct FactorFile {
    #[allow(dead_code)]
    description: String,
    factors: Vec<FactorEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct FactorEntry {
    impact: String,
    global_cost_per_unit: f64,
    unit: String,
    regional_multipliers: std::collections::HashMap<String, f64>,
    sector_multipliers: std::collections::HashMap<String, f64>,
    confidence: f64,
    source: String,
}

impl JsonFactorLoader {
    /// Load factor data from an embedded JSON string (for tests, wasm targets,
    /// and any caller that has already read the file).
    ///
    /// File I/O (`load(path)`) was removed during the Layer Dependency Cleanup
    /// (2026-07-08). Use [`crate::sandbox::load_factors_from_file`] for file
    /// loading, or do your own I/O and pass the string here.
    pub fn load_from_str(json: &str) -> Result<Self, FactorError> {
        let file: FactorFile = serde_json::from_str(json)?;
        Ok(Self {
            factors: file.factors,
        })
    }

    /// Cost of CO₂ equivalent emissions, USD per tonne.
    pub fn co2_cost(&self, region: Region, sector: Sector) -> Option<CostMetadata> {
        self.find("co2")
            .map(|e| Self::build_metadata(e, region, sector))
    }

    /// Cost of water consumption, USD per cubic meter.
    pub fn water_cost(&self, region: Region, sector: Sector) -> Option<CostMetadata> {
        self.find("water_consumption")
            .map(|e| Self::build_metadata(e, region, sector))
    }

    /// Cost of land use / biodiversity loss, USD per hectare.
    pub fn land_cost(&self, region: Region, sector: Sector) -> Option<CostMetadata> {
        self.find("land_use")
            .map(|e| Self::build_metadata(e, region, sector))
    }

    /// Cost of air pollution (non-CO₂), USD per tonne PM2.5 equivalent.
    pub fn air_pollution_cost(&self, _region: Region, _sector: Sector) -> Option<CostMetadata> {
        None
    }

    /// Cost of water pollution, USD per cubic meter contaminated.
    pub fn water_pollution_cost(&self, _region: Region, _sector: Sector) -> Option<CostMetadata> {
        None
    }

    /// Number of factors this implementation provides.
    pub fn factor_count(&self) -> usize {
        self.factors.len()
    }

    /// Whether this implementation has any real data (vs all defaults).
    pub fn is_operational(&self) -> bool {
        !self.factors.is_empty()
    }

    fn find(&self, impact: &str) -> Option<&FactorEntry> {
        self.factors.iter().find(|f| f.impact == impact)
    }

    fn build_metadata(entry: &FactorEntry, region: Region, sector: Sector) -> CostMetadata {
        let region_key = format!("{region:?}");
        let sector_key = format!("{sector:?}");
        let regional_multiplier = entry
            .regional_multipliers
            .get(&region_key)
            .copied()
            .unwrap_or(1.0);
        let sector_multiplier = entry
            .sector_multipliers
            .get(&sector_key)
            .copied()
            .unwrap_or(1.0);
        CostMetadata {
            impact_name: entry.impact.clone(),
            global_cost_per_unit: entry.global_cost_per_unit,
            unit: entry.unit.clone(),
            regional_multiplier,
            sector_multiplier,
            confidence: entry.confidence,
            source: entry.source.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEED_JSON: &str = r#"{
  "description": "test",
  "factors": [
    {
      "impact": "co2",
      "global_cost_per_unit": 100.0,
      "unit": "tonne",
      "regional_multipliers": {"Global": 1.0, "NorthAmerica": 1.5},
      "sector_multipliers": {"Generic": 1.0, "Energy": 2.0},
      "confidence": 0.8,
      "source": "test"
    },
    {
      "impact": "water_consumption",
      "global_cost_per_unit": 5.0,
      "unit": "m³",
      "regional_multipliers": {"Global": 1.0, "MENA": 3.0},
      "sector_multipliers": {"Generic": 1.0, "Agriculture": 2.0},
      "confidence": 0.6,
      "source": "test"
    }
  ]
}"#;

    fn test_loader() -> JsonFactorLoader {
        JsonFactorLoader::load_from_str(SEED_JSON).unwrap()
    }

    #[test]
    fn effective_cost_applies_both_multipliers() {
        let meta = CostMetadata {
            impact_name: "co2".into(),
            global_cost_per_unit: 100.0,
            unit: "tonne".into(),
            regional_multiplier: 1.5,
            sector_multiplier: 2.0,
            confidence: 0.8,
            source: "test".into(),
        };
        assert_float_eq!(meta.effective_cost(), 300.0);
    }

    #[test]
    fn co2_global_generic_returns_baseline() {
        let loader = test_loader();
        let meta = loader.co2_cost(Region::Global, Sector::Generic).unwrap();
        assert_float_eq!(meta.global_cost_per_unit, 100.0);
        assert_float_eq!(meta.effective_cost(), 100.0);
    }

    #[test]
    fn co2_north_america_energy_applies_adjustments() {
        let loader = test_loader();
        let meta = loader
            .co2_cost(Region::NorthAmerica, Sector::Energy)
            .unwrap();
        assert_float_eq!(meta.regional_multiplier, 1.5);
        assert_float_eq!(meta.sector_multiplier, 2.0);
        assert_float_eq!(meta.effective_cost(), 300.0);
    }

    #[test]
    fn water_mena_agriculture_applies_scarcity_multiplier() {
        let loader = test_loader();
        let meta = loader
            .water_cost(Region::MENA, Sector::Agriculture)
            .unwrap();
        assert_float_eq!(meta.regional_multiplier, 3.0);
        assert_float_eq!(meta.effective_cost(), 30.0);
    }

    #[test]
    fn missing_factor_returns_none() {
        let loader = test_loader();
        assert!(loader
            .air_pollution_cost(Region::Global, Sector::Generic)
            .is_none());
    }

    #[test]
    fn factor_count_is_correct() {
        let loader = test_loader();
        assert_eq!(loader.factor_count(), 2);
    }

    #[test]
    fn is_operational_true_when_factors_loaded() {
        let loader = test_loader();
        assert!(loader.is_operational());
    }

    #[test]
    fn unknown_region_falls_back_to_1_0() {
        let loader = test_loader();
        let meta = loader.co2_cost(Region::SSA, Sector::Generic).unwrap();
        assert_float_eq!(meta.regional_multiplier, 1.0);
    }

    #[test]
    fn confidence_is_preserved() {
        let loader = test_loader();
        let meta = loader.co2_cost(Region::Global, Sector::Generic).unwrap();
        assert_float_eq!(meta.confidence, 0.8);
    }
}
