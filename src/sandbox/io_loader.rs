//! File-loading helpers for domain types that need I/O.
//!
//! During the Layer Dependency Cleanup (2026-07-08), `std::fs` operations
//! were removed from `anchor/cost_factor.rs` and `meta/rules.rs`. This module
//! provides the file I/O wrappers that used to live in those domain modules.

use std::path::Path;

use crate::anchor::cost_factor::{FactorError, JsonFactorLoader};

/// Load cost factor data from a JSON file path.
///
/// This is the file I/O counterpart to [`JsonFactorLoader::load_from_str`].
/// Callers with a file path should use this; callers with an in-memory string
/// should call `JsonFactorLoader::load_from_str` directly.
pub fn load_factors_from_file(path: &Path) -> Result<JsonFactorLoader, FactorError> {
    let data = std::fs::read_to_string(path)?;
    JsonFactorLoader::load_from_str(&data)
}
