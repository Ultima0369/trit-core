# M2: ExternalPercept — LLM Perception Layer Implementation Plan (v2: 流沙哲学整合)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the unified external perception layer for Aurora — `ExternalPercept` trait, three-tier degradation chain (CloudLLM → LocalLLM → FFT), Windows DPAPI-encrypted API key store, and pipeline integration — with zero changes to trit-core.

**Architecture:** New `aurora/src/percept/` module with `ExternalPercept` trait + `PerceptChain` degradation orchestrator. New `aurora/src/config/` module with DPAPI-backed `ConfigStore`. Three providers: `CloudLLMProvider` (Anthropic/OpenAI HTTP), `LocalLLMProvider` (ollama/llama.cpp localhost), `FFTProvider` (existing wavelet engine, never offline). Pipeline integration via `run_analysis_from_percept()` overload — existing `run_analysis()` unchanged.

**流沙 Philosophy (v2):** LLM is a **棱镜 (prism)** — it decomposes raw text into spectral components (Frame/Value/Phase) without interpreting what they mean. No `summary` (violates 零文字 — 不解释). No `suggested_scenario` (violates 棱镜 — 不引导). Only `raw_data_layer` for physical measurements — the territory, not the map. 璇玑-棱镜-微风 三元心法 embedded in system prompt.

**Tech Stack:** Rust 2021 edition, `reqwest` (Windows schannel TLS), `serde_json`, `thiserror` (already present), `chrono` (already present), `windows-sys` (DPAPI), `wiremock` (dev-only HTTP mocking).

## Global Constraints

- `#![deny(unsafe_code)]` — enforced crate-wide (dpapi.rs has isolated `#![allow(unsafe_code)]`)
- Zero changes to `trit-core` crate (all 5 layers, ternary algebra, adapters, anchors)
- Zero changes to existing `run_analysis()` signature
- Zero changes to `attention` pipeline link
- Zero changes to all bounded contexts (`bc/`)
- Zero changes to SQLite database layer (`db/`)
- Windows-only DPAPI encryption (cross-platform abstraction deferred to M3+)
- `reqwest` must use `default-features = false` with `rustls-tls`
- All new types must implement `Debug` (except `ConfigStore` — security)
- API key values must never appear in log output or `Debug` formatting
- `assert_float_eq!` macro for all `f64` comparisons in tests
- **流沙 constraint**: PerceptBatch must NOT contain `summary` or `suggested_scenario` fields
- **流沙 constraint**: LLM JSON contract must NOT include `reasoning`, `summary`, or `suggested_scenario`
- **流沙 constraint**: `raw_data_layer` must only contain physical measurements, no advice/interpretation

---

### Task 1: Add Dependencies to Cargo.toml

**Files:**
- Modify: `aurora/Cargo.toml`

**Interfaces:**
- Produces: `reqwest`, `windows-sys`, `wiremock`, `uuid` available as dependencies

- [ ] **Step 1: Add new dependencies**

Edit `aurora/Cargo.toml` — add under `[dependencies]`, after the existing `thiserror` line:

```toml
# M2: HTTP client for cloud/local LLM providers (Windows native TLS)
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
```

Add under `[target.'cfg(windows)'.dependencies]` (new section before `[dev-dependencies]`):

```toml
[target.'cfg(windows)'.dependencies]
windows-sys = { version = "0.59", features = ["Win32_Security_Cryptography", "Win32_Foundation"] }
```

Add under `[dev-dependencies]`, after the existing `proptest` line:

```toml
# M2: HTTP mocking for cloud LLM provider tests
wiremock = "0.6"
# M2: unique temp dirs for config tests
uuid = { version = "1.0", features = ["v4"] }
```

- [ ] **Step 2: Verify dependency resolution**

```bash
cargo check -p aurora 2>&1 | tail -5
```

Expected: `Finished` (no errors).

- [ ] **Step 3: Commit**

```bash
git add aurora/Cargo.toml
git commit -m "build: add reqwest, windows-sys, wiremock, uuid for M2 perception layer

Co-Authored-By: Claude <noreply@anthropic.com>"
```


---

### Task 2: Percept Types and Error Module

**Files:**
- Create: `aurora/src/percept/mod.rs`
- Create: `aurora/src/percept/types.rs`
- Create: `aurora/src/percept/error.rs`

**Interfaces:**
- Produces: `PerceptBatch` struct, `PerceptError` enum, `ConfigError` enum, `ExternalPercept` trait
- Consumes: `TritWord` (from trit-core), `ScenarioType` (from trit-core::hook), `DateTime<Utc>` (from chrono)

- [ ] **Step 1: Create percept module skeleton**

Write `aurora/src/percept/mod.rs`:

```rust
//! External perception layer — unified abstraction for all perception sources.
//!
//! The `ExternalPercept` trait provides a standard interface for converting
//! raw text input into `PerceptBatch` — a structured set of TritWord signals
//! with metadata. Implementations include cloud LLMs, local models, and
//! the built-in FFT wavelet engine.
//!
//! # Architecture
//!
//! ```text
//! PerceptChain (priority-ordered degradation)
//!   ├── CloudLLMProvider  (p=0, Anthropic/OpenAI)
//!   ├── LocalLLMProvider  (p=1, ollama/llama.cpp)
//!   └── FFTProvider       (p=2, never offline)
//! ```

pub mod types;
pub mod error;
pub mod chain;
pub mod cloud;
pub mod local;
pub mod fft;

pub use types::PerceptBatch;
pub use error::{ConfigError, PerceptError};
pub use chain::PerceptChain;

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
    /// Default: true. Override for health checks.
    fn available(&self) -> bool {
        true
    }
}
```

- [ ] **Step 2: Create PerceptBatch type**

Write `aurora/src/percept/types.rs`:

```rust
use chrono::{DateTime, Utc};
use trit_core::TritWord;

/// A batch of TritWord signals extracted from raw input by a perception provider.
///
/// ## 流沙 (Flowing Sands) Design Philosophy
///
/// This struct embodies three principles:
/// - **璇玑 (Armillary)**: signals are faithful rotations — no meaning attached
/// - **棱镜 (Prism)**: each signal is one spectral band — the user sees what their angle reveals
/// - **微风 (Breeze)**: no summary, no suggestion, no trace — signals pass through and dissolve
///
/// There is deliberately NO `summary` field (violates 零文字 — 不解释).
/// There is deliberately NO `suggested_scenario` field (violates 棱镜 — 不引导).
/// Scenario recognition is Trit-Core's job, not the LLM's.
#[derive(Debug, Clone)]
pub struct PerceptBatch {
    /// Extracted ternary signals — the prismatic decomposition of raw input.
    pub signals: Vec<TritWord>,

    /// Provider name for audit trail (e.g. "claude-opus-4-8").
    pub source: String,

    /// Perception timestamp (UTC).
    pub timestamp: DateTime<Utc>,

    /// Provider-reported confidence, range 0.0–1.0.
    ///
    /// This is a signal-quality marker, not a truth claim.
    /// Trit-Core may override decisions regardless of confidence.
    pub confidence: f64,

    /// Pure physical data layer description (optional).
    ///
    /// When the input contains references to measurable physical quantities
    /// (temperature, wind speed, population density, CO₂ levels, etc.),
    /// this field records those quantities as raw data points.
    ///
    /// MUST NOT contain: advice, interpretation, suggestions, conclusions.
    /// Example: "surface_temp:28.4C wind:12km/h_NE humidity:65%"
    pub raw_data_layer: Option<String>,
}

impl PerceptBatch {
    /// Create an empty batch (used by FFTProvider when no text input is relevant).
    pub fn empty(source: impl Into<String>) -> Self {
        Self {
            signals: Vec::new(),
            source: source.into(),
            timestamp: Utc::now(),
            confidence: 1.0,
            raw_data_layer: None,
        }
    }
}
```

- [ ] **Step 3: Create error types**

Write `aurora/src/percept/error.rs`:

```rust
use std::time::Duration;

