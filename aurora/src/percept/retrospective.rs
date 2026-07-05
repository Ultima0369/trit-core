//! Future Retrospective Simulator — Lever 2 of the three-lever mechanism.
//!
//! The `RetrospectiveProvider` wraps an inner [`ExternalPercept`] (typically a
//! cloud LLM) and enriches raw user input with a future-retrospective framing
//! prompt. The LLM is asked to generate signals from the perspective of a
//! specified lookback year (default: 2066), producing a `PerceptBatch` where
//! signals are anchored in the selected SSP pathway's projected world state.
//!
//! ## Design
//!
//! This provider does NOT call an LLM directly. It delegates to an inner
//! `ExternalPercept`, constructing a system prompt that embeds the SSP
//! scenario context before the user's raw input. The inner provider handles
//! the actual API call.
//!
//! ## Architecture
//!
//! ```text
//! User text → RetrospectiveProvider
//!   → formats: SSP context + decision_prompt + user text
//!   → delegates to inner ExternalPercept (cloud/local LLM)
//!   → parses response → PerceptBatch
//! ```

use crate::percept::{ExternalPercept, PerceptBatch, PerceptError};
use serde::{Deserialize, Serialize};

/// A single SSP pathway scenario loaded from `scenarios/ssp/`.
#[derive(Debug, Clone, Deserialize)]
pub struct SspScenario {
    pub id: String,
    pub description: String,
    pub domain: String,
    pub ssp_pathway: String,
    pub lookback_year: u16,
    pub decision_prompt: String,
    #[serde(default)]
    pub projected_signals_2066: Option<ProjectedSignals>,
}

/// Projected physical quantities for the lookback year.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProjectedSignals {
    pub co2_ppm: f64,
    pub global_temp_anomaly_c: f64,
    pub biodiversity_intactness: f64,
    pub renewable_share: f64,
    pub population_billion: f64,
}

/// The retrospective document produced after perception.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RetrospectiveDoc {
    /// The SSP pathway used.
    pub pathway: String,
    /// The lookback year.
    pub lookback_year: u16,
    /// The decision prompt that framed the retrospective.
    pub decision_prompt: String,
    /// Signals produced by the inner percept.
    pub signal_count: usize,
    /// Provider that generated the signals.
    pub source: String,
    /// Confidence from the inner percept.
    pub confidence: f64,
    /// The projected 2066 physical state.
    pub projected: Option<ProjectedSignals>,
}

/// Retrospective perception provider — Lever 2.
///
/// Wraps an inner [`ExternalPercept`] (typically a cloud LLM) and enriches
/// raw input with future-retrospective framing before delegating.
pub struct RetrospectiveProvider {
    inner: Box<dyn ExternalPercept>,
    scenario: SspScenario,
}

impl RetrospectiveProvider {
    /// Create a new retrospective provider.
    ///
    /// `inner` is the perception provider to delegate to (cloud or local LLM).
    /// `scenario` is the SSP pathway scenario loaded from JSON.
    pub fn new(inner: Box<dyn ExternalPercept>, scenario: SspScenario) -> Self {
        Self { inner, scenario }
    }

    /// Build the retrospective prompt for the inner LLM.
    fn build_prompt(&self, raw: &str) -> String {
        let projected = self
            .scenario
            .projected_signals_2066
            .as_ref()
            .map(|p| {
                format!(
                    "Projected {year} physical state:\n\
                     - CO2: {co2} ppm\n\
                     - Global temperature anomaly: +{temp}°C\n\
                     - Biodiversity intactness index: {bio}\n\
                     - Renewable energy share: {ren}%\n\
                     - Global population: {pop} billion",
                    year = self.scenario.lookback_year,
                    co2 = p.co2_ppm,
                    temp = p.global_temp_anomaly_c,
                    bio = p.biodiversity_intactness,
                    ren = p.renewable_share * 100.0,
                    pop = p.population_billion
                )
            })
            .unwrap_or_default();

        format!(
            "You are a retrospective analyst writing from the year {year}.\n\
             SSP pathway: {ssp}\n\
             Scenario: {desc}\n\
             {projected}\n\n\
             Context: {prompt}\n\n\
             From the perspective of {year}, analyze the following decision and \
             extract ternary signals (Frame, Value, Phase) that a future observer \
             would perceive about today's choice. Return ONLY structured signals, \
             no narrative.\n\n\
             User input: {raw}",
            year = self.scenario.lookback_year,
            ssp = self.scenario.ssp_pathway,
            desc = self.scenario.description,
            projected = projected,
            prompt = self.scenario.decision_prompt,
        )
    }

    /// Load an SSP scenario from a JSON file path.
    pub fn load_scenario(path: &std::path::Path) -> Result<SspScenario, PerceptError> {
        let data = std::fs::read_to_string(path)
            .map_err(|e| PerceptError::ParseError(format!("read SSP scenario: {e}")))?;
        serde_json::from_str(&data)
            .map_err(|e| PerceptError::ParseError(format!("parse SSP scenario: {e}")))
    }

    /// The loaded SSP scenario (for inspection / serialization).
    pub fn scenario(&self) -> &SspScenario {
        &self.scenario
    }

