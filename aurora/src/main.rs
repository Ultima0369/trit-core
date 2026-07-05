use anyhow::{Context, Result};
use aurora::app::{AnalysisInput, AuroraApp};
use aurora::cli::{Args, Command, PerceiveArgs, PipelineArgs, SourcesAction, SourcesArgs};
use aurora::config::ConfigStore;
use aurora::percept::{CloudLLMProvider, FFTProvider, LocalLLMProvider, PerceptChain};
use aurora::pipeline::analysis::SignalSpec;
use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use dataforge::cache::L2Cache;
use dataforge::registry::SourceRegistry;
use dataforge::sources::{
    arxiv::ArxivSource, gbif::GbifSource, noaa_co2::NoaaCo2Source, open_meteo::OpenMeteoSource,
    ucdp::UcdpSource,
};

/// Atomic file write: write to a sibling temp file then rename.
fn atomic_write(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, data)?;
    fs::rename(&tmp, path)
}

// ── pipeline subcommand ──────────────────────────────────────────────

fn run_pipeline(args: &PipelineArgs) -> Result<()> {
    // Read and parse input
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

    let analysis_input = AnalysisInput {
        spec,
        frequency_threshold: args.frequency_threshold,
        user_feels_normal: args.user_feels_normal,
    };

    // Run pipeline (with or without LLM perception)
    let output = if let Some(ref text) = args.percept {
        app.run_with_percept(analysis_input, text)?
    } else {
        app.run_pipeline(analysis_input)?
    };

    match args.output {
        Some(ref path) => {
            atomic_write(path, output.html.as_bytes())
                .with_context(|| format!("failed to write HTML report to {:?}", path))?;
            println!("Report written to {}", path.display());
        }
        None => {
            println!("{}", output.json);
        }
    }

    Ok(())
}

// ── sources subcommand ───────────────────────────────────────────────

fn dataforge_cache_dir(custom: Option<&PathBuf>) -> PathBuf {
    custom.cloned().unwrap_or_else(|| {
        std::env::temp_dir().join(format!("aurora_dataforge_{}", std::process::id()))
    })
}

fn build_registry(cache_dir: &Path) -> SourceRegistry {
    let cache = Arc::new(L2Cache::new(cache_dir.to_path_buf(), 50 * 1024 * 1024)); // 50MB
    SourceRegistry::new(cache)
        .with_source(Box::new(OpenMeteoSource::new()))
        .with_source(Box::new(NoaaCo2Source::new()))
        .with_source(Box::new(GbifSource::new()))
        .with_source(Box::new(ArxivSource::new()))
        .with_source(Box::new(UcdpSource::new()))
}

fn run_sources(args: &SourcesArgs) -> Result<()> {
    let cache_dir = dataforge_cache_dir(None);
    let registry = build_registry(&cache_dir);

    match args.action {
        SourcesAction::List => {
            println!("Registered data sources:");
            for name in registry.source_names() {
                println!("  - {name}");
            }
            println!("\nTotal: {} sources", registry.source_count());
            println!("Cache directory: {}", cache_dir.display());
            println!(
                "Cache size: {} bytes",
                dataforge::cache::L2Cache::new(cache_dir.clone(), 0).total_bytes()
            );
        }
        SourcesAction::Refresh => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .context("failed to build tokio runtime")?;

            eprintln!("Refreshing all sources (ignore cache)...");
            let signals = rt.block_on(registry.refresh_all());
            println!(
                "Refreshed {} signals from {} sources",
                signals.len(),
                registry.source_count()
            );

            for sig in &signals {
                println!(
                    "  [{}] {} — {}",
                    sig.source_name,
                    &sig.raw_content.chars().take(80).collect::<String>(),
                    sig.captured_at
                );
            }
        }
    }

    Ok(())
}

// ── perceive subcommand ──────────────────────────────────────────────

/// Build a PerceptChain from config, same logic as AuroraApp::new().
///
/// ponytail: duplicated intentionally — AuroraApp ties the chain to
/// the analysis pipeline (FFT + attention), while perceive uses the
/// sandbox pipeline. Both need the same chain construction logic.
fn build_percept_chain(config: &Arc<ConfigStore>) -> PerceptChain {
    let mut chain = PerceptChain::new();

    if let Ok(Some(cloud_model)) = config.cloud_model() {
        match CloudLLMProvider::new(config.clone(), &cloud_model) {
            Ok(provider) => chain = chain.with(Box::new(provider)),
            Err(e) => tracing::warn!("cloud LLM not available: {e}"),
        }
    }
    if config.local_model_path().ok().flatten().is_some() {
        match LocalLLMProvider::new(config.clone()) {
            Ok(provider) => chain = chain.with(Box::new(provider)),
            Err(e) => tracing::warn!("local LLM not available: {e}"),
        }
    }
    // FFTProvider is always available — ultimate safety floor
    chain = chain.with(Box::new(FFTProvider::new(SignalSpec {
        freq: 2.0,
        sample_rate: 100.0,
        duration_secs: 1.0,
        noise_std: 0.0,
    })));

    chain
}

