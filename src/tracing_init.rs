//! Tracing initialization for Trit-Core binaries.
//!
//! Provides JSON-formatted structured logging via `tracing-subscriber`.
//! Controlled by the `TRIT_LOG` environment variable (default: `info`).

use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::EnvFilter;

/// Initialize the tracing subscriber with JSON output to stdout.
///
/// # Environment variables
/// - `TRIT_LOG`: log filter (e.g. `debug`, `info`, `warn`, `trit_core=debug`)
///   Falls back to `RUST_LOG` if `TRIT_LOG` is not set.
/// - `TRIT_LOG_JSON`: set to `0` or `false` for human-readable output instead of JSON.
pub fn init() {
    let env_filter = EnvFilter::try_from_env("TRIT_LOG")
        .or_else(|_| EnvFilter::try_from_env("RUST_LOG"))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let use_json = std::env::var("TRIT_LOG_JSON")
        .map(|v| v != "0" && v != "false")
        .unwrap_or(true);

    if use_json {
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .json()
            .with_span_events(FmtSpan::CLOSE)
            .with_target(true)
            .with_file(true)
            .with_line_number(true)
            .with_thread_ids(true)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .with_target(true)
            .init();
    }
}