    /// Produce a RetrospectiveDoc from a completed PerceptBatch.
    pub fn to_doc(&self, batch: &PerceptBatch) -> RetrospectiveDoc {
        RetrospectiveDoc {
            pathway: self.scenario.ssp_pathway.clone(),
            lookback_year: self.scenario.lookback_year,
            decision_prompt: self.scenario.decision_prompt.clone(),
            signal_count: batch.signals.len(),
            source: batch.source.clone(),
            confidence: batch.confidence,
            projected: self.scenario.projected_signals_2066.clone(),
        }
    }
}

impl ExternalPercept for RetrospectiveProvider {
    fn perceive(&self, raw: &str) -> Result<PerceptBatch, PerceptError> {
        let prompt = self.build_prompt(raw);
        self.inner.perceive(&prompt)
    }

    fn provider_name(&self) -> &str {
        "retrospective"
    }

    fn priority(&self) -> u8 {
        // Same priority as the inner provider, since we delegate to it.
        self.inner.priority()
    }

    fn available(&self) -> bool {
        self.inner.available()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_scenario() -> SspScenario {
        SspScenario {
            id: "ssp1_test".into(),
            description: "Test SSP1 scenario".into(),
            domain: "Environmental".into(),
            ssp_pathway: "SSP1".into(),
            lookback_year: 2066,
            decision_prompt: "Consider a test decision.".into(),
            projected_signals_2066: Some(ProjectedSignals {
                co2_ppm: 440.0,
                global_temp_anomaly_c: 2.0,
                biodiversity_intactness: 0.72,
                renewable_share: 0.85,
                population_billion: 8.5,
            }),
        }
    }

    #[test]
    fn build_prompt_includes_ssp_context() {
        let scenario = test_scenario();
        let inner = crate::percept::fft::FFTProvider::new(
            crate::pipeline::analysis::SignalSpec {
                freq: 2.0,
                sample_rate: 100.0,
                duration_secs: 1.0,
                noise_std: 0.0,
            },
        );
        let provider = RetrospectiveProvider::new(Box::new(inner), scenario);
        let prompt = provider.build_prompt("test input");

        assert!(prompt.contains("2066"));
        assert!(prompt.contains("SSP1"));
        assert!(prompt.contains("440 ppm"));
        assert!(prompt.contains("+2°C"));
        assert!(prompt.contains("test input"));
    }

    #[test]
    fn load_scenario_from_valid_json() {
        let json = r#"{
            "id": "ssp1",
            "description": "test",
            "domain": "Environmental",
            "ssp_pathway": "SSP1",
            "lookback_year": 2066,
            "decision_prompt": "test prompt",
            "projected_signals_2066": {
                "co2_ppm": 440.0,
                "global_temp_anomaly_c": 2.0,
                "biodiversity_intactness": 0.72,
                "renewable_share": 0.85,
                "population_billion": 8.5
            }
        }"#;
        let scenario: SspScenario = serde_json::from_str(json).unwrap();
        assert_eq!(scenario.ssp_pathway, "SSP1");
        assert_eq!(scenario.lookback_year, 2066);
    }

    #[test]
    fn load_scenario_without_projected_signals() {
        let json = r#"{
            "id": "ssp1",
            "description": "test",
            "domain": "Environmental",
            "ssp_pathway": "SSP1",
            "lookback_year": 2066,
            "decision_prompt": "test prompt"
        }"#;
        let scenario: SspScenario = serde_json::from_str(json).unwrap();
        assert!(scenario.projected_signals_2066.is_none());
    }

    #[test]
    fn to_doc_captures_scenario_metadata() {
        let scenario = test_scenario();
        let inner = crate::percept::fft::FFTProvider::new(
            crate::pipeline::analysis::SignalSpec {
                freq: 2.0,
                sample_rate: 100.0,
                duration_secs: 1.0,
                noise_std: 0.0,
            },
        );
        let provider = RetrospectiveProvider::new(Box::new(inner), scenario);
        let batch = PerceptBatch::empty("test-source");

        let doc = provider.to_doc(&batch);
        assert_eq!(doc.pathway, "SSP1");
        assert_eq!(doc.lookback_year, 2066);
        assert_eq!(doc.source, "test-source");
        assert!(doc.projected.is_some());
    }

    #[test]
    fn provider_name_is_retrospective() {
        let scenario = test_scenario();
        let inner = crate::percept::fft::FFTProvider::new(
            crate::pipeline::analysis::SignalSpec {
                freq: 2.0,
                sample_rate: 100.0,
                duration_secs: 1.0,
                noise_std: 0.0,
            },
        );
        let provider = RetrospectiveProvider::new(Box::new(inner), scenario);
        assert_eq!(provider.provider_name(), "retrospective");
    }

    #[test]
    fn priority_delegates_to_inner() {
        let scenario = test_scenario();
        let inner = crate::percept::fft::FFTProvider::new(
            crate::pipeline::analysis::SignalSpec {
                freq: 2.0,
                sample_rate: 100.0,
                duration_secs: 1.0,
                noise_std: 0.0,
            },
        );
        let provider = RetrospectiveProvider::new(Box::new(inner), scenario);
        // FFTProvider has priority 2
        assert_eq!(provider.priority(), 2);
    }

    #[test]
    fn available_delegates_to_inner() {
        let scenario = test_scenario();
        let inner = crate::percept::fft::FFTProvider::new(
            crate::pipeline::analysis::SignalSpec {
                freq: 2.0,
                sample_rate: 100.0,
                duration_secs: 1.0,
                noise_std: 0.0,
            },
        );
        let provider = RetrospectiveProvider::new(Box::new(inner), scenario);
        assert!(provider.available());
    }
}
