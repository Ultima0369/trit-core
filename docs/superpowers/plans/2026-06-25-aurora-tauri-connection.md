# Aurora Tauri Backend-Frontend Connection — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Connect the Aurora analysis/attention pipeline to the Tauri desktop app so the React frontend can invoke Aurora functionality and display results.

**Architecture:** Tauri commands (Rust) wrap `AuroraApp::run_pipeline` and expose it to the React frontend. The frontend gains a multi-panel layout: the 3D Earth remains as the background/璇玑 layer, and an overlay panel system shows the Aurora decision report (ASI gauge, conflict cards, reminder history). The existing `AuroraApp` facade is reused directly — no pipeline changes. Data flows one way: user input (frequency + user state) → Tauri invoke → Rust pipeline → JSON result → React renders.

**Tech Stack:** Tauri v2 (Rust), React 18 + TypeScript + Vite (frontend), CesiumJS (3D Earth background), existing Aurora pipeline (rust crates)

## Global Constraints

- `#![forbid(unsafe_code)]` — both crates enforce this.
- All Tauri command parameters and return types must be `Serialize + Deserialize`.
- Frontend must work offline (CesiumJS tiles loaded from local tile server on port 21337).
- "微风" UX: Esc exits immediately, no confirmation dialogs.
- Fullscreen, no window decorations, zero text by default on the Earth layer.
- CSP in `tauri.conf.json` must be respected: `connect-src http://127.0.0.1:21337` for tiles.
- `cargo fmt --check` and `cargo clippy --workspace --all-targets --all-features -- -D warnings` must pass.
- All existing tests must continue to pass (`cargo test --workspace --all-features`).

---

## File Structure

```
src-tauri/
├── Cargo.toml              # MODIFY: add aurora + truncore deps
├── src/
│   ├── main.rs             # MODIFY: add AuroraApp init, pass to Tauri state
│   ├── lib.rs              # MODIFY: add Tauri commands, manage AuroraApp state
│   ├── tile_server.rs      # (unchanged)
│   └── commands.rs         # CREATE: Tauri command handlers wrapping AuroraApp
│
ui/
├── src/
│   ├── main.tsx            # (unchanged)
│   ├── App.tsx             # MODIFY: add overlay panel system
│   ├── Earth.tsx           # (unchanged — 璇玑 background)
│   ├── Overlay.tsx         # CREATE: overlay container with toggle
│   ├── AsGauge.tsx         # CREATE: ASI gauge component
│   ├── ConflictPanel.tsx   # CREATE: conflict cards component
│   ├── ReminderHistory.tsx # CREATE: reminder history table
│   └── types.ts            # CREATE: TypeScript types for Tauri responses
│
aurora/
├── src/
│   ├── app.rs              # MODIFY: make AuroraApp not consume self.db (Arc<Mutex<>>)
│   └── lib.rs              # (unchanged)
```

---

### Task 1: Make AuroraApp reusable (don't consume self on run_pipeline)

**Files:**
- Modify: `aurora/src/app.rs`

**Interfaces:**
- Consumes: (nothing — first task)
- Produces: `AuroraApp::run_pipeline(&self, ...)` — borrows self instead of consuming; `AuroraApp::run_with_percept(&self, ...)` — same

**Why:** Currently `run_pipeline` takes `self` (ownership), which means one pipeline run consumes the app. For Tauri, we need to keep the app alive across multiple invocations. The `db` field needs to be wrapped for shared access.

- [ ] **Step 1: Wrap Database in Arc<Mutex<>> and change run_pipeline to &self**

Read the current `aurora/src/app.rs` and make these changes:

1. Change `db: Database` to `db: Arc<Mutex<Database>>`
2. Change `run_pipeline(self, ...)` to `run_pipeline(&self, ...)`
3. Change `run_with_percept(self, ...)` to `run_with_percept(&self, ...)`
4. Clone the Arc<Mutex<>> before passing to `run_attention`

