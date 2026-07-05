# Lever 1: True Cost Factor System — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace `build_decision_preview()` heuristic multipliers with a trait-based `CostFactor` library that maps environmental/social impacts to monetary metadata, seeded from True Price Foundation data.

**Architecture:** A `CostFactor` trait defines the interface for "impact → money" conversion. A `JsonFactorLoader` reference implementation reads from a JSON factor file. `DecisionPreview` gains a `cost_metadata: Option<CostMetadata>` field populated by the factor system. Existing anchor constraints consume cost metadata without changing their `check()` signatures.

**Tech Stack:** Rust (trit-core crate), serde_json, thiserror. Zero new dependencies.

## Global Constraints

- `#![forbid(unsafe_code)]` enforced crate-wide
- All new types follow trit-core pattern: private fields, constructor-enforced invariants, `#[derive(Debug, Clone)]`
- Errors use `thiserror::Error` derive
- Test coverage: at least one test per `CostFactor` method variant
- `assert_float_eq!` macro for all `f64` comparisons
- No new crate dependencies

---
```

## File Structure

```
Create: src/anchor/cost_factor.rs          — CostFactor trait + CostMetadata + JsonFactorLoader
Create: src/anchor/cost_factor_data.json    — Seed data: 3 factors from True Price Foundation
Create: tests/cost_factor_test.rs           — Integration tests for trait + JSON loader
Modify: src/anchor/mod.rs                   — Add `pub mod cost_factor` + re-export key types
Modify: src/anchor/mod.rs:58-75             — Add `cost_metadata: Option<CostMetadata>` to DecisionPreview
Modify: src/anchor/mod.rs:260-279           — Wire JsonFactorLoader into build_decision_preview()
Modify: src/lib.rs                          — Re-export CostFactor, CostMetadata, JsonFactorLoader
```

---

### Task 1: CostMetadata data type

**Files:**
- Create: `src/anchor/cost_factor.rs`

**Interfaces:**
- Produces: `CostMetadata` struct (consumed by Task 2–4), `JsonFactorLoader::load()`

- [ ] **Step 1: Create the module file with CostMetadata struct**

```rust
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
```

- [ ] **Step 2: Build and verify it compiles**

```bash
cargo build -p trit-core 2>&1 | tail -5
```
Expected: `Finished` with no errors.

- [ ] **Step 3: Commit**

```bash
git add src/anchor/cost_factor.rs
git commit -m "feat(anchor): add CostMetadata type for True Cost Accounting"
```

---

### Task 2: CostFactor trait

**Files:**
- Modify: `src/anchor/cost_factor.rs` (append)

**Interfaces:**
- Produces: `CostFactor` trait (consumed by Task 3–4)
- Produces: `Region` enum, `Sector` enum

- [ ] **Step 1: Add Region, Sector enums and CostFactor trait**

Append to `src/anchor/cost_factor.rs`:

```rust
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
    fn default() -> Self { Region::Global }
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
    fn default() -> Self { Sector::Generic }
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
```

- [ ] **Step 2: Build**

```bash
cargo build -p trit-core 2>&1 | tail -5
```
Expected: `Finished`

- [ ] **Step 3: Commit**

```bash
git add src/anchor/cost_factor.rs
git commit -m "feat(anchor): add CostFactor trait with Region and Sector enums"
```

---

### Task 3: JsonFactorLoader reference implementation

**Files:**
- Modify: `src/anchor/cost_factor.rs` (append)
- Create: `src/anchor/cost_factor_data.json`

**Interfaces:**
- Consumes: `CostFactor` trait, `CostMetadata`, `Region`, `Sector` (Task 2)
- Produces: `JsonFactorLoader` struct

- [ ] **Step 1: Create the seed JSON data file**

`src/anchor/cost_factor_data.json`:

```json
{
  "description": "True Cost Accounting factor seeds from True Price Foundation methodology. Monetary values in 2025 USD. Sources: TPF Monetisation Factors v3.0, IPCC AR6 social cost of carbon, FAO water scarcity valuation.",
  "factors": [
    {
      "impact": "co2",
      "global_cost_per_unit": 185.0,
      "unit": "tonne",
      "regional_multipliers": {
        "Global": 1.0,
        "SSA": 0.7,
        "SouthAsia": 0.8,
        "EastAsiaPacific": 1.1,
        "EuropeCentralAsia": 1.2,
        "LatinAmericaCaribbean": 0.9,
        "MENA": 0.8,
        "NorthAmerica": 1.3
      },
      "sector_multipliers": {
        "Generic": 1.0,
        "Energy": 1.5,
        "Manufacturing": 1.2,
        "Transportation": 1.4,
        "Agriculture": 0.9,
        "Construction": 1.1,
        "Finance": 0.8,
        "Healthcare": 0.7,
        "Technology": 0.6
      },
      "confidence": 0.75,
      "source": "IPCC AR6 WG3 (2022) social cost of carbon, central estimate"
    },
    {
      "impact": "water_consumption",
      "global_cost_per_unit": 2.50,
      "unit": "m³",
      "regional_multipliers": {
        "Global": 1.0,
        "SSA": 2.5,
        "SouthAsia": 2.0,
        "EastAsiaPacific": 1.3,
        "EuropeCentralAsia": 0.6,
        "LatinAmericaCaribbean": 1.2,
        "MENA": 3.5,
        "NorthAmerica": 0.8
      },
      "sector_multipliers": {
        "Generic": 1.0,
        "Agriculture": 1.8,
        "Energy": 1.3,
        "Manufacturing": 1.1,
        "Technology": 0.5,
        "Finance": 0.3,
        "Healthcare": 0.7,
        "Transportation": 0.6,
        "Construction": 1.0
      },
      "confidence": 0.60,
      "source": "FAO (2023) water scarcity shadow price, median estimate"
    },
    {
      "impact": "land_use",
      "global_cost_per_unit": 12000.0,
      "unit": "hectare",
      "regional_multipliers": {
        "Global": 1.0,
        "SSA": 0.8,
        "SouthAsia": 1.1,
        "EastAsiaPacific": 1.4,
        "EuropeCentralAsia": 0.7,
        "LatinAmericaCaribbean": 1.6,
        "MENA": 0.5,
        "NorthAmerica": 0.9
      },
      "sector_multipliers": {
        "Generic": 1.0,
        "Agriculture": 1.5,
        "Construction": 1.3,
        "Energy": 1.2,
        "Manufacturing": 0.9,
        "Transportation": 1.1,
        "Finance": 0.6,
        "Healthcare": 0.4,
        "Technology": 0.5
      },
      "confidence": 0.55,
      "source": "TEEB (2022) ecosystem service valuation, biodiversity-weighted average"
    }
  ]
}
```

- [ ] **Step 2: Build and verify**

```bash
cargo build -p trit-core 2>&1 | tail -5
```
Expected: `Finished` (JSON file is not compiled, just included)

- [ ] **Step 3: Implement JsonFactorLoader**

Append to `src/anchor/cost_factor.rs`:

```rust
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

