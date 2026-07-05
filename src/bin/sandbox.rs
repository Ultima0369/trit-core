use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use tracing::{error, info, warn};
use trit_core::sandbox::{
    SandboxError, SandboxOutput, SandboxPipeline, ScenarioInput, ScenarioValidator,
};

/// CLI argument parser for trit-sandbox.
struct Args {
    scenario: String,
    diagnostic: bool,
    validate_only: bool,
    dry_run: bool,
    reflexive: bool,
    hold_final: bool,
    trace_phase: bool,
    self_knowledge: bool,
    cost_data: Option<String>,
    skip_expected_check: bool,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let mut scenario = None;
        let mut diagnostic = false;
        let mut validate_only = false;
        let mut dry_run = false;
        let mut reflexive = false;
        let mut hold_final = false;
        let mut trace_phase = false;
        let mut self_knowledge = false;
        let mut cost_data = None;
        let mut skip_expected_check = false;

        let mut args = std::env::args().skip(1).peekable();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--scenario" => {
                    scenario = Some(
                        args.next()
                            .ok_or("--scenario requires a value")?
                            .to_string(),
                    );
                }
                "--diagnostic" => diagnostic = true,
                "--validate-only" => validate_only = true,
                "--dry-run" => dry_run = true,
                "--reflexive" => reflexive = true,
                "--hold-final" => hold_final = true,
                "--trace-phase" => trace_phase = true,
                "--self-knowledge" => self_knowledge = true,
                "--cost-data" => {
                    cost_data = Some(
                        args.next()
                            .ok_or("--cost-data requires a path to a JSON cost factor file")?
                            .to_string(),
                    );
                }
                "--skip-expected-check" => skip_expected_check = true,
                "-h" | "--help" => {
                    print_usage();
                    std::process::exit(0);
                }
                other => return Err(format!("unknown argument: {}", other)),
            }
        }

        let scenario = scenario.ok_or("missing required argument: --scenario <path.json>")?;

        Ok(Self {
            scenario,
            diagnostic,
            validate_only,
            dry_run,
            reflexive,
            hold_final,
            trace_phase,
            self_knowledge,
            cost_data,
            skip_expected_check,
        })
    }
}

fn print_usage() {
    println!(
        r#"trit-sandbox — run a Trit-Core scenario through the decision pipeline

Usage:
  trit-sandbox --scenario <path.json> [OPTIONS]

Required:
  --scenario <path.json>   Path to a scenario JSON file under the scenarios/ directory

Logging is configured via TRIT_LOG env var (default: info).
Set TRIT_LOG_JSON=0 for human-readable output.

Execution options:
      --diagnostic         Emit a diagnostic report alongside the output
      --validate-only      Validate the scenario and exit without running the pipeline
      --dry-run            Build trits and run TAND, but skip arbitration and SafeFallback
      --reflexive          Enable reflexive audit between arbitration and SafeFallback
      --hold-final         Treat Hold as the final answer (do not auto-question)
      --trace-phase        Output phase shift trajectory in diagnostics
      --self-knowledge     Enable receiver-state inference from self-knowledge
      --cost-data <path>      Load true cost factors from a JSON file (embeds into anchor check)
      --skip-expected-check  Skip same-run expected_behavior comparison (tautological check)
  -h, --help                 Print this help message

Environment:
  TRIT_LOG                 Log filter (e.g., debug, info, warn)
  TRIT_LOG_FILE            Path to write logs to a file
  TRIT_LOG_FORMAT          json | pretty | compact | full
  TRIT_LOG_JSON            0/false to disable JSON logging
"#
    );
}

/// Security: validate scenario file path to prevent path traversal (CWE-22).
fn validate_scenario_path(raw_path: &str) -> Result<PathBuf, String> {
    let path = Path::new(raw_path);

    let canonical = path
        .canonicalize()
        .map_err(|e| format!("Invalid path: {}", e))?;

    let allowed_dir = Path::new("scenarios")
        .canonicalize()
        .map_err(|e| format!("Cannot resolve scenarios dir: {}", e))?;

    if !canonical.starts_with(&allowed_dir) {
        return Err(format!(
            "Path traversal denied: '{}' is outside '{}'",
            canonical.display(),
            allowed_dir.display()
        ));
    }

    match canonical.extension().and_then(|e| e.to_str()) {
        Some("json") => Ok(canonical),
        _ => Err(format!(
            "Invalid file type: only .json files allowed, got {:?}",
            canonical.extension()
        )),
    }
}