```rust
// aurora/src/app.rs — changes

use std::sync::{Arc, Mutex};

pub struct AuroraApp {
    db: Arc<Mutex<Database>>,
    contacts: Vec<ContactProfile>,
    percept_chain: PerceptChain,
    config: Arc<ConfigStore>,
}

impl AuroraApp {
    pub fn new(db_path: Option<&Path>) -> Result<Self> {
        let db = match db_path {
            None => Database::open_in_memory()?,
            Some(p) if p == Path::new(":memory:") => Database::open_in_memory()?,
            Some(p) => Database::open(p)?,
        };

        // ... rest of new() unchanged ...

        Ok(Self {
            db: Arc::new(Mutex::new(db)),
            contacts: Vec::new(),
            percept_chain: chain,
            config,
        })
    }

    /// Run the full analysis + attention pipeline WITHOUT LLM perception.
    /// Now takes &self — reusable across multiple invocations.
    pub fn run_pipeline(&self, input: AnalysisInput) -> Result<AppOutput> {
        let contact_signals = analysis::contacts_to_tritwords(&self.contacts);

        let analysis_report = analysis::run_analysis(
            &input.spec,
            input.frequency_threshold,
            input.user_feels_normal,
            &contact_signals,
        )
        .map_err(|e| anyhow::anyhow!("analysis link failed: {e}"))?;

        let db = self.db.lock().unwrap().clone();
        let attention_outcome = attention::run_attention(
            &analysis_report.decision.input_signals,
            db,
            &self.contacts,
        )
        .map_err(|e| anyhow::anyhow!("attention link failed: {e}"))?;

        Self::render_output(analysis_report, attention_outcome)
    }

    /// Run the full pipeline WITH LLM perception.
    /// Now takes &self — reusable across multiple invocations.
    pub fn run_with_percept(&self, input: AnalysisInput, user_text: &str) -> Result<AppOutput> {
        let contact_signals = analysis::contacts_to_tritwords(&self.contacts);

        let percept = self
            .percept_chain
            .perceive_or_degrade(user_text)
            .map_err(|e| anyhow::anyhow!("perception failed: {e}"))?;

        let analysis_report = analysis::run_analysis_from_percept(
            &input.spec,
            input.frequency_threshold,
            input.user_feels_normal,
            &contact_signals,
            &percept.signals,
        )
        .map_err(|e| anyhow::anyhow!("analysis link failed: {e}"))?;

        let db = self.db.lock().unwrap().clone();
        let attention_outcome = attention::run_attention(
            &analysis_report.decision.input_signals,
            db,
            &self.contacts,
        )
        .map_err(|e| anyhow::anyhow!("attention link failed: {e}"))?;

        Self::render_output(analysis_report, attention_outcome)
    }
}
```

- [ ] **Step 2: Update Aurora CLI main.rs for new signature**

Read `aurora/src/main.rs` and update the call site — `app.run_pipeline(...)` now borrows instead of moving:

```rust
// aurora/src/main.rs — the run_pipeline call no longer needs to consume app
let output = app.run_pipeline(AnalysisInput {
    spec,
    frequency_threshold: args.frequency_threshold,
    user_feels_normal: args.user_feels_normal,
})?;
```

(No actual change needed — `app.run_pipeline(...)` already works with `&self` since Rust auto-refs.)

- [ ] **Step 3: Run tests to verify no regressions**

```bash
cd C:/trit-core && cargo test -p aurora -j1 2>&1 | grep "test result"
```

Expected: all aurora tests pass.

- [ ] **Step 4: Commit**

```bash
git add aurora/src/app.rs aurora/src/main.rs
git commit -m "refactor: make AuroraApp::run_pipeline take &self for Tauri reuse"
```

---

### Task 2: Add aurora dependency to Tauri crate + create Tauri commands

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: `AuroraApp::run_pipeline(&self, AnalysisInput) -> Result<AppOutput>` (from Task 1)
- Produces: Tauri commands `run_analysis_pipeline`, `get_app_state` — callable from frontend via `invoke()`

- [ ] **Step 1: Add aurora and truncore dependencies to src-tauri/Cargo.toml**

```toml
# src-tauri/Cargo.toml — add under [dependencies]
aurora = { path = "../aurora" }
truncore = { path = ".." }
anyhow = "1"
```

- [ ] **Step 2: Create src-tauri/src/commands.rs**

This file defines the Tauri command handlers. They receive serializable input from the frontend, call `AuroraApp`, and return serializable output.

