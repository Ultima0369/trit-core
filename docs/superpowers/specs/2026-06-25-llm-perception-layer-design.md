# M2: ExternalPercept — LLM Perception Layer Design (v2: 流沙哲学整合)

**Date**: 2026-06-25 (original), 2026-06-25 (v2 rewrite)
**Status**: design-approved → revised per 流沙 philosophy
**Target**: Aurora M2 — unified external perception abstraction with encrypted config, three-tier degradation chain, and LLM cognitive co-processor integration.

**v2 Changes** (from `整体架构图.md` 流沙哲学):
- Removed `summary` field — violates 零文字原则 (不解释)
- Removed `suggested_scenario` field — violates 棱镜原则 (不引导)
- Added `raw_data_layer` field — pure physical data description, no interpretation
- System prompt rewritten with 璇玑-棱镜-微风 三元心法
- LLM output contract: signals only, no reasoning, no suggestions

---

## 1. Architecture Overview

```
                         AuroraApp
                            │
                            ▼
┌──────────────────────────────────────────────────────────┐
│  PerceptChain (NEW)                                      │
│  ┌────────────────────────────────────────────────────┐  │
│  │  ExternalPercept trait — unified perception port    │  │
│  │  fn perceive(&self, raw: &str) → PerceptBatch      │  │
│  │  fn provider_name(&self) -> &str                    │  │
│  │  fn priority(&self) -> u8                           │  │
│  │  fn available(&self) -> bool                        │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  Priority degradation chain:                             │
│  CloudLLM (p=0) → LocalLLM (p=1) → FFTProvider (p=2)    │
└──────────┬───────────────────────────────────────────────┘
           │ PerceptBatch { signals: Vec<TritWord>, ... }
           ▼
┌──────────────────────────────────────────────────────────┐
│  Existing Trit-Core 5-Layer Engine (ZERO CHANGES)        │
│  ScenarioRecognizer → MountArbiter → Adapters →         │
│  TernaryAlgebra → ResolutionPolicy → SafeFallback        │
└──────────────────────────────────────────────────────────┘
```

**Key principle**: LLM is a cognitive co-processor — it perceives and suggests, but Trit-Core makes the final ternary judgment. LLM output is treated as *signals with confidence*, never as authoritative decisions.

**流沙哲学 integration**: The LLM acts as a **棱镜 (prism)** — it splits raw human text into independent spectral components (Frame/Value/Phase) without interpreting what those components mean. It is a **璇玑 (armillary sphere)** — faithfully rotating, never explaining why the stars move. Its output is **微风 (breeze)** — passing through without leaving a trace of opinion or guidance.

---

## 2. Core Types

### 2.1 PerceptBatch (v2 — 流沙-aligned)

```rust
/// A batch of TritWord signals extracted from raw input by a perception provider.
///
/// ## 流沙 Design Philosophy
///
/// This struct embodies the three principles:
/// - **璇玑 (Armillary)**: signals are faithful rotations of raw input — no meaning attached
/// - **棱镜 (Prism)**: each signal is one spectral band — the user sees what their angle reveals
/// - **微风 (Breeze)**: no summary, no suggestion, no trace — signals pass through and dissolve
///
/// There is deliberately NO `summary` field (would violate 零文字 — 不解释).
/// There is deliberately NO `suggested_scenario` field (would violate 棱镜 — 不引导).
/// Scenario recognition is Trit-Core's job, not the LLM's.
#[derive(Debug, Clone)]
pub struct PerceptBatch {
    /// Extracted ternary signals — the prismatic decomposition of raw input.
    /// Each signal is one spectral band: a Frame, a Value, a Phase.
    /// No signal carries an explanation of "why" — only "what".
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
    /// Format: free-form text describing physical measurements only.
    /// MUST NOT contain: advice, interpretation, suggestions, conclusions.
    /// Example: "surface_temp: 28.4°C, wind: 12km/h NE, humidity: 65%"
    ///
    /// This is the ONLY text field — it describes the territory, not the map.
    pub raw_data_layer: Option<String>,
}
```

### 2.2 ExternalPercept Trait (unchanged)

