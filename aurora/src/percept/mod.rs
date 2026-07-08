//! External perception layer — unified abstraction for all perception sources.
//!
//! The `ExternalPercept` trait provides a standard interface for converting
//! raw text input into `PerceptBatch` — a structured set of TritWord signals
//! with metadata. Implementations include cloud LLMs, local models, and
//! the built-in FFT wavelet engine.
//!
//! # 流沙 (Flowing Sands) Philosophy
//!
//! Every perception provider is a **棱镜 (prism)** — it decomposes raw input
//! into independent spectral components without interpreting what they mean.
//!
//! - **璇玑 (Armillary Sphere)**: faithfully rotate, never explain why
//! - **棱镜 (Prism)**: split into spectral bands, never synthesize
//! - **微风 (Breeze)**: pass through, leave no trace
//!
//! The LLM perceives and structures signals. Trit-Core makes ternary decisions.
//! The user observes their own reaction to the data. No one tells anyone what to think.
//!
//! # Architecture
//!
//! ```text
//! PerceptChain (priority-ordered degradation)
//!   ├── CloudLLMProvider  (p=0, Anthropic/OpenAI)
//!   ├── LocalLLMProvider  (p=1, ollama/llama.cpp)
//!   └── FFTProvider       (p=2, never offline)
//! ```

pub mod chain;
pub mod cloud;
pub mod error;
pub mod fft;
pub mod local;
pub(crate) mod openai_format;
pub mod prism;
pub mod retrospective;
pub mod types;

pub use chain::PerceptChain;
pub use cloud::CloudLLMProvider;
pub use error::{ConfigError, PerceptError};
pub use fft::FFTProvider;
pub use local::LocalLLMProvider;
pub use prism::{PrismEngine, SourceProfile, SourceWeights};
pub use retrospective::{RetrospectiveDoc, RetrospectiveProvider, SspScenario};
pub use types::PerceptBatch;
pub use types::SignalSpec;

/// Unified abstraction for all external perception sources.
///
/// Implementations include cloud LLMs, local models, FFT signal
/// analysis, and future hard-science data APIs (ecology, climate, geology).
pub trait ExternalPercept: Send + Sync {
    /// Perceive signals from raw text input.
    fn perceive(&self, raw: &str) -> Result<PerceptBatch, PerceptError>;

    /// Human-readable provider name for audit trails.
    fn provider_name(&self) -> &str;

    /// Lower number = higher priority in the degradation chain.
    fn priority(&self) -> u8;

    /// Whether this provider is currently usable.
    fn available(&self) -> bool;
}
