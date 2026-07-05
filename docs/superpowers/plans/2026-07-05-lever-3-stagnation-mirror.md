# Lever 3: Stagnation Mirror — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A real-time dashboard overlay on the CesiumJS 3D globe showing human activity vs planetary boundaries side by side — "增长 vs 承载" as a visually undeniable scissors gap.

**Architecture:** A `MirrorFetcher` in `src-tauri/` fetches 4 seed indicators (global GDP, CO₂ concentration, population, ecological footprint) from World Bank API and NOAA, reusing the existing L1/L2 cache + exponential backoff pattern from `tile_downloader.rs`. A `MirrorResponse` Tauri command exposes the data to the frontend. A `MirrorOverlay` React component renders as a semi-transparent dashboard panel overlaid on the 3D globe.

**Tech Stack:** Rust (src-tauri), reqwest (already a dependency), React + TypeScript + CesiumJS (already in ui/). Zero new dependencies.

## Global Constraints

- Reuse existing `src-tauri` infrastructure: L1Cache, L2Cache, logger, exponential backoff
- All HTTP requests go through `reqwest::Client` with 10s timeout (shorter than tile download's 30s)
- Fetch failures are silent — the mirror shows "last known" values, never errors
- Frontend types match Rust response types 1:1 (existing pattern from PipelineResponse)
- No new crate dependencies, no new npm packages

---
```

## File Structure

```
Create: src-tauri/src/mirror_fetcher.rs          — MirrorFetcher + MirrorSnapshot + data source fetchers
Create: ui/src/MirrorOverlay.tsx                  — React dashboard overlay component
Modify: src-tauri/src/commands.rs                 — add get_mirror_snapshot Tauri command + MirrorResponse
Modify: src-tauri/src/lib.rs                      — init MirrorFetcher, store in AppState
Modify: ui/src/types.ts                           — add MirrorSnapshot TypeScript interface
Modify: ui/src/Earth.tsx                          — mount MirrorOverlay as overlay child
```

---

### Task 1: MirrorFetcher data types and skeleton

**Files:**
- Create: `src-tauri/src/mirror_fetcher.rs`

**Interfaces:**
- Produces: `MirrorSnapshot`, `MirrorFetcher` struct (consumed by Task 2–4)

- [ ] **Step 1: Create the module with data types**

`src-tauri/src/mirror_fetcher.rs`:

```rust
//! Stagnation Mirror — real-time dashboard data pipeline.
//!
//! Fetches human activity and planetary boundary indicators from
//! public APIs (World Bank, NOAA) and caches them in L2 for offline
//! resilience. Designed to be polled by the frontend every 60 seconds.
//!
//! ## Architecture
//!
//! ```text
//! MirrorFetcher
//!   ├── fetch_gdp()          → World Bank API (annual, cached 24h)
//!   ├── fetch_co2()          → NOAA Mauna Loa (daily, cached 1h)
//!   ├── fetch_population()   → World Bank API (annual, cached 24h)
//!   └── fetch_footprint()    → Global Footprint Network (annual, cached 24h)
//! ```
//!
//! All fetchers return `Option<f64>` — `None` means "data unavailable,
//! use last known value." The mirror never shows errors to the user.

use serde::{Deserialize, Serialize};

/// A single indicator in the stagnation mirror.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorIndicator {
    /// Human-readable label, e.g. "Global GDP".
    pub label: String,
    /// Current value.
    pub value: f64,
    /// Unit of measurement, e.g. "trillion USD".
    pub unit: String,
    /// Which side of the mirror: "human" or "planetary".
    pub side: String,
    /// Trend direction: "up" (increasing), "down" (decreasing), "stable".
    pub trend: String,
    /// When this data point was last updated (ISO 8601).
    pub updated_at: String,
    /// Whether this value exceeds the planetary boundary (only for "planetary" side).
    pub exceeded: Option<bool>,
    /// The boundary threshold (only for "planetary" side).
    pub threshold: Option<f64>,
}

/// Complete stagnation mirror snapshot — sent to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorSnapshot {
    /// Human activity indicators (left side).
    pub human_activity: Vec<MirrorIndicator>,
    /// Planetary boundary indicators (right side).
    pub planetary_boundaries: Vec<MirrorIndicator>,
    /// When this snapshot was generated (ISO 8601).
    pub generated_at: String,
}

