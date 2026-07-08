//! datacore — data pipeline between acquisition and perception.
//!
//! Takes raw signals from dataforge and:
//! 1. **normalize** — converts RawSignal into NormalizedSignal (unified schema)
//! 2. **timeseries** — stores numeric observations as time-series data points
//!
//! Does NOT interpret, decide, or apply ternary logic. That belongs to
//! trit-core / aurora. This crate is the "plumbing" between dataforge's
//! acquisition and aurora's perception layer.
//!
//! # Architecture
//!
//! ```text
//! dataforge::RawSignal
//!     → normalize::SignalNormalizer → NormalizedSignal (unified schema)
//!     → timeseries::TimeSeriesStore  → TimeSeriesPoint (numeric, queryable)
//! ```

pub mod anomaly;
pub mod normalize;
pub mod pipeline;
pub mod timeseries;

pub use anomaly::{
    AnomalyConfig, AnomalyDetector, AnomalyResult, ThresholdAlert, ThresholdDetector,
};
pub use normalize::{NormalizedSignal, SignalNormalizer, SignalValue};
pub use pipeline::{Pipeline, PipelineResult};
pub use timeseries::{TimeSeriesPoint, TimeSeriesStore};
