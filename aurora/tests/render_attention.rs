//! Attention-aware HTML rendering tests.

use aurora::attention::{AttentionManager, UserResponse};
use aurora::pipeline::DecisionReport;
use truncore::core::{Frame, TritWord};
use truncore::meta::ConflictType;
use truncore::meta::MetaInterrupt;

#[test]
fn html_report_includes_asi_section() {
    let mut attention = AttentionManager::new("test_render");
    attention.run_cycle(&[
        TritWord::tru(Frame::Embodied),
        TritWord::fals(Frame::Individual),
    ]);
    attention.respond(UserResponse::ShiftedTo("ConflictTrace".into()));

    let report = DecisionReport {
        input_freq: 2.5,
        detected_freq: 2.5,
        embodied: TritWord::tru(Frame::Embodied),
        individual: TritWord::fals(Frame::Individual),
        result: TritWord::hold(Frame::Meta),
        interrupt: Some(MetaInterrupt::new(
            ConflictType::FrameMismatch,
            "Embodied vs Individual conflict".to_string(),
        )),
        attention_cmd: None,
        asi: attention.session().asi(),
        reminder_count: attention.session().reminder_count(),
    };

    let html = aurora::render::html::render(&report, attention.session());
    assert!(html.contains("Attention Sovereignty Index"));
    assert!(html.contains("ASI"));
    assert!(html.contains("reminder"));
}

#[test]
fn html_report_includes_conflict_panel_when_interrupt_present() {
    let attention = AttentionManager::new("test_render");
    let report = DecisionReport {
        input_freq: 2.5,
        detected_freq: 2.5,
        embodied: TritWord::tru(Frame::Embodied),
        individual: TritWord::fals(Frame::Individual),
        result: TritWord::hold(Frame::Meta),
        interrupt: Some(MetaInterrupt::new(
            ConflictType::FrameMismatch,
            "Embodied vs Individual conflict".to_string(),
        )),
        attention_cmd: None,
        asi: 0.0,
        reminder_count: 0,
    };

    let html = aurora::render::html::render(&report, attention.session());
    assert!(html.contains("Conflict Panel"));
    assert!(html.contains("FrameMismatch"));
}

#[test]
fn html_report_shows_no_conflict_when_interrupt_absent() {
    let attention = AttentionManager::new("test_render");
    let report = DecisionReport {
        input_freq: 2.5,
        detected_freq: 2.5,
        embodied: TritWord::tru(Frame::Individual),
        individual: TritWord::tru(Frame::Individual),
        result: TritWord::tru(Frame::Individual),
        interrupt: None,
        attention_cmd: None,
        asi: 0.0,
        reminder_count: 0,
    };

    let html = aurora::render::html::render(&report, attention.session());
    assert!(html.contains("No conflict detected"));
}
