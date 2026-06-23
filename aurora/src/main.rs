use anyhow::{Context, Result};
use aurora::bc::presentation::{AuroraRenderer, ConflictCard, RenderPort, ViewState};
use aurora::cli::Args;
use aurora::db::Database;
use aurora::pipeline::{analysis, attention};
use clap::Parser;
use std::fs;
use std::path::Path;

fn main() -> Result<()> {
    let args = Args::parse();

    // Open database (in-memory if no path specified)
    let db_path = Path::new(&args.db_path);
    let db = if db_path == Path::new(":memory:") {
        Database::open_in_memory()?
    } else {
        Database::open(db_path)?
    };

    // Load data source if provided
    if let Some(ref path) = args.data_source {
        let manager = aurora::ingest::IngestManager::with_json_fallback(path)?;
        eprintln!(
            "Loaded {} contacts from {}",
            manager.contact_count(),
            manager.source_name()
        );
    }

    // Read and parse input
    let input_text = fs::read_to_string(&args.input)
        .with_context(|| format!("failed to read input file {:?}", args.input))?;
    let spec: analysis::SignalSpec = serde_json::from_str(&input_text)
        .with_context(|| "failed to parse input JSON as SignalSpec")?;

    // ── Link 1: Analysis ────────────────────────────────────────────
    let analysis_report =
        analysis::run_analysis(&spec, args.frequency_threshold, args.user_feels_normal)
            .map_err(|e| anyhow::anyhow!("analysis link failed: {e}"))?;

    // ── Link 2: Attention ───────────────────────────────────────────
    let attention_outcome = attention::run_attention(&analysis_report.decision.input_signals, db)
        .map_err(|e| anyhow::anyhow!("attention link failed: {e}"))?;

    // ── Presentation ────────────────────────────────────────────────
    let mut view = ViewState::new(
        format!(
            "Detected frequency: {:.3} Hz | Decision: {:?}",
            analysis_report.spectrum.fundamental_hz,
            analysis_report.decision.result.value()
        ),
        attention_outcome.session,
    );

    // Add conflicts to view
    for interrupt in &analysis_report.decision.interrupts {
        view.add_conflict(ConflictCard {
            conflict_type: format!("{:?}", interrupt.conflict),
            reason: interrupt.reason.clone(),
            frame_a: "Embodied".into(),
            frame_b: "Individual".into(),
            acknowledged: false,
        });
    }

    let renderer = AuroraRenderer;
    let html = renderer.render_html(&view);

    match args.output {
        Some(path) => {
            fs::write(&path, &html)
                .with_context(|| format!("failed to write HTML report to {:?}", path))?;
            println!("Report written to {}", path.display());
        }
        None => {
            let json = renderer
                .render_json(&view)
                .map_err(|e| anyhow::anyhow!("JSON render failed: {e}"))?;
            println!("{}", json);
        }
    }

    Ok(())
}