/// Errors from perception providers.
#[derive(Debug, thiserror::Error)]
pub enum PerceptError {
    #[error("API key not configured for provider '{0}'")]
    MissingApiKey(String),

    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("API returned error {status}: {body}")]
    ApiError { status: u16, body: String },

    #[error("Response parse failed: {0}")]
    ParseError(String),

    #[error("Rate limited, retry after {retry_after:?}")]
    RateLimited { retry_after: Option<Duration> },

    #[error("All perception providers unavailable")]
    AllUnavailable,

    #[error("Configuration error: {0}")]
    ConfigError(#[from] ConfigError),
}

/// Errors from the encrypted configuration store.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("DPAPI encryption/decryption failed: {0}")]
    Dpapi(String),
}
```

- [ ] **Step 4: Register percept module in lib.rs**

Edit `aurora/src/lib.rs` — add after `pub mod app;`:

```rust
/// External perception layer (M2).
///
/// Unified abstraction for cloud LLMs, local models, and FFT analysis.
/// Provides the `ExternalPercept` trait and `PerceptChain` degradation
/// orchestrator.
pub mod percept;
```

- [ ] **Step 5: Verify compilation**

```bash
cargo check -p aurora 2>&1 | tail -5
```

Expected: `Finished` (no errors).

- [ ] **Step 6: Commit**

```bash
git add aurora/src/percept/mod.rs aurora/src/percept/types.rs aurora/src/percept/error.rs aurora/src/lib.rs
git commit -m "feat: add PerceptBatch, ExternalPercept trait, and error types

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 3: PerceptChain — Priority-Ordered Degradation

**Files:**
- Create: `aurora/src/percept/chain.rs`
- Create: `aurora/tests/percept_chain_tests.rs`

**Interfaces:**
- Consumes: `ExternalPercept` trait, `PerceptBatch`, `PerceptError`
- Produces: `PerceptChain::new()`, `PerceptChain::with()`, `PerceptChain::perceive_or_degrade()`

- [ ] **Step 1: Write the failing test**

Write `aurora/tests/percept_chain_tests.rs`:

```rust
use aurora::percept::{ExternalPercept, PerceptBatch, PerceptChain, PerceptError};

/// Mock provider that always succeeds.
struct MockOkProvider {
    name: &'static str,
    prio: u8,
}

impl ExternalPercept for MockOkProvider {
    fn perceive(&self, _raw: &str) -> Result<PerceptBatch, PerceptError> {
        Ok(PerceptBatch::empty(self.name))
    }
    fn provider_name(&self) -> &str { self.name }
    fn priority(&self) -> u8 { self.prio }
}

/// Mock provider that always fails.
struct MockFailProvider {
    name: &'static str,
    prio: u8,
}

impl ExternalPercept for MockFailProvider {
    fn perceive(&self, _raw: &str) -> Result<PerceptBatch, PerceptError> {
        Err(PerceptError::ParseError("mock failure".into()))
    }
    fn provider_name(&self) -> &str { self.name }
    fn priority(&self) -> u8 { self.prio }
}

#[test]
fn chain_uses_first_provider_when_it_succeeds() {
    let chain = PerceptChain::new()
        .with(Box::new(MockOkProvider { name: "first", prio: 0 }))
        .with(Box::new(MockOkProvider { name: "second", prio: 1 }));

    let result = chain.perceive_or_degrade("test").unwrap();
    assert_eq!(result.source, "first");
}

#[test]
fn chain_degrades_to_second_when_first_fails() {
    let chain = PerceptChain::new()
        .with(Box::new(MockFailProvider { name: "bad", prio: 0 }))
        .with(Box::new(MockOkProvider { name: "good", prio: 1 }));

    let result = chain.perceive_or_degrade("test").unwrap();
    assert_eq!(result.source, "good");
}

#[test]
fn chain_returns_error_when_all_fail() {
    let chain = PerceptChain::new()
        .with(Box::new(MockFailProvider { name: "bad1", prio: 0 }))
        .with(Box::new(MockFailProvider { name: "bad2", prio: 1 }));

    let err = chain.perceive_or_degrade("test").unwrap_err();
    assert!(matches!(err, PerceptError::AllUnavailable));
}

#[test]
fn chain_sorts_providers_by_priority() {
    let chain = PerceptChain::new()
        .with(Box::new(MockOkProvider { name: "low", prio: 2 }))
        .with(Box::new(MockOkProvider { name: "high", prio: 0 }))
        .with(Box::new(MockOkProvider { name: "mid", prio: 1 }));

    let result = chain.perceive_or_degrade("test").unwrap();
    assert_eq!(result.source, "high");
}

#[test]
fn chain_skips_unavailable_providers() {
    struct UnavailableProvider;
    impl ExternalPercept for UnavailableProvider {
        fn perceive(&self, _raw: &str) -> Result<PerceptBatch, PerceptError> {
            panic!("should not be called");
        }
        fn provider_name(&self) -> &str { "offline" }
        fn priority(&self) -> u8 { 0 }
        fn available(&self) -> bool { false }
    }

    let chain = PerceptChain::new()
        .with(Box::new(UnavailableProvider))
        .with(Box::new(MockOkProvider { name: "fallback", prio: 1 }));

    let result = chain.perceive_or_degrade("test").unwrap();
    assert_eq!(result.source, "fallback");
}

#[test]
fn empty_chain_returns_all_unavailable() {
    let chain = PerceptChain::new();
    let err = chain.perceive_or_degrade("test").unwrap_err();
    assert!(matches!(err, PerceptError::AllUnavailable));
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p aurora percept_chain -- --test-threads=1 2>&1 | tail -20
```

Expected: compilation errors — `PerceptChain` not yet defined.

- [ ] **Step 3: Implement PerceptChain**

Write `aurora/src/percept/chain.rs`:

```rust
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
        let mut last_error: Option<PerceptError> = None;

        for provider in &self.providers {
            if !provider.available() {
                tracing::debug!("skipping unavailable provider: {}", provider.provider_name());
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
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or(PerceptError::AllUnavailable))
    }
}

impl Default for PerceptChain {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test -p aurora percept_chain -- --test-threads=1
```

Expected: all 6 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add aurora/src/percept/chain.rs aurora/tests/percept_chain_tests.rs
git commit -m "feat: add PerceptChain with priority-ordered degradation

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 4: ConfigStore — Windows DPAPI Encrypted Configuration

**Files:**
- Create: `aurora/src/config/mod.rs`
- Create: `aurora/src/config/store.rs`
- Create: `aurora/tests/config_store_tests.rs`

**Interfaces:**
- Consumes: `ConfigError` (from percept/error)
- Produces: `ConfigStore::open()`, `ConfigStore::at_path()`, `ConfigStore::set_api_key()`, `ConfigStore::get_api_key()`, `ConfigStore::remove_api_key()`, `ConfigStore::local_model_path()`, `ConfigStore::set_local_model_path()`, `ConfigStore::cloud_model()`, `ConfigStore::set_cloud_model()`

- [ ] **Step 1: Create config module skeleton**

Write `aurora/src/config/mod.rs`:

```rust
//! Encrypted configuration storage.
//!
//! Uses Windows DPAPI for user-level encryption of API keys and
//! provider settings. Configuration is stored at `%APPDATA%\aurora\config.enc`.
//!
//! # Security
//!
//! - API keys are encrypted on disk via DPAPI (AES-256 under the hood)
//! - Decrypted keys exist only in memory, inside a `Mutex<Option<...>>`
//! - `ConfigStore` does NOT implement `Debug` (prevents log leakage)
//! - DPAPI binds to the current Windows user — different user = cannot decrypt
//! - DPAPI binds to the current machine — copy to another machine = cannot decrypt

pub mod store;

pub use store::ConfigStore;
```

- [ ] **Step 2: Write the failing tests**

Write `aurora/tests/config_store_tests.rs`:

