//! datacore-collect — standalone data acquisition binary.
//!
//! Fetches all 10 registered data sources, runs the full datacore pipeline,
//! and prints a report to stdout. This is the executable entry point
//! of the trit-core listening/monitoring system.
//!
//! Usage:
//!   cargo run --bin datacore-collect -p datacore                  # full JSON report
//!   cargo run --bin datacore-collect -p datacore -- --changes     # only new/changed signals
//!   cargo run --bin datacore-collect -p datacore -- --compact     # summary JSON
//!   cargo run --bin datacore-collect -p datacore -- --report      # markdown report
//!   cargo run --bin datacore-collect -p datacore -- --daemon      # continuous polling
//!
//! Environment:
//!   RUST_LOG=info          logging level (default: datacore_collect=info,dataforge=warn)
//!   COLLECT_INTERVAL_SECS  daemon poll interval in seconds (default: 300)

use std::sync::Arc;

use datacore::{AnomalyConfig, Pipeline};
use dataforge::{L2Cache, SourceRegistry};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "datacore_collect=info,dataforge=warn".into()),
        )
        .with_target(false)
        .init();

    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        eprintln!("datacore-collect — trit-core data acquisition pipeline");
        eprintln!();
        eprintln!("Usage:");
        eprintln!("  datacore-collect                  Full JSON report");
        eprintln!("  datacore-collect --changes        Only new/changed signals");
        eprintln!("  datacore-collect --compact        Summary JSON (one line)");
        eprintln!("  datacore-collect --report         Markdown report");
        eprintln!("  datacore-collect --daemon         Continuous polling mode");
        eprintln!();
        eprintln!("Flags can be combined.");
        eprintln!();
        eprintln!("Environment:");
        eprintln!(
            "  RUST_LOG              Log level (default: datacore_collect=info,dataforge=warn)"
        );
        eprintln!("  COLLECT_INTERVAL_SECS Daemon poll interval seconds (default: 300)");
        return;
    }
    let only_changes = args.iter().any(|a| a == "--changes");
    let compact = args.iter().any(|a| a == "--compact");
    let report_md = args.iter().any(|a| a == "--report");

    // Cache in OS cache dir
    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("datacore-collect");
    let cache = Arc::new(L2Cache::new(cache_dir, 100 * 1024 * 1024)); // 100 MB

    let registry = SourceRegistry::with_all_sources(cache);

    let mut pipeline = Pipeline::with_config(AnomalyConfig {
        window_size: 30,
        threshold: 3.0,
    });

    tracing::info!(
        sources = registry.source_names().join(", "),
        "datacore-collect starting — {} sources",
        registry.source_count()
    );

    let start = std::time::Instant::now();

    // Run pipeline
    let result = if only_changes {
        pipeline.run_changes(&registry).await
    } else {
        pipeline.run(&registry).await
    };

    let elapsed = start.elapsed();

    if report_md {
        println!("{}", result.to_markdown());
    } else if compact {
        let summary = serde_json::json!({
            "sources": registry.source_count(),
            "raw_signals": result.raw_count,
            "normalized": result.normalized_count,
            "data_points": result.point_count,
            "anomalies": result.anomaly_count,
            "elapsed_ms": elapsed.as_millis(),
            "health": result.health.iter().map(|h| serde_json::json!({
                "name": h.name,
                "successes": h.successes,
                "failures": h.failures,
                "avg_latency_us": h.avg_latency_us,
            })).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&summary).unwrap());
    } else {
        let report = serde_json::json!({
            "pipeline": {
                "sources": registry.source_count(),
                "raw_signals": result.raw_count,
                "normalized": result.normalized_count,
                "data_points": result.point_count,
                "anomalies": result.anomaly_count,
                "elapsed_ms": elapsed.as_millis(),
            },
            "health": result.health,
            "anomalies": serde_json::from_str::<serde_json::Value>(&result.anomalies_json).unwrap_or_default(),
            "timeseries": result.timeseries_json,
        });
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    }

    tracing::info!(
        raw = result.raw_count,
        normalized = result.normalized_count,
        points = result.point_count,
        anomalies = result.anomaly_count,
        elapsed_ms = elapsed.as_millis(),
        "datacore-collect complete"
    );

    // Daemon mode: keep polling, only report changes
    let daemon = args.iter().any(|a| a == "--daemon");
    if daemon {
        let interval = std::time::Duration::from_secs(
            std::env::var("COLLECT_INTERVAL_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(300), // default 5 minutes
        );
        eprintln!(
            "daemon mode active — polling every {}s (set COLLECT_INTERVAL_SECS to change)",
            interval.as_secs()
        );
        loop {
            tokio::time::sleep(interval).await;
            let result = pipeline.run_changes(&registry).await;
            if result.raw_count > 0 {
                if report_md {
                    println!("{}", result.to_markdown());
                } else {
                    let summary = serde_json::json!({
                        "ts": chrono::Utc::now().to_rfc3339(),
                        "raw_signals": result.raw_count,
                        "normalized": result.normalized_count,
                        "data_points": result.point_count,
                        "anomalies": result.anomaly_count,
                        "health": result.health.iter().map(|h| serde_json::json!({
                            "name": h.name,
                            "successes": h.successes,
                            "failures": h.failures,
                        })).collect::<Vec<_>>(),
                    });
                    println!("{}", serde_json::to_string(&summary).unwrap());
                }
            } else {
                eprintln!(
                    "{} no changes ({} points stored)",
                    chrono::Utc::now().format("%H:%M:%S"),
                    pipeline.store().len()
                );
            }
        }
    }
}