```rust
// src-tauri/src/commands.rs
//! Tauri command handlers — bridge between frontend invoke() and Aurora pipeline.

use aurora::app::{AnalysisInput, AuroraApp};
use aurora::pipeline::analysis::SignalSpec;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::State;

/// Serializable input from the frontend for running the pipeline.
#[derive(Debug, Clone, Deserialize)]
pub struct PipelineRequest {
    /// Signal frequency in Hz (e.g., 2.0).
    pub freq: f64,
    /// Sample rate in Hz (e.g., 100.0).
    pub sample_rate: f64,
    /// Signal duration in seconds (e.g., 1.0).
    pub duration_secs: f64,
    /// Noise standard deviation (e.g., 0.1).
    pub noise_std: f64,
    /// Frequency threshold for embodied detection.
    pub frequency_threshold: f64,
    /// Whether the user reports feeling normal.
    pub user_feels_normal: bool,
}

/// Serializable output returned to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct PipelineResponse {
    /// Detected fundamental frequency in Hz.
    pub detected_freq_hz: f64,
    /// Decision value: "True", "Hold", or "False".
    pub decision: String,
    /// Attention Sovereignty Index [0.0, 1.0].
    pub asi: f64,
    /// Number of reminders in this session.
    pub reminder_count: usize,
    /// Active shift count.
    pub active_shift_count: usize,
    /// Conflict cards.
    pub conflicts: Vec<ConflictResponse>,
    /// Reminder history entries.
    pub reminders: Vec<ReminderResponse>,
    /// Full HTML report (for iframe or direct render).
    pub html: String,
    /// JSON report string.
    pub json: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConflictResponse {
    pub conflict_type: String,
    pub reason: String,
    pub frame_a: String,
    pub frame_b: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReminderResponse {
    pub timestamp: String,
    pub action: String,
    pub target: String,
    pub response: Option<String>,
}

/// Tauri-managed application state.
pub struct AppState {
    pub app: Mutex<AuroraApp>,
}

/// Run the full Aurora analysis + attention pipeline.
///
/// Called from the frontend via:
///   invoke('run_analysis_pipeline', { request: PipelineRequest })
#[tauri::command]
pub fn run_analysis_pipeline(
    request: PipelineRequest,
    state: State<AppState>,
) -> Result<PipelineResponse, String> {
    let app = state.app.lock().map_err(|e| format!("lock error: {e}"))?;

    let input = AnalysisInput {
        spec: SignalSpec {
            freq: request.freq,
            sample_rate: request.sample_rate,
            duration_secs: request.duration_secs,
            noise_std: request.noise_std,
        },
        frequency_threshold: request.frequency_threshold,
        user_feels_normal: request.user_feels_normal,
    };

    let output = app
        .run_pipeline(input)
        .map_err(|e| format!("pipeline error: {e}"))?;

    let conflicts: Vec<ConflictResponse> = output
        .analysis_report
        .decision
        .interrupts
        .iter()
        .map(|i| ConflictResponse {
            conflict_type: format!("{:?}", i.conflict),
            reason: i.reason.clone(),
            frame_a: "Embodied".into(),
            frame_b: "Individual".into(),
        })
        .collect();

    let reminders: Vec<ReminderResponse> = output
        .attention_outcome
        .session
        .reminders()
        .iter()
        .map(|r| ReminderResponse {
            timestamp: r.timestamp.format("%H:%M:%S").to_string(),
            action: r.action.clone(),
            target: r.target.clone(),
            response: r.user_response.as_ref().map(|ur| format!("{:?}", ur)),
        })
        .collect();

    Ok(PipelineResponse {
        detected_freq_hz: output.analysis_report.spectrum.fundamental_hz,
        decision: format!("{:?}", output.analysis_report.decision.result.value()),
        asi: output.attention_outcome.asi,
        reminder_count: output.attention_outcome.reminder_count,
        active_shift_count: output.attention_outcome.session.user_active_shift_count(),
        conflicts,
        reminders,
        html: output.html,
        json: output.json,
    })
}
```

- [ ] **Step 3: Modify src-tauri/src/lib.rs to register commands and state**