fn run_perceive(args: &PerceiveArgs) -> Result<()> {
    let cache_dir = dataforge_cache_dir(args.cache_dir.as_ref());
    let registry = build_registry(&cache_dir);

    // Build tokio runtime for async dataforge fetching
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("failed to build tokio runtime")?;

    // Step 1: Fetch raw signals from all data sources
    eprintln!("Fetching data from {} sources...", registry.source_count());
    let raw_signals = rt.block_on(registry.fetch_all());
    eprintln!("Collected {} raw signals", raw_signals.len());

    if raw_signals.is_empty() {
        anyhow::bail!("no signals collected — all sources may be offline or rate-limited");
    }

    // Step 2: Build prism engine with the full percept chain (cloud → local → FFT).
    // The chain uses the same ConfigStore as AuroraApp, so if the user has
    // configured API keys, cloud LLM perception is available. Falls back to
    // structured decomposition when no LLM is configured.
    let config = match ConfigStore::open() {
        Ok(c) => Arc::new(c),
        Err(e) => {
            eprintln!("config store unavailable ({}), using FFT-only chain", e);
            return Err(anyhow::anyhow!("config store unavailable: {e}"));
        }
    };
    let percept_chain = build_percept_chain(&config);

    eprintln!(
        "Percept chain: {} provider(s) available",
        percept_chain.provider_count()
    );

    let prism = aurora::percept::prism::PrismEngine::new(
        percept_chain,
        aurora::percept::prism::SourceWeights::with_defaults(),
    );

    eprintln!("Decomposing signals through prism...");
    let batches = prism.perceive_batch(&raw_signals);
    eprintln!("Decomposed {} batches", batches.len());

    let all_tritwords = aurora::percept::prism::PrismEngine::flatten_signals(&batches);

    // If topic filter is set, print a note (topic filtering is advisory —
    // the full signal set goes to trit-core which handles relevance).
    if let Some(ref topic) = args.topic {
        eprintln!("Topic filter: \"{topic}\" (advisory — trit-core resolves relevance)");
    }

    // Step 3: Build a sandbox-compatible scenario from the perceived signals
    // and run through trit-core's ternary pipeline
    let scenario_id = format!(
        "aurora-perceive-{}",
        chrono::Utc::now().format("%Y%m%dT%H%M%SZ")
    );
    let scenario = truncore::sandbox::ScenarioInput {
        id: scenario_id,
        description: args
            .topic
            .clone()
            .unwrap_or_else(|| "perception pipeline".into()),
        domain: "Climate".into(),
        signals: all_tritwords
            .iter()
            .map(|tw| truncore::sandbox::SignalInput {
                frame: tw.frame().to_string(),
                value: tw.value().to_i8(),
                phase: tw.phase().inner(),
                sensor: None,
            })
            .collect(),
        expected_behavior: String::new(),
        environmental_context: None,
    };

    eprintln!("Running ternary decision pipeline...");
    let mut pipeline = truncore::sandbox::SandboxPipeline::default();
    let (output, _diagnostics) = pipeline
        .run_with_diagnostics(&scenario)
        .map_err(|e| anyhow::anyhow!("ternary pipeline failed: {e}"))?;

    // Build perception report
    let report = serde_json::json!({
        "scenario_id": output.scenario_id,
        "final_value": output.final_value,
        "final_value_code": output.final_value_code,
        "final_frame": output.final_frame,
        "final_phase": output.final_phase_raw,
        "interrupts": output.interrupts,
        "policy_action": output.policy_action,
        "reflexive_alert": output.reflexive_alert,
        "attention_cmd": output.attention_cmd,
        "cognitive_offload": output.cognitive_offload,
        "source_summary": {
            "total_raw_signals": raw_signals.len(),
            "decomposed_batches": batches.len(),
            "total_tritwords": all_tritwords.len(),
            "sources": registry.source_names(),
        }
    });

    let report_json = serde_json::to_string_pretty(&report)?;

    match args.output {
        Some(ref path) => {
            fs::write(path, &report_json)
                .with_context(|| format!("failed to write report to {:?}", path))?;
            println!("Perception report written to {}", path.display());
        }
        None => {
            println!("{}", report_json);
        }
    }

    Ok(())
}

// ── main ─────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Pipeline(ref pargs) => run_pipeline(pargs),
        Command::Sources(ref sargs) => {
            // run_sources in a separate module to avoid circular imports
            run_sources(sargs)
        }
        Command::Perceive(ref pargs) => run_perceive(pargs),
    }
}
