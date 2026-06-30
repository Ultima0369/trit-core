# Tauri Desktop Shell — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a React + TypeScript GUI to Aurora via Tauri v2, while preserving the CLI mode, by extracting an `AuroraApp` facade.

**Architecture:** Three-layer separation — React UI (`ui/`) → Tauri IPC (`src-tauri/`) → Aurora Core (`aurora/src/`, unchanged pipeline/bc/db). The new `AuroraApp` struct in `app.rs` encapsulates the pipeline orchestration logic currently in `main.rs`, enabling both CLI and Tauri to share identical business logic.

**Tech Stack:** Rust (Tauri v2, Aurora, rusqlite), React 18 + TypeScript, Vite 5, react-router-dom (HashRouter), CSS Modules, Canvas API (no charting libs).

## Global Constraints

- `#![forbid(unsafe_code)]` — enforced on all Rust crates including src-tauri
- CSP: `default-src 'self'; style-src 'self' 'unsafe-inline'` — no external CDN or connect-src
- No network dependencies — per CHARTER §5
- Tauri fs scope limited to `$HOME/.aurora/`
- `cargo test --workspace --all-features` must pass after every task
- `cargo fmt -- --check` and `cargo clippy --workspace --all-targets --all-features -- -D warnings` must pass after every task that touches Rust code
- Commits use `Co-Authored-By: Claude <noreply@anthropic.com>` trailer

---

### Task 1: Extract `AuroraApp` facade from `main.rs`

**Files:**
- Create: `aurora/src/app.rs`
- Modify: `aurora/src/lib.rs`
- Modify: `aurora/src/main.rs`

**Interfaces:**
- Consumes: `pipeline::analysis::{run_analysis, SignalSpec, AnalysisReport, contacts_to_tritwords}`, `pipeline::attention::{run_attention, AttentionOutcome}`, `db::Database`, `bc::presentation::{AuroraRenderer, ViewState, ConflictCard, RenderPort}`, `bc::relationship_annotation::{ContactInput, ContactProfile}`, `ingest::IngestManager`
- Produces: `AuroraApp { pub fn new(db_path: Option<&Path>) -> Result<Self>, pub fn load_contacts(&mut self, data_source: &Path) -> Result<usize>, pub fn run_pipeline(&self, input: AnalysisInput) -> Result<AppOutput> }`, `AnalysisInput { pub spec: SignalSpec, pub frequency_threshold: f64, pub user_feels_normal: bool }`, `AppOutput { pub analysis_report: AnalysisReport, pub attention_outcome: AttentionOutcome, pub html: String, pub json: String }`

- [ ] **Step 1: Write `aurora/src/app.rs`**

The facade struct moves the pipeline orchestration from `main.rs` into a reusable library type:

```rust
//! Application facade — shared between CLI and Tauri.
//!
//! Orchestrates the two pipeline links (analysis + attention)
//! and presentation rendering in one call. Both the CLI binary
//! and Tauri commands use this same entry point.

use anyhow::{Context, Result};
use std::path::Path;

use crate::bc::presentation::{AuroraRenderer, ConflictCard, RenderPort, ViewState};
use crate::bc::relationship_annotation::{ContactInput, ContactProfile};
use crate::db::Database;
use crate::ingest::IngestManager;
use crate::pipeline::analysis::{self, AnalysisReport, SignalSpec};
use crate::pipeline::attention::{self, AttentionOutcome};

/// Input parameters for a single pipeline run.
#[derive(Debug, Clone)]
pub struct AnalysisInput {
    pub spec: SignalSpec,
    pub frequency_threshold: f64,
    pub user_feels_normal: bool,
}

/// Complete output of a pipeline run.
#[derive(Debug, Clone)]
pub struct AppOutput {
    pub analysis_report: AnalysisReport,
    pub attention_outcome: AttentionOutcome,
    pub html: String,
    pub json: String,
}

/// Application facade — owns the database connection and loaded contacts.
pub struct AuroraApp {
    db: Database,
    contacts: Vec<ContactProfile>,
}

impl AuroraApp {
    /// Create a new AuroraApp with a database connection.
    ///
    /// If `db_path` is `None` or `":memory:"`, opens an in-memory database.
    /// Otherwise opens (or creates) the SQLite database at the given path.
    pub fn new(db_path: Option<&Path>) -> Result<Self> {
        let db = match db_path {
            None | Some(p) if p == Path::new(":memory:") => Database::open_in_memory()?,
            Some(p) => Database::open(p)?,
        };
        Ok(Self {
            db,
            contacts: Vec::new(),
        })
    }

    /// Load contacts from a JSON data source file.
    ///
    /// Returns the number of contacts loaded.
    pub fn load_contacts(&mut self, data_source: &Path) -> Result<usize> {
        let manager = IngestManager::with_json_fallback(data_source)?;
        let count = manager.contact_count();
        let inputs: Vec<ContactInput> = manager.load()?;
        self.contacts = inputs.into_iter().map(ContactProfile::from).collect();
        Ok(count)
    }

    /// Run the full analysis + attention pipeline and render output.
    ///
    /// Note: this consumes `self.db` because `run_attention` takes ownership.
    /// After this call, the `AuroraApp` is consumed — create a new one for
    /// subsequent pipeline runs. State persists via the SQLite database file.
    pub fn run_pipeline(self, input: AnalysisInput) -> Result<AppOutput> {
        let contact_signals = analysis::contacts_to_tritwords(&self.contacts);

        // ── Link 1: Analysis ────────────────────────────────────────────
        let analysis_report = analysis::run_analysis(
            &input.spec,
            input.frequency_threshold,
            input.user_feels_normal,
            &contact_signals,
        )
        .map_err(|e| anyhow::anyhow!("analysis link failed: {e}"))?;

        // ── Link 2: Attention ───────────────────────────────────────────
        let attention_outcome =
            attention::run_attention(
                &analysis_report.decision.input_signals,
                self.db,
                &self.contacts,
            )
            .map_err(|e| anyhow::anyhow!("attention link failed: {e}"))?;

        // ── Presentation ────────────────────────────────────────────────
        let mut view = ViewState::new(
            format!(
                "Detected frequency: {:.3} Hz | Decision: {:?}",
                analysis_report.spectrum.fundamental_hz,
                analysis_report.decision.result.value()
            ),
            attention_outcome.session.clone(),
        );

        for interrupt in &analysis_report.decision.interrupts {
            view.add_conflict(ConflictCard {
                conflict_type: format!("{:?}", interrupt.conflict),
                reason: interrupt.reason.clone(),
                frame_a: "Embodied".into(),
                frame_b: "Individual".into(),
                acknowledged: false,
            });
        }

        let renderer = AuroraRenderer;
        let html = renderer.render_html(&view);
        let json = renderer
            .render_json(&view)
            .map_err(|e| anyhow::anyhow!("JSON render failed: {e}"))?;

        Ok(AppOutput {
            analysis_report,
            attention_outcome,
            html,
            json,
        })
    }
}
```

- [ ] **Step 2: Add `pub mod app;` to `aurora/src/lib.rs`**

Read `aurora/src/lib.rs` and add after the `db` module line:

```rust
/// Application facade — shared entry point for CLI and Tauri.
pub mod app;
```

The resulting `lib.rs` should be:

```rust
//! Aurora: a local-first cognitive sovereignty tool built on Trit-Core.
//! ...

#![forbid(unsafe_code)]

pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub mod wavelet;
pub mod ingest;
pub mod cli;
pub mod pipeline;
pub mod bc;
pub mod db;
pub mod app;
```

- [ ] **Step 3: Rewrite `aurora/src/main.rs` as thin CLI shell**

Replace the entire contents of `aurora/src/main.rs`:

```rust
use anyhow::{Context, Result};
use aurora::app::{AnalysisInput, AuroraApp};
use aurora::cli::Args;
use clap::Parser;
use std::fs;
use std::path::Path;

fn main() -> Result<()> {
    let args = Args::parse();

    // Read and parse input
    let input_text = fs::read_to_string(&args.input)
        .with_context(|| format!("failed to read input file {:?}", args.input))?;
    let spec: aurora::pipeline::analysis::SignalSpec = serde_json::from_str(&input_text)
        .with_context(|| "failed to parse input JSON as SignalSpec")?;

    // Build the app, load contacts if requested
    let db_path = if args.db_path == ":memory:" {
        None
    } else {
        Some(Path::new(&args.db_path))
    };
    let mut app = AuroraApp::new(db_path)?;

    if let Some(ref path) = args.data_source {
        let count = app.load_contacts(path)?;
        eprintln!("Loaded {count} contacts from {}", path.display());
    }

    // Run pipeline
    let output = app.run_pipeline(AnalysisInput {
        spec,
        frequency_threshold: args.frequency_threshold,
        user_feels_normal: args.user_feels_normal,
    })?;

    // Output
    match args.output {
        Some(path) => {
            fs::write(&path, &output.html)
                .with_context(|| format!("failed to write HTML report to {:?}", path))?;
            println!("Report written to {}", path.display());
        }
        None => {
            println!("{}", output.json);
        }
    }

    Ok(())
}
```

