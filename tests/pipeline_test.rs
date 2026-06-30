//! Integration tests for SandboxPipeline.
//!
//! Extracted from `src/sandbox/pipeline.rs` to keep the pipeline module
//! under the 300-line SRP limit.

use trit_core::adapters::reflexive_audit::ReflexiveAuditor;
use trit_core::adapters::self_knowledge::{ResponsePattern, SelfKnowledge};
use trit_core::adapters::{AttentionCmd, ShiftTarget};
use trit_core::anchor::ecological_base::EcologicalBase;
use trit_core::anchor::thermal_baseline::ThermalBaseline;
use trit_core::anchor::wellbeing_priority::WellbeingPriority;
use trit_core::budget::{ComputeBudget, DepthLevel};
use trit_core::calibration::CalibrationLog;
use trit_core::clock::HarmonicClock;
use trit_core::core::frame::Frame;
use trit_core::core::sensor::EnvironmentalContext;
use trit_core::core::value::TritValue;
use trit_core::sandbox::pipeline::modulate_attention_with_clock_phase;
use trit_core::sandbox::{SandboxPipeline, ScenarioInput, SignalInput};

// These helpers are re-exported from the pipeline module for testing.
// They're used by the modulate_attention_with_clock_phase tests below.

fn scenario(domain: &str, signals: Vec<SignalInput>) -> ScenarioInput {
    ScenarioInput {
        id: "test".into(),
        description: "test".into(),
        domain: domain.into(),
        signals,
        expected_behavior: "hold".into(),
        environmental_context: None,
    }
}

fn signal(frame: &str, value: i8, phase: f64) -> SignalInput {
    SignalInput {
        frame: frame.into(),
        value,
        phase,
        sensor: None,
    }
}

// ── Core pipeline behavior ──────────────────────────────────────

#[test]
fn pipeline_medical_conflict_preserves_individual() {
    let s = scenario(
        "MedicalEthics",
        vec![signal("Science", 1, 0.8), signal("Individual", -1, 0.2)],
    );
    let mut pipeline = SandboxPipeline::default();
    let (out, diag) = pipeline.run_with_diagnostics(&s).unwrap();
    assert_eq!(out.final_value_code, -1);
    assert!(out.policy_action.contains("Preserve"));
    assert_eq!(diag.signal_count, 2);
    assert!(diag.elapsed_ns < 1_000_000_000);
}

#[test]
fn pipeline_value_judgment_holds() {
    let s = scenario(
        "ValueJudgment",
        vec![signal("Individual", -1, 0.3), signal("Consensus", 1, 0.7)],
    );
    let mut pipeline = SandboxPipeline::default();
    let (out, _) = pipeline.run_with_diagnostics(&s).unwrap();
    assert_eq!(out.final_value_code, 0);
    assert!(out.policy_action.contains("Hold"));
}

#[test]
fn pipeline_value_judgment_same_frame_still_holds() {
    let s = scenario(
        "ValueJudgment",
        vec![signal("Science", 1, 0.9), signal("Science", 1, 0.8)],
    );
    let mut pipeline = SandboxPipeline::default();
    let (out, _) = pipeline.run_with_diagnostics(&s).unwrap();
    assert_eq!(out.final_value_code, 0);
    assert_eq!(out.final_frame, "Meta");
    assert!(out.policy_action.contains("Hold"));
}

#[test]
fn pipeline_engineering_commits_false() {
    let s = scenario(
        "Engineering",
        vec![signal("Individual", 1, 0.6), signal("Science", -1, 0.4)],
    );
    let mut pipeline = SandboxPipeline::default();
    let out = pipeline.run(&s).unwrap();
    assert_eq!(out.final_value_code, -1);
}

#[test]
fn pipeline_rejects_invalid_frame() {
    let s = scenario("General", vec![signal("Bogus", 1, 0.5)]);
    let mut pipeline = SandboxPipeline::default();
    assert!(pipeline.run(&s).is_err());
}

#[test]
fn pipeline_rejects_invalid_phase() {
    let s = scenario("General", vec![signal("Science", 1, 1.5)]);
    let mut pipeline = SandboxPipeline::default();
    assert!(pipeline.run(&s).is_err());
}

