//! Stage 3 end-to-end test: CLI input → HTML/JSON output.
//!
//! Updated for M1 BC architecture: uses two independent pipeline links
//! (analysis + attention) with BC presentation renderer.

use std::process::Command;

// ── Existing tests ─────────────────────────────────────────────────────────

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
    assert!(html.contains("Aurora Report"));
    assert!(html.contains("Detected frequency"));
    // M1 assertions:
    assert!(
        html.contains("Attention Sovereignty Index"),
        "HTML should contain ASI section"
    );
    assert!(
        html.contains("Reminder History"),
        "HTML should contain reminder history"
    );
    assert!(
        html.contains("FrameMismatch"),
        "HTML should contain conflict info"
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
    assert!(stdout.contains("decision_summary"));
    assert!(stdout.contains("conflict_count"));
    assert!(stdout.contains("asi"), "JSON should contain ASI field");
    assert!(
        stdout.contains("reminder_count"),
        "JSON should contain reminder_count"
    );
}

#[test]
fn contacts_end_to_end_via_cli() {
    let dir = std::env::temp_dir().join("aurora_test_contacts_e2e");
    std::fs::create_dir_all(&dir).unwrap();

    // Write input signal spec
    let input_path = dir.join("input.json");
    let input_json = r#"{"freq":2.5,"sample_rate":100.0,"duration_secs":1.0,"noise_std":0.0}"#;
    std::fs::write(&input_path, input_json).unwrap();

    // Write contacts JSON
    let contacts_path = dir.join("contacts.json");
    let contacts_json = r#"[
        {"id":"c1","name":"Alice","relation_label":"friend","annotations":[{"frame":"Embodied","annotation":"高频","phase":0.8}]},
        {"id":"c2","name":"Bob","relation_label":"colleague","annotations":[{"frame":"Individual","annotation":"低频","phase":0.3}]}
    ]"#;
    std::fs::write(&contacts_path, contacts_json).unwrap();

    let output_path = dir.join("report.html");

    // Run aurora CLI — resolve from workspace root
    // When running via `cargo test`, CARGO_MANIFEST_DIR points to the package dir.
    // The binary is at workspace_root/target/debug/aurora[.exe]
    let manifest_dir = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into()),
    );
    let workspace_root = manifest_dir.parent().unwrap_or(&manifest_dir);
    let mut binary = workspace_root.join("target").join("debug").join("aurora");
    #[cfg(windows)]
    binary.set_extension("exe");

    assert!(
        binary.exists(),
        "aurora binary not found at {} (workspace_root={})",
        binary.display(),
        workspace_root.display()
    );

    let output = std::process::Command::new(&binary)
        .arg("--input")
        .arg(&input_path)
        .arg("--data-source")
        .arg(&contacts_path)
        .arg("--output")
        .arg(&output_path)
        .arg("--user-feels-normal")
        .output()
        .expect("failed to run aurora");

    assert!(
        output.status.success(),
        "aurora exited with: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    // Read the HTML report
    let html = std::fs::read_to_string(&output_path).unwrap();

    // Verify the report contains the core decision info
    assert!(html.contains("2.5") || html.contains("Hold"));

    // Verify contacts were loaded — check stderr for the "Loaded N contacts" message
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Loaded 2 contacts"),
        "stderr should show contacts loaded: {}",
        stderr
    );

    // Cleanup
    std::fs::remove_dir_all(&dir).ok();
}
