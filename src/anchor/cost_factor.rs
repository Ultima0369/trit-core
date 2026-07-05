//! True Cost Accounting factor library.
//!
//! Maps environmental and social impacts to monetary metadata using
//! the True Cost Accounting (TCA) methodology developed by the
//! True Price Foundation and related research communities.
//!
//! ## Design
//!
//! Each [`CostFactor`] implementation answers one question:
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum Region {
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

impl Default for Region {
    fn default() -> Self {
        Region::Global
    }
}

/// Economic sector for cost factor adjustment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum Sector {
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

impl Default for Sector {
    fn default() -> Self {
        Sector::Generic
    }
}

/// Maps an environmental or social impact to its monetary cost.
///
/// Implementations can be:
/// - A JSON file loader (reference implementation)
/// - A database-backed lookup
/// - An API call to a real-time pricing service
///
/// The trait is object-safe so it can be stored in `Box<dyn CostFactor>`.
pub trait CostFactor: Send + Sync {
    /// Cost of CO₂ equivalent emissions, USD per tonne.
    fn co2_cost(&self, region: Region, sector: Sector) -> Option<CostMetadata>;

    /// Cost of water consumption, USD per cubic meter.
    fn water_cost(&self, region: Region, sector: Sector) -> Option<CostMetadata>;

    /// Cost of land use / biodiversity loss, USD per hectare.
    fn land_cost(&self, region: Region, sector: Sector) -> Option<CostMetadata>;

    /// Cost of air pollution (non-CO₂), USD per tonne PM2.5 equivalent.
    fn air_pollution_cost(&self, region: Region, sector: Sector) -> Option<CostMetadata>;

    /// Cost of water pollution, USD per cubic meter contaminated.
    fn water_pollution_cost(&self, region: Region, sector: Sector) -> Option<CostMetadata>;

    /// Number of factors this implementation provides.
    fn factor_count(&self) -> usize;

    /// Whether this implementation has any real data (vs all defaults).
    fn is_operational(&self) -> bool;
}