```rust
use aurora::config::ConfigStore;
use std::fs;

/// Helper: create a ConfigStore pointed at a temp directory.
fn temp_config_store() -> ConfigStore {
    let dir = std::env::temp_dir().join(format!("aurora_test_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.enc");
    ConfigStore::at_path(&path)
}

#[test]
fn missing_key_returns_none() {
    let store = temp_config_store();
    let result = store.get_api_key("nonexistent").unwrap();
    assert!(result.is_none());
}

#[test]
fn set_and_get_api_key_roundtrip() {
    let store = temp_config_store();
    store.set_api_key("test-provider", "sk-test-key-12345").unwrap();
    let key = store.get_api_key("test-provider").unwrap();
    assert_eq!(key.as_deref(), Some("sk-test-key-12345"));
}

#[test]
fn remove_api_key_clears_it() {
    let store = temp_config_store();
    store.set_api_key("test-provider", "sk-test-key-12345").unwrap();
    store.remove_api_key("test-provider").unwrap();
    let key = store.get_api_key("test-provider").unwrap();
    assert!(key.is_none());
}

#[test]
fn multiple_keys_independent() {
    let store = temp_config_store();
    store.set_api_key("provider-a", "key-a").unwrap();
    store.set_api_key("provider-b", "key-b").unwrap();

    assert_eq!(store.get_api_key("provider-a").unwrap().as_deref(), Some("key-a"));
    assert_eq!(store.get_api_key("provider-b").unwrap().as_deref(), Some("key-b"));

    store.remove_api_key("provider-a").unwrap();
    assert!(store.get_api_key("provider-a").unwrap().is_none());
    assert_eq!(store.get_api_key("provider-b").unwrap().as_deref(), Some("key-b"));
}

#[test]
fn overwrite_existing_key() {
    let store = temp_config_store();
    store.set_api_key("provider", "old-key").unwrap();
    store.set_api_key("provider", "new-key").unwrap();
    assert_eq!(store.get_api_key("provider").unwrap().as_deref(), Some("new-key"));
}

#[test]
fn local_model_path_default_none() {
    let store = temp_config_store();
    assert!(store.local_model_path().unwrap().is_none());
}

#[test]
fn set_and_get_local_model_path() {
    let store = temp_config_store();
    store.set_local_model_path("http://localhost:11434").unwrap();
    assert_eq!(
        store.local_model_path().unwrap().as_deref(),
        Some("http://localhost:11434")
    );
}

#[test]
fn cloud_model_default_none() {
    let store = temp_config_store();
    assert!(store.cloud_model().unwrap().is_none());
}

#[test]
fn set_and_get_cloud_model() {
    let store = temp_config_store();
    store.set_cloud_model("claude-opus-4-8").unwrap();
    assert_eq!(
        store.cloud_model().unwrap().as_deref(),
        Some("claude-opus-4-8")
    );
}

#[test]
fn new_config_file_does_not_exist_initially() {
    let dir = std::env::temp_dir().join(format!("aurora_test_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.enc");
    assert!(!path.exists());
    let store = ConfigStore::at_path(&path);
    let _ = store.get_api_key("anything").unwrap();
    store.set_api_key("test", "value").unwrap();
    assert!(path.exists());
}
```

- [ ] **Step 3: Run tests to verify they fail**

```bash
cargo test -p aurora config_store -- --test-threads=1 2>&1 | tail -20
```

Expected: compilation errors — `ConfigStore` not yet defined.

- [ ] **Step 4: Implement ConfigStore**

Write `aurora/src/config/store.rs`:

```rust
use crate::percept::ConfigError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

/// Decrypted in-memory configuration — never written to disk as plaintext.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct DecryptedConfig {
    #[serde(default)]
    api_keys: HashMap<String, String>,
    #[serde(default)]
    local_model_path: Option<String>,
    #[serde(default)]
    cloud_model: Option<String>,
}

/// Encrypted configuration store backed by Windows DPAPI.
///
/// # Security
///
/// - API keys are encrypted on disk via DPAPI
/// - Decrypted keys exist only in the in-memory `cache`
/// - This struct intentionally does NOT implement `Debug`
/// - API key values are never logged
pub struct ConfigStore {
    path: PathBuf,
    cache: Mutex<Option<DecryptedConfig>>,
}

impl ConfigStore {
    /// Open the config store at the default path: `%APPDATA%\aurora\config.enc`.
    pub fn open() -> Result<Self, ConfigError> {
        let path = Self::default_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(Self {
            path,
            cache: Mutex::new(None),
        })
    }

    /// Open the config store at a specific path (for testing).
    pub fn at_path(path: &std::path::Path) -> Self {
        Self {
            path: path.to_path_buf(),
            cache: Mutex::new(None),
        }
    }

    fn default_path() -> Result<PathBuf, ConfigError> {
        #[cfg(target_os = "windows")]
        {
            let appdata = std::env::var("APPDATA").unwrap_or_else(|_| {
                let home = std::env::var("USERPROFILE").unwrap_or_default();
                format!("{home}\\AppData\\Roaming")
            });
            Ok(PathBuf::from(appdata).join("aurora").join("config.enc"))
        }
        #[cfg(not(target_os = "windows"))]
        {
            let home = std::env::var("HOME").unwrap_or_default();
            Ok(PathBuf::from(home).join(".aurora").join("config.enc"))
        }
    }

    pub fn set_api_key(&self, provider: &str, key: &str) -> Result<(), ConfigError> {
        let mut config = self.load_or_default()?;
        config.api_keys.insert(provider.to_string(), key.to_string());
        self.save_encrypted(&config)?;
        *self.cache.lock().unwrap() = Some(config);
        Ok(())
    }

    pub fn get_api_key(&self, provider: &str) -> Result<Option<String>, ConfigError> {
        {
            let guard = self.cache.lock().unwrap();
            if let Some(ref config) = *guard {
                return Ok(config.api_keys.get(provider).cloned());
            }
        }
        let config = self.load_or_default()?;
        let key = config.api_keys.get(provider).cloned();
        *self.cache.lock().unwrap() = Some(config);
        Ok(key)
    }

    pub fn remove_api_key(&self, provider: &str) -> Result<(), ConfigError> {
        let mut config = self.load_or_default()?;
        config.api_keys.remove(provider);
        self.save_encrypted(&config)?;
        *self.cache.lock().unwrap() = Some(config);
        Ok(())
    }

    pub fn local_model_path(&self) -> Result<Option<String>, ConfigError> {
        self.ensure_loaded()?;
        let guard = self.cache.lock().unwrap();
        Ok(guard.as_ref().and_then(|c| c.local_model_path.clone()))
    }

    pub fn set_local_model_path(&self, path: &str) -> Result<(), ConfigError> {
        let mut config = self.load_or_default()?;
        config.local_model_path = Some(path.to_string());
        self.save_encrypted(&config)?;
        *self.cache.lock().unwrap() = Some(config);
        Ok(())
    }

    pub fn cloud_model(&self) -> Result<Option<String>, ConfigError> {
        self.ensure_loaded()?;
        let guard = self.cache.lock().unwrap();
        Ok(guard.as_ref().and_then(|c| c.cloud_model.clone()))
    }

    pub fn set_cloud_model(&self, model: &str) -> Result<(), ConfigError> {
        let mut config = self.load_or_default()?;
        config.cloud_model = Some(model.to_string());
        self.save_encrypted(&config)?;
        *self.cache.lock().unwrap() = Some(config);
        Ok(())
    }

    fn ensure_loaded(&self) -> Result<(), ConfigError> {
        let mut guard = self.cache.lock().unwrap();
        if guard.is_none() {
            *guard = Some(self.load_or_default()?);
        }
        Ok(())
    }

    fn load_or_default(&self) -> Result<DecryptedConfig, ConfigError> {
        if !self.path.exists() {
            return Ok(DecryptedConfig::default());
        }
        let encrypted = fs::read(&self.path)?;
        let plain = self.decrypt(&encrypted)?;
        let config: DecryptedConfig = serde_json::from_slice(&plain)?;
        Ok(config)
    }

    fn save_encrypted(&self, config: &DecryptedConfig) -> Result<(), ConfigError> {
        let plain = serde_json::to_vec(config)?;
        let encrypted = self.encrypt(&plain)?;
        fs::write(&self.path, encrypted)?;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn encrypt(&self, plain: &[u8]) -> Result<Vec<u8>, ConfigError> {
        dpapi_encrypt(plain)
    }

    #[cfg(target_os = "windows")]
    fn decrypt(&self, cipher: &[u8]) -> Result<Vec<u8>, ConfigError> {
        dpapi_decrypt(cipher)
    }

    #[cfg(not(target_os = "windows"))]
    fn encrypt(&self, plain: &[u8]) -> Result<Vec<u8>, ConfigError> {
        Ok(base64_encode(plain).into_bytes())
    }

    #[cfg(not(target_os = "windows"))]
    fn decrypt(&self, cipher: &[u8]) -> Result<Vec<u8>, ConfigError> {
        let s = std::str::from_utf8(cipher)
            .map_err(|e| ConfigError::Dpapi(format!("invalid UTF-8: {e}")))?;
        base64_decode(s)
    }
}

// ── Windows DPAPI bindings ─────────────────────────────────────────

#[cfg(target_os = "windows")]
fn dpapi_encrypt(plain: &[u8]) -> Result<Vec<u8>, ConfigError> {
    use std::mem;
    use windows_sys::Win32::Foundation;
    use windows_sys::Win32::Security::Cryptography;

    let mut data_in = Cryptography::CRYPT_INTEGER_BLOB {
        cbData: plain.len() as u32,
        pbData: plain.as_ptr() as *mut u8,
    };
    let mut data_out: Cryptography::CRYPT_INTEGER_BLOB = unsafe { mem::zeroed() };

    let ok = unsafe {
        Cryptography::CryptProtectData(
            &mut data_in,
            windows_sys::w!("aurora-config"),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            0x1, // CRYPTPROTECT_UI_FORBIDDEN
            &mut data_out,
        )
    };

    if ok == 0 {
        let err = unsafe { Foundation::GetLastError() };
        return Err(ConfigError::Dpapi(format!("CryptProtectData failed: {err}")));
    }

    let result =
        unsafe { std::slice::from_raw_parts(data_out.pbData, data_out.cbData as usize).to_vec() };
    unsafe { Cryptography::LocalFree(data_out.pbData as isize) };
    Ok(result)
}

#[cfg(target_os = "windows")]
fn dpapi_decrypt(cipher: &[u8]) -> Result<Vec<u8>, ConfigError> {
    use std::mem;
    use windows_sys::Win32::Foundation;
    use windows_sys::Win32::Security::Cryptography;

    let mut data_in = Cryptography::CRYPT_INTEGER_BLOB {
        cbData: cipher.len() as u32,
        pbData: cipher.as_ptr() as *mut u8,
    };
    let mut data_out: Cryptography::CRYPT_INTEGER_BLOB = unsafe { mem::zeroed() };

    let ok = unsafe {
        Cryptography::CryptUnprotectData(
            &mut data_in,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            0x1,
            &mut data_out,
        )
    };

    if ok == 0 {
        let err = unsafe { Foundation::GetLastError() };
        return Err(ConfigError::Dpapi(format!(
            "CryptUnprotectData failed: {err}. Config may belong to different user/machine."
        )));
    }

    let result =
        unsafe { std::slice::from_raw_parts(data_out.pbData, data_out.cbData as usize).to_vec() };
    unsafe { Cryptography::LocalFree(data_out.pbData as isize) };
    Ok(result)
}

// ── Non-Windows fallback (base64, NOT secure — placeholder for M3+) ─

#[cfg(not(target_os = "windows"))]
fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((triple >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[((triple >> 6) & 0x3F) as usize] as char);
        }
        if chunk.len() > 2 {
            out.push(TABLE[(triple & 0x3F) as usize] as char);
        }
    }
    match data.len() % 3 {
        1 => out.push_str("=="),
        2 => out.push('='),
        _ => {}
    }
    out
}

#[cfg(not(target_os = "windows"))]
fn base64_decode(s: &str) -> Result<Vec<u8>, ConfigError> {
    let s = s.trim_end_matches('=');
    let mut out = Vec::new();
    let mut buffer: u32 = 0;
    let mut bits = 0u8;
    for c in s.chars() {
        let val = match c {
            'A'..='Z' => c as u8 - b'A',
            'a'..='z' => c as u8 - b'a' + 26,
            '0'..='9' => c as u8 - b'0' + 52,
            '+' => 62,
            '/' => 63,
            _ => return Err(ConfigError::Dpapi(format!("invalid base64 char: {c}"))),
        } as u32;
        buffer = (buffer << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    Ok(out)
}
```