#[test]
fn pipeline_rejects_empty_signals() {
    let s = scenario("General", vec![]);
    let mut pipeline = SandboxPipeline::default();
    assert!(pipeline.run(&s).is_err());
}

#[test]
fn pipeline_rejects_invalid_domain() {
    let s = scenario("Bogus", vec![signal("Science", 1, 0.5)]);
    let mut pipeline = SandboxPipeline::default();
    assert!(pipeline.run(&s).is_err());
}

#[test]
fn pipeline_single_signal_commits() {
    let s = scenario("General", vec![signal("Science", 1, 0.9)]);
    let mut pipeline = SandboxPipeline::default();
    let out = pipeline.run(&s).unwrap();
    assert_eq!(out.final_value_code, 1);
    assert_eq!(out.final_frame, "Science");
}

#[test]
fn pipeline_custom_domain_runs() {
    let s = scenario(
        "Custom(literature)",
        vec![signal("Science", 1, 0.8), signal("Individual", -1, 0.2)],
    );
    let mut pipeline = SandboxPipeline::default();
    let out = pipeline.run(&s).unwrap();
    assert!(out.policy_action.contains("Negotiate"));
}

#[test]
fn pipeline_output_helpers_work() {
    let s = scenario("General", vec![signal("Science", 1, 0.9)]);
    let mut pipeline = SandboxPipeline::default();
    let out = pipeline.run(&s).unwrap();
    assert!(out.is_commit_true());
    assert!(!out.is_commit_false());
    assert!(!out.is_hold());
}

#[test]
fn pipeline_physical_hold_with_interrupts_forces_false() {
    let s = scenario(
        "Physical",
        vec![signal("Individual", 1, 0.9), signal("Consensus", -1, 0.2)],
    );
    let mut pipeline = SandboxPipeline::default();
    let (out, diag) = pipeline.run_with_diagnostics(&s).unwrap();
    assert_eq!(out.final_value_code, -1);
    assert!(!out.interrupts.is_empty());
    assert!(diag.safe_fallback_triggered);
}

// ── Registry tests ──────────────────────────────────────────────

#[test]
fn pipeline_rejects_nonexistent_frame() {
    let s = ScenarioInput {
        id: "test".into(),
        description: "test".into(),
        domain: "General".into(),
        signals: vec![SignalInput {
            frame: "NonExistent".into(),
            value: 1,
            phase: 0.5,
            sensor: None,
        }],
        expected_behavior: "hold".into(),
        environmental_context: None,
    };
    let mut pipeline = SandboxPipeline::new();
    assert!(pipeline.run(&s).is_err());
}

#[test]
fn pipeline_default_runs_scenario() {
    let mut pipeline = SandboxPipeline::default();
    let s = ScenarioInput {
        id: "test".into(),
        description: "test".into(),
        domain: "General".into(),
        signals: vec![SignalInput {
            frame: "Consensus".into(),
            value: -1,
            phase: 0.3,
            sensor: None,
        }],
        expected_behavior: "hold".into(),
        environmental_context: None,
    };
    assert!(pipeline.run(&s).is_ok());
}

// ── Reflexive guard ─────────────────────────────────────────────

#[test]
fn pipeline_reflexive_guard_overrides_forced_collapse() {
    let s = scenario(
        "MedicalEthics",
        vec![signal("Science", 1, 0.9), signal("Individual", -1, 0.2)],
    );
    let mut pipeline = SandboxPipeline::default().with_reflexive(ReflexiveAuditor::new());
    let (out, diag) = pipeline.run_with_diagnostics(&s).unwrap();
    assert!(diag.reflexive_guard_triggered);
    assert_eq!(out.final_value_code, 0);
    assert!(out.reflexive_alert.is_some());
}

#[test]
fn pipeline_first_person_preserved_over_science() {
    let s = scenario(
        "General",
        vec![signal("Science", -1, 0.4), signal("FirstPerson", 1, 0.8)],
    );
    let mut pipeline = SandboxPipeline::default();
    let (out, _) = pipeline.run_with_diagnostics(&s).unwrap();
    assert_eq!(out.final_value_code, 1);
    assert!(out.policy_action.contains("Preserve"));
    assert_eq!(out.final_frame, "FirstPerson");
}