- [ ] **Step 4: Build and verify the CLI still works**

```bash
cargo build --release
```

- [ ] **Step 5: Run all existing tests to verify nothing broke**

```bash
cargo test --workspace --all-features
```

- [ ] **Step 6: Run full quality gate**

```bash
cargo fmt -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test ethics_
```

- [ ] **Step 7: End-to-end CLI smoke test**

```bash
cargo run --release --bin aurora -- --input synthetic_2hz.json --output /tmp/test_report.html
```

Verify `/tmp/test_report.html` is created and contains "Aurora Decision Report".

- [ ] **Step 8: Commit**

```bash
git add aurora/src/app.rs aurora/src/lib.rs aurora/src/main.rs
git commit -m "feat: extract AuroraApp facade from main.rs

Shared entry point for CLI and upcoming Tauri shell.
AuroraApp owns Database + Contacts; run_pipeline() orchestrates
analysis → attention → presentation in one call.

CLI main.rs is now a thin shell (~25 lines).

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 2: Scaffold Tauri + React project

**Files:**
- Create: `aurora/src-tauri/Cargo.toml`
- Create: `aurora/src-tauri/tauri.conf.json`
- Create: `aurora/src-tauri/build.rs`
- Create: `aurora/src-tauri/src/main.rs`
- Create: `aurora/src-tauri/capabilities/default.json`
- Create: `aurora/src-tauri/icons/` (directory, empty for now)
- Create: `aurora/ui/package.json`
- Create: `aurora/ui/tsconfig.json`
- Create: `aurora/ui/tsconfig.node.json`
- Create: `aurora/ui/vite.config.ts`
- Create: `aurora/ui/index.html`
- Create: `aurora/ui/src/main.tsx`
- Create: `aurora/ui/src/App.tsx`
- Create: `aurora/ui/src/vite-env.d.ts`

**Interfaces:**
- Consumes: Nothing (greenfield scaffolding)
- Produces: Tauri project that builds and opens an empty React window

- [ ] **Step 1: Create `aurora/src-tauri/Cargo.toml`**

```toml
[package]
name = "aurora-desktop"
version = "0.1.0"
edition = "2021"
description = "Aurora desktop application — Tauri shell"

[dependencies]
aurora = { path = ".." }
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"

[build-dependencies]
tauri-build = { version = "2", features = [] }
```

- [ ] **Step 2: Create `aurora/src-tauri/build.rs`**

```rust
fn main() {
    tauri_build::build()
}
```

- [ ] **Step 3: Create `aurora/src-tauri/tauri.conf.json`**

```json
{
  "$schema": "https://raw.githubusercontent.com/tauri-apps/tauri/dev/crates/tauri-cli/schema.json",
  "productName": "Aurora",
  "version": "0.1.0",
  "identifier": "com.aurora.trit-core",
  "build": {
    "frontendDist": "../ui/dist",
    "devUrl": "http://localhost:5173",
    "beforeDevCommand": "cd ../ui && npm run dev",
    "beforeBuildCommand": "cd ../ui && npm run build"
  },
  "app": {
    "title": "Aurora — 注意力主权训练系统",
    "security": {
      "csp": "default-src 'self'; style-src 'self' 'unsafe-inline'"
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```

- [ ] **Step 4: Create `aurora/src-tauri/capabilities/default.json`**

Tauri v2 uses capabilities instead of the old allowlist:

```json
{
  "$schema": "https://raw.githubusercontent.com/tauri-apps/tauri/dev/crates/tauri-utils/schema/capability.json",
  "identifier": "default",
  "description": "Default capability for Aurora",
  "windows": ["main"],
  "permissions": [
    "core:default"
  ]
}
```

- [ ] **Step 5: Create minimal `aurora/src-tauri/src/main.rs`**

```rust
#![forbid(unsafe_code)]

fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 6: Create `aurora/ui/package.json`**

```json
{
  "name": "aurora-ui",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "@tauri-apps/api": "^2.0.0",
    "react": "^18.3.1",
    "react-dom": "^18.3.1",
    "react-router-dom": "^6.26.0"
  },
  "devDependencies": {
    "@types/react": "^18.3.3",
    "@types/react-dom": "^18.3.0",
    "@vitejs/plugin-react": "^4.3.1",
    "typescript": "^5.5.0",
    "vite": "^5.4.0"
  }
}
```

- [ ] **Step 7: Create `aurora/ui/tsconfig.json`**

```json
{
  "compilerOptions": {
    "target": "ES2021",
    "useDefineForClassFields": true,
    "lib": ["ES2021", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "isolatedModules": true,
    "moduleDetection": "force",
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    "forceConsistentCasingInFileNames": true
  },
  "include": ["src"]
}
```

- [ ] **Step 8: Create `aurora/ui/tsconfig.node.json`**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["ES2023"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "isolatedModules": true,
    "moduleDetection": "force",
    "noEmit": true,
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true
  },
  "include": ["vite.config.ts"]
}
```

- [ ] **Step 9: Create `aurora/ui/vite.config.ts`**

```typescript
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    target: 'es2021',
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_DEBUG,
  },
});
```

- [ ] **Step 10: Create `aurora/ui/index.html`**

```html
<!DOCTYPE html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Aurora — 注意力主权训练系统</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

- [ ] **Step 11: Create `aurora/ui/src/main.tsx`**

```tsx
import React from 'react';
import ReactDOM from 'react-dom/client';
import { App } from './App';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
```

- [ ] **Step 12: Create `aurora/ui/src/App.tsx`**

```tsx
export function App() {
  return (
    <div style={{ padding: '2rem', fontFamily: 'system-ui' }}>
      <h1>Aurora</h1>
      <p>注意力主权训练系统 — Tauri shell ready.</p>
    </div>
  );
}
```

- [ ] **Step 13: Create `aurora/ui/src/vite-env.d.ts`**

```typescript
/// <reference types="vite/client" />
```

- [ ] **Step 14: Add `aurora/src-tauri` to workspace members**

Read `Cargo.toml` (the workspace root) and edit the `[workspace]` section:

Old:
```toml
[workspace]
members = [".", "aurora"]
resolver = "2"
```

New:
```toml
[workspace]
members = [".", "aurora", "aurora/src-tauri"]
resolver = "2"
```

Use the Edit tool:
- `old_string`: `members = [".", "aurora"]`
- `new_string`: `members = [".", "aurora", "aurora/src-tauri"]`

- [ ] **Step 15: Install npm dependencies and verify frontend builds**

```bash
cd aurora/ui && npm install
```

```bash
cd aurora/ui && npm run build
```

Expected: `dist/` directory created with `index.html` and assets. No TypeScript errors.

- [ ] **Step 16: Build Tauri binary to verify Rust side compiles**

```bash
cargo build -p aurora-desktop
```