```rust
// src-tauri/src/lib.rs

mod commands;

use commands::AppState;
use aurora::app::AuroraApp;
use std::sync::Mutex;
use tauri::Manager;

#[tauri::command]
fn show_window(window: tauri::Window) {
    let _ = window.show();
}

#[tauri::command]
fn exit_app(window: tauri::Window) {
    let _ = window.close();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize AuroraApp with an in-memory database for now.
    // M1 will add persistent DB path configuration.
    let aurora_app = AuroraApp::new(None).expect("failed to initialize AuroraApp");

    tauri::Builder::default()
        .manage(AppState {
            app: Mutex::new(aurora_app),
        })
        .invoke_handler(tauri::generate_handler![
            show_window,
            exit_app,
            commands::run_analysis_pipeline,
        ])
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            let window_clone = window.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(1500));
                let _ = window_clone.show();
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                let _ = window.destroy();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 4: Build check**

```bash
cd C:/trit-core && cargo build -p aurora-desktop -j1 2>&1 | tail -5
```

Expected: compiles successfully.

- [ ] **Step 5: Run all tests**

```bash
cd C:/trit-core && cargo test --workspace --all-features -j1 -- --test-threads=1 2>&1 | grep "test result"
```

Expected: all pass, 0 failures.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat: add Tauri commands wrapping AuroraApp pipeline"
```

---

### Task 3: Create TypeScript types and frontend overlay components

**Files:**
- Create: `ui/src/types.ts`
- Create: `ui/src/Overlay.tsx`
- Create: `ui/src/AsiGauge.tsx`
- Create: `ui/src/ConflictPanel.tsx`
- Create: `ui/src/ReminderHistory.tsx`

**Interfaces:**
- Consumes: `PipelineResponse` shape from Task 2 (mirrored in TypeScript)
- Produces: React components for overlay UI — consumed by App.tsx in Task 4

- [ ] **Step 1: Create ui/src/types.ts**

```typescript
// ui/src/types.ts — TypeScript types matching Tauri PipelineResponse

export interface PipelineRequest {
  freq: number;
  sample_rate: number;
  duration_secs: number;
  noise_std: number;
  frequency_threshold: number;
  user_feels_normal: boolean;
}

export interface ConflictResponse {
  conflict_type: string;
  reason: string;
  frame_a: string;
  frame_b: string;
}

export interface ReminderResponse {
  timestamp: string;
  action: string;
  target: string;
  response: string | null;
}

export interface PipelineResponse {
  detected_freq_hz: number;
  decision: string;
  asi: number;
  reminder_count: number;
  active_shift_count: number;
  conflicts: ConflictResponse[];
  reminders: ReminderResponse[];
  html: string;
  json: string;
}
```

- [ ] **Step 2: Create ui/src/AsiGauge.tsx**

```tsx
// ui/src/AsiGauge.tsx — Attention Sovereignty Index gauge bar

interface Props {
  asi: number;
  activeShiftCount: number;
  reminderCount: number;
}

export default function AsiGauge({ asi, activeShiftCount, reminderCount }: Props) {
  const pct = Math.round(asi * 100);
  const color = asi > 0.6 ? '#3fb950' : asi > 0.3 ? '#d2991d' : '#f85149';

  return (
    <div style={styles.container}>
      <h3 style={styles.title}>Attention Sovereignty Index (ASI)</h3>
      <div style={styles.gauge}>
        <div style={styles.bar}>
          <div style={{ ...styles.fill, width: `${pct}%`, background: color }} />
        </div>
        <div style={{ ...styles.value, color }}>{asi.toFixed(2)}</div>
      </div>
      <p style={styles.detail}>
        Active shifts: {activeShiftCount} / Reminders: {reminderCount}
      </p>
      <p style={styles.hint}>
        ASI = 用户主动调度次数 / 系统提醒次数。越高 = 你越自主。
      </p>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  container: {
    background: '#161b22',
    border: '1px solid #30363d',
    borderRadius: 8,
    padding: '1rem',
    marginBottom: '1rem',
  },
  title: { color: '#7ee787', margin: '0 0 0.75rem 0', fontSize: '1rem' },
  gauge: { display: 'flex', alignItems: 'center', gap: '0.75rem' },
  bar: { flex: 1, height: 20, background: '#21262d', borderRadius: 10, overflow: 'hidden' },
  fill: { height: '100%', borderRadius: 10, transition: 'width 0.5s' },
  value: { fontSize: '1.25rem', fontWeight: 'bold', minWidth: '3.5rem', textAlign: 'right' },
  detail: { color: '#8b949e', margin: '0.5rem 0 0 0', fontSize: '0.85rem' },
  hint: { color: '#484f58', margin: '0.25rem 0 0 0', fontSize: '0.75rem' },
};
```

- [ ] **Step 3: Create ui/src/ConflictPanel.tsx**

