//! Aurora: a local-first cognitive sovereignty tool built on Trit-Core.
//!
//! This crate is currently at the M0 proof-of-concept stage. The immediate
//! goal is an end-to-end Rust CLI that takes synthetic communication-frequency
//! data, extracts a base frequency via wavelet analysis, feeds it into
//! Trit-Core for a ternary decision (Embodied vs Individual), and renders the
//! result as static HTML.

/// Returns the current Aurora crate version.
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