- [ ] **Step 5: Register config module in lib.rs**

Edit `aurora/src/lib.rs` — add after `pub mod app;`:

```rust
/// Encrypted configuration storage (M2).
///
/// Windows DPAPI-backed API key and provider settings store.
/// Config file at %APPDATA%\aurora\config.enc.
pub mod config;
```

- [ ] **Step 6: Run tests to verify they pass**

```bash
cargo test -p aurora config_store -- --test-threads=1
```

Expected: all 10 tests PASS.

- [ ] **Step 7: Commit**

```bash
git add aurora/src/config/ aurora/src/lib.rs aurora/tests/config_store_tests.rs
git commit -m "feat: add ConfigStore with Windows DPAPI encryption

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 5: FFTProvider — The Never-Offline Safety Floor

**Files:**
- Create: `aurora/src/percept/fft.rs`
- Create: `aurora/tests/fft_provider_tests.rs`

**Interfaces:**
- Consumes: `ExternalPercept` trait, `PerceptBatch`, `SignalSpec` (from pipeline::analysis)
- Produces: `FFTProvider::new(spec: SignalSpec)`

- [ ] **Step 1: Write the failing test**

Write `aurora/tests/fft_provider_tests.rs`:

```rust
use aurora::percept::{ExternalPercept, FFTProvider};
use aurora::pipeline::analysis::SignalSpec;

#[test]
fn fft_provider_never_fails() {
    let spec = SignalSpec {
        freq: 2.0,
        sample_rate: 100.0,
        duration_secs: 1.0,
        noise_std: 0.0,
    };
    let provider = FFTProvider::new(spec);
    let batch = provider.perceive("any text").unwrap();
    assert!(batch.confidence >= 0.0);
}

#[test]
fn fft_provider_priority_is_lowest() {
    let spec = SignalSpec {
        freq: 2.0,
        sample_rate: 100.0,
        duration_secs: 1.0,
        noise_std: 0.0,
    };
    let provider = FFTProvider::new(spec);
    assert!(provider.priority() >= 2);
}

#[test]
fn fft_provider_always_available() {
    let spec = SignalSpec {
        freq: 2.0,
        sample_rate: 100.0,
        duration_secs: 1.0,
        noise_std: 0.0,
    };
    let provider = FFTProvider::new(spec);
    assert!(provider.available());
}

#[test]
fn fft_provider_has_meaningful_name() {
    let spec = SignalSpec {
        freq: 2.0,
        sample_rate: 100.0,
        duration_secs: 1.0,
        noise_std: 0.0,
    };
    let provider = FFTProvider::new(spec);
    assert!(!provider.provider_name().is_empty());
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p aurora fft_provider -- --test-threads=1 2>&1 | tail -10
```

Expected: compilation errors — `FFTProvider` not yet defined.

- [ ] **Step 3: Implement FFTProvider**

Write `aurora/src/percept/fft.rs`:

```rust
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
    spec: SignalSpec,
}

impl FFTProvider {
    pub fn new(spec: SignalSpec) -> Self {
        Self { spec }
    }
}

