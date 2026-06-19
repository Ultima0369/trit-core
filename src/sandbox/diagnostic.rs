use crate::core::word::TritWord;
use crate::meta::{ArbitrationResult, MetaInterrupt};
use serde::Serialize;
use std::collections::HashMap;
use std::time::Instant;

/// Per-stage timing and counters for a single pipeline run.
#[derive(Debug, Clone, Default, Serialize)]
pub struct SandboxDiagnostics {
    /// When the pipeline run started.
    #[serde(with = "serde_millis")]
    pub started_at: Option<Instant>,
    /// Total elapsed time for the run, in nanoseconds.
    pub elapsed_ns: u64,
    /// Number of input signals.
    pub signal_count: usize,
    /// Distribution of input signals by frame.
    pub frame_distribution: HashMap<String, usize>,
    /// Number of cross-frame interrupts detected.
    pub interrupt_count: usize,
    /// Types of interrupts observed.
    pub interrupt_types: Vec<String>,
    /// The actual interrupt events (for output construction).
    #[serde(skip)]
    pub interrupts: Vec<MetaInterrupt>,
    /// Name of the policy action taken.
    pub policy_action: String,
    /// Whether SafeFallback was triggered.
    pub safe_fallback_triggered: bool,
    /// Per-stage timing in nanoseconds.
    pub stage_timings_ns: HashMap<String, u64>,
    /// Whether the reflexive guard triggered an alert.
    pub reflexive_guard_triggered: bool,
    /// Optional attention command produced during the run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attention_cmd: Option<String>,
    /// Optional receiver estimate produced by self-knowledge.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receiver_estimate: Option<crate::knowledge::ReceiverEstimate>,
    /// Optional phase shift trace (when --trace-phase is enabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase_trace: Option<Vec<f64>>,
}

impl SandboxDiagnostics {
    /// Create a new diagnostics collector and record the start time.
    pub fn new() -> Self {
        Self {
            started_at: Some(Instant::now()),
            ..Default::default()
        }
    }

    /// Record timing for a named stage.
    pub fn record_stage(&mut self, name: &str, start: Instant) {
        let elapsed = start.elapsed().as_nanos() as u64;
        self.stage_timings_ns.insert(name.to_string(), elapsed);
    }

    /// Record input signal distribution.
    pub fn record_inputs(&mut self, inputs: &[TritWord]) {
        self.signal_count = inputs.len();
        for word in inputs {
            *self
                .frame_distribution
                .entry(format!("{}", word.frame()))
                .or_insert(0) += 1;
        }
    }

    /// Record interrupt summary.
    pub fn record_interrupts(&mut self, interrupts: &[MetaInterrupt]) {
        self.interrupt_count = interrupts.len();
        self.interrupt_types = interrupts
            .iter()
            .map(|i| format!("{:?}", i.conflict))
            .collect();
    }

    /// Record the policy action.
    pub fn record_policy_action(&mut self, action: &ArbitrationResult) {
        self.policy_action = format!("{}", action);
    }

    /// Mark SafeFallback as triggered.
    pub fn mark_safe_fallback(&mut self) {
        self.safe_fallback_triggered = true;
    }

    /// Mark the reflexive guard as triggered.
    pub fn mark_reflexive_guard(&mut self) {
        self.reflexive_guard_triggered = true;
    }

    /// Record an attention command.
    pub fn record_attention_cmd(&mut self, cmd: &crate::attention::AttentionCmd) {
        self.attention_cmd = Some(format!("{:?}", cmd));
    }

    /// Record a receiver estimate.
    pub fn record_receiver_estimate(&mut self, estimate: crate::knowledge::ReceiverEstimate) {
        self.receiver_estimate = Some(estimate);
    }

    /// Record a phase value in the phase trace.
    pub fn record_phase(&mut self, phase: f64) {
        self.phase_trace
            .get_or_insert_with(Vec::new)
            .push(phase.clamp(0.0, 1.0));
    }

    /// Total elapsed time in microseconds.
    pub fn elapsed_us(&self) -> u64 {
        self.elapsed_ns / 1_000
    }

    /// Finalize timing.
    pub fn finish(&mut self) {
        if let Some(start) = self.started_at {
            self.elapsed_ns = start.elapsed().as_nanos() as u64;
        }
    }

    /// Human-readable summary.
    pub fn summary(&self) -> String {
        format!(
            "signals={}, interrupts={}, fallback={}, elapsed={}µs",
            self.signal_count,
            self.interrupt_count,
            self.safe_fallback_triggered,
            self.elapsed_us()
        )
    }
}

mod serde_millis {
    use serde::{Deserializer, Serializer};
    use std::time::{Duration, Instant, SystemTime};

    pub fn serialize<S: Serializer>(
        instant: &Option<Instant>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        if let Some(inst) = instant {
            // Approximate: elapsed since instant mapped to system time
            let now = SystemTime::now();
            let approx = now - inst.elapsed();
            let millis = approx
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or(Duration::ZERO)
                .as_millis() as u64;
            serializer.serialize_some(&millis)
        } else {
            serializer.serialize_none()
        }
    }

    #[allow(dead_code)]
    pub fn deserialize<'de, D: Deserializer<'de>>(
        _deserializer: D,
    ) -> Result<Option<Instant>, D::Error> {
        // Instant cannot be deserialized; callers should treat diagnostics as output-only.
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::frame::Frame;
    use crate::core::word::TritWord;

    #[test]
    fn diagnostics_records_inputs_and_interrupts() {
        let mut diag = SandboxDiagnostics::new();
        let inputs = vec![
            TritWord::tru(Frame::Science),
            TritWord::fals(Frame::Individual),
        ];
        diag.record_inputs(&inputs);
        assert_eq!(diag.signal_count, 2);
        assert_eq!(diag.frame_distribution.get("Science"), Some(&1));
        assert_eq!(diag.frame_distribution.get("Individual"), Some(&1));
    }

    #[test]
    fn diagnostics_stage_timing_is_monotonic() {
        let mut diag = SandboxDiagnostics::new();
        let start = Instant::now();
        std::thread::sleep(std::time::Duration::from_millis(1));
        diag.record_stage("test", start);
        assert!(diag.stage_timings_ns.get("test").copied().unwrap_or(0) > 0);
    }
}