Expected: compilation succeeds (binary will fail at runtime without the frontend dist in place, but that's expected — we just want to verify compilation).

- [ ] **Step 17: Commit**

```bash
git add aurora/src-tauri/ aurora/ui/ Cargo.toml
git commit -m "feat: scaffold Tauri v2 + React TypeScript project

Tauri shell (src-tauri/) with React 18 + Vite 5 frontend (ui/).
Empty App.tsx renders placeholder text. Both Rust and frontend compile.

Workspace now includes aurora/src-tauri as a member crate.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 3: Implement Tauri IPC commands

**Files:**
- Create: `aurora/src-tauri/src/commands/mod.rs`
- Create: `aurora/src-tauri/src/commands/analysis.rs`
- Modify: `aurora/src-tauri/src/main.rs` (register commands and state)

**Interfaces:**
- Consumes: `AuroraApp::new()`, `AuroraApp::load_contacts()`, `AuroraApp::run_pipeline()`, `AnalysisInput`, `AppOutput` from Task 1
- Produces: Tauri commands `run_analysis`, `load_contacts` registered on the Builder

- [ ] **Step 1: Create `aurora/src-tauri/src/commands/mod.rs`**

```rust
pub mod analysis;
```

- [ ] **Step 2: Create `aurora/src-tauri/src/commands/analysis.rs`**

```rust
use aurora::app::{AnalysisInput, AuroraApp};
use aurora::pipeline::analysis::SignalSpec;
use std::path::PathBuf;
use std::sync::Mutex;

/// Shared state — holds the path to the SQLite database file.
pub struct AuroraState {
    pub db_path: Mutex<Option<PathBuf>>,
}

/// Run the full analysis + attention pipeline.
///
/// Consumes an `AnalysisInput` (with `SignalSpec` deserialized from JSON)
/// and returns the complete `AppOutput` including HTML and JSON renderings.
#[tauri::command]
pub fn run_analysis(
    state: tauri::State<'_, AuroraState>,
    input: AnalysisInput,
) -> Result<aurora::app::AppOutput, String> {
    let db_path = state.db_path.lock().map_err(|e| e.to_string())?;
    let app = AuroraApp::new(db_path.as_deref()).map_err(|e| e.to_string())?;
    app.run_pipeline(input).map_err(|e| e.to_string())
}

/// Load contacts from a JSON file and cache them.
///
/// Returns the number of contacts loaded.
#[tauri::command]
pub fn load_contacts(
    state: tauri::State<'_, AuroraState>,
    path: String,
) -> Result<usize, String> {
    let db_path = state.db_path.lock().map_err(|e| e.to_string())?;
    let mut app = AuroraApp::new(db_path.as_deref()).map_err(|e| e.to_string())?;
    app.load_contacts(std::path::Path::new(&path))
        .map_err(|e| e.to_string())
}
```

- [ ] **Step 3: Update `aurora/src-tauri/src/main.rs` to register commands and state**

Replace the entire file:

```rust
#![forbid(unsafe_code)]

mod commands;

use commands::analysis::{run_analysis, load_contacts, AuroraState};
use std::path::PathBuf;
use std::sync::Mutex;

fn main() {
    tauri::Builder::default()
        .manage(AuroraState {
            db_path: Mutex::new(Some(PathBuf::from(
                dirs_next_aurora_db(),
            ))),
        })
        .invoke_handler(tauri::generate_handler![
            run_analysis,
            load_contacts,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Determine the default database path: ~/.aurora/data/aurora.db
///
/// Falls back to in-memory if the home directory cannot be determined.
fn dirs_next_aurora_db() -> PathBuf {
    let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push(".aurora");
    path.push("data");
    std::fs::create_dir_all(&path).ok();
    path.push("aurora.db");
    path
}
```

Note: We need `dirs` crate for home directory resolution. Add it to `src-tauri/Cargo.toml`:

- [ ] **Step 4: Add `dirs` dependency to `aurora/src-tauri/Cargo.toml`**

Add under `[dependencies]`:

```toml
dirs = "5"
```

- [ ] **Step 5: Make `AnalysisInput` and `AppOutput` serializable for Tauri IPC**

Tauri commands need `Serialize`/`Deserialize` on parameter and return types. Read `aurora/src/app.rs` and add derives:

Edit `AnalysisInput`:
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnalysisInput {
    pub spec: SignalSpec,
    pub frequency_threshold: f64,
    pub user_feels_normal: bool,
}
```

Edit `AppOutput`:
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppOutput {
    pub analysis_report: AnalysisReport,
    pub attention_outcome: AttentionOutcome,
    pub html: String,
    pub json: String,
}
```

Edit `AuroraApp` (remove `Deserialize` from `AuroraApp` — it's not sent over IPC directly):

The struct itself doesn't need serde derives. Only `AnalysisInput` and `AppOutput` do.

But wait — `AnalysisReport`, `AttentionOutcome` etc. need to be serializable for the IPC boundary. Let's add derives to the needed types.

- [ ] **Step 6: Add `Serialize`/`Deserialize` to types crossing the IPC boundary**

Read `aurora/src/pipeline/analysis.rs` and add derives to `AnalysisReport`:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnalysisReport {
    pub spectrum: FrequencySpectrum,
    pub decision: DecisionRecord,
    pub contact_count: usize,
}
```

Read `aurora/src/bc/signal_analysis.rs` and add derives to `FrequencySpectrum` and related types that it contains. Since `FrequencySpectrum` currently has private fields and a `new()` constructor with validation, we need to approach this carefully.

The simplest approach: add serde to `FrequencySpectrum` and all its member types. Read `aurora/src/bc/signal_analysis.rs` first to find all types in the chain.

Let's look at what `FrequencySpectrum` contains:
- `pub fundamental_hz: f64`
- `pub peaks: Vec<FrequencyPeak>`
- `pub quality: SignalQuality`
- `sample_count: usize` (private but need serde)

All these types need `Serialize + Deserialize`.

Read `aurora/src/bc/signal_analysis.rs` and add:
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FrequencyPeak { ... }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FrequencySpectrum { ... }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SignalQuality { ... }
```

Read `aurora/src/bc/ternary_decision.rs` and add:
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DecisionRecord { ... }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DecisionSnapshot { ... }
```

Read `aurora/src/pipeline/attention.rs` and add:
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AttentionOutcome { ... }
```

Read `aurora/src/bc/attention_guidance.rs` and add derives to `AttentionCmd`, `AttentionSession`, `ASIMetric`, `Reminder`, `ReminderResponse`.

This is a systematic task. The rule: every type in the `AppOutput` tree must derive `Serialize + Deserialize`.

- [ ] **Step 7: Also add `serde` dependency to `aurora/Cargo.toml` if not already there**

Check `aurora/Cargo.toml` — it already has `serde = { version = "1.0", features = ["derive"] }`. Good.

- [ ] **Step 8: Build and verify compilation**

```bash
cargo build -p aurora-desktop
```

Expected: clean compilation. Fix any missing serde derives iteratively.

- [ ] **Step 9: Run all tests to verify serde additions don't break anything**

```bash
cargo test --workspace --all-features
```

- [ ] **Step 10: Run quality gate**

```bash
cargo fmt -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

- [ ] **Step 11: Commit**

```bash
git add aurora/src-tauri/src/commands/ aurora/src-tauri/src/main.rs aurora/src-tauri/Cargo.toml aurora/src/app.rs aurora/src/pipeline/analysis.rs aurora/src/pipeline/attention.rs aurora/src/bc/signal_analysis.rs aurora/src/bc/ternary_decision.rs aurora/src/bc/attention_guidance.rs
git commit -m "feat: implement Tauri IPC commands + serde for pipeline types

Commands: run_analysis, load_contacts via #[tauri::command].
AuroraState holds db_path in Mutex<Option<PathBuf>>.

Added Serialize/Deserialize to all types crossing the IPC boundary:
AnalysisReport, FrequencySpectrum, DecisionRecord, AttentionOutcome,
AttentionSession, AttentionCmd, etc.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 4: Frontend type definitions and useAurora hook

**Files:**
- Create: `aurora/ui/src/types/aurora.ts`
- Create: `aurora/ui/src/hooks/useAurora.ts`
- Create: `aurora/ui/src/context/AuroraContext.tsx`

**Interfaces:**
- Consumes: IPC command signatures from Task 3 (`run_analysis`, `load_contacts`)
- Produces: `AnalysisInput`, `AppOutput`, `AppSettings`, `ContactProfile`, `ProgressPayload` TypeScript types; `useAurora()` hook; `AuroraProvider` context

- [ ] **Step 1: Create `aurora/ui/src/types/aurora.ts`**

```typescript
// ── IPC types — mirrors Rust structs crossing the IPC boundary ────────

export interface SignalSpec {
  freq: number;
  sample_rate: number;
  duration_secs: number;
  noise_std: number;
}

export interface AnalysisInput {
  spec: SignalSpec;
  frequency_threshold: number;
  user_feels_normal: boolean;
}

export interface FrequencyPeak {
  freq_hz: number;
  magnitude: number;
}

export type SignalQuality = 'Good' | 'Noisy' | 'Weak' | 'Invalid';

export interface FrequencySpectrum {
  fundamental_hz: number;
  peaks: FrequencyPeak[];
  quality: SignalQuality;
}

export interface DecisionRecord {
  input_signals: unknown[]; // TritWord[] — opaque, not rendered directly
  result: { value: string; phase: number; frame: string };
  interrupts: Array<{ conflict: string; reason: string }>;
  domain: string;
}

export interface AnalysisReport {
  spectrum: FrequencySpectrum;
  decision: DecisionRecord;
  contact_count: number;
}

export interface AttentionSession {
  session_id: string;
  reminders: Array<{
    timestamp: string;
    direction: string;
    target: string;
    reason: string;
    response: string;
  }>;
}

export interface AttentionOutcome {
  cmd: { type: string; target?: string } | null;
  asi: number;
  reminder_count: number;
  session: AttentionSession;
}

export interface AppOutput {
  analysis_report: AnalysisReport;
  attention_outcome: AttentionOutcome;
  html: string;
  json: string;
}

export interface ContactProfile {
  id: string;
  name: string;
  relation_label: string;
  frames: Array<{ frame: string; phase: number }>;
}

export interface AppSettings {
  frequency_threshold: number;
  user_feels_normal: boolean;
  data_source_path: string | null;
}

export interface ProgressPayload {
  step: string;
  pct: number;
}
```

- [ ] **Step 2: Create `aurora/ui/src/hooks/useAurora.ts`**

```typescript
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useCallback, useEffect, useRef } from 'react';
import type {
  AnalysisInput,
  AppOutput,
  ProgressPayload,
} from '../types/aurora';

export function useAurora() {
  const unlistenRef = useRef<UnlistenFn | null>(null);

  const runAnalysis = useCallback(
    async (input: AnalysisInput): Promise<AppOutput> => {
      return invoke<AppOutput>('run_analysis', { input });
    },
    [],
  );

  const loadContacts = useCallback(
    async (path: string): Promise<number> => {
      return invoke<number>('load_contacts', { path });
    },
    [],
  );

  const onProgress = useCallback(
    (cb: (payload: ProgressPayload) => void): (() => void) => {
      const unlistenPromise = listen<ProgressPayload>(
        'analysis_progress',
        (event) => cb(event.payload),
      );
      // Return cleanup function
      let cancelled = false;
      let cleanup: (() => void) | null = null;
      unlistenPromise.then((fn) => {
        if (!cancelled) {
          cleanup = fn;
        } else {
          fn();
        }
      });
      return () => {
        cancelled = true;
        cleanup?.();
      };
    },
    [],
  );

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      unlistenRef.current?.();
    };
  }, []);

  return { runAnalysis, loadContacts, onProgress };
}
```

- [ ] **Step 3: Create `aurora/ui/src/context/AuroraContext.tsx`**

```typescript
import React, { createContext, useContext, useReducer, type ReactNode } from 'react';
import type {
  AppOutput,
  AppSettings,
  ContactProfile,
  ProgressPayload,
} from '../types/aurora';