```rust
/// Unified abstraction for all external perception sources.
///
/// Implementations include cloud LLMs, local models, FFT signal
/// analysis, and future hard-science data APIs (ecology, climate, geology).
///
/// ## 流沙 Philosophy
///
/// Every implementation of this trait is a **棱镜 (prism)** — it takes
/// raw input and decomposes it into spectral components. It does NOT:
/// - Explain what the components mean (that's the user's job)
/// - Suggest what to do (that's Trit-Core's job)
/// - Summarize or conclude (that would be 灌输, not 感知)
pub trait ExternalPercept: Send + Sync {
    /// Perceive signals from raw text input.
    fn perceive(&self, raw: &str) -> Result<PerceptBatch, PerceptError>;

    /// Human-readable provider name for audit trails.
    fn provider_name(&self) -> &str;

    /// Lower number = higher priority in the degradation chain.
    fn priority(&self) -> u8;

    /// Whether this provider is currently usable.
    /// Default: true. Override for health checks.
    fn available(&self) -> bool { true }
}
```

### 2.3 PerceptChain (unchanged)

```rust
pub struct PerceptChain {
    providers: Vec<Box<dyn ExternalPercept>>,
}

impl PerceptChain {
    pub fn new() -> Self;
    pub fn with(self, provider: Box<dyn ExternalPercept>) -> Self;

    /// Try providers in priority order, degrade on failure.
    /// Returns Err(AllUnavailable) only if every provider fails.
    pub fn perceive_or_degrade(&self, raw: &str) -> Result<PerceptBatch, PerceptError>;
}
```

---

## 3. Provider Implementations

### 3.1 CloudLLMProvider (priority=0)

- Wraps `reqwest::Client` with API-key auth from `ConfigStore`
- System prompt constrains LLM to output structured JSON signals ONLY — no reasoning, no summary, no scenario suggestions
- Marks value conflicts with `hold`, never forces a binary choice
- Endpoints: Anthropic Messages API, OpenAI Chat Completions API
- Timeout: 30s per request
- On failure: returns `PerceptError` → `PerceptChain` degrades to next provider

### 3.2 LocalLLMProvider (priority=1)

- Communicates with local inference servers (ollama, llama.cpp) via HTTP on localhost
- Same JSON output contract as CloudLLMProvider
- No API key needed (localhost trust boundary)
- Configurable endpoint via `ConfigStore.local_model_path`

### 3.3 FFTProvider (priority=2, never offline)

- Pure-local passthrough: delegates to the existing `run_analysis` FFT wavelet engine
- Input: raw text is ignored; uses `SignalSpec` from `AnalysisInput` instead
- Guaranteed to never fail — this is the ultimate safety floor
- Ensures Trit-Core always has signal input, even when all LLMs are unreachable

---

## 4. Encrypted Configuration (Windows DPAPI) — unchanged

### 4.1 ConfigStore

```rust
pub struct ConfigStore {
    path: PathBuf,                              // %APPDATA%\aurora\config.enc
    cache: Mutex<Option<DecryptedConfig>>,      // in-memory only, never written to disk plaintext
}

struct DecryptedConfig {
    api_keys: HashMap<String, String>,          // provider_name → api_key
    local_model_path: Option<String>,           // e.g. "http://localhost:11434"
    cloud_model: Option<String>,                // e.g. "claude-opus-4-8"
}
```

### 4.2 Security Properties

| Property | Mechanism |
|---|---|
| Disk storage | Windows DPAPI user-level encryption (AES-256) |
| Memory lifetime | Decrypted only into `cache: Mutex<Option<...>>` |
| User isolation | DPAPI binds to Windows user account — different user = cannot decrypt |
| Machine isolation | DPAPI binds to machine — copy to another machine = cannot decrypt |
| Debug safety | `ConfigStore` does NOT implement `Debug` |
| Log safety | API key values are never logged |

### 4.3 API Key Lifecycle

1. First run: no `config.enc` exists → `load_or_default()` returns empty `DecryptedConfig`
2. User sets key via CLI: `aurora config set-key claude-opus-4-8 <key>`
3. `ConfigStore::set_api_key()` encrypts with DPAPI, writes `config.enc`
4. Subsequent runs: `get_api_key()` decrypts on first access, caches in memory
5. Key rotation: simply call `set_api_key()` again
6. Key deletion: `aurora config remove-key claude-opus-4-8`

---

## 5. Error Handling — unchanged

### 5.1 PerceptError

```rust
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
    #[error("Config error: {0}")]
    ConfigError(#[from] ConfigError),
}
```

### 5.2 ConfigError