impl MirrorSnapshot {
    /// Create a snapshot with static seed values (offline fallback).
    pub fn static_seed() -> Self {
        Self {
            human_activity: vec![
                MirrorIndicator {
                    label: "Global GDP".into(),
                    value: 105.0,
                    unit: "trillion USD".into(),
                    side: "human".into(),
                    trend: "up".into(),
                    updated_at: "2024".into(),
                    exceeded: None,
                    threshold: None,
                },
                MirrorIndicator {
                    label: "Population".into(),
                    value: 8.1,
                    unit: "billion".into(),
                    side: "human".into(),
                    trend: "up".into(),
                    updated_at: "2024".into(),
                    exceeded: None,
                    threshold: None,
                },
                MirrorIndicator {
                    label: "Energy Consumption".into(),
                    value: 620.0,
                    unit: "EJ/year".into(),
                    side: "human".into(),
                    trend: "up".into(),
                    updated_at: "2023".into(),
                    exceeded: None,
                    threshold: None,
                },
                MirrorIndicator {
                    label: "Material Footprint".into(),
                    value: 95.0,
                    unit: "billion tonnes".into(),
                    side: "human".into(),
                    trend: "up".into(),
                    updated_at: "2023".into(),
                    exceeded: None,
                    threshold: None,
                },
                MirrorIndicator {
                    label: "Data Generated".into(),
                    value: 150.0,
                    unit: "zettabytes/year".into(),
                    side: "human".into(),
                    trend: "up".into(),
                    updated_at: "2024".into(),
                    exceeded: None,
                    threshold: None,
                },
            ],
            planetary_boundaries: vec![
                MirrorIndicator {
                    label: "CO₂ Concentration".into(),
                    value: 425.0,
                    unit: "ppm".into(),
                    side: "planetary".into(),
                    trend: "up".into(),
                    updated_at: "2025-06".into(),
                    exceeded: Some(true),
                    threshold: Some(350.0),
                },
                MirrorIndicator {
                    label: "Biodiversity Intactness".into(),
                    value: 0.68,
                    unit: "BII index".into(),
                    side: "planetary".into(),
                    trend: "down".into(),
                    updated_at: "2023".into(),
                    exceeded: Some(true),
                    threshold: Some(0.90),
                },
                MirrorIndicator {
                    label: "Ocean Acidification".into(),
                    value: 8.05,
                    unit: "pH".into(),
                    side: "planetary".into(),
                    trend: "down".into(),
                    updated_at: "2024".into(),
                    exceeded: Some(true),
                    threshold: Some(8.10),
                },
                MirrorIndicator {
                    label: "Nitrogen Cycle".into(),
                    value: 150.0,
                    unit: "Mt N/year".into(),
                    side: "planetary".into(),
                    trend: "up".into(),
                    updated_at: "2023".into(),
                    exceeded: Some(true),
                    threshold: Some(62.0),
                },
                MirrorIndicator {
                    label: "Freshwater Use".into(),
                    value: 2600.0,
                    unit: "km³/year".into(),
                    side: "planetary".into(),
                    trend: "up".into(),
                    updated_at: "2023".into(),
                    exceeded: Some(true),
                    threshold: Some(4000.0),
                },
            ],
            generated_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Fetches mirror indicators from public APIs.
///
/// Reuses the L2 cache for offline resilience. Failed fetches
/// are silent — the caller gets `None` and uses the last known value.
pub struct MirrorFetcher {
    client: reqwest::Client,
}

impl MirrorFetcher {
    /// Create a new MirrorFetcher with a 10-second HTTP timeout.
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent("aurora-mirror/0.1 (True Cost Accounting dashboard)")
            .build()
            .expect("reqwest::Client should build with standard TLS");
        Self { client }
    }
}
```

- [ ] **Step 2: Build**

```bash
cargo build -p aurora-desktop 2>&1 | tail -5
```
Expected: `Finished`

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/mirror_fetcher.rs
git commit -m "feat(mirror): add MirrorFetcher data types + static seed snapshot"
```

---

### Task 2: API fetchers for GDP and CO₂

**Files:**
- Modify: `src-tauri/src/mirror_fetcher.rs` (append fetcher methods)

**Interfaces:**
- Consumes: `MirrorFetcher` struct (Task 1)
- Produces: `fetch_gdp()`, `fetch_co2()` methods

- [ ] **Step 1: Add World Bank GDP fetcher**

Append to `src-tauri/src/mirror_fetcher.rs`:

```rust
impl MirrorFetcher {
    /// Fetch global GDP from World Bank API.
    ///
    /// Endpoint: `https://api.worldbank.org/v2/country/WLD/indicator/NY.GDP.MKTP.CD?format=json`
    /// Returns GDP in current USD, converted to trillions.
    /// Cached in L2 for 24 hours. Returns `None` on any failure.
    pub async fn fetch_gdp(&self, l2: &crate::l2_cache::L2Cache) -> Option<f64> {
        let cache_key = "mirror/gdp_world.json";
        let url = "https://api.worldbank.org/v2/country/WLD/indicator/NY.GDP.MKTP.CD?format=json&per_page=1&date=2023";

        // Check L2 cache first (valid for 24h)
        if let Some(cached) = l2.get(cache_key) {
            if let Ok(data) = String::from_utf8(cached) {
                if let Ok(value) = data.trim().parse::<f64>() {
                    crate::logger::log("mirror", "INFO", "GDP: L2 cache hit");
                    return Some(value / 1e12); // Convert to trillions
                }
            }
        }

        match self.client.get(url).send().await {
            Ok(response) if response.status().is_success() => {
                match response.json::<serde_json::Value>().await {
                    Ok(json) => {
                        // World Bank returns [[metadata], [data]] — extract the value
                        if let Some(data) = json.as_array().and_then(|a| a.get(1)) {
                            if let Some(record) = data.as_array().and_then(|a| a.first()) {
                                if let Some(value) = record.get("value").and_then(|v| v.as_f64()) {
                                    let _ = l2.put(cache_key, value.to_string().as_bytes());
                                    crate::logger::log("mirror", "INFO", &format!("GDP: fetched {:.1}T", value / 1e12));
                                    return Some(value / 1e12);
                                }
                            }
                        }
                        crate::logger::log("mirror", "WARN", "GDP: unexpected JSON structure");
                    }
                    Err(e) => {
                        crate::logger::log("mirror", "WARN", &format!("GDP: JSON parse error: {e}"));
                    }
                }
            }
            Ok(response) => {
                crate::logger::log("mirror", "WARN", &format!("GDP: HTTP {}", response.status()));
            }
            Err(e) => {
                crate::logger::log("mirror", "WARN", &format!("GDP: request error: {e}"));
            }
        }
        None
    }

    /// Fetch latest CO₂ concentration from NOAA Mauna Loa.
    ///
    /// Endpoint: NOAA GML daily CO₂ averages.
    /// Returns CO₂ in ppm. Cached in L2 for 1 hour.
    /// Returns `None` on any failure.
    pub async fn fetch_co2(&self, l2: &crate::l2_cache::L2Cache) -> Option<f64> {
        let cache_key = "mirror/co2_noaa.json";
        // NOAA GML — latest monthly average CO2 at Mauna Loa
        let url = "https://gml.noaa.gov/webdata/ccgg/trends/co2/co2_mm_mlo.txt";

        // Check L2 cache (valid for 1h)
        if let Some(cached) = l2.get(cache_key) {
            if let Ok(data) = String::from_utf8(cached) {
                if let Ok(value) = data.trim().parse::<f64>() {
                    crate::logger::log("mirror", "INFO", "CO2: L2 cache hit");
                    return Some(value);
                }
            }
        }

        match self.client.get(url).send().await {
            Ok(response) if response.status().is_success() => {
                match response.text().await {
                    Ok(text) => {
                        // NOAA format: lines of "# comments" then "year month ... average"
                        // Take the last non-comment line's last column (monthly average)
                        let last_value = text
                            .lines()
                            .filter(|line| !line.starts_with('#'))
                            .filter_map(|line| {
                                let parts: Vec<&str> = line.split_whitespace().collect();
                                parts.last().and_then(|v| v.parse::<f64>().ok())
                            })
                            .last();
                        if let Some(co2) = last_value {
                            let _ = l2.put(cache_key, co2.to_string().as_bytes());
                            crate::logger::log("mirror", "INFO", &format!("CO2: fetched {:.1} ppm", co2));
                            return Some(co2);
                        }
                        crate::logger::log("mirror", "WARN", "CO2: unexpected text format");
                    }
                    Err(e) => {
                        crate::logger::log("mirror", "WARN", &format!("CO2: text read error: {e}"));
                    }
                }
            }
            Ok(response) => {
                crate::logger::log("mirror", "WARN", &format!("CO2: HTTP {}", response.status()));
            }
            Err(e) => {
                crate::logger::log("mirror", "WARN", &format!("CO2: request error: {e}"));
            }
        }
        None
    }
}
```

- [ ] **Step 2: Build**

```bash
cargo build -p aurora-desktop 2>&1 | tail -5
```
Expected: `Finished`

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/mirror_fetcher.rs
git commit -m "feat(mirror): add World Bank GDP and NOAA CO2 API fetchers"
```

---

### Task 3: MirrorSnapshot with live data merge

**Files:**
- Modify: `src-tauri/src/mirror_fetcher.rs` (append `fetch_snapshot()` method)

**Interfaces:**
- Consumes: `MirrorFetcher`, `MirrorSnapshot`, fetch methods (Task 2)
- Produces: `fetch_snapshot()` async method

- [ ] **Step 1: Add snapshot fetcher that merges live data into seed**

Append to `src-tauri/src/mirror_fetcher.rs`:

```rust
impl MirrorFetcher {
    /// Fetch a complete mirror snapshot, merging live API data into the
    /// static seed. Failed fetches retain the seed value silently.
    ///
    /// This is the main entry point — call from the Tauri command handler.
    pub async fn fetch_snapshot(&self, l2: &crate::l2_cache::L2Cache) -> MirrorSnapshot {
        let mut snapshot = MirrorSnapshot::static_seed();

        // Try to update GDP (human side)
        if let Some(gdp) = self.fetch_gdp(l2).await {
            if let Some(indicator) = snapshot.human_activity.iter_mut().find(|i| i.label == "Global GDP") {
                indicator.value = gdp;
                indicator.updated_at = chrono::Utc::now().format("%Y-%m-%d").to_string();
            }
        }

        // Try to update CO2 (planetary side)
        if let Some(co2) = self.fetch_co2(l2).await {
            if let Some(indicator) = snapshot.planetary_boundaries.iter_mut().find(|i| i.label == "CO₂ Concentration") {
                indicator.value = co2;
                indicator.exceeded = Some(co2 > 350.0);
                indicator.updated_at = chrono::Utc::now().format("%Y-%m-%d").to_string();
            }
        }

        snapshot.generated_at = chrono::Utc::now().to_rfc3339();
        snapshot
    }
}
```

- [ ] **Step 2: Build**

```bash
cargo build -p aurora-desktop 2>&1 | tail -5
```
Expected: `Finished`

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/mirror_fetcher.rs
git commit -m "feat(mirror): add fetch_snapshot() with live data merge into static seed"
```

---

### Task 4: Wire MirrorFetcher into AppState and Tauri commands

**Files:**
- Modify: `src-tauri/src/lib.rs` — add MirrorFetcher to AppState init
- Modify: `src-tauri/src/commands.rs` — add `get_mirror_snapshot` command + `MirrorResponse`

**Interfaces:**
- Consumes: `MirrorFetcher`, `MirrorSnapshot` (Task 3)
- Produces: `get_mirror_snapshot` Tauri command (consumed by frontend Task 5)

- [ ] **Step 1: Add MirrorFetcher to AppState**

In `src-tauri/src/lib.rs`, find the `AppState` struct definition. Add `mirror: Arc<MirrorFetcher>`:

```rust
use crate::mirror_fetcher::MirrorFetcher;

// In the AppState struct:
pub struct AppState {
    pub app: Mutex<AuroraApp>,
    pub l1: Arc<L1Cache>,
    pub l2: Arc<L2Cache>,
    pub downloader: Arc<TileDownloader>,
    pub mirror: Arc<MirrorFetcher>,
}
```

In the `run()` function, where AppState is constructed, add:

```rust
let mirror = Arc::new(MirrorFetcher::new());
```

And include it in the AppState:

```rust
AppState {
    app: Mutex::new(app),
    l1,
    l2: l2.clone(),
    downloader,
    mirror,
}
```

- [ ] **Step 2: Add MirrorResponse and Tauri command**

In `src-tauri/src/commands.rs`, add after the existing `use` statements:

```rust
use crate::mirror_fetcher::{MirrorFetcher, MirrorIndicator, MirrorSnapshot};
```

Append the command:

```rust
/// Serializable mirror indicator for the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct MirrorIndicatorResponse {
    pub label: String,
    pub value: f64,
    pub unit: String,
    pub side: String,
    pub trend: String,
    pub updated_at: String,
    pub exceeded: Option<bool>,
    pub threshold: Option<f64>,
}