```tsx
// ui/src/ConflictPanel.tsx — conflict cards display

import type { ConflictResponse } from './types';

interface Props {
  conflicts: ConflictResponse[];
}

export default function ConflictPanel({ conflicts }: Props) {
  if (conflicts.length === 0) {
    return (
      <div style={styles.noConflict}>
        <p style={{ color: '#3fb950', margin: 0 }}>
          ✅ No conflict detected — signals are aligned.
        </p>
      </div>
    );
  }

  return (
    <div>
      <h3 style={styles.title}>⚡ Conflicts ({conflicts.length})</h3>
      {conflicts.map((c, i) => (
        <div key={i} style={styles.card}>
          <h4 style={styles.conflictType}>{c.conflict_type}</h4>
          <p style={styles.reason}><strong>Reason:</strong> {c.reason}</p>
          <p style={styles.frames}><strong>Structure:</strong> {c.frame_a} vs {c.frame_b}</p>
          <p style={styles.hint}>
            💡 系统不替你判断哪个更"真实"。这是你的注意力被两个方向拉扯的信号。
          </p>
        </div>
      ))}
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  title: { color: '#d2991d', margin: '0 0 0.75rem 0', fontSize: '1rem' },
  card: {
    background: '#161b22',
    border: '1px solid #d2991d',
    borderRadius: 6,
    padding: '0.75rem 1rem',
    marginBottom: '0.5rem',
  },
  conflictType: { color: '#d2991d', margin: '0 0 0.5rem 0', fontSize: '0.9rem' },
  reason: { color: '#c9d1d9', margin: '0.25rem 0', fontSize: '0.85rem' },
  frames: { color: '#8b949e', margin: '0.25rem 0', fontSize: '0.85rem' },
  hint: { color: '#8b949e', fontStyle: 'italic', margin: '0.5rem 0 0 0', fontSize: '0.8rem' },
  noConflict: {
    background: '#161b22',
    border: '1px solid #3fb950',
    borderRadius: 6,
    padding: '1rem',
    marginBottom: '1rem',
  },
};
```

- [ ] **Step 4: Create ui/src/ReminderHistory.tsx**

```tsx
// ui/src/ReminderHistory.tsx — reminder history table

import type { ReminderResponse } from './types';

interface Props {
  reminders: ReminderResponse[];
}

export default function ReminderHistory({ reminders }: Props) {
  if (reminders.length === 0) {
    return (
      <div>
        <h3 style={styles.title}>Reminder History</h3>
        <p style={{ color: '#484f58', fontSize: '0.85rem' }}>No reminders yet.</p>
      </div>
    );
  }

  return (
    <div>
      <h3 style={styles.title}>Reminder History</h3>
      <table style={styles.table}>
        <thead>
          <tr>
            <th style={styles.th}>Time</th>
            <th style={styles.th}>Action</th>
            <th style={styles.th}>Target</th>
            <th style={styles.th}>Response</th>
          </tr>
        </thead>
        <tbody>
          {reminders.map((r, i) => {
            const rowClass = r.response?.includes('Shifted') || r.response?.includes('Overrode')
              ? 'shifted' : r.response === 'Ignored' || r.response === 'Dismissed'
              ? 'ignored' : 'pending';
            const borderColor = rowClass === 'shifted' ? '#3fb950'
              : rowClass === 'ignored' ? '#f85149' : '#d2991d';
            return (
              <tr key={i} style={{ ...styles.tr, borderLeft: `3px solid ${borderColor}` }}>
                <td style={styles.td}>{r.timestamp}</td>
                <td style={styles.td}>{r.action}</td>
                <td style={styles.td}>{r.target}</td>
                <td style={styles.td}>{r.response ?? 'Pending'}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  title: { color: '#7ee787', margin: '0 0 0.75rem 0', fontSize: '1rem' },
  table: { borderCollapse: 'collapse', width: '100%', fontSize: '0.8rem' },
  th: { border: '1px solid #30363d', padding: '0.4rem 0.6rem', textAlign: 'left', background: '#161b22', color: '#8b949e' },
  td: { border: '1px solid #30363d', padding: '0.4rem 0.6rem', color: '#c9d1d9' },
  tr: { background: '#0d1117' },
};
```

- [ ] **Step 5: Create ui/src/Overlay.tsx**