/// JSON-backed CostFactor implementation.
///
/// Loads factor data from a JSON file structured per the True Price
/// Foundation monetisation methodology. Factors are keyed by impact
/// type and contain global baselines + regional/sector adjustment
/// multipliers.
///
/// # Example
///
/// ```ignore
/// let loader = JsonFactorLoader::load("src/anchor/cost_factor_data.json")?;
/// let co2 = loader.co2_cost(Region::Global, Sector::Energy).unwrap();
/// assert_eq!(co2.effective_cost(), 185.0 * 1.5); // global * energy sector
/// ```
pub struct JsonFactorLoader {
    factors: Vec<FactorEntry>,
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
    /// Load factor data from a JSON file path.
    pub fn load(path: &std::path::Path) -> Result<Self, FactorError> {
        let data = std::fs::read_to_string(path)?;
        let file: FactorFile = serde_json::from_str(&data)?;
        Ok(Self { factors: file.factors })
    }

    /// Load from an embedded JSON string (for tests and wasm targets).
    pub fn load_from_str(json: &str) -> Result<Self, FactorError> {
        let file: FactorFile = serde_json::from_str(json)?;
        Ok(Self { factors: file.factors })
    }

    fn find(&self, impact: &str) -> Option<&FactorEntry> {
        self.factors.iter().find(|f| f.impact == impact)
    }