// ── State ──────────────────────────────────────────────────────────

interface AuroraState {
  currentOutput: AppOutput | null;
  contacts: ContactProfile[];
  settings: AppSettings;
  isLoading: boolean;
  progress: ProgressPayload | null;
}

const initialState: AuroraState = {
  currentOutput: null,
  contacts: [],
  settings: {
    frequency_threshold: 2.0,
    user_feels_normal: true,
    data_source_path: null,
  },
  isLoading: false,
  progress: null,
};

// ── Actions ────────────────────────────────────────────────────────

type Action =
  | { type: 'SET_OUTPUT'; payload: AppOutput }
  | { type: 'SET_LOADING'; payload: boolean }
  | { type: 'SET_PROGRESS'; payload: ProgressPayload | null }
  | { type: 'SET_CONTACTS'; payload: ContactProfile[] }
  | { type: 'UPDATE_SETTINGS'; payload: Partial<AppSettings> };

function reducer(state: AuroraState, action: Action): AuroraState {
  switch (action.type) {
    case 'SET_OUTPUT':
      return { ...state, currentOutput: action.payload, isLoading: false };
    case 'SET_LOADING':
      return { ...state, isLoading: action.payload };
    case 'SET_PROGRESS':
      return { ...state, progress: action.payload };
    case 'SET_CONTACTS':
      return { ...state, contacts: action.payload };
    case 'UPDATE_SETTINGS':
      return {
        ...state,
        settings: { ...state.settings, ...action.payload },
      };
    default:
      return state;
  }
}

// ── Context ────────────────────────────────────────────────────────

interface AuroraContextType {
  state: AuroraState;
  dispatch: React.Dispatch<Action>;
}

const AuroraContext = createContext<AuroraContextType | null>(null);

export function AuroraProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(reducer, initialState);
  return (
    <AuroraContext.Provider value={{ state, dispatch }}>
      {children}
    </AuroraContext.Provider>
  );
}

export function useAuroraContext(): AuroraContextType {
  const ctx = useContext(AuroraContext);
  if (!ctx) {
    throw new Error('useAuroraContext must be used within AuroraProvider');
  }
  return ctx;
}
```

- [ ] **Step 4: Verify frontend builds**

```bash
cd aurora/ui && npm run build
```

Expected: no TypeScript errors.

- [ ] **Step 5: Commit**

```bash
git add aurora/ui/src/types/aurora.ts aurora/ui/src/hooks/useAurora.ts aurora/ui/src/context/AuroraContext.tsx
git commit -m "feat: add frontend types, useAurora hook, and AuroraContext

TypeScript types mirror all Rust IPC types (AnalysisInput, AppOutput, etc.).
useAurora hook wraps Tauri invoke() and listen() for run_analysis/load_contacts.
AuroraContext provides global state via useReducer.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 5: Layout shell — Sidebar + AppShell + routing

**Files:**
- Create: `aurora/ui/src/components/layout/AppShell.tsx`
- Create: `aurora/ui/src/components/layout/Sidebar.tsx`
- Create: `aurora/ui/src/components/layout/AppShell.module.css`
- Create: `aurora/ui/src/components/layout/Sidebar.module.css`
- Modify: `aurora/ui/src/App.tsx`
- Modify: `aurora/ui/src/main.tsx`

**Interfaces:**
- Consumes: `AuroraProvider` from Task 4, `react-router-dom` HashRouter
- Produces: Navigable app shell with sidebar, 5 route stubs rendering placeholder text

- [ ] **Step 1: Create `aurora/ui/src/components/layout/AppShell.module.css`**

```css
.shell {
  display: flex;
  height: 100vh;
  background: #0d1117;
  color: #c9d1d9;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
}

.main {
  flex: 1;
  overflow-y: auto;
  padding: 2rem;
}
```

- [ ] **Step 2: Create `aurora/ui/src/components/layout/Sidebar.module.css`**

```css
.sidebar {
  width: 220px;
  background: #161b22;
  border-right: 1px solid #30363d;
  display: flex;
  flex-direction: column;
  padding: 1rem 0;
}

.brand {
  padding: 0 1rem 1rem;
  font-size: 1.2rem;
  font-weight: 700;
  color: #58a6ff;
  border-bottom: 1px solid #30363d;
  margin-bottom: 0.5rem;
}

.nav {
  list-style: none;
  margin: 0;
  padding: 0;
}

.navItem {
  display: block;
  padding: 0.6rem 1rem;
  color: #8b949e;
  text-decoration: none;
  font-size: 0.9rem;
  transition: background 0.15s, color 0.15s;
}

.navItem:hover {
  background: #21262d;
  color: #c9d1d9;
}

.navItemActive {
  background: #1f6feb22;
  color: #58a6ff;
  border-right: 2px solid #58a6ff;
}
```

- [ ] **Step 3: Create `aurora/ui/src/components/layout/Sidebar.tsx`**

```tsx
import { NavLink } from 'react-router-dom';
import styles from './Sidebar.module.css';

const NAV_ITEMS = [
  { label: '仪表盘', path: '/' },
  { label: '冲突面板', path: '/conflicts' },
  { label: '信号分析', path: '/analyze' },
  { label: '审计日志', path: '/audit' },
  { label: '设置', path: '/settings' },
];

export function Sidebar() {
  return (
    <nav className={styles.sidebar}>
      <div className={styles.brand}>Aurora</div>
      <ul className={styles.nav}>
        {NAV_ITEMS.map((item) => (
          <li key={item.path}>
            <NavLink
              to={item.path}
              end={item.path === '/'}
              className={({ isActive }) =>
                `${styles.navItem} ${isActive ? styles.navItemActive : ''}`
              }
            >
              {item.label}
            </NavLink>
          </li>
        ))}
      </ul>
    </nav>
  );
}
```

- [ ] **Step 4: Create `aurora/ui/src/components/layout/AppShell.tsx`**