```tsx
// ui/src/Overlay.tsx — slide-in overlay panel for Aurora results

import { useState } from 'react';
import type { PipelineResponse } from './types';
import AsiGauge from './AsiGauge';
import ConflictPanel from './ConflictPanel';
import ReminderHistory from './ReminderHistory';

interface Props {
  data: PipelineResponse | null;
  loading: boolean;
  onRun: () => void;
}

export default function Overlay({ data, loading, onRun }: Props) {
  const [open, setOpen] = useState(false);

  return (
    <>
      {/* Toggle button — always visible, minimal */}
      <button
        onClick={() => setOpen(!open)}
        style={styles.toggle}
        title={open ? 'Close panel' : 'Open Aurora panel'}
      >
        {open ? '→' : '←'}
      </button>

      {/* Slide-in panel */}
      {open && (
        <div style={styles.panel}>
          <div style={styles.header}>
            <h2 style={styles.headerTitle}>Aurora Decision Report</h2>
            <button onClick={onRun} disabled={loading} style={styles.runButton}>
              {loading ? 'Running...' : '▶ Run Analysis'}
            </button>
          </div>

          <div style={styles.content}>
            {loading && <p style={{ color: '#d2991d' }}>Analyzing signal data...</p>}

            {!loading && !data && (
              <p style={{ color: '#8b949e' }}>
                Click "Run Analysis" to start the Aurora pipeline.
              </p>
            )}

            {!loading && data && (
              <>
                <div style={styles.summary}>
                  <p style={styles.summaryText}>
                    Detected frequency: <strong>{data.detected_freq_hz.toFixed(3)} Hz</strong>
                    {' — '}
                    Decision: <strong style={{
                      color: data.decision === 'Hold' ? '#d2991d'
                        : data.decision === 'True' ? '#3fb950' : '#f85149'
                    }}>{data.decision}</strong>
                  </p>
                </div>

                <AsiGauge
                  asi={data.asi}
                  activeShiftCount={data.active_shift_count}
                  reminderCount={data.reminder_count}
                />

                <ConflictPanel conflicts={data.conflicts} />

                <ReminderHistory reminders={data.reminders} />
              </>
            )}
          </div>

          <div style={styles.footer}>
            <p>Aurora v0.1.0 — 不是指教，是提醒。</p>
          </div>
        </div>
      )}
    </>
  );
}

const styles: Record<string, React.CSSProperties> = {
  toggle: {
    position: 'fixed',
    top: '1rem',
    right: '1rem',
    zIndex: 1001,
    background: 'rgba(22, 27, 34, 0.85)',
    border: '1px solid #30363d',
    borderRadius: 6,
    color: '#8b949e',
    fontSize: '1.2rem',
    padding: '0.4rem 0.6rem',
    cursor: 'pointer',
    backdropFilter: 'blur(8px)',
  },
  panel: {
    position: 'fixed',
    top: 0,
    right: 0,
    width: '420px',
    maxWidth: '100vw',
    height: '100vh',
    background: 'rgba(13, 17, 23, 0.95)',
    borderLeft: '1px solid #30363d',
    zIndex: 1000,
    display: 'flex',
    flexDirection: 'column',
    backdropFilter: 'blur(12px)',
    overflow: 'hidden',
  },
  header: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    padding: '1rem',
    borderBottom: '1px solid #30363d',
  },
  headerTitle: { color: '#58a6ff', margin: 0, fontSize: '1.1rem' },
  runButton: {
    background: '#238636',
    color: '#fff',
    border: 'none',
    borderRadius: 6,
    padding: '0.4rem 0.8rem',
    cursor: 'pointer',
    fontSize: '0.8rem',
    fontWeight: 'bold',
  },
  content: {
    flex: 1,
    overflow: 'auto',
    padding: '1rem',
  },
  summary: {
    background: '#161b22',
    border: '1px solid #30363d',
    borderRadius: 6,
    padding: '0.75rem 1rem',
    marginBottom: '1rem',
  },
  summaryText: { color: '#c9d1d9', margin: 0, fontSize: '0.9rem' },
  footer: {
    padding: '0.75rem 1rem',
    borderTop: '1px solid #30363d',
    color: '#484f58',
    fontSize: '0.75rem',
    textAlign: 'center',
  },
};
```

- [ ] **Step 6: Verify TypeScript compiles**

```bash
cd C:/trit-core/ui && npx tsc --noEmit 2>&1
```

Expected: no errors.

- [ ] **Step 7: Commit**

