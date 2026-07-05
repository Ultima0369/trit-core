//! Command-line interface for Aurora.
//!
//! Supports three subcommands:
//! - `pipeline`: The original FFT → ternary decision → attention pipeline.
//! - `sources`: Data source management (list, refresh).
//! - `perceive`: Run the dataforge → prism → trit perception pipeline.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "aurora")]
#[command(about = "Aurora — local-first cognitive sovereignty tool")]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run the analysis + attention pipeline (original FFT path).
    Pipeline(PipelineArgs),
    /// Manage data sources (list, refresh).
    Sources(SourcesArgs),
    /// Run the perception pipeline: dataforge → prism → trit.
    Perceive(PerceiveArgs),
}

// ── pipeline subcommand ──────────────────────────────────────────────

#[derive(Debug, clap::Args)]
pub struct PipelineArgs {
    /// Path to input JSON file describing the synthetic signal.
    #[arg(short, long)]
    pub input: PathBuf,

    /// Path to output HTML report (optional; defaults to stdout with JSON).
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Frequency threshold separating high/active from low/quiet embodied states.
    #[arg(long, default_value_t = 2.0)]
    pub frequency_threshold: f64,

    /// Whether the user self-reports feeling normal.
    #[arg(long)]
    pub user_feels_normal: bool,

    /// Path to JSON contacts file for relationship-aware analysis (optional).
    #[arg(long)]
    pub data_source: Option<PathBuf>,

    /// Path to SQLite database (optional; uses in-memory fallback if not set).
    #[arg(long, default_value = ":memory:")]
    pub db_path: String,

    /// Text to run through the LLM perception chain (optional).
    #[arg(long)]
    pub percept: Option<String>,
}

// ── sources subcommand ───────────────────────────────────────────────

#[derive(Debug, clap::Args)]
#[command(about = "Manage data sources (list, refresh)")]
pub struct SourcesArgs {
    #[command(subcommand)]
    pub action: SourcesAction,
}

#[derive(Debug, Subcommand)]
pub enum SourcesAction {
    /// List all registered data sources and their status.
    List,
    /// Force-refresh all data sources (ignore cache).
    Refresh,
}

// ── perceive subcommand ──────────────────────────────────────────────

#[derive(Debug, clap::Args)]
#[command(about = "Run the perception pipeline: fetch data → LLM decompose → ternary firewall")]
pub struct PerceiveArgs {
    /// Topic description for the perception pipeline (e.g. "global climate state").
    ///
    /// If not provided, all registered sources are fetched and perceived
    /// without a topic filter.
    #[arg(short, long)]
    pub topic: Option<String>,

    /// Path to SQLite database (optional; uses in-memory fallback if not set).
    #[arg(long, default_value = ":memory:")]
    pub db_path: String,

    /// Cache directory for dataforge (optional; uses system temp dir).
    #[arg(long)]
    pub cache_dir: Option<PathBuf>,

    /// Output file for the perception report (JSON). Defaults to stdout.
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}