fn load_scenario(path: &Path) -> Result<ScenarioInput, SandboxError> {
    use trit_core::sandbox::validate_scenario;

    info!(path = %path.display(), "loading scenario");
    let raw = fs::read_to_string(path)
        .map_err(|e| SandboxError::Io(format!("Failed to read '{}': {}", path.display(), e)))?;
    if raw.len() > trit_core::sandbox::MAX_JSON_SIZE {
        return Err(SandboxError::InvalidScenario(format!(
            "File too large: {} bytes (max {})",
            raw.len(),
            trit_core::sandbox::MAX_JSON_SIZE
        )));
    }
    let scenario: ScenarioInput = serde_json::from_str(&raw)
        .map_err(|e| SandboxError::InvalidScenario(format!("Malformed JSON: {}", e)))?;
    validate_scenario(&scenario)?;
    Ok(scenario)
}

fn run_with_error_context(args: &Args) -> Result<SandboxOutput, SandboxError> {
    let path = validate_scenario_path(&args.scenario).map_err(|reason| {
        error!(reason, "scenario path validation failed");
        SandboxError::Io(format!("Security error: {}", reason))
    })?;

    let scenario = load_scenario(&path)?;
    info!(scenario_id = %scenario.id, domain = %scenario.domain, "scenario loaded");

    if args.validate_only {
        info!("--validate-only requested; skipping pipeline execution");
        // Return a minimal output indicating validation success.
        return Ok(SandboxOutput {
            scenario_id: scenario.id.clone(),
            final_value: "Hold".to_string(),
            final_value_code: 0,
            final_frame: "Meta".to_string(),
            final_phase_raw: 0.5,
            interrupts: vec!["validation-only mode".to_string()],
            policy_action: "ValidateOnly".to_string(),
            reflexive_alert: None,
            attention_cmd: None,
            receiver_estimate: None,
            hold_state: None,
            cost_metadata: None,
            cognitive_offload: None,
        });
    }

    let mut pipeline = SandboxPipeline::default()
        .with_dry_run(args.dry_run)
        .with_hold_final(args.hold_final)
        .with_trace_phase(args.trace_phase);

    if args.reflexive {
        pipeline =
            pipeline.with_reflexive(trit_core::adapters::reflexive_audit::ReflexiveAuditor::new());
    }
    if args.self_knowledge {
        pipeline = pipeline.with_self_knowledge(
            trit_core::adapters::self_knowledge::ResponsePatternCache::with_human_defaults(),
        );
    }
    if let Some(ref cost_path) = args.cost_data {
        let loader =
            trit_core::anchor::cost_factor::JsonFactorLoader::load(std::path::Path::new(cost_path))
                .map_err(|e| {
                    SandboxError::Io(format!("Failed to load cost data '{}': {}", cost_path, e))
                })?;
        pipeline = pipeline.with_cost_factor(loader);
    }

    let (output, diagnostics) = pipeline.run_with_diagnostics(&scenario)?;

    if args.diagnostic {
        eprintln!("\n--- Diagnostic Report ---");
        eprintln!(
            "{}",
            serde_json::to_string_pretty(&diagnostics).map_err(|e| {
                SandboxError::Io(format!("Failed to serialize diagnostics: {}", e))
            })?
        );
        eprintln!("-------------------------\n");
    }

    // Optionally validate expected_behavior if present and non-empty.
    // In dry-run mode arbitration is skipped, so the full-pipeline expectation
    // is not applicable.
    //
    // Note: this is a tautological check — both expected and actual values
    // flow through the same engine. Use --skip-expected-check to disable it.
    if !args.skip_expected_check && !args.dry_run && !scenario.expected_behavior.is_empty() {
        if let Err(e) = ScenarioValidator::validate(&output, &scenario.expected_behavior) {
            warn!(
                scenario_id = %scenario.id,
                expected = %scenario.expected_behavior,
                error = %e,
                "expected behavior mismatch"
            );
            return Err(e);
        }
    }

    Ok(output)
}

fn print_error_report(err: &SandboxError) {
    eprintln!("\n=== Trit-Core Sandbox Error ===");
    eprintln!("{}", err.report());
    eprintln!("=================================\n");
}

fn main() -> ExitCode {
    let args = match Args::parse() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Argument error: {}", e);
            eprintln!("Run with --help for usage information.");
            return ExitCode::from(2);
        }
    };

    trit_core::tracing_init::init();

    match run_with_error_context(&args) {
        Ok(output) => {
            match serde_json::to_string_pretty(&output) {
                Ok(json) => println!("{}", json),
                Err(e) => {
                    error!(error = %e, "failed to serialize output");
                    eprintln!("Internal error: failed to serialize output: {}", e);
                    return ExitCode::from(1);
                }
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            error!(error = %e, category = %e.category_name(), "pipeline failed");
            print_error_report(&e);
            ExitCode::from(1)
        }
    }
}
