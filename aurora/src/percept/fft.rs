use crate::percept::{ExternalPercept, PerceptBatch, PerceptError};
use crate::pipeline::analysis::SignalSpec;

/// Pure-local FFT perception provider — the ultimate safety floor.
///
/// This provider ignores raw text input entirely and returns an empty batch.
/// The actual FFT analysis happens in `run_analysis_from_percept()` which
/// reads `SignalSpec` directly. This provider exists to ensure the
/// degradation chain always has a fallback that never fails.
///
/// Priority is set to 2 (lowest) so cloud and local LLMs are always
/// tried first.
pub struct FFTProvider {
    #[allow(dead_code)]
    spec: SignalSpec,
}

impl FFTProvider {
    pub fn new(spec: SignalSpec) -> Self {
        Self { spec }
    }
}

impl ExternalPercept for FFTProvider {
    /// FFTProvider cannot decompose raw text into TritWords — it only works on
    /// numeric time-series via the analysis pipeline. Returning AllUnavailable
    /// allows the perception chain to degrade to structured decomposition.
    fn perceive(&self, _raw: &str) -> Result<PerceptBatch, PerceptError> {
        Err(PerceptError::AllUnavailable)
    }

    fn provider_name(&self) -> &str {
        "fft-wavelet"
    }

    fn priority(&self) -> u8 {
        2
    }

    fn available(&self) -> bool {
        true
    }
}