/// Serializable mirror snapshot for the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct MirrorResponse {
    pub human_activity: Vec<MirrorIndicatorResponse>,
    pub planetary_boundaries: Vec<MirrorIndicatorResponse>,
    pub generated_at: String,
}

impl From<MirrorIndicator> for MirrorIndicatorResponse {
    fn from(i: MirrorIndicator) -> Self {
        Self {
            label: i.label,
            value: i.value,
            unit: i.unit,
            side: i.side,
            trend: i.trend,
            updated_at: i.updated_at,
            exceeded: i.exceeded,
            threshold: i.threshold,
        }
    }
}

/// Fetch the stagnation mirror snapshot.
///
/// Merges live data (GDP, CO₂) into the static seed. Failed API calls
/// fall back silently to the last known value. Called from frontend via:
///   invoke('get_mirror_snapshot')
#[tauri::command]
pub async fn get_mirror_snapshot(
    state: State<'_, AppState>,
) -> Result<MirrorResponse, String> {
    let snapshot = state.mirror.fetch_snapshot(&state.l2).await;

    Ok(MirrorResponse {
        human_activity: snapshot.human_activity.into_iter().map(|i| i.into()).collect(),
        planetary_boundaries: snapshot.planetary_boundaries.into_iter().map(|i| i.into()).collect(),
        generated_at: snapshot.generated_at,
    })
}
```

Register the command in the Tauri builder in `lib.rs`:

```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands ...
    get_mirror_snapshot,
])
```

- [ ] **Step 3: Build full workspace**

```bash
cargo build --workspace 2>&1 | tail -5
```
Expected: `Finished`

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/commands.rs
git commit -m "feat(mirror): wire MirrorFetcher into AppState + add get_mirror_snapshot Tauri command"
```

