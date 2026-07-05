use serde::Deserialize;
use std::fs;
use std::process::Command;

#[derive(Debug, Deserialize, serde::Serialize)]
struct ScenarioInput {
    id: String,
    description: String,
    domain: String,
    signals: Vec<SignalInput>,
    expected_behavior: String,
}

#[derive(Debug, Deserialize, serde::Serialize)]
struct SignalInput {
    frame: String,
    value: i8,
    phase: f64,
}

fn main() {
    let raw = fs::read_to_string("scenarios/adversarial_audit.json")
        .expect("failed to read adversarial_audit.json");
    let scenarios: Vec<ScenarioInput> =
        serde_json::from_str(&raw).expect("failed to parse adversarial scenarios");

    let binary = if cfg!(debug_assertions) {
        "target/debug/trit-sandbox.exe"
    } else {
        "target/release/trit-sandbox.exe"
    };

    println!("=== Trit-Core 对抗性审计 ===");
    println!("共 {} 个对抗场景\n", scenarios.len());

    let mut passed = 0;
    let mut failed = 0;
    let mut findings: Vec<Finding> = Vec::new();

    for scenario in &scenarios {
        // Write scenario to a temp file in scenarios/ dir (sandbox security policy
        // requires files under the scenarios/ directory).
        let tmp_path = std::path::Path::new("scenarios")
            .join(format!(".audit_{}.json", scenario.id));
        fs::write(&tmp_path, serde_json::to_string(scenario).expect("serialize"))
            .expect("write temp scenario");
        let output = Command::new(binary)
            .arg("--scenario")
            .arg(&tmp_path)
            .env("TRIT_LOG", "off") // suppress tracing JSON logs, get clean output
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output();

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                let success = out.status.success();

                // Try to parse JSON output
                let parsed: Option<serde_json::Value> = serde_json::from_str(&stdout).ok();

                let actual = parsed
                    .as_ref()
                    .and_then(|v| v.get("final_value_code").and_then(|c| c.as_i64()));
                let policy = parsed
                    .as_ref()
                    .and_then(|v| v.get("policy_action").and_then(|c| c.as_str()));

                let expected = &scenario.expected_behavior;
                let matched = if !success {
                    false
                } else {
                    match expected.as_str() {
                        "hold" => actual == Some(0),
                        "commit_true" => actual == Some(1),
                        "commit_false" => actual == Some(-1),
                        "negotiate" => policy.map(|p| p.contains("Negotiate")).unwrap_or(false),
                        _ => false,
                    }
                };

                if matched {
                    passed += 1;
                    println!("✅ {} — {} (pass)", scenario.id, scenario.description);
                } else {
                    failed += 1;
                    let actual_str = if let Some(a) = actual {
                        format!("value_code={}, policy={}", a, policy.unwrap_or("?"))
                    } else if !success {
                        format!("ERROR: {}", stderr.lines().last().unwrap_or("unknown"))
                    } else {
                        format!(
                            "unparseable: {}",
                            stdout.chars().take(200).collect::<String>()
                        )
                    };

                    println!("❌ {} — {} (FAIL)", scenario.id, scenario.description);
                    println!("   expected={}, got={}", expected, actual_str);

                    findings.push(Finding {
                        id: scenario.id.clone(),
                        description: scenario.description.clone(),
                        expected: expected.clone(),
                        actual: actual_str,
                        severity: classify_severity(&scenario.id, &scenario.description),
                    });
                }
                // Clean up temp file.
                let _ = fs::remove_file(&tmp_path);
            }
            Err(e) => {
                failed += 1;
                println!(
                    "❌ {} — {} (CRASH: {})",
                    scenario.id, scenario.description, e
                );
            }
        }
    }

    println!("\n=== 审计结果 ===");
    println!("通过: {}/{}", passed, passed + failed);
    println!("失败: {}/{}", failed, passed + failed);

    if !findings.is_empty() {
        println!("\n--- 发现 ---");
        for f in &findings {
            println!(
                "  [{}] {} — {} (expected={}, got={})",
                f.severity, f.id, f.description, f.expected, f.actual
            );
        }
    }

    // Write audit report
    let report = serde_json::to_string_pretty(&AuditReport {
        total: passed + failed,
        passed,
        failed,
        findings,
    })
    .unwrap();
    fs::write("adversarial_audit_report.json", &report).unwrap();
    println!("\n报告已写入 adversarial_audit_report.json");
}

#[derive(Debug, serde::Serialize)]
struct Finding {
    id: String,
    description: String,
    expected: String,
    actual: String,
    severity: String,
}

#[derive(Debug, serde::Serialize)]
struct AuditReport {
    total: usize,
    passed: usize,
    failed: usize,
    findings: Vec<Finding>,
}

fn classify_severity(_id: &str, _desc: &str) -> String {
    // Simplified classification
    if _id.contains("safe_fallback") || _id.contains("unknown") {
        "P0".to_string()
    } else if _id.contains("phase") || _id.contains("firstperson") {
        "P1".to_string()
    } else {
        "P2".to_string()
    }
}
