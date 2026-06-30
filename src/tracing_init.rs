//! Tracing initialization for Trit-Core binaries.
//!
//! Reads `TRIT_LOG` (fallback `RUST_LOG`) env var, defaults to `info`.
//! Set `TRIT_LOG_JSON=0` for human-readable output.

use tracing_subscriber::EnvFilter;

/// Initialize tracing subscriber from environment variables.
///
/// - `TRIT_LOG` / `RUST_LOG`: log filter (default: `info`)
/// - `TRIT_LOG_JSON`: set to `0`/`false`/`off` for human-readable output
pub fn init() {
    let filter = std::env::var("TRIT_LOG")
        .or_else(|_| std::env::var("RUST_LOG"))
        .unwrap_or_else(|_| "info".to_string());

    let use_json = std::env::var("TRIT_LOG_JSON")
        .map(|v| v != "0" && v != "false" && v != "off")
        .unwrap_or(true);

    let env_filter = match EnvFilter::try_new(&filter) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("[trit-core] invalid log filter '{}': {}", filter, e);
            return;
        }
    };

    let result = if use_json {
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .json()
            .with_target(true)
            .with_file(true)
            .with_line_number(true)
            .try_init()
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .with_target(true)
            .with_file(true)
            .with_line_number(true)
            .try_init()
    };

    if let Err(e) = result {
        eprintln!("[trit-core] tracing already initialized: {}", e);
    }
}