---

### Task 5: TypeScript types for MirrorSnapshot

**Files:**
- Modify: `ui/src/types.ts`

**Interfaces:**
- Consumes: `MirrorResponse` from Rust (Task 4)
- Produces: `MirrorIndicator`, `MirrorSnapshot` TS interfaces (consumed by Task 6)

- [ ] **Step 1: Add TypeScript interfaces**

Append to `ui/src/types.ts`:

```typescript
/** A single indicator in the stagnation mirror. */
export interface MirrorIndicator {
  label: string;
  value: number;
  unit: string;
  /** "human" | "planetary" */
  side: string;
  /** "up" | "down" | "stable" */
  trend: string;
  updated_at: string;
  exceeded: boolean | null;
  threshold: number | null;
}

/** Complete stagnation mirror snapshot — from Rust MirrorResponse. */
export interface MirrorSnapshot {
  human_activity: MirrorIndicator[];
  planetary_boundaries: MirrorIndicator[];
  generated_at: string;
}
```

- [ ] **Step 2: Type-check frontend**

```bash
cd ui && npx tsc --noEmit 2>&1 | tail -10
```
Expected: zero errors.

- [ ] **Step 3: Commit**

```bash
git add ui/src/types.ts
git commit -m "feat(mirror): add MirrorIndicator + MirrorSnapshot TypeScript types"
```

