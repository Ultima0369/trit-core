//! Stage 3 end-to-end test: CLI input → HTML output.

use std::process::Command;

#[test]
fn cli_generates_html_report() {
    let output = std::env::temp_dir().join("aurora_test_report.html");

    let status = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--bin",
            "aurora",
            "--",
            "--input",
            "examples/synthetic_2hz.json",
            "--output",
            output.to_str().unwrap(),
            "--frequency-threshold",
            "2.0",
            "--user-feels-normal",
        ])
        .status()
        .expect("failed to run aurora CLI");

    assert!(status.success());
    let html = std::fs::read_to_string(&output).expect("failed to read HTML output");
    assert!(html.contains("Aurora Decision Report"));
    assert!(html.contains("Detected frequency"));
    // New M0 assertions:
    assert!(
        html.contains("Attention Sovereignty Index"),
        "HTML should contain ASI section"
    );
    assert!(
        html.contains("Conflict Panel"),
        "HTML should contain conflict panel"
    );
    assert!(
        html.contains("Reminder History"),
        "HTML should contain reminder history"
    );
    std::fs::remove_file(&output).ok();
}

#[test]
fn cli_prints_json_without_output_flag() {
    let output = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--bin",
            "aurora",
            "--",
            "--input",
            "examples/synthetic_2hz.json",
            "--user-feels-normal",
        ])
        .output()
        .expect("failed to run aurora CLI");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("detected_frequency_hz"));
    assert!(stdout.contains("conflict_detected"));
    assert!(stdout.contains("asi"), "JSON should contain ASI field");
    assert!(
        stdout.contains("reminder_count"),
        "JSON should contain reminder_count"
    );
}
