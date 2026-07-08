//! Interrupt types — moved to `core::interrupt` (Layer Dependency Cleanup 2026-07-08).
//!
//! The types previously defined here (MetaInterrupt, ConflictType, MetaMonitor, etc.)
//! now live in [`crate::core::interrupt`]. This module is a re-export shim used by
//! `meta/mod.rs` to maintain backward compatibility.

// ponytail: re-exports are all done through meta/mod.rs which pulls from core::interrupt directly.