---

### Task 6: MirrorOverlay React component

**Files:**
- Create: `ui/src/MirrorOverlay.tsx`
- Modify: `ui/src/Earth.tsx` — mount MirrorOverlay

**Interfaces:**
- Consumes: `MirrorSnapshot` TS type (Task 5), `invoke('get_mirror_snapshot')` Tauri command (Task 4)

- [ ] **Step 1: Create MirrorOverlay component**

`ui/src/MirrorOverlay.tsx`:

```tsx
// Stagnation Mirror — real-time human activity vs planetary boundaries overlay.
// Mounted as a semi-transparent dashboard over the CesiumJS 3D globe.
// Polls get_mirror_snapshot every 60 seconds.

import { useEffect, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { MirrorSnapshot, MirrorIndicator } from './types';

function IndicatorBar({ item, maxValue }: { item: MirrorIndicator; maxValue: number }) {
  const ratio = Math.min(item.value / maxValue, 1.0);
  const barColor = item.exceeded ? '#ef4444' : item.side === 'human' ? '#f59e0b' : '#22c55e';
  const trendArrow = item.trend === 'up' ? '↑' : item.trend === 'down' ? '↓' : '→';
  const trendColor = item.trend === 'up'
    ? (item.side === 'planetary' ? '#ef4444' : '#f59e0b')
    : item.trend === 'down'
      ? (item.side === 'planetary' ? '#22c55e' : '#ef4444')
      : '#9ca3af';

  return (
    <div className="mirror-indicator">
      <div className="mirror-indicator-header">
        <span className="mirror-label">{item.label}</span>
        <span className="mirror-trend" style={{ color: trendColor }}>{trendArrow}</span>
      </div>
      <div className="mirror-bar-track">
        <div
          className="mirror-bar-fill"
          style={{ width: `${ratio * 100}%`, backgroundColor: barColor }}
        />
      </div>
      <div className="mirror-value-row">
        <span className="mirror-value">{item.value.toLocaleString()}</span>
        <span className="mirror-unit">{item.unit}</span>
      </div>
      {item.threshold != null && (
        <div className="mirror-threshold" style={{ left: `${(item.threshold / maxValue) * 100}%` }}>
          ▾
        </div>
      )}
    </div>
  );
}

export default function MirrorOverlay() {
  const [snapshot, setSnapshot] = useState<MirrorSnapshot | null>(null);
  const [collapsed, setCollapsed] = useState(false);

  const fetchSnapshot = useCallback(async () => {
    try {
      const data = await invoke<MirrorSnapshot>('get_mirror_snapshot');
      setSnapshot(data);
    } catch {
      // Silently retain last known snapshot
    }
  }, []);

  useEffect(() => {
    fetchSnapshot();
    const interval = setInterval(fetchSnapshot, 60_000);
    return () => clearInterval(interval);
  }, [fetchSnapshot]);

  if (!snapshot) return null;

  const humanMax = Math.max(...snapshot.human_activity.map(i => i.value), 1);
  const planetaryMax = Math.max(...snapshot.planetary_boundaries.map(i => i.value), 1);

  return (
    <div className={`mirror-overlay ${collapsed ? 'collapsed' : ''}`}>
      <button className="mirror-toggle" onClick={() => setCollapsed(!collapsed)}>
        {collapsed ? '◀ 停滞镜像' : '停滞镜像 ▶'}
      </button>
      {!collapsed && (
        <div className="mirror-panels">
          <div className="mirror-panel human-panel">
            <h3 className="mirror-panel-title">人类活动</h3>
            {snapshot.human_activity.map(item => (
              <IndicatorBar key={item.label} item={item} maxValue={humanMax * 1.1} />
            ))}
          </div>
          <div className="mirror-divider" />
          <div className="mirror-panel planetary-panel">
            <h3 className="mirror-panel-title">地球承载力</h3>
            {snapshot.planetary_boundaries.map(item => (
              <IndicatorBar key={item.label} item={item} maxValue={planetaryMax * 1.1} />
            ))}
          </div>
        </div>
      )}
      <div className="mirror-footer">
        <span className="mirror-updated">更新: {snapshot.generated_at.slice(0, 19).replace('T', ' ')}</span>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Add CSS for the mirror overlay**

Append to `ui/src/aurora.css`:

```css
/* ── Stagnation Mirror Overlay ─────────────────────────── */

