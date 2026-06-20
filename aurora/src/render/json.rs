//! JSON rendering for decision reports.

use crate::pipeline::DecisionReport;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct JsonReport {
    pub input_frequency_hz: f64,
    pub detected_frequency_hz: f64,
    pub embodied_value: String,
    pub embodied_frame: String,
    pub individual_value: String,
    pub individual_frame: String,
    pub result_value: String,
    pub result_frame: String,
    pub conflict_detected: bool,
    pub conflict_type: Option<String>,
    pub conflict_reason: Option<String>,
    pub asi: f64,
    pub reminder_count: usize,
}

impl From<&DecisionReport> for JsonReport {
    fn from(r: &DecisionReport) -> Self {
        Self {
            input_frequency_hz: r.input_freq,
            detected_frequency_hz: r.detected_freq,
            embodied_value: format!("{:?}", r.embodied.value()),
            embodied_frame: r.embodied.frame().to_string(),
            individual_value: format!("{:?}", r.individual.value()),
            individual_frame: r.individual.frame().to_string(),
            result_value: format!("{:?}", r.result.value()),
            result_frame: r.result.frame().to_string(),
            conflict_detected: r.interrupt.is_some(),
            conflict_type: r.interrupt.as_ref().map(|i| format!("{:?}", i.conflict)),
            conflict_reason: r.interrupt.as_ref().map(|i| i.reason.clone()),
            asi: r.asi,
            reminder_count: r.reminder_count,
        }
    }
}

pub fn to_string(report: &DecisionReport) -> Result<String, serde_json::Error> {
    let json: JsonReport = report.into();
    serde_json::to_string_pretty(&json)
}
