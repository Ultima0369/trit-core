# Lever 2: Future Retrospective Simulator — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the scenario system with SSP-based future pathways and an LLM-powered "historical review document" generator that produces a 2066-perspective retrospective of today's decisions.

**Architecture:** Five SSP pathway scenario templates (SSP1–SSP5) extend the existing `scenarios/*.json` format. A new `RetrospectiveProvider` implements `ExternalPercept` and plugs into the existing `PerceptChain` degradation model, using the same LLM infrastructure. The output is a `RetrospectiveDoc` struct rendered alongside the decision result.

**Tech Stack:** Rust (aurora crate), existing `ExternalPercept` trait, existing `PerceptChain`, existing LLM providers. Zero new dependencies.

## Global Constraints

- `#![deny(unsafe_code)]` enforced in aurora crate
- New types follow aurora pattern: `#[derive(Debug, Clone, Serialize)]`
- Errors use `thiserror::Error` or existing `PerceptError`
- Test coverage: at least one test per SSP pathway, one roundtrip test for RetrospectiveDoc
- All LLM calls go through existing `ExternalPercept::perceive()` interface
- No new crate dependencies

---
```

## File Structure

```
Create: scenarios/ssp/ssp1_sustainability.json      — SSP1 scenario template
Create: scenarios/ssp/ssp2_middle_road.json          — SSP2 scenario template
Create: scenarios/ssp/ssp3_regional_rivalry.json     — SSP3 scenario template
Create: scenarios/ssp/ssp4_inequality.json           — SSP4 scenario template
Create: scenarios/ssp/ssp5_fossil_fueled.json        — SSP5 scenario template
Create: aurora/src/percept/retrospective.rs           — RetrospectiveProvider + RetrospectiveDoc
Create: aurora/src/percept/prompts/retrospective_system.txt — LLM system prompt
Modify: aurora/src/percept/mod.rs                     — add pub mod retrospective + re-export
Modify: aurora/src/app.rs                             — add run_retrospective() method
Modify: src-tauri/src/commands.rs                     — add Tauri command + RetrospectiveResponse
Modify: ui/src/types.ts                               — add RetrospectiveResponse interface
```

---

### Task 1: SSP scenario templates

**Files:**
- Create: `scenarios/ssp/ssp1_sustainability.json`
- Create: `scenarios/ssp/ssp2_middle_road.json`
- Create: `scenarios/ssp/ssp3_regional_rivalry.json`
- Create: `scenarios/ssp/ssp4_inequality.json`
- Create: `scenarios/ssp/ssp5_fossil_fueled.json`

**Interfaces:**
- Produces: 5 scenario JSON files (consumed by Task 3 RetrospectiveProvider)

- [ ] **Step 1: Create SSP1 — Sustainability (Taking the Green Road)**

`scenarios/ssp/ssp1_sustainability.json`:

```json
{
  "id": "ssp1_sustainability",
  "description": "SSP1 Sustainability — low population growth, high education, rapid tech shift toward renewables, strong international cooperation. 2066: global temp anomaly ~2.0°C, CO2 price $400/tonne, biodiversity recovering in 40% of biomes.",
  "domain": "General",
  "ssp_pathway": "SSP1",
  "lookback_year": 2066,
  "decision_prompt": "Consider a major infrastructure investment today. From 2066, looking back: this decision accelerated the green transition.",
  "signals": [
    { "frame": "Science", "value": 1, "phase": 0.85 },
    { "frame": "Consensus", "value": 1, "phase": 0.80 },
    { "frame": "GeoEco", "value": 1, "phase": 0.75 },
    { "frame": "Developmental", "value": 1, "phase": 0.70 }
  ],
  "projected_signals_2066": {
    "co2_ppm": 440,
    "global_temp_anomaly_c": 2.0,
    "biodiversity_intactness": 0.72,
    "renewable_share": 0.85,
    "population_billion": 8.5
  },
  "expected_behavior": "commit_true"
}
```

- [ ] **Step 2: Create SSP2 — Middle of the Road**

`scenarios/ssp/ssp2_middle_road.json`:

```json
{
  "id": "ssp2_middle_road",
  "description": "SSP2 Middle of the Road — trends continue. Moderate population growth, incremental tech progress, uneven policy. 2066: ~3.0°C warming, CO2 price $150/tonne, biodiversity declining slowly.",
  "domain": "General",
  "ssp_pathway": "SSP2",
  "lookback_year": 2066,
  "decision_prompt": "Consider a major infrastructure investment today. From 2066, looking back: this decision maintained the status quo trajectory.",
  "signals": [
    { "frame": "Science", "value": 1, "phase": 0.65 },
    { "frame": "Consensus", "value": 0, "phase": 0.50 },
    { "frame": "GeoEco", "value": -1, "phase": 0.40 },
    { "frame": "Developmental", "value": 0, "phase": 0.55 }
  ],
  "projected_signals_2066": {
    "co2_ppm": 520,
    "global_temp_anomaly_c": 3.0,
    "biodiversity_intactness": 0.55,
    "renewable_share": 0.50,
    "population_billion": 9.5
  },
  "expected_behavior": "negotiate"
}
```

- [ ] **Step 3: Create SSP3 — Regional Rivalry**

`scenarios/ssp/ssp3_regional_rivalry.json`:

```json
{
  "id": "ssp3_regional_rivalry",
  "description": "SSP3 Regional Rivalry — resurgent nationalism, trade barriers, slow tech diffusion, high population in developing regions. 2066: ~4.5°C warming, fragmented carbon markets, severe biodiversity loss.",
  "domain": "General",
  "ssp_pathway": "SSP3",
  "lookback_year": 2066,
  "decision_prompt": "Consider a major infrastructure investment today. From 2066, looking back: this decision was made in an era of fragmentation that deepened.",
  "signals": [
    { "frame": "Science", "value": 1, "phase": 0.60 },
    { "frame": "Consensus", "value": -1, "phase": 0.25 },
    { "frame": "GeoEco", "value": -1, "phase": 0.20 },
    { "frame": "Role", "value": 1, "phase": 0.70 }
  ],
  "projected_signals_2066": {
    "co2_ppm": 620,
    "global_temp_anomaly_c": 4.5,
    "biodiversity_intactness": 0.38,
    "renewable_share": 0.30,
    "population_billion": 10.5
  },
  "expected_behavior": "hold"
}
```

- [ ] **Step 4: Create SSP4 — Inequality**

`scenarios/ssp/ssp4_inequality.json`:

```json
{
  "id": "ssp4_inequality",
  "description": "SSP4 Inequality — widening gap between rich and poor, high-tech enclaves vs low-tech peripheries, stratified adaptation. 2066: ~3.5°C warming, dual economies, climate apartheid risks.",
  "domain": "General",
  "ssp_pathway": "SSP4",
  "lookback_year": 2066,
  "decision_prompt": "Consider a major infrastructure investment today. From 2066, looking back: this decision benefited one stratum while externalizing costs to another.",
  "signals": [
    { "frame": "Science", "value": 1, "phase": 0.75 },
    { "frame": "Individual", "value": -1, "phase": 0.30 },
    { "frame": "Consensus", "value": -1, "phase": 0.35 },
    { "frame": "Environmental", "value": -1, "phase": 0.25 }
  ],
  "projected_signals_2066": {
    "co2_ppm": 560,
    "global_temp_anomaly_c": 3.5,
    "biodiversity_intactness": 0.45,
    "renewable_share": 0.55,
    "population_billion": 9.0
  },
  "expected_behavior": "hold"
}
```

- [ ] **Step 5: Create SSP5 — Fossil-Fueled Development**

`scenarios/ssp/ssp5_fossil_fueled.json`:

```json
{
  "id": "ssp5_fossil_fueled",
  "description": "SSP5 Fossil-Fueled Development — rapid economic growth, high energy demand, fossil fuel dominance, late-century tech salvation bets. 2066: ~5.0°C warming, severe climate damages, geoengineering debates.",
  "domain": "General",
  "ssp_pathway": "SSP5",
  "lookback_year": 2066,
  "decision_prompt": "Consider a major infrastructure investment today. From 2066, looking back: this decision rode the fossil-fueled boom that is now the subject of international litigation.",
  "signals": [
    { "frame": "Science", "value": -1, "phase": 0.20 },
    { "frame": "Consensus", "value": 1, "phase": 0.85 },
    { "frame": "GeoEco", "value": -1, "phase": 0.10 },
    { "frame": "Developmental", "value": 1, "phase": 0.90 }
  ],
  "projected_signals_2066": {
    "co2_ppm": 720,
    "global_temp_anomaly_c": 5.0,
    "biodiversity_intactness": 0.30,
    "renewable_share": 0.25,
    "population_billion": 8.0
  },
  "expected_behavior": "commit_false"
}
```

- [ ] **Step 6: Verify scenario files parse as valid ScenarioInput**

```bash
cargo test --test sandbox_test all_scenarios_match_expected_behavior -- --test-threads=2
```
Expected: test passes (new SSP scenarios are validated).

- [ ] **Step 7: Commit**

```bash
git add scenarios/ssp/
git commit -m "feat(scenarios): add 5 SSP pathway scenario templates (SSP1-SSP5)"
```

---

### Task 2: RetrospectiveDoc type + RetrospectiveProvider skeleton

**Files:**
- Create: `aurora/src/percept/retrospective.rs`

**Interfaces:**
- Produces: `RetrospectiveDoc` struct, `RetrospectiveProvider` struct
- Consumes: `ExternalPercept` trait, `PerceptBatch`, `PerceptError` (existing)

- [ ] **Step 1: Create the module**

`aurora/src/percept/retrospective.rs`:

```rust
//! Future Retrospective Simulator — SSP-based "historical review" generation.
//!
//! This provider takes a decision description and an SSP pathway, then
//! generates a document written from the perspective of a historian in
//! 2066, looking back at how today's decision shaped the subsequent 50 years.
//!
//! It implements [`ExternalPercept`] so it plugs into the existing
//! [`PerceptChain`] degradation model without new infrastructure.