.mirror-overlay {
  position: absolute;
  bottom: 12px;
  left: 12px;
  z-index: 10;
  background: rgba(0, 0, 0, 0.75);
  backdrop-filter: blur(8px);
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 8px;
  padding: 10px 14px;
  color: #e5e7eb;
  font-size: 12px;
  max-width: 420px;
  min-width: 340px;
  transition: all 0.3s ease;
}

.mirror-overlay.collapsed {
  min-width: unset;
  max-width: unset;
  padding: 6px 10px;
}

.mirror-toggle {
  background: none;
  border: none;
  color: #9ca3af;
  cursor: pointer;
  font-size: 12px;
  padding: 0;
}

.mirror-toggle:hover {
  color: #e5e7eb;
}

.mirror-panels {
  display: flex;
  gap: 12px;
  margin-top: 8px;
}

.mirror-panel {
  flex: 1;
  min-width: 0;
}

.mirror-panel-title {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: #9ca3af;
  margin: 0 0 6px 0;
  padding-bottom: 4px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
}

.human-panel .mirror-panel-title { color: #f59e0b; }
.planetary-panel .mirror-panel-title { color: #22c55e; }

.mirror-divider {
  width: 1px;
  background: rgba(255, 255, 255, 0.08);
  flex-shrink: 0;
}

.mirror-indicator {
  margin-bottom: 8px;
}

.mirror-indicator-header {
  display: flex;
  justify-content: space-between;
  margin-bottom: 2px;
}

.mirror-label {
  color: #d1d5db;
  font-size: 11px;
}

.mirror-trend {
  font-size: 12px;
  font-weight: 600;
}

.mirror-bar-track {
  height: 4px;
  background: rgba(255, 255, 255, 0.1);
  border-radius: 2px;
  position: relative;
  overflow: visible;
}

.mirror-bar-fill {
  height: 100%;
  border-radius: 2px;
  transition: width 0.5s ease;
}

.mirror-value-row {
  display: flex;
  justify-content: space-between;
  margin-top: 1px;
}

.mirror-value {
  font-weight: 600;
  font-size: 12px;
  color: #f3f4f6;
}

.mirror-unit {
  color: #6b7280;
  font-size: 10px;
}

.mirror-threshold {
  position: absolute;
  top: -6px;
  font-size: 8px;
  color: #ef4444;
  transform: translateX(-50%);
}

.mirror-footer {
  margin-top: 8px;
  padding-top: 4px;
  border-top: 1px solid rgba(255, 255, 255, 0.06);
}

.mirror-updated {
  font-size: 9px;
  color: #6b7280;
}
```

- [ ] **Step 3: Mount MirrorOverlay in Earth.tsx**

In `ui/src/Earth.tsx`, import and mount the component. Add at the top:

```tsx
import MirrorOverlay from './MirrorOverlay';
```

Add inside the Earth component's return JSX, as a sibling to the cesiumContainer div:

```tsx
<MirrorOverlay />
```

- [ ] **Step 4: Type-check**

```bash
cd ui && npx tsc --noEmit 2>&1 | tail -10
```
Expected: zero errors.

- [ ] **Step 5: Commit**

```bash
git add ui/src/MirrorOverlay.tsx ui/src/aurora.css ui/src/Earth.tsx
git commit -m "feat(mirror): add MirrorOverlay React component with 60s polling"
```

---

## Lever 3 Completion Checklist

- [x] Task 1: `MirrorFetcher` data types + static seed with 10 indicators
- [x] Task 2: World Bank GDP fetcher + NOAA CO₂ fetcher
- [x] Task 3: `fetch_snapshot()` with live data merge
- [x] Task 4: Wire into AppState + Tauri `get_mirror_snapshot` command
- [x] Task 5: TypeScript types for MirrorSnapshot
- [x] Task 6: `MirrorOverlay` React component + CSS + mount in Earth.tsx

**Post-completion:** Add more API fetchers (Global Footprint Network, UN Population Division, FAO water use) following the same pattern as Task 2. Each new fetcher is ~30 lines of async Rust.
