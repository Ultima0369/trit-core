//! dataforge — internet data acquisition layer.
//!
//! Collects raw, unstructured observations from public data APIs:
//! climate (Open-Meteo, NOAA), ecology (GBIF), scientific research (arXiv),
//! and geopolitics (UCDP).
//!
//! This crate does NOT interpret, analyze, or decide. It only fetches and
//! caches. Interpretation belongs to prism (aurora percept layer), and
//! ternary decision-making belongs to trit-core.
//!
//! # Architecture
//!
//! ```text
//! SourceRegistry (periodic fetch + L2 cache)
//!   ├── OpenMeteoSource  — temperature anomalies at 5 global stations
//!   ├── NoaaCo2Source     — Mauna Loa monthly mean CO2 (ppm)
//!   ├── GbifSource        — species occurrence records
//!   ├── ArxivSource       — preprint metadata (Atom XML)
//!   └── UcdpSource        — armed conflict events (coordinates + fatalities)
//! ```
//!
//! # Design
//!
//! - **No trit-core dependency.** RawSignal is pure data, no Frame/Phase/Value.
//! - **Fail-safe.** Every fetch returns empty Vec on error — never propagate.
//! - **Cache-first.** L2 disk cache with configurable TTL; stale reads allowed.
//! - **Rate-limit friendly.** Sequential fetch, user-agent header, polite delays.

pub mod cache;
pub mod error;
pub mod registry;
pub mod source;
pub mod sources;
pub mod types;

pub use cache::L2Cache;
pub use error::DataforgeError;
pub use registry::{SourceHealth, SourceRegistry};
pub use source::DataSource;
pub use types::{DataCategory, GeoPoint, RawSignal};