// ── Anchor constraints ──────────────────────────────────────────

#[test]
fn anchor_thermal_baseline_aborts_decision() {
    let s = scenario("General", vec![signal("Science", 1, 0.9)]);
    let mut pipeline =
        SandboxPipeline::default().with_anchor(Box::new(ThermalBaseline::exceeded()));
    let (out, diag) = pipeline.run_with_diagnostics(&s).unwrap();
    assert_eq!(out.final_value_code, 0);
    assert_eq!(out.final_frame, "Meta");
    assert!(diag.anchor_report.is_some());
    assert!(diag.anchor_report.unwrap().has_abort());
}

#[test]
fn anchor_safe_thermal_passes_through() {
    let s = scenario("General", vec![signal("Science", 1, 0.9)]);
    let mut pipeline = SandboxPipeline::default().with_anchor(Box::new(ThermalBaseline::safe()));
    let (out, diag) = pipeline.run_with_diagnostics(&s).unwrap();
    assert_eq!(out.final_value_code, 1);
    assert!(diag.anchor_report.is_none());
}

#[test]
fn anchor_ecological_degraded_aborts() {
    let s = scenario("General", vec![signal("Science", -1, 0.9)]);
    let mut pipeline = SandboxPipeline::default().with_anchor(Box::new(EcologicalBase::degraded()));
    let (out, diag) = pipeline.run_with_diagnostics(&s).unwrap();
    assert_eq!(out.final_value_code, 0);
    assert!(diag.anchor_report.is_some());
}

#[test]
fn anchor_wellbeing_high_irreversible_risk_aborts() {
    let s = ScenarioInput {
        id: "test".into(),
        description: "test".into(),
        domain: "Engineering".into(),
        signals: vec![signal("Science", 1, 0.9)],
        expected_behavior: "hold".into(),
        environmental_context: Some(EnvironmentalContext {
            ambient_arousal: 0.9,
            social_density: 0.5,
            ..Default::default()
        }),
    };
    let mut pipeline = SandboxPipeline::default().with_anchor(Box::new(WellbeingPriority::new()));
    let (out, diag) = pipeline.run_with_diagnostics(&s).unwrap();
    assert_eq!(out.final_value_code, 0);
    assert!(diag.anchor_report.is_some());
}

#[test]
fn anchor_multiple_constraints_all_checked() {
    let s = scenario("General", vec![signal("Science", 1, 0.9)]);
    let mut pipeline = SandboxPipeline::default()
        .with_anchor(Box::new(ThermalBaseline::safe()))
        .with_anchor(Box::new(EcologicalBase::degraded()))
        .with_anchor(Box::new(WellbeingPriority::new()));
    let (out, diag) = pipeline.run_with_diagnostics(&s).unwrap();
    assert_eq!(out.final_value_code, 0);
    assert!(diag.anchor_report.is_some());
    let report = diag.anchor_report.unwrap();
    assert!(report.has_abort());
    assert!(!report.violations.is_empty());
}

// ── Pipeline integration: budget, clock, calibration ────────────

#[test]
fn pipeline_diagnostics_include_depth_level_and_clock_phase() {
    let s = scenario("General", vec![signal("Science", 1, 0.9)]);
    let mut pipeline = SandboxPipeline::default();
    let (_, diag) = pipeline.run_with_diagnostics(&s).unwrap();
    assert!((1..=5).contains(&diag.depth_level));
    assert!((0.0..=1.0).contains(&diag.clock_phase));
}

#[test]
fn pipeline_records_stage_sample_os_budget() {
    let s = scenario("General", vec![signal("Science", 1, 0.9)]);
    let mut pipeline = SandboxPipeline::default();
    let (_, diag) = pipeline.run_with_diagnostics(&s).unwrap();
    assert!(diag.stage_timings_ns.contains_key("sample_os_budget"));
}

#[test]
fn pipeline_records_stage_clock_tick() {
    let s = scenario("General", vec![signal("Science", 1, 0.9)]);
    let mut pipeline = SandboxPipeline::default();
    let (_, diag) = pipeline.run_with_diagnostics(&s).unwrap();
    assert!(diag.stage_timings_ns.contains_key("clock_tick"));
}

