use anyhow::{Context, Result};
use aurora::attention::AttentionManager;
use aurora::cli::Args;
use aurora::pipeline::{run_pipeline, SignalSpec};
use aurora::render::{html, json};
use clap::Parser;
use std::fs;

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize attention manager for this session (M0: one session per run)
    let mut attention = AttentionManager::new("aurora_session");

    // Load data source if provided (M0: JSON fallback; M1: mail)
    if let Some(ref path) = args.data_source {
        let manager = aurora::ingest::IngestManager::with_json_fallback(path)?;
        eprintln!(
            "Loaded {} contacts from {}",
            manager.contact_count(),
            manager.source_name()
        );
    }

    let input_text = fs::read_to_string(&args.input)
        .with_context(|| format!("failed to read input file {:?}", args.input))?;
    let spec: SignalSpec = serde_json::from_str(&input_text)
        .with_context(|| "failed to parse input JSON as SignalSpec")?;

    let report = run_pipeline(
        &spec,
        args.frequency_threshold,
        args.user_feels_normal,
        &mut attention,
    )
    .map_err(|e| anyhow::anyhow!("pipeline failed: {e}"))?;

    match args.output {
        Some(path) => {
            let html = html::render(&report, attention.session());
            fs::write(&path, html)
                .with_context(|| format!("failed to write HTML report to {:?}", path))?;
            println!("Report written to {}", path.display());
        }
        None => {
            let json =
                json::to_string(&report).map_err(|e| anyhow::anyhow!("JSON render failed: {e}"))?;
            println!("{}", json);
        }
    }

    Ok(())
}