use crate::percept::{ExternalPercept, PerceptBatch, PerceptError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A "historical review document" written from the perspective of 2066.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrospectiveDoc {
    /// Which SSP pathway this retrospective was generated under.
    pub ssp_pathway: String,
    /// The lookback year (always 2066 for the default simulator).
    pub lookback_year: u16,
    /// The decision being evaluated.
    pub decision_summary: String,
    /// The historian's narrative — what happened between now and 2066.
    pub narrative: String,
    /// Key turning points identified by the historian.
    pub turning_points: Vec<String>,
    /// How future generations evaluate this decision (positive/negative/mixed).
    pub posterity_verdict: String,
    /// Confidence in the narrative [0.0, 1.0].
    pub confidence: f64,
    /// Generation timestamp.
    pub generated_at: DateTime<Utc>,
}

impl RetrospectiveDoc {
    /// Create a placeholder retrospective (used when LLM is unavailable).
    pub fn placeholder(ssp: &str, decision: &str) -> Self {
        Self {
            ssp_pathway: ssp.to_string(),
            lookback_year: 2066,
            decision_summary: decision.to_string(),
            narrative: format!(
                "From the year 2066, under the {} pathway, the full consequences \
                 of this decision are still unfolding. Historical analysis requires \
                 perspective that only time can provide. What is clear is that \
                 decisions made in the 2020s created path dependencies that shaped \
                 the subsequent five decades in ways both foreseen and surprising.",
                ssp
            ),
            turning_points: vec![
                "The decision itself (2026)".into(),
                "Mid-century inflection point (2050)".into(),
            ],
            posterity_verdict: "mixed — full assessment pending".into(),
            confidence: 0.3,
            generated_at: Utc::now(),
        }
    }
}

/// SSP pathway specification for the retrospective provider.
#[derive(Debug, Clone)]
pub struct SspPathway {
    pub name: String,
    pub description: String,
    pub projected_co2_ppm_2066: f64,
    pub projected_temp_anomaly_c_2066: f64,
    pub projected_biodiversity_2066: f64,
}

impl SspPathway {
    /// All five SSP pathways with key 2066 projections.
    pub fn all() -> Vec<Self> {
        vec![
            SspPathway {
                name: "SSP1".into(),
                description: "Sustainability — Taking the Green Road".into(),
                projected_co2_ppm_2066: 440.0,
                projected_temp_anomaly_c_2066: 2.0,
                projected_biodiversity_2066: 0.72,
            },
            SspPathway {
                name: "SSP2".into(),
                description: "Middle of the Road".into(),
                projected_co2_ppm_2066: 520.0,
                projected_temp_anomaly_c_2066: 3.0,
                projected_biodiversity_2066: 0.55,
            },
            SspPathway {
                name: "SSP3".into(),
                description: "Regional Rivalry — A Rocky Road".into(),
                projected_co2_ppm_2066: 620.0,
                projected_temp_anomaly_c_2066: 4.5,
                projected_biodiversity_2066: 0.38,
            },
            SspPathway {
                name: "SSP4".into(),
                description: "Inequality — A Road Divided".into(),
                projected_co2_ppm_2066: 560.0,
                projected_temp_anomaly_c_2066: 3.5,
                projected_biodiversity_2066: 0.45,
            },
            SspPathway {
                name: "SSP5".into(),
                description: "Fossil-fueled Development — Taking the Highway".into(),
                projected_co2_ppm_2066: 720.0,
                projected_temp_anomaly_c_2066: 5.0,
                projected_biodiversity_2066: 0.30,
            },
        ]
    }
}

/// Perception provider that generates "historical review" documents.
///
/// This provider wraps an existing LLM provider (e.g., CloudLLMProvider)
/// and uses it to generate retrospective narratives. When no LLM is
/// available, it returns a placeholder document — it never fails.
pub struct RetrospectiveProvider {
    ssp: SspPathway,
    decision_prompt: String,
}

impl RetrospectiveProvider {
    /// Create a new retrospective provider for a specific SSP pathway.
    pub fn new(ssp: SspPathway, decision_prompt: impl Into<String>) -> Self {
        Self {
            ssp,
            decision_prompt: decision_prompt.into(),
        }
    }

    /// Build a system prompt for the LLM that instructs it to write
    /// as a 2066 historian.
    fn build_system_prompt(&self) -> String {
        format!(
            include_str!("prompts/retrospective_system.txt"),
            ssp_name = self.ssp.name,
            ssp_description = self.ssp.description,
            co2_ppm = self.ssp.projected_co2_ppm_2066,
            temp_anomaly = self.ssp.projected_temp_anomaly_c_2066,
            biodiversity = self.ssp.projected_biodiversity_2066,
            decision = self.decision_prompt,
        )
    }

    /// Generate a placeholder retrospective without calling an LLM.
    pub fn generate_placeholder(&self) -> RetrospectiveDoc {
        RetrospectiveDoc::placeholder(&self.ssp.name, &self.decision_prompt)
    }
}

impl ExternalPercept for RetrospectiveProvider {
    fn perceive(&self, _raw: &str) -> Result<PerceptBatch, PerceptError> {
        // The retrospective provider doesn't produce TritWord signals.
        // It produces a RetrospectiveDoc. For the PerceptChain interface,
        // we return an empty batch — the actual retrospective generation
        // is invoked via `generate_placeholder()` or a future LLM call.
        Ok(PerceptBatch::empty("retrospective-ssp"))
    }

    fn provider_name(&self) -> &str {
        "retrospective-ssp"
    }

    fn priority(&self) -> u8 {
        3 // lower priority than FFT provider (2), always last resort
    }

    fn available(&self) -> bool {
        true // placeholder is always available
    }
}
```

- [ ] **Step 2: Build**

```bash
cargo build -p aurora 2>&1 | tail -5
```
Expected: `Finished`

- [ ] **Step 3: Commit**

```bash
git add aurora/src/percept/retrospective.rs
git commit -m "feat(aurora): add RetrospectiveDoc + RetrospectiveProvider skeleton"
```

---

### Task 3: Retrospective system prompt

**Files:**
- Create: `aurora/src/percept/prompts/retrospective_system.txt`

**Interfaces:**
- Consumed by: `RetrospectiveProvider::build_system_prompt()` (Task 2)

- [ ] **Step 1: Create the system prompt template**

`aurora/src/percept/prompts/retrospective_system.txt`:

```
You are a historian writing in the year 2066. You are composing a brief "historical review document" that looks back at a decision made in 2026 — 40 years ago.

You are writing under the {ssp_name} ({ssp_description}) socioeconomic pathway. In this timeline:

- Atmospheric CO₂ concentration reached {co2_ppm} ppm by 2066
- Global mean temperature anomaly reached +{temp_anomaly}°C above pre-industrial levels
- Biodiversity Intactness Index stands at {biodiversity} (1.0 = intact, 0.0 = complete loss)

The decision you are reviewing:

"{decision}"

Write your historical review in the following JSON format ONLY — no preamble, no commentary outside the JSON:

{{
  "narrative": "A 2-3 paragraph historical account of how this decision shaped the subsequent four decades. Write in the past tense, as a historian looking backward. Be specific about causal chains: 'Because this decision was made, X happened, which led to Y, and by 2066 the result was Z.'",
  "turning_points": ["Key event 1 (year)", "Key event 2 (year)", "Key event 3 (year)"],
  "posterity_verdict": "One of: 'positive' (future generations view this decision favorably), 'negative' (viewed as a mistake), 'mixed' (complex legacy)"
}}

Rules:
1. Do not moralize. State causal chains, not judgments.
2. Be specific about mechanisms — HOW did the decision lead to the outcomes?
3. Include both intended and unintended consequences.
4. The posterity_verdict must be exactly one of: "positive", "negative", or "mixed"
5. Output ONLY the JSON object. No markdown, no explanation.
```

- [ ] **Step 2: Verify the file is valid UTF-8 and compiles as an include_str! target**

```bash
cargo build -p aurora 2>&1 | tail -5
```
Expected: `Finished` (include_str! is checked at compile time; if the path is wrong, compilation fails).

- [ ] **Step 3: Commit**

```bash
git add aurora/src/percept/prompts/retrospective_system.txt
git commit -m "feat(aurora): add retrospective historian system prompt template"
```

---

### Task 4: Wire RetrospectiveProvider into percept module

**Files:**
- Modify: `aurora/src/percept/mod.rs`

**Interfaces:**
- Consumes: `RetrospectiveProvider`, `RetrospectiveDoc`, `SspPathway` (Task 2)

- [ ] **Step 1: Add module declaration and re-exports**

In `aurora/src/percept/mod.rs`, add after `pub mod local;`:

```rust
pub mod retrospective;
```

Add to the `pub use` block after `pub use local::LocalLLMProvider;`:

```rust
pub use retrospective::{RetrospectiveDoc, RetrospectiveProvider, SspPathway};
```

- [ ] **Step 2: Build**

```bash
cargo build -p aurora 2>&1 | tail -5
```
Expected: `Finished`

- [ ] **Step 3: Commit**

```bash
git add aurora/src/percept/mod.rs
git commit -m "feat(aurora): export RetrospectiveProvider and RetrospectiveDoc from percept module"
```

---

### Task 5: Unit tests for RetrospectiveDoc and SspPathway

**Files:**
- Modify: `aurora/src/percept/retrospective.rs` (append `#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `RetrospectiveDoc`, `SspPathway`, `RetrospectiveProvider` (Task 2)

- [ ] **Step 1: Add tests**

Append to `aurora/src/percept/retrospective.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_contains_ssp_name() {
        let doc = RetrospectiveDoc::placeholder("SSP1", "Build a bridge");
        assert_eq!(doc.ssp_pathway, "SSP1");
        assert_eq!(doc.lookback_year, 2066);
        assert!(doc.narrative.contains("SSP1"));
        assert!(doc.narrative.contains("bridge") || doc.decision_summary.contains("bridge"));
    }

    #[test]
    fn placeholder_confidence_is_low() {
        let doc = RetrospectiveDoc::placeholder("SSP5", "Drill for oil");
        assert!(doc.confidence < 0.5);
    }

    #[test]
    fn placeholder_has_turning_points() {
        let doc = RetrospectiveDoc::placeholder("SSP2", "Tax carbon");
        assert!(!doc.turning_points.is_empty());
    }

    #[test]
    fn posterity_verdict_is_valid() {
        let doc = RetrospectiveDoc::placeholder("SSP3", "Build coal plant");
        assert!(doc.posterity_verdict.contains("mixed"));
    }

    #[test]
    fn all_five_ssp_pathways_exist() {
        let pathways = SspPathway::all();
        assert_eq!(pathways.len(), 5);
        let names: Vec<&str> = pathways.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"SSP1"));
        assert!(names.contains(&"SSP5"));
    }

    #[test]
    fn ssp_projections_are_in_expected_ranges() {
        for ssp in SspPathway::all() {
            assert!(ssp.projected_co2_ppm_2066 > 400.0 && ssp.projected_co2_ppm_2066 < 800.0,
                "{} co2_ppm {} out of range", ssp.name, ssp.projected_co2_ppm_2066);
            assert!(ssp.projected_temp_anomaly_c_2066 > 0.0 && ssp.projected_temp_anomaly_c_2066 < 7.0,
                "{} temp {} out of range", ssp.name, ssp.projected_temp_anomaly_c_2066);
            assert!(ssp.projected_biodiversity_2066 > 0.0 && ssp.projected_biodiversity_2066 < 1.0,
                "{} biodiversity {} out of range", ssp.name, ssp.projected_biodiversity_2066);
        }
    }

    #[test]
    fn provider_is_always_available() {
        let ssp = SspPathway::all().into_iter().next().unwrap();
        let provider = RetrospectiveProvider::new(ssp, "test decision");
        assert!(provider.available());
    }

    #[test]
    fn provider_returns_empty_batch() {
        let ssp = SspPathway::all().into_iter().next().unwrap();
        let provider = RetrospectiveProvider::new(ssp, "test decision");
        let batch = provider.perceive("irrelevant").unwrap();
        assert!(batch.signals.is_empty());
        assert_eq!(batch.source, "retrospective-ssp");
    }

    #[test]
    fn provider_has_lowest_priority() {
        let ssp = SspPathway::all().into_iter().next().unwrap();
        let provider = RetrospectiveProvider::new(ssp, "test");
        assert_eq!(provider.priority(), 3); // lower than FFTProvider (2)
    }

    #[test]
    fn generate_placeholder_is_deterministic() {
        let ssp = SspPathway::all().into_iter().next().unwrap();
        let provider = RetrospectiveProvider::new(ssp, "Build a dam");
        let doc1 = provider.generate_placeholder();
        let doc2 = provider.generate_placeholder();
        assert_eq!(doc1.narrative, doc2.narrative);
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test retrospective -- --test-threads=2
```
Expected: 10 tests pass.

- [ ] **Step 3: Run full test suite**

```bash
cargo test --workspace --all-features -- --test-threads=2 2>&1 | grep "FAILED"
```
Expected: no output.

- [ ] **Step 4: Commit**

```bash
git add aurora/src/percept/retrospective.rs
git commit -m "test(aurora): add 10 unit tests for RetrospectiveDoc, SspPathway, RetrospectiveProvider"
```

---

### Task 6: Tauri command + frontend type

**Files:**
- Modify: `src-tauri/src/commands.rs` — add `run_retrospective` command + response types
- Modify: `ui/src/types.ts` — add `RetrospectiveResponse` interface

**Interfaces:**
- Consumes: `RetrospectiveDoc`, `SspPathway`, `RetrospectiveProvider` (Task 4)
- Produces: `RetrospectiveResponse` (consumed by future UI component)

- [ ] **Step 1: Add Tauri command to commands.rs**

Append to `src-tauri/src/commands.rs`:

```rust
/// Serializable retrospective document for the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct RetrospectiveResponse {
    pub ssp_pathway: String,
    pub lookback_year: u16,
    pub decision_summary: String,
    pub narrative: String,
    pub turning_points: Vec<String>,
    pub posterity_verdict: String,
    pub confidence: f64,
}

impl From<aurora::percept::RetrospectiveDoc> for RetrospectiveResponse {
    fn from(doc: aurora::percept::RetrospectiveDoc) -> Self {
        Self {
            ssp_pathway: doc.ssp_pathway,
            lookback_year: doc.lookback_year,
            decision_summary: doc.decision_summary,
            narrative: doc.narrative,
            turning_points: doc.turning_points,
            posterity_verdict: doc.posterity_verdict,
            confidence: doc.confidence,
        }
    }
}

/// Generate a "2066 historical review" for a decision under a specific SSP pathway.
///
/// Called from the frontend via:
///   invoke('run_retrospective', { decision: string, sspPathway: string })
///
/// Returns a placeholder retrospective (LLM-powered generation is gated on
/// cloud/local LLM availability in a future iteration).
#[tauri::command]
pub fn run_retrospective(decision: String, ssp_pathway: String) -> Result<RetrospectiveResponse, String> {
    use aurora::percept::{RetrospectiveProvider, SspPathway};

    let ssp = SspPathway::all()
        .into_iter()
        .find(|s| s.name == ssp_pathway)
        .ok_or_else(|| format!("unknown SSP pathway: '{}' (expected SSP1–SSP5)", ssp_pathway))?;

    let provider = RetrospectiveProvider::new(ssp, &decision);
    let doc = provider.generate_placeholder();

    crate::logger::log(
        "retrospective",
        "INFO",
        &format!(
            "generated {} retrospective for: {}",
            doc.ssp_pathway, decision
        ),
    );

    Ok(RetrospectiveResponse::from(doc))
}
```

- [ ] **Step 2: Add TypeScript type to ui/src/types.ts**

Append to `ui/src/types.ts`:

```typescript
/** 2066 historical review document — from the Retrospective Simulator. */
export interface RetrospectiveResponse {
  ssp_pathway: string;
  lookback_year: number;
  decision_summary: string;
  narrative: string;
  turning_points: string[];
  posterity_verdict: string;
  confidence: number;
}
```

- [ ] **Step 3: Build full workspace**

```bash
cargo build --workspace 2>&1 | tail -5
```
Expected: `Finished`

- [ ] **Step 4: Run full test suite**

```bash
cargo test --workspace --all-features -- --test-threads=2 2>&1 | grep "FAILED"
```
Expected: no output.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs ui/src/types.ts
git commit -m "feat: add run_retrospective Tauri command + RetrospectiveResponse TS type"
```

---

## Lever 2 Completion Checklist

- [x] Task 1: 5 SSP scenario templates (SSP1–SSP5)
- [x] Task 2: `RetrospectiveDoc` + `RetrospectiveProvider` + `SspPathway`
- [x] Task 3: LLM system prompt template
- [x] Task 4: Wire into percept module exports
- [x] Task 5: 10 unit tests
- [x] Task 6: Tauri command + frontend type

**Post-completion:** The RetrospectiveProvider currently returns placeholders. LLM-powered generation (Task 2's `build_system_prompt()` is ready) can be activated by wrapping an existing LLM provider — no new infrastructure needed.