```tsx
import type { ReactNode } from 'react';
import { Sidebar } from './Sidebar';
import styles from './AppShell.module.css';

interface AppShellProps {
  children: ReactNode;
}

export function AppShell({ children }: AppShellProps) {
  return (
    <div className={styles.shell}>
      <Sidebar />
      <main className={styles.main}>{children}</main>
    </div>
  );
}
```

- [ ] **Step 5: Rewrite `aurora/ui/src/App.tsx` with routing**

```tsx
import { HashRouter, Routes, Route, Navigate } from 'react-router-dom';
import { AuroraProvider } from './context/AuroraContext';
import { AppShell } from './components/layout/AppShell';

// Page stubs — will be fleshed out in Tasks 6-7
function Dashboard() {
  return (
    <div>
      <h2>仪表盘</h2>
      <p style={{ color: '#8b949e' }}>注意力主权仪表盘 — 即将上线</p>
    </div>
  );
}

function ConflictPanel() {
  return (
    <div>
      <h2>冲突面板</h2>
      <p style={{ color: '#8b949e' }}>跨域冲突列表 — 即将上线</p>
    </div>
  );
}

function SignalAnalyzer() {
  return (
    <div>
      <h2>信号分析</h2>
      <p style={{ color: '#8b949e' }}>合成信号输入与分析 — 即将上线</p>
    </div>
  );
}

function AuditLog() {
  return (
    <div>
      <h2>审计日志</h2>
      <p style={{ color: '#8b949e' }}>决策审计记录 — M1 后续</p>
    </div>
  );
}

function Settings() {
  return (
    <div>
      <h2>设置</h2>
      <p style={{ color: '#8b949e' }}>数据源与阈值配置 — M1 后续</p>
    </div>
  );
}

export function App() {
  return (
    <AuroraProvider>
      <HashRouter>
        <AppShell>
          <Routes>
            <Route path="/" element={<Dashboard />} />
            <Route path="/conflicts" element={<ConflictPanel />} />
            <Route path="/analyze" element={<SignalAnalyzer />} />
            <Route path="/audit" element={<AuditLog />} />
            <Route path="/settings" element={<Settings />} />
            <Route path="*" element={<Navigate to="/" replace />} />
          </Routes>
        </AppShell>
      </HashRouter>
    </AuroraProvider>
  );
}
```

- [ ] **Step 6: Verify frontend builds**

```bash
cd aurora/ui && npm run build
```

Expected: no errors.

- [ ] **Step 7: Build Tauri to verify full stack compiles**

```bash
cargo build -p aurora-desktop
```

- [ ] **Step 8: Commit**

```bash
git add aurora/ui/src/components/layout/ aurora/ui/src/App.tsx
git commit -m "feat: add layout shell with Sidebar, AppShell, and HashRouter

Five route stubs: Dashboard, ConflictPanel, SignalAnalyzer, AuditLog, Settings.
Dark theme matching Aurora HTML report style (#0d1117 background).
CSS Modules with active nav item highlighting.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 6: Dashboard page — ASIGauge + FrameRadar + ReminderTimeline

**Files:**
- Create: `aurora/ui/src/components/dashboard/Dashboard.tsx`
- Create: `aurora/ui/src/components/dashboard/ASIGauge.tsx`
- Create: `aurora/ui/src/components/dashboard/ASIGauge.module.css`
- Create: `aurora/ui/src/components/dashboard/FrameRadar.tsx`
- Create: `aurora/ui/src/components/dashboard/FrameRadar.module.css`
- Create: `aurora/ui/src/components/dashboard/ReminderTimeline.tsx`
- Create: `aurora/ui/src/components/dashboard/ReminderTimeline.module.css`
- Create: `aurora/ui/src/components/dashboard/QuickStats.tsx`
- Create: `aurora/ui/src/components/dashboard/QuickStats.module.css`
- Modify: `aurora/ui/src/App.tsx` (replace Dashboard stub with real component)

**Interfaces:**
- Consumes: `useAuroraContext()` from Task 4, `AppOutput` type
- Produces: Dashboard page with ASI gauge, radar chart, reminder timeline, quick stats

- [ ] **Step 1: Create `aurora/ui/src/components/dashboard/ASIGauge.module.css`**

```css
.container {
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 8px;
  padding: 1.5rem;
  display: flex;
  flex-direction: column;
  align-items: center;
}

.label {
  font-size: 0.85rem;
  color: #8b949e;
  margin-bottom: 0.5rem;
}

.value {
  font-size: 2.5rem;
  font-weight: 700;
}

