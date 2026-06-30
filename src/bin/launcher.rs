//! One-click launcher — builds and runs the default scenario, pauses for output.
//!
//! Double-click `trit-launcher.exe` in the project root after `build.bat`.

use std::fs;
use std::path::Path;
use trit_core::sandbox::{SandboxPipeline, ScenarioInput};
use trit_core::tracing_init;

fn main() {
    tracing_init::init();

    let scenario_path = Path::new("scenarios/medical_conflict_01.json");

    let raw = match fs::read_to_string(scenario_path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to read '{}': {}", scenario_path.display(), e);
            eprintln!("\nPress Enter to exit...");
            let _ = std::io::stdin().read_line(&mut String::new());
            return;
        }
    };

    let scenario: ScenarioInput = match serde_json::from_str(&raw) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Malformed JSON: {}", e);
            eprintln!("\nPress Enter to exit...");
            let _ = std::io::stdin().read_line(&mut String::new());
            return;
        }
    };

    println!("\n  trit-core sandbox");
    println!("  scenario : {}", scenario.id);
    println!("  domain   : {}", scenario.domain);
    println!();

    let mut pipeline = SandboxPipeline::default();
    match pipeline.run_with_diagnostics(&scenario) {
        Ok((output, diag)) => {
            println!("  result        : {}", output.final_value);
            println!("  value code    : {}", output.final_value_code);
            println!("  frame         : {}", output.final_frame);
            println!("  phase         : {:.3}", output.final_phase_raw);
            println!("  policy        : {}", output.policy_action);
            println!("  elapsed       : {} µs", diag.elapsed_ns / 1000);
            if !output.interrupts.is_empty() {
                println!("  interrupts    :");
                for i in &output.interrupts {
                    println!("    - {}", i);
                }
            }
        }
        Err(e) => {
            eprintln!("\n  Pipeline error: {}", e.report());
        }
    }

    println!("\n  Done. Press Enter to exit...");
    let _ = std::io::stdin().read_line(&mut String::new());
}
