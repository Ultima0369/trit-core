//! Command-line interface for Aurora.

use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "aurora")]
#[command(about = "Aurora — local-first cognitive sovereignty tool")]
pub struct Args {
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
}
