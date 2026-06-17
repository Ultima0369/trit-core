use std::fs;
use std::path::{Path, PathBuf};
use trit_core::frame::Frame;
use trit_core::meta::{Domain, MetaInterrupt, MetaMonitor, ResolutionPolicy, SafeFallback};
use trit_core::sandbox::{SandboxOutput, ScenarioInput};
use trit_core::trit::algebra::TernaryAlgebra;
use trit_core::trit::{TritValue, TritWord};

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

/// Security: validate scenario content to prevent DoS and injection (CWE-502, CWE-129).
const MAX_JSON_SIZE: usize = 64 * 1024;
const MAX_SIGNALS: usize = 100;
const MAX_STRING_LEN: usize = 1024;

fn sanitize_log_field(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_control() && c != ' ' {
                '\u{FFFD}'
            } else {
                c
            }
        })
        .take(256)
        .collect()
}

fn validate_scenario(scenario: &ScenarioInput) -> Result<(), String> {
    if scenario.id.len() > MAX_STRING_LEN {
        return Err(format!(
            "id too long: {} chars (max {})",
            scenario.id.len(),
            MAX_STRING_LEN
        ));
    }
    if scenario.description.len() > MAX_STRING_LEN * 4 {
        return Err("description too long".to_string());
    }
    if scenario.signals.is_empty() {
        return Err("At least one signal is required".to_string());
    }
    if scenario.signals.len() > MAX_SIGNALS {
        return Err(format!(
            "Too many signals: {} (max {})",
            scenario.signals.len(),
            MAX_SIGNALS
        ));
    }

    match scenario.domain.as_str() {
        "Physical" | "Engineering" | "MedicalEthics" | "ValueJudgment" | "General" => {}
        d if d.starts_with("Custom(") => {} // Custom domain, e.g. "Custom(chemistry)"
        d => return Err(format!("Unknown domain: '{}'", d)),
    }

    for (i, signal) in scenario.signals.iter().enumerate() {
        if signal.phase.is_nan()
            || signal.phase.is_infinite()
            || !(0.0..=1.0).contains(&signal.phase)
        {
            return Err(format!(
                "Signal {}: phase {} is invalid (must be finite in [0.0, 1.0])",
                i, signal.phase
            ));
        }
        if !matches!(signal.value, -1..=1) {
            return Err(format!(
                "Signal {}: value {} is invalid (must be 1, 0, or -1)",
                i, signal.value
            ));
        }
        match signal.frame.as_str() {
            "Science" | "Individual" | "Consensus" | "Absolute" => {}
            f => return Err(format!("Signal {}: unknown frame '{}'", i, f)),
        }
    }

    Ok(())
}

fn main() {
    trit_core::tracing_init::init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 || args[1] != "--scenario" {
        eprintln!("Usage: trit-sandbox --scenario <path.json>");
        std::process::exit(1);
    }

    let path = match validate_scenario_path(&args[2]) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Security error: {}", e);
            std::process::exit(1);
        }
    };

    let raw = match fs::read_to_string(&path) {
        Ok(s) if s.len() <= MAX_JSON_SIZE => s,
        Ok(s) => {
            eprintln!("File too large: {} bytes (max {})", s.len(), MAX_JSON_SIZE);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Failed to read '{}': {}", path.display(), e);
            std::process::exit(1);
        }
    };

    let scenario: ScenarioInput = match serde_json::from_str(&raw) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Malformed JSON: {}", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = validate_scenario(&scenario) {
        eprintln!("Validation error: {}", e);
        std::process::exit(1);
    }

    let policy = match scenario.domain.as_str() {
        "Physical" => ResolutionPolicy::new(Domain::Physical),
        "Engineering" => ResolutionPolicy::new(Domain::Engineering),
        "MedicalEthics" => ResolutionPolicy::new(Domain::MedicalEthics),
        "ValueJudgment" => ResolutionPolicy::new(Domain::ValueJudgment),
        d if d.starts_with("Custom(") => {
            let name = d
                .strip_prefix("Custom(")
                .and_then(|s| s.strip_suffix(")"))
                .unwrap_or("unknown")
                .to_string();
            ResolutionPolicy::new(Domain::Custom(name))
        }
        _ => ResolutionPolicy::new(Domain::General),
    };

    let mut monitor = MetaMonitor::new(policy.clone());

    let trits: Vec<TritWord> = scenario
        .signals
        .iter()
        .map(|s| {
            let frame: Frame = s.frame.parse().unwrap_or(Frame::Meta);
            let val = TritValue::from(s.value);
            TritWord::new(val, s.phase, frame)
        })
        .collect();

    // Aggregate via TAND cascade
    let mut current = trits[0].clone();
    let mut interrupts: Vec<MetaInterrupt> = vec![];

    for next in &trits[1..] {
        let (result, maybe_int) = TernaryAlgebra::t_and(&current, next);
        if let Some(int) = maybe_int {
            monitor.record(int.clone());
            interrupts.push(int);
        }
        current = result;
    }

    // Policy arbitration if still in conflict
    let policy_result = policy.arbitrate(&trits);
    let arbitrated_word = match &policy_result {
        trit_core::meta::ArbitrationResult::Commit(w) => w.clone(),
        trit_core::meta::ArbitrationResult::Preserve(w) => w.clone(),
        _ => current.clone(),
    };

    // SafeFallback: in dangerous domains (Physical, Engineering, registered
    // custom domains), Hold + interrupts forces False per IEC 61508 principles.
    let safe_fallback = SafeFallback::new();
    let (final_word, fb_interrupt) =
        safe_fallback.guard(&policy.domain, &arbitrated_word, interrupts.len());
    if let Some(int) = fb_interrupt {
        monitor.record(int.clone());
        interrupts.push(int);
    }

    let output = SandboxOutput {
        scenario_id: sanitize_log_field(&scenario.id),
        final_value: format!("{:?}", final_word.value),
        final_value_code: final_word.value.to_i8(),
        final_frame: format!("{}", final_word.frame),
        final_phase: final_word.phase.inner(),
        interrupts: interrupts
            .iter()
            .map(|i| format!("{:?}: {}", i.conflict, sanitize_log_field(&i.reason)))
            .collect(),
        policy_action: format!("{:?}", policy_result),
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&output).unwrap_or_else(|e| {
            eprintln!("Failed to serialize output: {}", e);
            std::process::exit(1);
        })
    );
}