```bash
git add ui/src/types.ts ui/src/AsiGauge.tsx ui/src/ConflictPanel.tsx ui/src/ReminderHistory.tsx ui/src/Overlay.tsx
git commit -m "feat: add Aurora overlay UI components (ASI gauge, conflicts, reminders)"
```

---

### Task 4: Integrate overlay into App.tsx with Tauri invoke

**Files:**
- Modify: `ui/src/App.tsx`

**Interfaces:**
- Consumes: `Overlay` component (Task 3), `PipelineRequest` / `PipelineResponse` types (Task 3), Tauri `invoke('run_analysis_pipeline')` (Task 2)
- Produces: Full integrated app — Earth background + Aurora overlay panel

- [ ] **Step 1: Modify ui/src/App.tsx**

```tsx
// ui/src/App.tsx — Earth background + Aurora overlay

import { useState, useCallback } from 'react';
import Earth from './Earth';
import Overlay from './Overlay';
import type { PipelineRequest, PipelineResponse } from './types';

// Tauri invoke — typed wrapper
async function invokeRunPipeline(req: PipelineRequest): Promise<PipelineResponse> {
  // @ts-ignore Tauri global
  if (window.__TAURI_INTERNALS__) {
    // @ts-ignore
    return window.__TAURI_INTERNALS__.invoke('run_analysis_pipeline', { request: req });
  }
  // Fallback for dev without Tauri: return mock data
  return {
    detected_freq_hz: 2.0,
    decision: 'Hold',
    asi: 0.5,
    reminder_count: 0,
    active_shift_count: 0,
    conflicts: [],
    reminders: [],
    html: '<p>Dev mode — no Tauri backend</p>',
    json: '{}',
  };
}

export default function App() {
  const [data, setData] = useState<PipelineResponse | null>(null);
  const [loading, setLoading] = useState(false);

  const handleRun = useCallback(async () => {
    setLoading(true);
    try {
      const request: PipelineRequest = {
        freq: 2.0,
        sample_rate: 100.0,
        duration_secs: 1.0,
        noise_std: 0.1,
        frequency_threshold: 1.5,
        user_feels_normal: true,
      };
      const result = await invokeRunPipeline(request);
      setData(result);
    } catch (err) {
      console.error('Pipeline failed:', err);
    } finally {
      setLoading(false);
    }
  }, []);

  return (
    <div style={{
      width: '100vw',
      height: '100vh',
      overflow: 'hidden',
      background: '#000',
      cursor: 'default',
      userSelect: 'none',
    }}>
      <Earth />
      <Overlay data={data} loading={loading} onRun={handleRun} />
    </div>
  );
}
```

- [ ] **Step 2: Verify TypeScript compiles**

```bash
cd C:/trit-core/ui && npx tsc --noEmit 2>&1
```

Expected: no errors.

- [ ] **Step 3: Verify Vite build succeeds**

```bash
cd C:/trit-core/ui && npm run build 2>&1
```

Expected: builds successfully to `ui/dist/`.

- [ ] **Step 4: Commit**

```bash
git add ui/src/App.tsx
git commit -m "feat: integrate Aurora overlay panel into App with Tauri invoke"
```

---

### Task 5: End-to-end verification

**Files:**
- (none — verification only)

- [ ] **Step 1: Run all Rust tests**

```bash
cd C:/trit-core && cargo test --workspace --all-features -j1 -- --test-threads=1 2>&1 | grep -E "(test result|FAILED)"
```

Expected: all pass, 0 failures.

- [ ] **Step 2: Run clippy**

```bash
cd C:/trit-core && cargo clippy --workspace --all-targets --all-features -j1 -- -D warnings 2>&1 | tail -5
```

Expected: clean (no errors).

- [ ] **Step 3: Run fmt check**

```bash
cd C:/trit-core && cargo fmt --check 2>&1
```

Expected: clean (no output).

- [ ] **Step 4: Run ethics gate**

```bash
cd C:/trit-core && cargo test ethics_ -j1 2>&1 | grep "test result"
```

Expected: all pass.

- [ ] **Step 5: Build release**

```bash
cd C:/trit-core && cargo build --release -j1 2>&1 | tail -5
```

Expected: builds successfully.

- [ ] **Step 6: Commit final verification**

```bash
git add -A
git commit -m "chore: final verification — all tests pass, clippy + fmt clean"
```