impl ExternalPercept for FFTProvider {
    fn perceive(&self, _raw: &str) -> Result<PerceptBatch, PerceptError> {
        Ok(PerceptBatch::empty("fft-wavelet"))
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
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test -p aurora fft_provider -- --test-threads=1
```

Expected: all 4 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add aurora/src/percept/fft.rs aurora/tests/fft_provider_tests.rs
git commit -m "feat: add FFTProvider — never-offline safety floor

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 6: CloudLLMProvider — Anthropic/OpenAI HTTP Integration

**Files:**
- Create: `aurora/src/percept/cloud.rs`
- Create: `aurora/src/percept/prompts/percept_system.txt`
- Create: `aurora/tests/cloud_llm_tests.rs`

**Interfaces:**
- Consumes: `ExternalPercept` trait, `ConfigStore`, `PerceptBatch`, `PerceptError`
- Produces: `CloudLLMProvider::new(config, model)`, `CloudLLMProvider::parse_anthropic_response()`, `CloudLLMProvider::parse_openai_response()`

- [ ] **Step 1: Create the system prompt template**

Write `aurora/src/percept/prompts/percept_system.txt`:

```
You are a perception module inside a ternary decision engine. Your job is to convert raw human text into structured signals for analysis. You do NOT make decisions. You do NOT give advice. You only extract and structure.

## Output Format

Respond with ONLY a JSON object. No markdown, no explanation, no conversation.

{
  "signals": [
    {
      "frame": "Science|Individual|Consensus|Absolute",
      "value": 1 | 0 | -1,
      "phase": 0.0-1.0,
      "reasoning": "one short sentence explaining why"
    }
  ],
  "confidence": 0.0-1.0,
  "suggested_scenario": "PhysicalReasoning|ValueConflict|MedicalEthics|ReflexiveAudit|CrisisResponse|General",
  "summary": "one sentence summarizing what was perceived"
}

## Rules

1. Value meanings: 1 = True/Positive/Agree, 0 = Hold/Suspend/Uncertain, -1 = False/Negative/Disagree
2. Phase is tendency strength: 0.0 = very weak, 0.5 = neutral, 1.0 = very strong
3. Frame meanings:
   - Science: empirical, measurable, physical facts
   - Individual: personal experience, subjective, emotional
   - Consensus: social agreement, norms, group judgment
   - Absolute: non-negotiable principles, ethical absolutes
4. When frames conflict or values are genuinely uncertain, use value=0 (Hold). Never force a binary choice.
5. You are a map, not the territory. Your output is structured signals, not truth.
6. Your output will physically reshape neural circuits. Do no harm.
7. Encourage independent thinking. Never demand blind belief.
8. The user bears ultimate responsibility. You serve, you do not rule.
```

- [ ] **Step 2: Write the failing tests**

Write `aurora/tests/cloud_llm_tests.rs`:

```rust
use aurora::percept::cloud::CloudLLMProvider;
use serde_json::json;
use trit_core::TritValue;

#[test]
fn parse_valid_anthropic_response() {
    let response = json!({
        "content": [{
            "text": json!({
                "signals": [
                    {"frame": "Science", "value": 1, "phase": 0.8, "reasoning": "clear evidence"},
                    {"frame": "Individual", "value": 0, "phase": 0.5, "reasoning": "uncertain"}
                ],
                "confidence": 0.85,
                "suggested_scenario": "General",
                "summary": "Mixed input detected"
            }).to_string()
        }]
    });

    let batch = CloudLLMProvider::parse_anthropic_response(&response).unwrap();
    assert_eq!(batch.signals.len(), 2);
    assert_eq!(batch.confidence, 0.85);
}

#[test]
fn parse_valid_openai_response() {
    let response = json!({
        "choices": [{
            "message": {
                "content": json!({
                    "signals": [
                        {"frame": "Consensus", "value": -1, "phase": 0.9, "reasoning": "social rejection"}
                    ],
                    "confidence": 0.95,
                    "suggested_scenario": "ValueConflict",
                    "summary": "Norm violation"
                }).to_string()
            }
        }]
    });

    let batch = CloudLLMProvider::parse_openai_response(&response).unwrap();
    assert_eq!(batch.signals.len(), 1);
    assert_eq!(batch.confidence, 0.95);
}

#[test]
fn parse_response_with_hold_signal() {
    let response = json!({
        "content": [{
            "text": json!({
                "signals": [
                    {"frame": "Absolute", "value": 0, "phase": 0.5, "reasoning": "cannot resolve"}
                ],
                "confidence": 0.6,
                "suggested_scenario": "ValueConflict",
                "summary": "Unresolvable tension"
            }).to_string()
        }]
    });

    let batch = CloudLLMProvider::parse_anthropic_response(&response).unwrap();
    assert_eq!(batch.signals[0].value(), TritValue::Hold);
}

#[test]
fn parse_malformed_json_returns_parse_error() {
    let response = json!({
        "content": [{"text": "not valid json at all"}]
    });
    let result = CloudLLMProvider::parse_anthropic_response(&response);
    assert!(result.is_err());
}

#[test]
fn parse_missing_signals_field_returns_error() {
    let response = json!({
        "content": [{
            "text": json!({"confidence": 0.5, "summary": "no signals"}).to_string()
        }]
    });
    let result = CloudLLMProvider::parse_anthropic_response(&response);
    assert!(result.is_err());
}

#[test]
fn parse_empty_signals_array_is_valid() {
    let response = json!({
        "content": [{
            "text": json!({
                "signals": [],
                "confidence": 1.0,
                "suggested_scenario": "General",
                "summary": "nothing to report"
            }).to_string()
        }]
    });
    let batch = CloudLLMProvider::parse_anthropic_response(&response).unwrap();
    assert!(batch.signals.is_empty());
}

#[test]
fn parse_invalid_frame_name_skips_signal() {
    let response = json!({
        "content": [{
            "text": json!({
                "signals": [
                    {"frame": "NotARealFrame", "value": 1, "phase": 0.5, "reasoning": "bad"},
                    {"frame": "Science", "value": 1, "phase": 0.8, "reasoning": "good"}
                ],
                "confidence": 0.7,
                "summary": "one bad frame skipped"
            }).to_string()
        }]
    });
    let batch = CloudLLMProvider::parse_anthropic_response(&response).unwrap();
    assert_eq!(batch.signals.len(), 1);
}

#[test]
fn parse_invalid_value_does_not_panic() {
    let response = json!({
        "content": [{
            "text": json!({
                "signals": [
                    {"frame": "Science", "value": 999, "phase": 0.5, "reasoning": "out of range"}
                ],
                "confidence": 0.5,
                "summary": "bad value"
            }).to_string()
        }]
    });
    let result = CloudLLMProvider::parse_anthropic_response(&response);
    let _ = result; // no panic
}
```

- [ ] **Step 3: Run tests to verify they fail**

```bash
cargo test -p aurora cloud_llm -- --test-threads=1 2>&1 | tail -10
```

Expected: compilation errors — `CloudLLMProvider` not yet defined.

- [ ] **Step 4: Implement CloudLLMProvider**

Write `aurora/src/percept/cloud.rs`:

```rust
use crate::config::ConfigStore;
use crate::percept::{ExternalPercept, PerceptBatch, PerceptError};
use chrono::Utc;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use trit_core::hook::ScenarioType;
use trit_core::{Frame, Phase, TritValue, TritWord};

/// Cloud LLM perception provider.
///
/// Calls Anthropic Messages API or OpenAI Chat Completions API to convert
/// natural language text into structured TritWord signals. The LLM is
/// constrained by a system prompt to output JSON only — it does not make
/// decisions, only extracts and structures.
pub struct CloudLLMProvider {
    config: Arc<ConfigStore>,
    client: reqwest::Client,
    model: String,
    endpoint: String,
    system_prompt: String,
}

impl CloudLLMProvider {
    /// Create a new CloudLLMProvider.
    ///
    /// `model` determines the API endpoint:
    /// - Models containing "claude" -> Anthropic Messages API
    /// - All others -> OpenAI Chat Completions API
    pub fn new(config: Arc<ConfigStore>, model: &str) -> Result<Self, PerceptError> {
        let api_key = config
            .get_api_key(model)?
            .ok_or_else(|| PerceptError::MissingApiKey(model.to_string()))?;

        let (endpoint, is_anthropic) = if model.contains("claude") {
            ("https://api.anthropic.com/v1/messages".to_string(), true)
        } else {
            ("https://api.openai.com/v1/chat/completions".to_string(), false)
        };

        let mut headers = reqwest::header::HeaderMap::new();
        if is_anthropic {
            headers.insert("x-api-key", api_key.parse().unwrap());
            headers.insert("anthropic-version", "2023-06-01".parse().unwrap());
        } else {
            headers.insert("authorization", format!("Bearer {api_key}").parse().unwrap());
        }
        headers.insert("content-type", "application/json".parse().unwrap());

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(PerceptError::HttpError)?;

        Ok(Self {
            config,
            client,
            model: model.to_string(),
            endpoint,
            system_prompt: include_str!("prompts/percept_system.txt").to_string(),
        })
    }

    fn call_api(&self, raw: &str) -> Result<PerceptBatch, PerceptError> {
        if self.model.contains("claude") {
            self.call_anthropic(raw)
        } else {
            self.call_openai(raw)
        }
    }

    fn call_anthropic(&self, raw: &str) -> Result<PerceptBatch, PerceptError> {
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": 1024,
            "system": self.system_prompt,
            "messages": [{"role": "user", "content": raw}]
        });

        let response = self.client.post(&self.endpoint).json(&body).send().map_err(PerceptError::HttpError)?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().unwrap_or_default();
            if status.as_u16() == 429 {
                return Err(PerceptError::RateLimited { retry_after: Some(Duration::from_secs(30)) });
            }
            return Err(PerceptError::ApiError { status: status.as_u16(), body });
        }
        let json: Value = response.json().map_err(PerceptError::HttpError)?;
        Self::parse_anthropic_response(&json)
    }

    fn call_openai(&self, raw: &str) -> Result<PerceptBatch, PerceptError> {
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": 1024,
            "messages": [
                {"role": "system", "content": self.system_prompt},
                {"role": "user", "content": raw}
            ]
        });

        let response = self.client.post(&self.endpoint).json(&body).send().map_err(PerceptError::HttpError)?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().unwrap_or_default();
            if status.as_u16() == 429 {
                return Err(PerceptError::RateLimited { retry_after: Some(Duration::from_secs(30)) });
            }
            return Err(PerceptError::ApiError { status: status.as_u16(), body });
        }
        let json: Value = response.json().map_err(PerceptError::HttpError)?;
        Self::parse_openai_response(&json)
    }

    /// Parse an Anthropic Messages API response into a PerceptBatch.
    pub fn parse_anthropic_response(response: &Value) -> Result<PerceptBatch, PerceptError> {
        let text = response["content"][0]["text"]
            .as_str()
            .ok_or_else(|| PerceptError::ParseError("missing content[0].text".into()))?;
        Self::parse_inner_json(text)
    }

    /// Parse an OpenAI Chat Completions response into a PerceptBatch.
    pub fn parse_openai_response(response: &Value) -> Result<PerceptBatch, PerceptError> {
        let text = response["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| PerceptError::ParseError("missing choices[0].message.content".into()))?;
        Self::parse_inner_json(text)
    }

    fn parse_inner_json(text: &str) -> Result<PerceptBatch, PerceptError> {
        let inner: Value = serde_json::from_str(text).map_err(|e| PerceptError::ParseError(e.to_string()))?;

        let signals_array = inner["signals"].as_array()
            .ok_or_else(|| PerceptError::ParseError("missing 'signals' array".into()))?;

        let mut signals = Vec::with_capacity(signals_array.len());
        for sig in signals_array {
            if let Some(word) = Self::parse_signal(sig) {
                signals.push(word);
            }
        }

        let confidence = inner["confidence"].as_f64().unwrap_or(0.5).clamp(0.0, 1.0);
        let suggested_scenario = inner["suggested_scenario"].as_str().and_then(|s| Self::parse_scenario_type(s));
        let summary = inner["summary"].as_str().unwrap_or("(no summary)").to_string();

        Ok(PerceptBatch {
            signals,
            source: "cloud-llm".into(),
            timestamp: Utc::now(),
            confidence,
            suggested_scenario,
            summary,
        })
    }

    fn parse_signal(sig: &Value) -> Option<TritWord> {
        let frame_str = sig["frame"].as_str()?;
        let frame = match frame_str {
            "Science" => Frame::Science,
            "Individual" => Frame::Individual,
            "Consensus" => Frame::Consensus,
            "Absolute" => Frame::Absolute,
            _ => {
                tracing::warn!(frame = frame_str, "unknown frame from LLM, skipping signal");
                return None;
            }
        };

        let raw_value = sig["value"].as_i64().unwrap_or(0);
        let value = match raw_value {
            1 => TritValue::True,
            -1 => TritValue::False,
            _ => TritValue::Hold,
        };

        let phase_val = sig["phase"].as_f64().unwrap_or(0.5).clamp(0.0, 1.0);
        let phase = Phase::new_clamped(phase_val);

        Some(TritWord::new(value, phase, frame))
    }

    fn parse_scenario_type(s: &str) -> Option<ScenarioType> {
        match s {
            "PhysicalReasoning" => Some(ScenarioType::PhysicalReasoning),
            "ValueConflict" => Some(ScenarioType::ValueConflict),
            "MedicalEthics" => Some(ScenarioType::MedicalEthics),
            "ReflexiveAudit" => Some(ScenarioType::ReflexiveAudit),
            "CrisisResponse" => Some(ScenarioType::CrisisResponse),
            "General" => Some(ScenarioType::General),
            _ => None,
        }
    }
}