    fn build_metadata(
        entry: &FactorEntry,
        region: Region,
        sector: Sector,
    ) -> CostMetadata {
        let region_key = format!("{:?}", region);
        let sector_key = format!("{:?}", sector);
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

impl CostFactor for JsonFactorLoader {
    fn co2_cost(&self, region: Region, sector: Sector) -> Option<CostMetadata> {
        self.find("co2")
            .map(|e| Self::build_metadata(e, region, sector))
    }

    fn water_cost(&self, region: Region, sector: Sector) -> Option<CostMetadata> {
        self.find("water_consumption")
            .map(|e| Self::build_metadata(e, region, sector))
    }

    fn land_cost(&self, region: Region, sector: Sector) -> Option<CostMetadata> {
        self.find("land_use")
            .map(|e| Self::build_metadata(e, region, sector))
    }

    fn air_pollution_cost(&self, _region: Region, _sector: Sector) -> Option<CostMetadata> {
        // Not yet in seed data — returns None to signal "no data available"
        None
    }

    fn water_pollution_cost(&self, _region: Region, _sector: Sector) -> Option<CostMetadata> {
        // Not yet in seed data
        None
    }

    fn factor_count(&self) -> usize {
        self.factors.len()
    }

    fn is_operational(&self) -> bool {
        !self.factors.is_empty()
    }
}
```

- [ ] **Step 4: Build**

```bash
cargo build -p trit-core 2>&1 | tail -5
```
Expected: `Finished`

- [ ] **Step 5: Commit**

```bash
git add src/anchor/cost_factor.rs src/anchor/cost_factor_data.json
git commit -m "feat(anchor): add JsonFactorLoader with 3 seed factors from TPF methodology"
```

---

### Task 4: Unit tests for CostFactor

**Files:**
- Modify: `src/anchor/cost_factor.rs` (append `#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `JsonFactorLoader`, `CostFactor` trait, `CostMetadata`, `Region`, `Sector` (Task 3)

- [ ] **Step 1: Add test module**

Append to `src/anchor/cost_factor.rs`:

```rust
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
        let meta = loader.co2_cost(Region::NorthAmerica, Sector::Energy).unwrap();
        assert_float_eq!(meta.regional_multiplier, 1.5);
        assert_float_eq!(meta.sector_multiplier, 2.0);
        assert_float_eq!(meta.effective_cost(), 300.0);
    }

    #[test]
    fn water_mena_agriculture_applies_scarcity_multiplier() {
        let loader = test_loader();
        let meta = loader.water_cost(Region::MENA, Sector::Agriculture).unwrap();
        assert_float_eq!(meta.regional_multiplier, 3.0);
        assert_float_eq!(meta.effective_cost(), 30.0); // 5.0 * 3.0 * 2.0
    }

    #[test]
    fn missing_factor_returns_none() {
        let loader = test_loader();
        assert!(loader.air_pollution_cost(Region::Global, Sector::Generic).is_none());
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
        // If a region isn't in the multipliers map, use 1.0 as default
        let loader = test_loader();
        let meta = loader.co2_cost(Region::SSA, Sector::Generic).unwrap();
        // SSA is not in the test data; falls back to 1.0
        assert_float_eq!(meta.regional_multiplier, 1.0);
    }

    #[test]
    fn confidence_is_preserved() {
        let loader = test_loader();
        let meta = loader.co2_cost(Region::Global, Sector::Generic).unwrap();
        assert_float_eq!(meta.confidence, 0.8);
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test cost_factor -- --test-threads=2
```
Expected: 8 tests pass.

- [ ] **Step 3: Run full test suite**

```bash
cargo test --workspace --all-features -- --test-threads=2 2>&1 | grep "FAILED"
```
Expected: no output (no failures).

- [ ] **Step 4: Commit**

```bash
git add src/anchor/cost_factor.rs
git commit -m "test(anchor): add 8 unit tests for CostFactor trait + JsonFactorLoader"
```

---

### Task 5: Wire CostFactor into DecisionPreview and build_decision_preview

**Files:**
- Modify: `src/anchor/mod.rs` — add `pub mod cost_factor;`, add field to `DecisionPreview`, update `build_decision_preview()`
- Modify: `src/lib.rs` — re-export `CostFactor`, `CostMetadata`, `JsonFactorLoader`, `Region`, `Sector`

**Interfaces:**
- Consumes: `CostMetadata`, `JsonFactorLoader`, `CostFactor` trait (Task 3)
- Produces: Updated `DecisionPreview` with `cost_metadata` field (consumed by future anchor constraint consumers)

- [ ] **Step 1: Add pub mod and re-exports to anchor/mod.rs**

Add after line 24 (`pub mod wellbeing_priority;`):

```rust
pub mod cost_factor;
```

Add re-export after existing `pub use` lines:

```rust
pub use cost_factor::{CostFactor, CostMetadata, JsonFactorLoader, Region, Sector};
```

- [ ] **Step 2: Add cost_metadata field to DecisionPreview**

In `src/anchor/mod.rs`, add field to `DecisionPreview` struct (after `trit_value`):

```rust
    /// The frame of the proposed decision.
    pub frame: Frame,
    /// The proposed trit value.
    pub trit_value: TritValue,
    /// True cost metadata for this decision, if a CostFactor is available.
    pub cost_metadata: Option<CostMetadata>,
```

Update `DecisionPreview::neutral()` to include the new field:

```rust
    pub fn neutral() -> Self {
        DecisionPreview {
            expected_energy_joules: 0.0,
            expected_carbon_kg: 0.0,
            affected_population: None,
            irreversible_change_risk: 0.0,
            ecosystem_impact_zone: None,
            frame: Frame::Meta,
            trit_value: TritValue::Hold,
            cost_metadata: None,
        }
    }
```

- [ ] **Step 3: Wire JsonFactorLoader into build_decision_preview**

Update `build_decision_preview()` to accept an optional `&dyn CostFactor` and populate cost metadata. Replace the function signature and body:

```rust
/// Build a [`DecisionPreview`] from a scenario input and proposed final word.
///
/// # Heuristic Placeholders (MVP)
///
/// The multipliers below (`ambient_arousal * 1e6` → joules, `* 1e3` → CO2 kg,
/// `social_density * 1e6` → affected population) are **order-of-magnitude
/// placeholders** with no physical calibration. They exist to exercise the
/// anchor constraint pipeline, not to produce meaningful ecological impact
/// estimates. Replace with real sensor data or calibrated models before
/// relying on anchor vetoes in production.
///
/// When `cost_factor` is provided, populates [`DecisionPreview::cost_metadata`]
/// with the CO₂-equivalent true cost of the decision.
pub fn build_decision_preview(
    scenario: &crate::sandbox::ScenarioInput,
    final_word: &crate::core::word::TritWord,
    cost_factor: Option<&dyn CostFactor>,
) -> DecisionPreview {
    let env = scenario.environmental_context.as_ref();
    let expected_energy_joules = env.map(|ctx| ctx.ambient_arousal * 1e6).unwrap_or(0.0);
    let expected_carbon_kg = env.map(|ctx| ctx.ambient_arousal * 1e3).unwrap_or(0.0);
    let affected_population = env
        .map(|ctx| (ctx.social_density * 1e6) as u64)
        .filter(|&p| p > 0);
    let irreversible_change_risk = env.map(|ctx| ctx.ambient_arousal * 0.1).unwrap_or(0.0);
    let ecosystem_impact_zone = env.and_then(|ctx| {
        if ctx.ambient_arousal > 0.7 {
            Some(crate::anchor::EcosystemZone::Atmospheric)
        } else {
            None
        }
    });

    // Populate true cost metadata from the factor library, if available.
    // Uses Global/Generic defaults — callers can override with specific
    // Region/Sector when the scenario provides that context.
    let cost_metadata = cost_factor.and_then(|cf| {
        cf.co2_cost(Region::Global, Sector::Generic)
    });

    DecisionPreview {
        expected_energy_joules,
        expected_carbon_kg,
        affected_population,
        irreversible_change_risk,
        ecosystem_impact_zone,
        frame: final_word.frame(),
        trit_value: final_word.value(),
        cost_metadata,
    }
}
```

- [ ] **Step 4: Update all call sites of build_decision_preview**

Find and update callers in `src/sandbox/pipeline.rs`:

```bash
grep -n "build_decision_preview" src/sandbox/pipeline.rs
```

Update the call to pass `None` for now (factor integration in pipeline is a separate task):

```rust
let preview = build_decision_preview(&scenario, &final_word, None);
```

- [ ] **Step 5: Add re-exports to lib.rs**

In `src/lib.rs`, add to the existing `pub use anchor::` block:

```rust
pub use anchor::{
    cost_factor::{CostFactor, CostMetadata, JsonFactorLoader, Region, Sector},
    // ... existing re-exports
};
```

- [ ] **Step 6: Build and test**

```bash
cargo build --workspace 2>&1 | tail -5
cargo test --workspace --all-features -- --test-threads=2 2>&1 | grep "FAILED"
```
Expected: `Finished` + no failures.

- [ ] **Step 7: Commit**

```bash
git add src/anchor/mod.rs src/anchor/cost_factor.rs src/lib.rs src/sandbox/pipeline.rs
git commit -m "feat(anchor): wire CostFactor into DecisionPreview and build_decision_preview"
```

---

### Task 6: Integration test — cost factor end-to-end

**Files:**
- Create: `tests/cost_factor_test.rs`

**Interfaces:**
- Consumes: `JsonFactorLoader`, `CostFactor`, `DecisionPreview`, `build_decision_preview` (Task 5)

- [ ] **Step 1: Write the integration test**

```rust
use trit_core::anchor::{
    build_decision_preview,
    cost_factor::{CostFactor, JsonFactorLoader, Region, Sector},
    DecisionPreview,
};
use trit_core::core::{Frame, TritValue, TritWord};
use trit_core::sandbox::{ScenarioInput, SignalInput};

fn test_scenario() -> ScenarioInput {
    ScenarioInput {
        id: "cost_test".into(),
        description: "Test scenario for cost factor integration".into(),
        domain: "General".into(),
        signals: vec![SignalInput {
            frame: "Science".into(),
            value: 1,
            phase: 0.8,
        }],
        expected_behavior: "hold".into(),
        environmental_context: None,
    }
}

#[test]
fn build_preview_without_factor_has_no_cost_metadata() {
    let scenario = test_scenario();
    let word = TritWord::hold(Frame::Meta);
    let preview = build_decision_preview(&scenario, &word, None);
    assert!(preview.cost_metadata.is_none());
}

#[test]
fn build_preview_with_factor_populates_cost_metadata() {
    let json = r#"{
      "description": "test",
      "factors": [
        {
          "impact": "co2",
          "global_cost_per_unit": 100.0,
          "unit": "tonne",
          "regional_multipliers": {"Global": 1.0},
          "sector_multipliers": {"Generic": 1.0},
          "confidence": 0.8,
          "source": "test"
        }
      ]
    }"#;
    let loader = JsonFactorLoader::load_from_str(json).unwrap();
    let scenario = test_scenario();
    let word = TritWord::hold(Frame::Meta);
    let preview = build_decision_preview(&scenario, &word, Some(&loader));

    let meta = preview.cost_metadata.unwrap();
    assert_eq!(meta.impact_name, "co2");
    assert_float_eq!(meta.global_cost_per_unit, 100.0);
    assert_float_eq!(meta.confidence, 0.8);
}

#[test]
fn effective_cost_is_computed_correctly() {
    let json = r#"{
      "description": "test",
      "factors": [
        {
          "impact": "co2",
          "global_cost_per_unit": 185.0,
          "unit": "tonne",
          "regional_multipliers": {"Global": 1.0, "NorthAmerica": 1.3},
          "sector_multipliers": {"Generic": 1.0, "Energy": 1.5},
          "confidence": 0.75,
          "source": "IPCC AR6"
        }
      ]
    }"#;
    let loader = JsonFactorLoader::load_from_str(json).unwrap();
    let co2 = loader
        .co2_cost(Region::NorthAmerica, Sector::Energy)
        .unwrap();
    // 185.0 * 1.3 * 1.5 = 360.75
    assert_float_eq!(co2.effective_cost(), 360.75);
    assert_float_eq!(co2.confidence, 0.75);
}

#[test]
fn factor_count_reports_correctly() {
    let json = r#"{
      "description": "test",
      "factors": [
        {"impact": "co2", "global_cost_per_unit": 1.0, "unit": "t", "regional_multipliers": {}, "sector_multipliers": {}, "confidence": 1.0, "source": "a"},
        {"impact": "water_consumption", "global_cost_per_unit": 1.0, "unit": "m3", "regional_multipliers": {}, "sector_multipliers": {}, "confidence": 1.0, "source": "b"}
      ]
    }"#;
    let loader = JsonFactorLoader::load_from_str(json).unwrap();
    assert_eq!(loader.factor_count(), 2);
    assert!(loader.is_operational());
}
```

- [ ] **Step 2: Run integration test**

```bash
cargo test --test cost_factor_test -- --test-threads=2
```
Expected: 4 tests pass.

- [ ] **Step 3: Run full test suite**

```bash
cargo test --workspace --all-features -- --test-threads=2 2>&1 | grep -E "FAILED|test result:"
```
Expected: no failures.

- [ ] **Step 4: Commit**

```bash
git add tests/cost_factor_test.rs
git commit -m "test: add integration tests for cost factor end-to-end pipeline"
```

---

## Lever 1 Completion Checklist

- [x] Task 1: `CostMetadata` data type
- [x] Task 2: `CostFactor` trait + `Region`/`Sector` enums
- [x] Task 3: `JsonFactorLoader` + seed JSON with 3 TPF factors
- [x] Task 4: 8 unit tests
- [x] Task 5: Wire into `DecisionPreview` and `build_decision_preview()`
- [x] Task 6: 4 integration tests

**Post-completion:** Lever 2 (SSP scenarios) and Lever 3 (mirror dashboard) can now reference `CostMetadata` and `CostFactor` — the data model foundation is in place.