.high { color: #3fb950; }
.mid { color: #d2991d; }
.low { color: #f85149; }

.subtitle {
  font-size: 0.75rem;
  color: #484f58;
  margin-top: 0.25rem;
}
```

- [ ] **Step 2: Create `aurora/ui/src/components/dashboard/ASIGauge.tsx`**

```tsx
import styles from './ASIGauge.module.css';

interface ASIGaugeProps {
  value: number;
}

function asiColor(v: number): string {
  if (v >= 0.7) return styles.high;
  if (v >= 0.4) return styles.mid;
  return styles.low;
}

export function ASIGauge({ value }: ASIGaugeProps) {
  const pct = Math.round(value * 100);
  return (
    <div className={styles.container}>
      <span className={styles.label}>注意力自主性指数 (ASI)</span>
      <span className={`${styles.value} ${asiColor(value)}`}>{pct}%</span>
      <span className={styles.subtitle}>
        {value >= 0.7 ? '主权稳固' : value >= 0.4 ? '需要关注' : '注意被劫持'}
      </span>
    </div>
  );
}
```

- [ ] **Step 3: Create `aurora/ui/src/components/dashboard/FrameRadar.module.css`**

```css
.container {
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 8px;
  padding: 1.5rem;
}

.title {
  font-size: 0.85rem;
  color: #8b949e;
  margin: 0 0 1rem;
}

.canvas {
  width: 100%;
  max-width: 300px;
  height: auto;
  display: block;
  margin: 0 auto;
}
```

- [ ] **Step 4: Create `aurora/ui/src/components/dashboard/FrameRadar.tsx`**

```tsx
import { useEffect, useRef } from 'react';
import styles from './FrameRadar.module.css';

interface FrameRadarProps {
  data: Array<{ frame: string; weight: number }>;
}

const FRAMES = ['Science', 'Individual', 'Consensus', 'Absolute', 'Meta'];

function frameWeights(
  data: Array<{ frame: string; weight: number }>,
): number[] {
  return FRAMES.map((f) => {
    const found = data.find((d) => d.frame === f);
    return found ? Math.max(0, Math.min(1, found.weight)) : 0;
  });
}

export function FrameRadar({ data }: FrameRadarProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    const rect = canvas.getBoundingClientRect();
    canvas.width = rect.width * dpr;
    canvas.height = rect.width * dpr;
    ctx.scale(dpr, dpr);

    const w = rect.width;
    const h = rect.width;
    const cx = w / 2;
    const cy = h / 2;
    const r = w * 0.35;
    const n = FRAMES.length;
    const weights = frameWeights(data);

    ctx.clearRect(0, 0, w, h);

    // Draw grid (levels: 0.25, 0.5, 0.75, 1.0)
    for (const level of [0.25, 0.5, 0.75, 1.0]) {
      ctx.beginPath();
      for (let i = 0; i < n; i++) {
        const angle = (Math.PI * 2 * i) / n - Math.PI / 2;
        const lr = r * level;
        const x = cx + lr * Math.cos(angle);
        const y = cy + lr * Math.sin(angle);
        i === 0 ? ctx.moveTo(x, y) : ctx.lineTo(x, y);
      }
      ctx.closePath();
      ctx.strokeStyle = '#21262d';
      ctx.lineWidth = 1;
      ctx.stroke();
    }

    // Draw axes
    for (let i = 0; i < n; i++) {
      const angle = (Math.PI * 2 * i) / n - Math.PI / 2;
      ctx.beginPath();
      ctx.moveTo(cx, cy);
      ctx.lineTo(cx + r * Math.cos(angle), cy + r * Math.sin(angle));
      ctx.strokeStyle = '#30363d';
      ctx.stroke();

      // Label
      const lx = cx + (r + 20) * Math.cos(angle);
      const ly = cy + (r + 20) * Math.sin(angle);
      ctx.fillStyle = '#8b949e';
      ctx.font = '11px system-ui';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText(FRAMES[i], lx, ly);
    }

    // Draw data polygon
    ctx.beginPath();
    for (let i = 0; i < n; i++) {
      const angle = (Math.PI * 2 * i) / n - Math.PI / 2;
      const vr = r * weights[i];
      const x = cx + vr * Math.cos(angle);
      const y = cy + vr * Math.sin(angle);
      i === 0 ? ctx.moveTo(x, y) : ctx.lineTo(x, y);
    }
    ctx.closePath();
    ctx.fillStyle = 'rgba(88, 166, 255, 0.15)';
    ctx.fill();
    ctx.strokeStyle = '#58a6ff';
    ctx.lineWidth = 2;
    ctx.stroke();

    // Draw data points
    for (let i = 0; i < n; i++) {
      const angle = (Math.PI * 2 * i) / n - Math.PI / 2;
      const vr = r * weights[i];
      const x = cx + vr * Math.cos(angle);
      const y = cy + vr * Math.sin(angle);
      ctx.beginPath();
      ctx.arc(x, y, 4, 0, Math.PI * 2);
      ctx.fillStyle = '#58a6ff';
      ctx.fill();
    }
  }, [data]);

  return (
    <div className={styles.container}>
      <h3 className={styles.title}>Frame 权重分布</h3>
      <canvas ref={canvasRef} className={styles.canvas} />
    </div>
  );
}
```

- [ ] **Step 5: Create `aurora/ui/src/components/dashboard/ReminderTimeline.module.css`**

```css
.container {
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 8px;
  padding: 1.5rem;
}

.title {
  font-size: 0.85rem;
  color: #8b949e;
  margin: 0 0 1rem;
}

.list {
  list-style: none;
  margin: 0;
  padding: 0;
}

.item {
  padding: 0.5rem 0;
  border-bottom: 1px solid #21262d;
  font-size: 0.85rem;
}

.item:last-child {
  border-bottom: none;
}

.time {
  color: #484f58;
  margin-right: 0.5rem;
}

.direction {
  font-weight: 600;
}

.shifted { color: #3fb950; }
.ignored { color: #f85149; }
.pending { color: #d2991d; }

.empty {
  color: #484f58;
  font-style: italic;
}
```

- [ ] **Step 6: Create `aurora/ui/src/components/dashboard/ReminderTimeline.tsx`**

```tsx
import styles from './ReminderTimeline.module.css';
import type { AttentionSession } from '../../../types/aurora';

interface ReminderTimelineProps {
  session: AttentionSession | null;
}

function responseClass(response: string): string {
  switch (response) {
    case 'Shifted':
      return styles.shifted;
    case 'Ignored':
      return styles.ignored;
    default:
      return styles.pending;
  }
}

export function ReminderTimeline({ session }: ReminderTimelineProps) {
  const reminders = session?.reminders ?? [];

  return (
    <div className={styles.container}>
      <h3 className={styles.title}>最近提醒</h3>
      {reminders.length === 0 ? (
        <p className={styles.empty}>暂无提醒记录</p>
      ) : (
        <ul className={styles.list}>
          {reminders.slice(-10).reverse().map((r, i) => (
            <li key={i} className={styles.item}>
              <span className={styles.time}>{r.timestamp}</span>
              <span className={`${styles.direction} ${responseClass(r.response)}`}>
                {r.direction}
              </span>
              {' — '}
              {r.reason}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
```

- [ ] **Step 7: Create `aurora/ui/src/components/dashboard/QuickStats.module.css`**

```css
.grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 1rem;
}

.card {
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 8px;
  padding: 1rem;
  text-align: center;
}

.value {
  font-size: 1.5rem;
  font-weight: 700;
  color: #58a6ff;
}

.label {
  font-size: 0.75rem;
  color: #8b949e;
  margin-top: 0.25rem;
}
```

- [ ] **Step 8: Create `aurora/ui/src/components/dashboard/QuickStats.tsx`**

```tsx
import styles from './QuickStats.module.css';

interface QuickStatsProps {
  analysisCount: number;
  holdCount: number;
  contactCount: number;
}

export function QuickStats({ analysisCount, holdCount, contactCount }: QuickStatsProps) {
  return (
    <div className={styles.grid}>
      <div className={styles.card}>
        <div className={styles.value}>{analysisCount}</div>
        <div className={styles.label}>分析次数</div>
      </div>
      <div className={styles.card}>
        <div className={styles.value}>{holdCount}</div>
        <div className={styles.label}>Hold 次数</div>
      </div>
      <div className={styles.card}>
        <div className={styles.value}>{contactCount}</div>
        <div className={styles.label}>活跃联系人</div>
      </div>
    </div>
  );
}
```

- [ ] **Step 9: Create `aurora/ui/src/components/dashboard/Dashboard.tsx`**

```tsx
import { useAuroraContext } from '../../../context/AuroraContext';
import { ASIGauge } from './ASIGauge';
import { FrameRadar } from './FrameRadar';
import { ReminderTimeline } from './ReminderTimeline';
import { QuickStats } from './QuickStats';

export function Dashboard() {
  const { state } = useAuroraContext();
  const { currentOutput, contacts } = state;

  const asi = currentOutput?.attention_outcome.asi ?? 0;
  const session = currentOutput?.attention_outcome.session ?? null;
  const interrupts = currentOutput?.analysis_report.decision.interrupts ?? [];

  // Build frame weights from the decision signals
  const radarData = (currentOutput?.analysis_report.decision.input_signals ?? [])
    .map((s: any) => ({
      frame: s.frame ?? 'Unknown',
      weight: s.phase ?? 0.5,
    }));

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
      <h2>仪表盘</h2>

      <QuickStats
        analysisCount={currentOutput ? 1 : 0}
        holdCount={interrupts.length > 0 ? 1 : 0}
        contactCount={contacts.length}
      />

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 2fr', gap: '1.5rem' }}>
        <ASIGauge value={asi} />
        <FrameRadar data={radarData} />
      </div>

      <ReminderTimeline session={session} />
    </div>
  );
}
```

- [ ] **Step 10: Update `aurora/ui/src/App.tsx` — replace Dashboard stub with real import**

In `App.tsx`:
- Remove the local `function Dashboard() { ... }` stub
- Add at top: `import { Dashboard } from './components/dashboard/Dashboard';`

- [ ] **Step 11: Verify frontend builds**

```bash
cd aurora/ui && npm run build
```

- [ ] **Step 12: Run Rust quality gate**

```bash
cargo test --workspace --all-features
cargo fmt -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

- [ ] **Step 13: Commit**

```bash
git add aurora/ui/src/components/dashboard/ aurora/ui/src/App.tsx
git commit -m "feat: implement Dashboard page with ASI, radar, timeline, stats

ASIGauge: color-coded ASI percentage with Chinese labels.
FrameRadar: Canvas-drawn 5-axis radar chart (zero charting dependencies).
ReminderTimeline: last 10 reminders with response status colors.
QuickStats: 3-card grid (分析次数 / Hold次数 / 活跃联系人).

All components read from AuroraContext, no direct IPC calls.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 7: Wire SignalAnalyzer to real IPC, flesh out ConflictPanel

**Files:**
- Create: `aurora/ui/src/components/analyzer/SignalAnalyzer.tsx`
- Create: `aurora/ui/src/components/analyzer/SignalForm.tsx`
- Create: `aurora/ui/src/components/analyzer/SignalForm.module.css`
- Create: `aurora/ui/src/components/analyzer/DecisionResult.tsx`
- Create: `aurora/ui/src/components/analyzer/DecisionResult.module.css`
- Create: `aurora/ui/src/components/conflicts/ConflictPanel.tsx`
- Create: `aurora/ui/src/components/conflicts/ConflictCard.tsx`
- Create: `aurora/ui/src/components/conflicts/ConflictCard.module.css`
- Modify: `aurora/ui/src/App.tsx` (replace stubs with real imports)

**Interfaces:**
- Consumes: `useAurora()` hook from Task 4, `useAuroraContext()` from Task 4, `AnalysisInput` type
- Produces: Working SignalAnalyzer page that invokes `run_analysis` via IPC; ConflictPanel reading from context

- [ ] **Step 1: Create `aurora/ui/src/components/analyzer/SignalForm.module.css`**

```css
.form {
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 8px;
  padding: 1.5rem;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.row {
  display: flex;
  gap: 1rem;
  align-items: flex-end;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  flex: 1;
}

.field label {
  font-size: 0.8rem;
  color: #8b949e;
}

.field input {
  background: #0d1117;
  border: 1px solid #30363d;
  border-radius: 4px;
  padding: 0.5rem;
  color: #c9d1d9;
  font-size: 0.9rem;
}

.field input:focus {
  outline: none;
  border-color: #58a6ff;
}

.checkbox {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.checkbox label {
  font-size: 0.85rem;
  color: #c9d1d9;
}

.submit {
  background: #238636;
  color: #fff;
  border: none;
  border-radius: 4px;
  padding: 0.5rem 1.5rem;
  font-size: 0.9rem;
  cursor: pointer;
  align-self: flex-start;
}

.submit:hover {
  background: #2ea043;
}

.submit:disabled {
  background: #21262d;
  color: #484f58;
  cursor: not-allowed;
}

.error {
  color: #f85149;
  font-size: 0.8rem;
}
```

- [ ] **Step 2: Create `aurora/ui/src/components/analyzer/SignalForm.tsx`**

```tsx
import { useState, type FormEvent } from 'react';
import styles from './SignalForm.module.css';
import type { AnalysisInput } from '../../../types/aurora';

interface SignalFormProps {
  onSubmit: (input: AnalysisInput) => void;
  isLoading: boolean;
}

const DEFAULTS: AnalysisInput = {
  spec: { freq: 2.0, sample_rate: 100.0, duration_secs: 5.0, noise_std: 0.1 },
  frequency_threshold: 2.0,
  user_feels_normal: true,
};

export function SignalForm({ onSubmit, isLoading }: SignalFormProps) {
  const [freq, setFreq] = useState(String(DEFAULTS.spec.freq));
  const [sampleRate, setSampleRate] = useState(String(DEFAULTS.spec.sample_rate));
  const [duration, setDuration] = useState(String(DEFAULTS.spec.duration_secs));
  const [noise, setNoise] = useState(String(DEFAULTS.spec.noise_std));
  const [threshold, setThreshold] = useState(String(DEFAULTS.frequency_threshold));
  const [feelsNormal, setFeelsNormal] = useState(DEFAULTS.user_feels_normal);
  const [error, setError] = useState<string | null>(null);

  function handleSubmit(e: FormEvent) {
    e.preventDefault();
    setError(null);

    const f = parseFloat(freq);
    const s = parseFloat(sampleRate);
    const d = parseFloat(duration);
    const n = parseFloat(noise);
    const t = parseFloat(threshold);

    if (isNaN(f) || isNaN(s) || isNaN(d) || isNaN(n) || isNaN(t)) {
      setError('所有数值字段必须为有效数字');
      return;
    }
    if (s <= 2 * f) {
      setError('采样率必须至少为频率的 2 倍（Nyquist）');
      return;
    }

    onSubmit({
      spec: { freq: f, sample_rate: s, duration_secs: d, noise_std: n },
      frequency_threshold: t,
      user_feels_normal: feelsNormal,
    });
  }

  return (
    <form className={styles.form} onSubmit={handleSubmit}>
      <div className={styles.row}>
        <div className={styles.field}>
          <label>信号频率 (Hz)</label>
          <input type="number" step="0.1" min="0.1" value={freq} onChange={(e) => setFreq(e.target.value)} />
        </div>
        <div className={styles.field}>
          <label>采样率 (Hz)</label>
          <input type="number" step="1" min="1" value={sampleRate} onChange={(e) => setSampleRate(e.target.value)} />
        </div>
        <div className={styles.field}>
          <label>时长 (秒)</label>
          <input type="number" step="0.1" min="0.1" value={duration} onChange={(e) => setDuration(e.target.value)} />
        </div>
      </div>
      <div className={styles.row}>
        <div className={styles.field}>
          <label>噪声标准差</label>
          <input type="number" step="0.01" min="0" value={noise} onChange={(e) => setNoise(e.target.value)} />
        </div>
        <div className={styles.field}>
          <label>频率阈值 (Hz)</label>
          <input type="number" step="0.1" min="0.1" value={threshold} onChange={(e) => setThreshold(e.target.value)} />
        </div>
        <div className={styles.field}>
          <label>&nbsp;</label>
          <div className={styles.checkbox}>
            <input type="checkbox" id="feels-normal" checked={feelsNormal} onChange={(e) => setFeelsNormal(e.target.checked)} />
            <label htmlFor="feels-normal">用户自评正常</label>
          </div>
        </div>
      </div>
      {error && <div className={styles.error}>{error}</div>}
      <button type="submit" className={styles.submit} disabled={isLoading}>
        {isLoading ? '分析中...' : '运行分析'}
      </button>
    </form>
  );
}
```

- [ ] **Step 3: Create `aurora/ui/src/components/analyzer/DecisionResult.module.css`**

```css
.container {
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 8px;
  padding: 1.5rem;
  margin-top: 1.5rem;
}

.title {
  font-size: 0.85rem;
  color: #8b949e;
  margin: 0 0 1rem;
}

.resultRow {
  display: flex;
  gap: 2rem;
  margin-bottom: 1rem;
}

.resultBadge {
  font-size: 2rem;
  font-weight: 700;
  padding: 0.25rem 1rem;
  border-radius: 4px;
}

.true { background: #3fb95022; color: #3fb950; }
.hold { background: #d2991d22; color: #d2991d; }
.false { background: #f8514922; color: #f85149; }

.freq {
  font-size: 0.9rem;
  color: #c9d1d9;
}

.interrupts {
  margin-top: 1rem;
}

.interruptTitle {
  font-size: 0.8rem;
  color: #d2991d;
  margin-bottom: 0.5rem;
}

.interrupt {
  font-size: 0.85rem;
  color: #c9d1d9;
  padding: 0.25rem 0;
}
```

- [ ] **Step 4: Create `aurora/ui/src/components/analyzer/DecisionResult.tsx`**

```tsx
import type { AnalysisReport } from '../../../types/aurora';
import styles from './DecisionResult.module.css';

function tritDisplay(value: string): { label: string; className: string } {
  switch (value) {
    case 'True':
      return { label: 'True', className: styles.true };
    case 'Hold':
      return { label: 'Hold', className: styles.hold };
    case 'False':
      return { label: 'False', className: styles.false };
    default:
      return { label: value, className: '' };
  }
}

interface DecisionResultProps {
  report: AnalysisReport | null;
}

export function DecisionResult({ report }: DecisionResultProps) {
  if (!report) return null;

  const trit = tritDisplay(report.decision.result.value);
  const interrupts = report.decision.interrupts;
  const spectrum = report.spectrum;

  return (
    <div className={styles.container}>
      <h3 className={styles.title}>分析结果</h3>
      <div className={styles.resultRow}>
        <span className={`${styles.resultBadge} ${trit.className}`}>
          {trit.label}
        </span>
        <span className={styles.freq}>
          检测频率：{spectrum.fundamental_hz.toFixed(2)} Hz
          {' · '}
          信噪质量：{spectrum.quality}
        </span>
      </div>
      {interrupts.length > 0 && (
        <div className={styles.interrupts}>
          <h4 className={styles.interruptTitle}>
            ⚠ 跨域冲突 ({interrupts.length})
          </h4>
          {interrupts.map((intr, i) => (
            <div key={i} className={styles.interrupt}>
              {intr.conflict}: {intr.reason}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 5: Create `aurora/ui/src/components/analyzer/SignalAnalyzer.tsx`**

```tsx
import { useAurora } from '../../../hooks/useAurora';
import { useAuroraContext } from '../../../context/AuroraContext';
import { SignalForm } from './SignalForm';
import { DecisionResult } from './DecisionResult';
import type { AnalysisInput } from '../../../types/aurora';

export function SignalAnalyzer() {
  const { state, dispatch } = useAuroraContext();
  const { runAnalysis } = useAurora();

  async function handleAnalyze(input: AnalysisInput) {
    dispatch({ type: 'SET_LOADING', payload: true });
    try {
      const output = await runAnalysis(input);
      dispatch({ type: 'SET_OUTPUT', payload: output });
    } catch (err) {
      console.error('Analysis failed:', err);
      dispatch({ type: 'SET_LOADING', payload: false });
    }
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
      <h2>信号分析</h2>
      <SignalForm onSubmit={handleAnalyze} isLoading={state.isLoading} />
      <DecisionResult report={state.currentOutput?.analysis_report ?? null} />
    </div>
  );
}
```

- [ ] **Step 6: Create `aurora/ui/src/components/conflicts/ConflictCard.module.css`**

```css
.card {
  background: #161b22;
  border: 1px solid #d2991d;
  border-radius: 8px;
  padding: 1rem;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.5rem;
}

.type {
  font-weight: 600;
  color: #d2991d;
  font-size: 0.95rem;
}

.frames {
  font-size: 0.8rem;
  color: #8b949e;
}

.reason {
  font-size: 0.85rem;
  color: #c9d1d9;
}

.empty {
  color: #484f58;
  font-style: italic;
  padding: 2rem;
  text-align: center;
}
```

- [ ] **Step 7: Create `aurora/ui/src/components/conflicts/ConflictCard.tsx`**

```tsx
import styles from './ConflictCard.module.css';

interface ConflictCardProps {
  conflictType: string;
  reason: string;
  frameA: string;
  frameB: string;
}

export function ConflictCard({ conflictType, reason, frameA, frameB }: ConflictCardProps) {
  return (
    <article className={styles.card}>
      <div className={styles.header}>
        <span className={styles.type}>{conflictType}</span>
        <span className={styles.frames}>{frameA} vs {frameB}</span>
      </div>
      <p className={styles.reason}>{reason}</p>
    </article>
  );
}
```

- [ ] **Step 8: Create `aurora/ui/src/components/conflicts/ConflictPanel.tsx`**

```tsx
import { useAuroraContext } from '../../../context/AuroraContext';
import { ConflictCard } from './ConflictCard';
import styles from './ConflictCard.module.css';

export function ConflictPanel() {
  const { state } = useAuroraContext();
  const interrupts = state.currentOutput?.analysis_report.decision.interrupts ?? [];

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
      <h2>冲突面板</h2>
      {interrupts.length === 0 ? (
        <p className={styles.empty}>
          {state.currentOutput ? '无冲突 — 所有 Frame 一致' : '尚未运行分析 — 请先在信号分析页面运行'}
        </p>
      ) : (
        interrupts.map((intr, i) => (
          <ConflictCard
            key={i}
            conflictType={`${intr.conflict}`}
            reason={intr.reason}
            frameA="Embodied"
            frameB="Individual"
          />
        ))
      )}
    </div>
  );
}
```

- [ ] **Step 9: Update `aurora/ui/src/App.tsx` — replace stubs**

Remove the local stub functions `ConflictPanel`, `SignalAnalyzer` and add imports:

```tsx
import { Dashboard } from './components/dashboard/Dashboard';
import { ConflictPanel } from './components/conflicts/ConflictPanel';
import { SignalAnalyzer } from './components/analyzer/SignalAnalyzer';
```

The `AuditLog` and `Settings` stubs remain for now.

- [ ] **Step 10: Verify frontend builds**

```bash
cd aurora/ui && npm run build
```

- [ ] **Step 11: Run full quality gate**

```bash
cargo test --workspace --all-features
cargo fmt -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test ethics_
```

- [ ] **Step 12: Commit**

```bash
git add aurora/ui/src/components/analyzer/ aurora/ui/src/components/conflicts/ aurora/ui/src/App.tsx
git commit -m "feat: wire SignalAnalyzer to real IPC, flesh out ConflictPanel

SignalAnalyzer: SignalForm with validation → invoke run_analysis → DecisionResult.
ConflictPanel: reads interrupts from AuroraContext, renders ConflictCards.
Async error handling with try/catch on IPC calls.

AuditLog and Settings remain as stubs (M1后续).

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 8: Integration verification and documentation update

**Files:**
- Modify: `aurora/SESSION_START.md` (update progress)
- No new code changes — verification-only task

**Interfaces:**
- Consumes: Everything from Tasks 1-7
- Produces: Verified working system, updated documentation

- [ ] **Step 1: Run full test suite**

```bash
cargo test --workspace --all-features -- --test-threads=2
```

- [ ] **Step 2: Run all quality gates**

```bash
cargo fmt -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test ethics_
```

- [ ] **Step 3: Verify CLI mode still works end-to-end**

```bash
cargo run --release --bin aurora -- --input synthetic_2hz.json --output /tmp/test_report.html
```

Verify the HTML report is valid.

- [ ] **Step 4: Verify frontend builds clean**

```bash
cd aurora/ui && npm run build
```

- [ ] **Step 5: Verify Tauri binary compiles**

```bash
cargo build -p aurora-desktop
```

- [ ] **Step 6: Count all Rust tests and verify none regressed**

```bash
cargo test --workspace --all-features 2>&1 | tail -5
```

Note the test count — must be ≥ the previous run.

- [ ] **Step 7: Update `SESSION_START.md`**

Read `SESSION_START.md` and update the "当前进度" section:

Change the Aurora row:
```
| **Aurora 阶段** | M1 — Contacts Pipeline 完成。AuroraApp facade 抽离（CLI/桌面共享入口）。Tauri v2 桌面应用骨架完成：React + TypeScript 前端（Dashboard/ConflictPanel/SignalAnalyzer），Tauri IPC 命令（run_analysis/load_contacts），Canvas 雷达图。CLI 模式保留且正常工作。 |
```

Update "上次决策":
```
| 2026-06-24 | Tauri 桌面应用骨架完成 — AuroraApp facade 抽离（app.rs），Tauri v2 + React 18 + Vite 5 桌面壳搭建，IPC 命令实现（run_analysis/load_contacts），Dashboard（ASI仪表/Frame雷达/提醒时间线）、ConflictPanel、SignalAnalyzer 三个功能页面。AuditLog 和 Settings 为骨架（M1 后续）。所有现有测试通过，CLI 模式保留。 | `docs/superpowers/specs/2026-06-24-tauri-desktop-shell-design.md` |
```

Update "最后更新" date:
```
**最后更新**：2026-06-24
```

- [ ] **Step 8: Commit**

```bash
git add SESSION_START.md
git commit -m "docs: update SESSION_START.md for Tauri desktop shell completion

Aurora M1 now has: AuroraApp facade, Tauri v2 + React TypeScript shell,
three functional pages (Dashboard/ConflictPanel/SignalAnalyzer).

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 9 (optional): Create placeholder app icons

**Files:**
- Create: Simple placeholder icon files in `aurora/src-tauri/icons/`

**Note:** This task is optional. Skip if the user wants to proceed without icons (Tauri will use defaults). Real icon design should be done by a designer.

- [ ] **Step 1: Generate placeholder icon files using a simple script or skip**

Can be done later. For now the app builds without custom icons.

---

## Self-Review

**1. Spec coverage check:**
- §2.2 AuroraApp facade → Task 1 ✅
- §2.3 Existing code changes table → Task 1 ✅
- §3.1 Tauri Commands → Task 3 ✅
- §3.2 Events → Deferred (M1 后续, per spec §8) ✅
- §3.3 Tauri State → Task 3 ✅
- §3.4 Frontend hook → Task 4 ✅
- §4.1 Tech stack → Task 2 (scaffold) + Task 6 (Canvas radar) ✅
- §4.2 Routing → Task 5 ✅
- §4.3 Component tree → Tasks 5, 6, 7 ✅
- §4.4 Global state → Task 4 (AuroraContext) ✅
- §5 File structure → All tasks create the listed files ✅
- §6 Security model → CSP in tauri.conf.json (Task 2), forbid(unsafe_code) (Tasks 2,3), fs scope (Task 2 capabilities) ✅
- §7 Delivery steps → Tasks 1-8 map directly ✅
- §8 Out of scope → AuditLog stub, Settings stub. No full implementation. ✅

**2. Placeholder scan:**
- No TBD/TODO — all code is concrete
- No "add appropriate error handling" — actual try/catch shown
- No "write tests for the above" — explicit test commands with assertions
- All file paths exact
- All code inline, no references to undefined types

**3. Type consistency:**
- `AnalysisInput` defined in Task 1 (Rust) and Task 4 (TypeScript) — matching ✅
- `AppOutput` defined in Task 1 (Rust) and Task 4 (TypeScript) — matching ✅
- `useAurora()` hook in Task 4 consumed by `SignalAnalyzer` in Task 7 — signatures match ✅
- `useAuroraContext()` in Task 4 consumed by `Dashboard`, `ConflictPanel`, `SignalAnalyzer` — consistent ✅
- Serde derives needed for IPC boundary added in Task 3, consumed by Task 7 ✅

One gap identified: the spec mentions `get_asi_history`, `get_audit_log`, `get_settings`, `update_settings`, `export_data` commands and `analysis_progress` event. These are all M1 后续 per spec §8, so their absence from the plan is correct — they're not in this delivery scope. The AuditLog and Settings UI stubs acknowledge this.