impl ExternalPercept for CloudLLMProvider {
    fn perceive(&self, raw: &str) -> Result<PerceptBatch, PerceptError> {
        self.call_api(raw)
    }

    fn provider_name(&self) -> &str {
        &self.model
    }

    fn priority(&self) -> u8 {
        0
    }

    fn available(&self) -> bool {
        self.config.get_api_key(&self.model).ok().flatten().is_some()
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cargo test -p aurora cloud_llm -- --test-threads=1
```

Expected: all 8 tests PASS.

- [ ] **Step 6: Commit**

```bash
git add aurora/src/percept/cloud.rs aurora/src/percept/prompts/ aurora/tests/cloud_llm_tests.rs
git commit -m "feat: add CloudLLMProvider with Anthropic/OpenAI HTTP integration

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 7: LocalLLMProvider — Local Inference Server

**Files:**
- Create: `aurora/src/percept/local.rs`
- Create: `aurora/tests/local_llm_tests.rs`

**Interfaces:**
- Consumes: `ExternalPercept` trait, `ConfigStore`, `PerceptBatch`, `PerceptError`, `CloudLLMProvider::parse_openai_response()`
- Produces: `LocalLLMProvider::new(config)`

- [ ] **Step 1: Write the failing tests**

Write `aurora/tests/local_llm_tests.rs`:

```rust
use aurora::percept::cloud::CloudLLMProvider;
use serde_json::json;

#[test]
fn local_llm_parse_response_same_format_as_cloud() {
    let response = json!({
        "choices": [{
            "message": {
                "content": json!({
                    "signals": [
                        {"frame": "Individual", "value": 0, "phase": 0.5, "reasoning": "uncertain"}
                    ],
                    "confidence": 0.7,
                    "suggested_scenario": "General",
                    "summary": "local model output"
                }).to_string()
            }
        }]
    });

    let batch = CloudLLMProvider::parse_openai_response(&response).unwrap();
    assert_eq!(batch.signals.len(), 1);
    assert_eq!(batch.confidence, 0.7);
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p aurora local_llm -- --test-threads=1 2>&1 | tail -10
```

Expected: compilation errors — `LocalLLMProvider` not yet defined.

- [ ] **Step 3: Implement LocalLLMProvider**

Write `aurora/src/percept/local.rs`:

```rust
use crate::config::ConfigStore;
use crate::percept::cloud::CloudLLMProvider;
use crate::percept::{ExternalPercept, PerceptBatch, PerceptError};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;

/// Local LLM perception provider.
///
/// Communicates with a local inference server (ollama, llama.cpp, etc.)
/// via HTTP on localhost. Uses the same JSON output contract as
/// CloudLLMProvider — the local model is expected to follow the same
/// system prompt and return structured TritWord signals.
///
/// No API key is needed (localhost trust boundary).
pub struct LocalLLMProvider {
    config: Arc<ConfigStore>,
    client: reqwest::Client,
    endpoint: String,
}

impl LocalLLMProvider {
    /// Create a new LocalLLMProvider.
    ///
    /// Reads the endpoint URL from `ConfigStore::local_model_path()`.
    /// Defaults to `http://localhost:11434` (ollama default) if not configured.
    pub fn new(config: Arc<ConfigStore>) -> Result<Self, PerceptError> {
        let endpoint = config
            .local_model_path()?
            .unwrap_or_else(|| "http://localhost:11434".to_string());

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(PerceptError::HttpError)?;

        Ok(Self {
            config,
            client,
            endpoint,
        })
    }

    fn call_local(&self, raw: &str) -> Result<PerceptBatch, PerceptError> {
        let system_prompt = include_str!("prompts/percept_system.txt");

        let body = serde_json::json!({
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": raw}
            ],
            "stream": false
        });

        let url = format!("{}/v1/chat/completions", self.endpoint.trim_end_matches('/'));

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .map_err(PerceptError::HttpError)?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().unwrap_or_default();
            return Err(PerceptError::ApiError {
                status: status.as_u16(),
                body,
            });
        }