```rust
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

---

## 6. Pipeline Integration — unchanged structure

### 6.1 New Analysis Entry Point

Existing `run_analysis()` is unchanged. A new overload is added:

```rust
pub fn run_analysis_from_percept(
    spec: &SignalSpec,
    frequency_threshold: f64,
    user_feels_normal: bool,
    contact_signals: &[TritWord],
    percept_signals: &[TritWord],          // NEW
) -> Result<AnalysisReport, BcError>;
```

The percept signals are merged into the signal vector alongside embodied, individual, and contact signals before ternary evaluation.

### 6.2 AuroraApp Integration

```rust
impl AuroraApp {
    /// Run pipeline with LLM perception.
    pub fn run_with_percept(
        self,
        input: AnalysisInput,
        user_text: &str,
    ) -> Result<AppOutput> {
        let percept = self.percept_chain.perceive_or_degrade(user_text)?;
        let analysis = analysis::run_analysis_from_percept(
            &input.spec,
            input.frequency_threshold,
            input.user_feels_normal,
            &self.contacts_as_tritwords(),
            &percept.signals,
        )?;
        // ... attention + presentation unchanged ...
    }
}
```

### 6.3 Change Inventory

| Change | Type | File |
|---|---|---|
| `ExternalPercept` trait | NEW | `aurora/src/percept/mod.rs` |
| `PerceptBatch` (v2: no summary, no suggested_scenario, +raw_data_layer) | NEW | `aurora/src/percept/types.rs` |
| `PerceptChain` | NEW | `aurora/src/percept/chain.rs` |
| `CloudLLMProvider` (v2: updated parser) | NEW | `aurora/src/percept/cloud.rs` |
| `LocalLLMProvider` (v2: updated parser) | NEW | `aurora/src/percept/local.rs` |
| `FFTProvider` | NEW | `aurora/src/percept/fft.rs` |
| `ConfigStore` | NEW | `aurora/src/config/store.rs` |
| `PerceptError` / `ConfigError` | NEW | `aurora/src/percept/error.rs` |
| `run_analysis_from_percept` | NEW overload | `aurora/src/pipeline/analysis.rs` |
| `AuroraApp::run_with_percept` | NEW method | `aurora/src/app.rs` |
| System prompt template (v2: 流沙 philosophy) | NEW | `aurora/src/percept/prompts/percept_system.txt` |

### 6.4 Zero-Change Zones

- All of `trit-core` (5 layers, ternary algebra, adapters, anchors)
- `attention` pipeline link
- All bounded contexts (`bc/`)
- SQLite database layer (`db/`)
- Existing `run_analysis()` signature

---

## 7. Ethical Constraints (from 文字.md + 整体架构图.md 流沙哲学)

### 7.1 文字.md Baseline (unchanged)

The LLM system prompt MUST enforce:

1. **Text is map, not territory** — LLM output is structured signals, not authoritative truth
2. **Output is neural surgery** — every response physically reshapes the user's brain; do no harm
3. **Teaching, not brainwashing** — encourage independent thinking; never demand blind belief
4. **Hold on value conflicts** — when frames collide, output `hold`, do not force binary choice
5. **User sovereignty** — the user bears ultimate responsibility for their decisions; the system serves, does not rule
6. **Self-destruct on tampering** — future deployment hardens this; M2 builds the foundation

### 7.2 流沙 Philosophy (NEW — from 整体架构图.md)

7. **璇玑 (Armillary Sphere)** — faithfully rotate, never explain why. The LLM presents signals as they are, without attaching meaning.

8. **棱镜 (Prism)** — split, don't synthesize. The LLM decomposes raw input into independent spectral bands (Frame/Value/Phase). It does NOT tell the user what the spectrum "means."

9. **微风 (Breeze)** — pass through, leave no trace. No summaries, no suggestions, no "you should." The signals dissolve after perception; only the user's own reaction remains.

10. **零文字 (Zero Text)** — no explanations, no guidance, no promises. The only text field (`raw_data_layer`) describes physical measurements — the territory, not the map.

11. **明察波澜 (Seeing Your Own Ripples)** — the ultimate purpose is not to tell the user what to think, but to let them observe their own reaction to the data. The LLM is a lens, not a teacher.

### 7.3 LLM Output Contract (v2)

The LLM must output ONLY:

```json
{
  "signals": [
    {
      "frame": "Science|Individual|Consensus|Absolute",
      "value": 1 | 0 | -1,
      "phase": 0.0-1.0
    }
  ],
  "confidence": 0.0-1.0,
  "raw_data_layer": "optional physical measurements only, no interpretation"
}
```

**Removed from v1 contract:**
- `reasoning` — LLM does not explain "why" (violates 璇玑)
- `suggested_scenario` — LLM does not categorize (violates 棱镜)
- `summary` — LLM does not conclude (violates 微风)

---

## 8. Test Strategy (v2 updated)

| Layer | Test Type | Coverage |
|---|---|---|
| Unit | `PerceptChain` degradation logic | Mock providers simulate success→failure→degrade |
| Unit | `ConfigStore` encrypt/decrypt round-trip | Write→read→verify plaintext match; verify cross-user isolation |
| Unit | `CloudLLMProvider::parse_response` | Valid/invalid/malicious JSON robustness |
| Integration | `PerceptChain` + `run_analysis_from_percept` | End-to-end: percept signals flowing into ternary decision |
| Integration | Three-tier degradation E2E | Network-down → local model → FFT fallback path |
| Ethics gate | LLM output marks `hold` on conflict | Value-conflict scenarios must not collapse to binary |
| Ethics gate | LLM output contains NO imperative commands | System prompt constraint verification |
| Ethics gate | LLM output has NO summary or suggested_scenario | v2: verify these fields are absent from LLM JSON contract |
| Ethics gate | `raw_data_layer` contains only physical measurements | v2: no advice, no interpretation in raw_data_layer |
| Doc-test | `ExternalPercept` trait example | Trait-level documentation test |

### Test Environment

- Cloud LLM HTTP tests use `wiremock` to simulate API endpoints
- `ConfigStore` tests use temp directories, never touch real DPAPI
- Ethics gate tests use pre-recorded LLM response JSON fixtures (updated for v2 contract)
- Local model tests use a mock HTTP server returning known-good responses

---

## 9. Future Extensions (M3+) — unchanged

The `ExternalPercept` trait is designed to accommodate:

- **SciDataSource** — ecology/climate/geology data APIs feeding hard-science signals into Trit-Core
- **SensorProvider** — IoT/embedded sensor streams for real-time environmental perception
- **MultiModalProvider** — image/audio perception beyond text
- **流沙 Visual Engine** — CesiumJS-based 3D earth visualization with zero-text data layers (璇玑 visual mode)

All implement the same `ExternalPercept` trait — no architecture changes needed.

---

## 10. Windows-Specific Notes — unchanged

- DPAPI encryption via `windows-sys` crate (no external C dependencies)
- Config path: `%APPDATA%\aurora\config.enc` (resolved via `known_folder` API)
- `reqwest` uses Windows native TLS (`schannel`) — no OpenSSL dependency
- Local model communication: localhost HTTP (firewall-friendly)
- Future cross-platform: abstract DPAPI behind `PlatformCrypto` trait with per-platform backends

---

## 11. File Structure — unchanged

```
aurora/src/
├── app.rs                          # MODIFIED: +run_with_percept()
├── config/
│   ├── mod.rs                      # NEW
│   └── store.rs                    # NEW: ConfigStore + DPAPI
├── percept/
│   ├── mod.rs                      # NEW: ExternalPercept trait
│   ├── types.rs                    # NEW: PerceptBatch (v2: 流沙-aligned)
│   ├── chain.rs                    # NEW: PerceptChain
│   ├── cloud.rs                    # NEW: CloudLLMProvider (v2: updated parser)
│   ├── local.rs                    # NEW: LocalLLMProvider (v2: updated parser)
│   ├── fft.rs                      # NEW: FFTProvider
│   ├── error.rs                    # NEW: PerceptError + ConfigError
│   └── prompts/
│       └── percept_system.txt      # NEW: LLM system prompt (v2: 流沙 philosophy)
├── pipeline/
│   ├── mod.rs
│   └── analysis.rs                 # MODIFIED: +run_analysis_from_percept()
└── ... (existing files unchanged)
```

Dependencies added to `Cargo.toml`:
- `reqwest` (HTTP client, Windows schannel TLS)
- `serde_json` (LLM response parsing, config serialization)
- `thiserror` (error derives)
- `chrono` (already present)
- `windows-sys` (DPAPI bindings)
- `wiremock` (dev-dependency, HTTP mocking)