#[test]
fn pipeline_records_stage_calibrate() {
    let s = scenario("General", vec![signal("Science", 1, 0.9)]);
    let mut pipeline = SandboxPipeline::default();
    let (_, diag) = pipeline.run_with_diagnostics(&s).unwrap();
    assert!(diag.stage_timings_ns.contains_key("calibrate"));
}

#[test]
fn pipeline_calibration_log_grows_after_runs() {
    let s = scenario("General", vec![signal("Science", 1, 0.9)]);
    let mut pipeline = SandboxPipeline::default();
    assert_eq!(pipeline.calibration_log_len(), 0);
    pipeline.run(&s).unwrap();
    assert_eq!(pipeline.calibration_log_len(), 1);
    pipeline.run(&s).unwrap();
    assert_eq!(pipeline.calibration_log_len(), 2);
}

#[test]
fn pipeline_clock_advances_after_run() {
    let s = scenario("General", vec![signal("Science", 1, 0.9)]);
    let mut pipeline = SandboxPipeline::default();
    let t_before = pipeline.clock_elapsed();
    pipeline.run(&s).unwrap();
    let t_after = pipeline.clock_elapsed();
    assert!(t_after > t_before, "clock should tick forward each run");
}

#[test]
fn pipeline_with_explicit_budget_uses_depth_level() {
    let s = scenario("General", vec![signal("Science", 1, 0.9)]);
    let budget = ComputeBudget::new(DepthLevel::Minimal, 0.95, 0.95, 1);
    let mut pipeline = SandboxPipeline::default().with_budget(budget);
    let (_, diag) = pipeline.run_with_diagnostics(&s).unwrap();
    assert!((1..=5).contains(&diag.depth_level));
}

#[test]
fn pipeline_with_self_knowledge_calibrates() {
    let s = scenario(
        "MedicalEthics",
        vec![signal("Science", 1, 0.8), signal("Individual", -1, 0.2)],
    );
    let knowledge = SelfKnowledge::with_human_defaults();
    let before_count = knowledge.calibration_count();
    let mut pipeline = SandboxPipeline::default()
        .with_self_knowledge(knowledge)
        .with_reflexive(ReflexiveAuditor::new());
    pipeline.run(&s).unwrap();
    let after_count = pipeline.self_knowledge_ref().unwrap().calibration_count();
    assert!(
        after_count >= before_count,
        "calibration count {after_count} should be >= {before_count}"
    );
}

#[test]
fn pipeline_clean_decision_strengthens_pattern() {
    let s = scenario("General", vec![signal("Science", 1, 0.9)]);
    let mut knowledge = SelfKnowledge::with_human_defaults();
    knowledge.add_pattern(ResponsePattern {
        frame: Frame::Science,
        value: TritValue::True,
        phase: 0.5,
        context: "calibrated".to_string(),
    });
    let mut pipeline = SandboxPipeline::default().with_self_knowledge(knowledge);
    pipeline.run(&s).unwrap();
    let sk = pipeline.self_knowledge_ref().unwrap();
    let cal_count = sk.calibration_count();
    assert!(
        cal_count > 0,
        "should have recorded at least one calibration"
    );
}

#[test]
fn pipeline_with_custom_clock_uses_preset() {
    let s = scenario("Physical", vec![signal("Science", 1, 0.9)]);
    let clock = HarmonicClock::physical();
    let mut pipeline = SandboxPipeline::default().with_clock(clock);
    let (_, diag) = pipeline.run_with_diagnostics(&s).unwrap();
    assert!(diag.clock_phase >= 0.0);
    let phase = pipeline.clock_phase_value();
    assert!((0.0..=1.0).contains(&phase));
}

#[test]
fn pipeline_calibration_log_window_evicts() {
    let log = CalibrationLog::new(3);
    let s = scenario("General", vec![signal("Science", 1, 0.9)]);
    let mut pipeline = SandboxPipeline::default().with_calibration_log(log);
    pipeline.run(&s).unwrap();
    pipeline.run(&s).unwrap();
    pipeline.run(&s).unwrap();
    assert_eq!(pipeline.calibration_log_len(), 3);
    pipeline.run(&s).unwrap();
    assert_eq!(pipeline.calibration_log_len(), 3);
}

