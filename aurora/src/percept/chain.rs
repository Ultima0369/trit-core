use crate::percept::{ExternalPercept, PerceptBatch, PerceptError};

/// A priority-ordered chain of perception providers with automatic degradation.
///
/// Providers are tried in ascending priority order (lower number = higher priority).
/// If a provider fails with a [`PerceptError`], the chain degrades to the next
/// available provider. Returns [`PerceptError::AllUnavailable`] only when every
/// provider has been tried and failed.
pub struct PerceptChain {
    providers: Vec<Box<dyn ExternalPercept>>,
}

impl PerceptChain {
    /// Create an empty chain.
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// Add a provider and re-sort by priority.
    pub fn with(mut self, provider: Box<dyn ExternalPercept>) -> Self {
        self.providers.push(provider);
        self.providers.sort_by_key(|p| p.priority());
        self
    }

    /// Try providers in priority order, degrading on failure.
    ///
    /// Skips providers where `available()` returns `false`.
    /// Returns `Err(PerceptError::AllUnavailable)` only if every
    /// provider fails or is unavailable.
    pub fn perceive_or_degrade(&self, raw: &str) -> Result<PerceptBatch, PerceptError> {
        for provider in &self.providers {
            if !provider.available() {
                tracing::debug!(
                    "skipping unavailable provider: {}",
                    provider.provider_name()
                );
                continue;
            }

            match provider.perceive(raw) {
                Ok(batch) => {
                    tracing::info!(
                        source = %batch.source,
                        confidence = batch.confidence,
                        signal_count = batch.signals.len(),
                        "perception succeeded"
                    );
                    return Ok(batch);
                }
                Err(e) => {
                    tracing::warn!(
                        provider = %provider.provider_name(),
                        error = %e,
                        "perception provider failed, degrading"
                    );
                }
            }
        }

        Err(PerceptError::AllUnavailable)
    }
}

impl Default for PerceptChain {
    fn default() -> Self {
        Self::new()
    }
}
