//! Data ingestion layer — reads contacts from a local JSON file.
//!
//! Users manually export their communication metadata as JSON and point
//! Aurora at the file. See [`json_fallback::JsonFallbackSource`].

pub mod json_fallback;

pub use json_fallback::IngestError;