// ── Clock phase modulation ──────────────────────────────────────

#[test]
fn clock_phase_peak_biases_toward_shift() {
    let result = modulate_attention_with_clock_phase(AttentionCmd::HoldCurrent, 0.9);
    assert!(matches!(
        result,
        AttentionCmd::ShiftTo(ShiftTarget::ConflictTrace)
    ));
}

#[test]
fn clock_phase_trough_biases_toward_hold() {
    let result = modulate_attention_with_clock_phase(AttentionCmd::ShiftTo(ShiftTarget::Body), 0.1);
    assert_eq!(result, AttentionCmd::HoldCurrent);
}

#[test]
fn clock_phase_midrange_passes_through() {
    let result = modulate_attention_with_clock_phase(AttentionCmd::Continue, 0.5);
    assert_eq!(result, AttentionCmd::Continue);
}

#[test]
fn clock_phase_peak_respects_recalibrate() {
    let result = modulate_attention_with_clock_phase(AttentionCmd::Recalibrate, 0.95);
    assert_eq!(result, AttentionCmd::Recalibrate);
}

#[test]
fn clock_phase_trough_respects_recalibrate() {
    let result = modulate_attention_with_clock_phase(AttentionCmd::Recalibrate, 0.05);
    assert_eq!(result, AttentionCmd::Recalibrate);
}

// ── Feedback loop (Layer 5) ────────────────────────────────────────

use trit_core::feedback::proxy_env::StaticRuleModel;
use trit_core::feedback::FeedbackLoop;

#[test]
fn pipeline_with_feedback_runs_cycle() {
    let s = scenario("General", vec![signal("Science", 1, 0.9)]);
    let proxy = StaticRuleModel::new();
    let feedback = FeedbackLoop::new(proxy);
    let mut pipeline = SandboxPipeline::default().with_feedback(feedback);
    let (out, diag) = pipeline.run_with_diagnostics(&s).unwrap();
    assert_eq!(out.final_value_code, 1);
    // Feedback stage should have been recorded
    assert!(diag.stage_timings_ns.contains_key("feedback_loop"));
}

#[test]
fn pipeline_without_feedback_still_works() {
    let s = scenario("General", vec![signal("Science", 1, 0.9)]);
    let mut pipeline = SandboxPipeline::default();
    let (out, diag) = pipeline.run_with_diagnostics(&s).unwrap();
    assert_eq!(out.final_value_code, 1);
    // Without feedback configured, stage should still be recorded (as no-op)
    assert!(diag.stage_timings_ns.contains_key("feedback_loop"));
}

#[test]
fn pipeline_feedback_medical_ethics_preserves_individual() {
    let s = scenario(
        "MedicalEthics",
        vec![signal("Science", 1, 0.8), signal("Individual", -1, 0.2)],
    );
    let proxy = StaticRuleModel::new();
    let feedback = FeedbackLoop::new(proxy);
    let mut pipeline = SandboxPipeline::default().with_feedback(feedback);
    let (out, diag) = pipeline.run_with_diagnostics(&s).unwrap();
    assert_eq!(out.final_value_code, -1);
    assert!(diag.stage_timings_ns.contains_key("feedback_loop"));
}

#[test]
fn pipeline_feedback_disabled_does_nothing() {
    let s = scenario("General", vec![signal("Science", 1, 0.9)]);
    let proxy = StaticRuleModel::new();
    let feedback = FeedbackLoop::new(proxy).with_enabled(false);
    let mut pipeline = SandboxPipeline::default().with_feedback(feedback);
    let (out, _diag) = pipeline.run_with_diagnostics(&s).unwrap();
    assert_eq!(out.final_value_code, 1);
}

#[test]
fn pipeline_feedback_value_judgment_holds() {
    let s = scenario(
        "ValueJudgment",
        vec![signal("Individual", -1, 0.3), signal("Consensus", 1, 0.7)],
    );
    let proxy = StaticRuleModel::new();
    let feedback = FeedbackLoop::new(proxy);
    let mut pipeline = SandboxPipeline::default().with_feedback(feedback);
    let (out, _diag) = pipeline.run_with_diagnostics(&s).unwrap();
    assert_eq!(out.final_value_code, 0);
}