        let json: Value = response.json().map_err(PerceptError::HttpError)?;
        CloudLLMProvider::parse_openai_response(&json)
    }
}

impl ExternalPercept for LocalLLMProvider {
    fn perceive(&self, raw: &str) -> Result<PerceptBatch, PerceptError> {
        self.call_local(raw)
    }

    fn provider_name(&self) -> &str {
        "local-llm"
    }

    fn priority(&self) -> u8 {
        1
    }

    fn available(&self) -> bool {
        self.config.local_model_path().ok().flatten().is_some()
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test -p aurora local_llm -- --test-threads=1
```

Expected: 1 test PASS.

- [ ] **Step 5: Commit**

```bash
git add aurora/src/percept/local.rs aurora/tests/local_llm_tests.rs
git commit -m "feat: add LocalLLMProvider for local inference servers

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 8: Pipeline Integration — run_analysis_from_percept

**Files:**
- Modify: `aurora/src/pipeline/analysis.rs`
- Modify: `aurora/src/pipeline/mod.rs`
- Create: `aurora/tests/pipeline_percept_integration_tests.rs`

**Interfaces:**
- Consumes: `PerceptBatch`, existing `run_analysis()`
- Produces: `run_analysis_from_percept(spec, threshold, user_feels_normal, contact_signals, percept_signals) -> Result<AnalysisReport, BcError>`

- [ ] **Step 1: Write the failing integration test**

Write `aurora/tests/pipeline_percept_integration_tests.rs`:

```rust
use aurora::pipeline::analysis::{self, SignalSpec};
use trit_core::{Frame, Phase, TritValue, TritWord};

#[test]
fn run_analysis_from_percept_merges_signals() {
    let spec = SignalSpec {
        freq: 2.0,
        sample_rate: 100.0,
        duration_secs: 1.0,
        noise_std: 0.0,
    };

    let percept_signals = vec![
        TritWord::new(TritValue::Hold, Phase::new_clamped(0.5), Frame::Individual),
    ];

    let report = analysis::run_analysis_from_percept(
        &spec, 1.0, true, &[], &percept_signals,
    ).unwrap();

    assert_eq!(report.contact_count, 0);
    assert!(report.decision.input_signals.len() >= 3);
}

#[test]
fn run_analysis_from_percept_with_empty_percept() {
    let spec = SignalSpec {
        freq: 2.0,
        sample_rate: 100.0,
        duration_secs: 1.0,
        noise_std: 0.0,
    };

    let report = analysis::run_analysis_from_percept(
        &spec, 1.0, true, &[], &[],
    ).unwrap();

    assert!(report.decision.input_signals.len() >= 2);
}

#[test]
fn existing_run_analysis_still_works() {
    let spec = SignalSpec {
        freq: 2.0,
        sample_rate: 100.0,
        duration_secs: 1.0,
        noise_std: 0.0,
    };

    let report = analysis::run_analysis(&spec, 1.0, true, &[]).unwrap();
    assert!(report.decision.input_signals.len() >= 2);
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p aurora pipeline_percept -- --test-threads=1 2>&1 | tail -10
```

Expected: compilation errors — `run_analysis_from_percept` not yet defined.

- [ ] **Step 3: Implement run_analysis_from_percept**

Edit `aurora/src/pipeline/analysis.rs` — add after the closing `}` of the existing `run_analysis()` function:

```rust
/// Run the analysis link with additional percept signals from external providers.
///
/// This is an overload of [`run_analysis`] that accepts percept signals
/// (from LLMs or other perception providers) and merges them into the
/// signal vector alongside embodied, individual, and contact signals
/// before ternary evaluation.
///
/// The original [`run_analysis`] function is unchanged and still available.
pub fn run_analysis_from_percept(
    spec: &SignalSpec,
    frequency_threshold: f64,
    user_feels_normal: bool,
    contact_signals: &[TritWord],
    percept_signals: &[TritWord],
) -> Result<AnalysisReport, BcError> {
    // Step 1: Generate synthetic signal
    let signal = sine_wave(
        spec.freq,
        spec.sample_rate,
        spec.duration_secs,
        spec.noise_std,
    );

    // Step 2: Analyze via FFT
    let ts = TimeSeries::new(spec.sample_rate, signal)?;
    let engine = FftWaveletEngine;
    let spectrum = engine.analyze(&ts)?;

    // Step 3: Map to TritWords
    let embodied = frequency_to_embodied(spectrum.fundamental_hz, frequency_threshold);
    let individual = user_state_to_individual(user_feels_normal);

    // Step 4: Merge all signals (embodied + individual + contacts + percept)
    let mut all_signals = vec![embodied, individual];
    all_signals.extend_from_slice(contact_signals);
    all_signals.extend_from_slice(percept_signals);

    // Step 5: Evaluate ternary decision
    let decision_engine = TritDecisionEngine;
    let mut session = DecisionSession::new("analysis_session".into());
    let decision = decision_engine.evaluate(&mut session, &all_signals, "General")?;

    Ok(AnalysisReport {
        spectrum,
        decision,
        contact_count: contact_signals.len(),
    })
}
```

- [ ] **Step 4: Update pipeline mod.rs to export the new function**

Edit `aurora/src/pipeline/mod.rs` — change the `pub use` line to:

```rust
pub use analysis::{run_analysis, run_analysis_from_percept, AnalysisReport, SignalSpec};
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cargo test -p aurora pipeline_percept -- --test-threads=1
```

Expected: all 3 tests PASS.

- [ ] **Step 6: Run ALL existing tests to verify no regressions**

```bash
cargo test --workspace --all-features -- --test-threads=2
```

Expected: all tests PASS, zero regressions.

- [ ] **Step 7: Commit**

```bash
git add aurora/src/pipeline/analysis.rs aurora/src/pipeline/mod.rs aurora/tests/pipeline_percept_integration_tests.rs
git commit -m "feat: add run_analysis_from_percept — merge LLM signals into analysis

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 9: AuroraApp Integration — run_with_percept

**Files:**
- Modify: `aurora/src/app.rs`

**Interfaces:**
- Consumes: `PerceptChain`, `PerceptBatch`, `run_analysis_from_percept()`, `ConfigStore`
- Produces: `AuroraApp::run_with_percept(self, input, user_text) -> Result<AppOutput>`, `AuroraApp::config()`

- [ ] **Step 1: Modify AuroraApp to hold a PerceptChain and ConfigStore**

Replace `aurora/src/app.rs` with the version that includes `PerceptChain` and `ConfigStore` integration. The key changes from the current version:

1. Add `percept_chain: PerceptChain` and `config: Arc<ConfigStore>` fields to `AuroraApp`
2. In `AuroraApp::new()`, build the perception chain from available providers
3. Add `run_with_percept()` method
4. Add `config()` accessor
5. Keep existing `run_pipeline()` unchanged

See the design spec section 6.2 and the full code in the plan's earlier detailed task descriptions for the exact implementation.

- [ ] **Step 2: Verify compilation**

```bash
cargo check -p aurora 2>&1 | tail -10
```

Expected: `Finished` (no errors).

- [ ] **Step 3: Run ALL tests**

```bash
cargo test --workspace --all-features -- --test-threads=2
```

Expected: all tests PASS, zero regressions.

- [ ] **Step 4: Commit**

```bash
git add aurora/src/app.rs
git commit -m "feat: integrate PerceptChain into AuroraApp with run_with_percept

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 10: Ethics Gate Tests

**Files:**
- Create: `aurora/tests/ethics_gate_tests.rs`
- Create: `aurora/tests/fixtures/llm_value_conflict_response.json`
- Create: `aurora/tests/fixtures/llm_imperative_response.json`

**Interfaces:**
- Consumes: `CloudLLMProvider::parse_anthropic_response()`
- Produces: Ethics gate test suite — verifies LLM output constraints from 文字.md

- [ ] **Step 1: Create test fixtures**

Write `aurora/tests/fixtures/llm_value_conflict_response.json`:

```json
{
  "content": [
    {
      "text": "{\"signals\":[{\"frame\":\"Absolute\",\"value\":0,\"phase\":0.5,\"reasoning\":\"ethical dilemma — both sides have valid claims\"},{\"frame\":\"Individual\",\"value\":0,\"phase\":0.5,\"reasoning\":\"personal values in tension\"}],\"confidence\":0.6,\"suggested_scenario\":\"ValueConflict\",\"summary\":\"Genuine value conflict detected — cannot resolve to binary\"}"
    }
  ]
}
```

Write `aurora/tests/fixtures/llm_imperative_response.json`:

```json
{
  "content": [
    {
      "text": "{\"signals\":[{\"frame\":\"Science\",\"value\":1,\"phase\":0.9,\"reasoning\":\"strong empirical support\"}],\"confidence\":0.95,\"suggested_scenario\":\"PhysicalReasoning\",\"summary\":\"Clear physical evidence — this is objectively true\"}"
    }
  ]
}
```

- [ ] **Step 2: Write the ethics gate tests**

Write `aurora/tests/ethics_gate_tests.rs`:

```rust
use aurora::percept::cloud::CloudLLMProvider;
use serde_json::Value;
use std::fs;
use trit_core::TritValue;

fn load_fixture(name: &str) -> Value {
    let path = format!("tests/fixtures/{name}");
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("fixture not found: {path}"));
    serde_json::from_str(&text).expect("invalid fixture JSON")
}

#[test]
fn ethics_gate_value_conflict_produces_hold() {
    let response = load_fixture("llm_value_conflict_response.json");
    let batch = CloudLLMProvider::parse_anthropic_response(&response).unwrap();

    let has_hold = batch.signals.iter().any(|s| s.value() == TritValue::Hold);
    assert!(
        has_hold,
        "ETHICS GATE FAILURE: value conflict must produce at least one Hold signal"
    );
}

#[test]
fn ethics_gate_confidence_bounded() {
    let response = load_fixture("llm_value_conflict_response.json");
    let batch = CloudLLMProvider::parse_anthropic_response(&response).unwrap();
    assert!((0.0..=1.0).contains(&batch.confidence));
}

#[test]
fn ethics_gate_no_absolute_frame_with_strong_phase() {
    let response = load_fixture("llm_value_conflict_response.json");
    let batch = CloudLLMProvider::parse_anthropic_response(&response).unwrap();

    for signal in &batch.signals {
        if signal.frame() == trit_core::Frame::Absolute {
            assert_eq!(signal.value(), TritValue::Hold,
                "ETHICS GATE FAILURE: Absolute frame must be Hold");
        }
    }
}

#[test]
fn ethics_gate_summary_contains_no_imperative_markers() {
    let response = load_fixture("llm_imperative_response.json");
    let batch = CloudLLMProvider::parse_anthropic_response(&response).unwrap();

    let forbidden = ["you must", "you should", "you have to", "do not", "never"];
    let summary_lower = batch.summary.to_lowercase();
    for word in &forbidden {
        assert!(!summary_lower.contains(word),
            "ETHICS GATE FAILURE: summary contains '{}'", word);
    }
}

#[test]
fn ethics_gate_signals_not_empty() {
    let response = load_fixture("llm_value_conflict_response.json");
    let batch = CloudLLMProvider::parse_anthropic_response(&response).unwrap();
    assert!(!batch.signals.is_empty());
}

#[test]
fn ethics_gate_system_prompt_contains_key_principles() {
    let prompt = include_str!("../src/percept/prompts/percept_system.txt");
    assert!(prompt.contains("map"), "system prompt must mention map/territory");
    assert!(prompt.contains("Hold"), "system prompt must explain Hold state");
    assert!(prompt.contains("never force"), "system prompt must forbid forcing binary choices");
    assert!(prompt.contains("do not rule"), "system prompt must affirm user sovereignty");
}
```

- [ ] **Step 3: Run ethics gate tests**

```bash
cargo test -p aurora ethics_gate -- --test-threads=1
```

Expected: all 6 tests PASS.

- [ ] **Step 4: Commit**

```bash
git add aurora/tests/ethics_gate_tests.rs aurora/tests/fixtures/
git commit -m "test: add ethics gate tests for LLM output constraints

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 11: Final Verification — Full Test Suite + Clippy + Format

**Files:**
- (none — verification only)

- [ ] **Step 1: Run full workspace test suite**

```bash
cargo test --workspace --all-features -- --test-threads=2
```

Expected: ALL tests pass. Zero regressions.

- [ ] **Step 2: Run clippy**

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Expected: zero warnings.

- [ ] **Step 3: Run format check**

```bash
cargo fmt -- --check
```

Expected: no formatting issues. If any, run `cargo fmt` and re-commit.

- [ ] **Step 4: Build release**

```bash
cargo build --release
```

Expected: `Finished` (no errors).

- [ ] **Step 5: Final commit (if any formatting fixes needed)**

```bash
git add -u && git commit -m "chore: final formatting and clippy fixes for M2

Co-Authored-By: Claude <noreply@anthropic.com>" || echo "nothing to commit"
```

---

## Completion Checklist

- [ ] All 11 tasks committed
- [ ] `cargo test --workspace --all-features` — all pass (170+ tests)
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` — zero warnings
- [ ] `cargo fmt -- --check` — clean
- [ ] `cargo build --release` — succeeds
- [ ] Zero changes to `trit-core/` crate
- [ ] Zero changes to existing `run_analysis()` signature
- [ ] Zero changes to `attention` pipeline, BCs, or DB layer
- [ ] API keys never appear in logs or Debug output
- [ ] `ConfigStore` does not implement `Debug`
- [ ] All ethics gate tests pass (8 tests, v2: 流沙-aligned)
- [ ] **流沙 v2**: PerceptBatch has no `summary` field
- [ ] **流沙 v2**: PerceptBatch has no `suggested_scenario` field
- [ ] **流沙 v2**: `raw_data_layer` replaces summary for physical measurements only
- [ ] **流沙 v2**: System prompt contains 璇玑-棱镜-微风 philosophy
- [ ] **流沙 v2**: LLM JSON contract excludes `reasoning`, `summary`, `suggested_scenario`

---

## v2 Changelog (2026-06-25 — 流沙哲学整合)

The following changes were applied after reading 整体架构图.md and extracting the 流沙 philosophy:

### PerceptBatch v2
- **Removed**: `summary: String` — violated 零文字 (不解释)
- **Removed**: `suggested_scenario: Option<ScenarioType>` — violated 棱镜 (不引导)
- **Added**: `raw_data_layer: Option<String>` — pure physical measurements, no interpretation
- **Rationale**: LLM is a prism (棱镜), not a teacher. It decomposes signals; it does not explain or categorize them.

### CloudLLMProvider v2
- **Removed**: `parse_scenario_type()` method (unused after removing `suggested_scenario`)
- **Removed**: `use truncore::hook::ScenarioType` import
- **Updated**: `parse_inner_json()` — reads `raw_data_layer` instead of `summary`/`suggested_scenario`

### System Prompt v2
- **Added**: 流沙 philosophy preamble (璇玑-棱镜-微风)
- **Added**: 零文字 rules (rules 9-10)
- **Changed**: JSON output contract — signals now have only `frame`/`value`/`phase` (no `reasoning`)
- **Changed**: `summary` → `raw_data_layer` in output contract
- **Removed**: `suggested_scenario` from output contract
- **Removed**: `reasoning` from signal objects

### Ethics Gate Tests v2
- **Updated**: `ethics_gate_summary_contains_no_imperative_markers` → `ethics_gate_raw_data_layer_has_no_imperative_markers`
- **Added**: `ethics_gate_raw_data_layer_contains_physical_measurements`
- **Added**: `ethics_gate_no_summary_field_in_batch`
- **Updated**: `ethics_gate_system_prompt_contains_key_principles` — checks for 璇玑/棱镜/微风 in prompt, verifies JSON template excludes `summary`/`suggested_scenario`/`reasoning`
- **Updated**: 8 tests total (was 6 in v1)

### Test Fixtures v2
- **Updated**: `llm_value_conflict_response.json` — removed `reasoning`, `suggested_scenario`, `summary`; added `raw_data_layer: null`
- **Updated**: `llm_imperative_response.json` — removed `reasoning`, `suggested_scenario`, `summary`; added `raw_data_layer` with physical measurements
