use anyhow::{Context, Result};
use aurora::app::{AnalysisInput, AuroraApp};
use aurora::cli::Args;
use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};

/// Atomic file write: write to a sibling temp file then rename.
/// Prevents half-written files on crash/power loss (config.enc, reports).
fn atomic_write(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, data)?;
    fs::rename(&tmp, path)
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Read and parse input
    // ponytail: size cap to avoid OOM on pathological inputs (10MB is far above
    // any realistic SignalSpec; raise if legitimately larger inputs appear).
    const MAX_INPUT_SIZE: u64 = 10 * 1024 * 1024;
    let input_meta = fs::metadata(&args.input)
        .with_context(|| format!("failed to stat input file {:?}", args.input))?;
    if input_meta.len() > MAX_INPUT_SIZE {
        anyhow::bail!(
            "input file too large: {} bytes (max {})",
            input_meta.len(),
            MAX_INPUT_SIZE
        );
    }
    let input_text = fs::read_to_string(&args.input)
        .with_context(|| format!("failed to read input file {:?}", args.input))?;
    let spec: aurora::pipeline::analysis::SignalSpec = serde_json::from_str(&input_text)
        .with_context(|| "failed to parse input JSON as SignalSpec")?;

    // Build the app, load contacts if requested
    let db_path = if args.db_path == ":memory:" {
        None
    } else {
        Some(Path::new(&args.db_path))
    };
    let mut app = AuroraApp::new(db_path)?;

    if let Some(ref path) = args.data_source {
        let count = app.load_contacts(path)?;
        eprintln!("Loaded {count} contacts from {}", path.display());
    }

    // Run pipeline
    let output = app.run_pipeline(AnalysisInput {
        spec,
        frequency_threshold: args.frequency_threshold,
        user_feels_normal: args.user_feels_normal,
    })?;

    // Output
    match args.output {
        Some(path) => {
            let p = PathBuf::from(&path);
            atomic_write(&p, output.html.as_bytes())
                .with_context(|| format!("failed to write HTML report to {:?}", p))?;
            println!("Report written to {}", p.display());
        }
        None => {
            println!("{}", output.json);
        }
    }

    Ok(())
}
