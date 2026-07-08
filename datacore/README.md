# datacore

Data pipeline between dataforge acquisition and aurora perception.

Takes raw signals from 10 public data sources, normalizes them into structured
observations, stores them as time-series, and detects anomalies via statistical
and threshold-based methods.

## Modules

| Module | Purpose |
|--------|---------|
| `normalize` | Parse `RawSignal.raw_content` → `NormalizedSignal` (key:value scanner) |
| `timeseries` | `TimeSeriesStore` — in-memory store with CSV/JSON export/import |
| `anomaly` | `AnomalyDetector` (z-score + rate-of-change) + `ThresholdDetector` (11 rules) |
| `pipeline` | `Pipeline::run()` — end-to-end: fetch → normalize → store → detect → export |

## Quick Start

```rust
use std::sync::Arc;
use dataforge::{L2Cache, SourceRegistry};
use datacore::Pipeline;

#[tokio::main]
async fn main() {
    let cache = Arc::new(L2Cache::new("/tmp/dc-cache".into(), 100_000_000));
    let registry = SourceRegistry::with_all_sources(cache);
    let mut pipeline = Pipeline::new();

    let result = pipeline.run(&registry).await;

    println!("{}", result.to_markdown());
}
```

## Binary

```bash
cargo run --bin datacore-collect -p datacore           # full JSON
cargo run --bin datacore-collect -p datacore -- --report  # markdown
cargo run --bin datacore-collect -p datacore -- --daemon --log-dir ./logs  # continuous
```

## Anomaly Detection

Two complementary detectors run in every pipeline run:

- **Z-score**: sliding window (30 points, σ=3.0). Catches unknown deviations.
- **Rate-of-change**: z-score on first derivative. Catches sudden accelerations.
- **Threshold**: 11 fixed safety bounds (CO₂ > 430 ppm, mag ≥ 6.0, etc.).

Anomalous signals get their TritWord phase attenuated in aurora's prism
(z-score ×0.5, threshold ×0.25), reducing their influence on ternary decisions.

## Docker

```bash
docker-compose up -d        # daemon with health check
docker-compose logs -f      # watch output
```
