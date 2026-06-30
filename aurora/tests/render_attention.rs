//! Attention-aware HTML rendering tests.
//!
//! Migrated from old render module to new bc::presentation (AuroraRenderer).

use aurora::bc::attention_guidance::AttentionSession;
use aurora::bc::presentation::{AuroraRenderer, ConflictCard, ViewState};

#[test]
fn html_report_includes_asi_section() {
    let mut session = AttentionSession::new("test_render");
    session.record_reminder("ShiftTo", "ConflictTrace", "test");
    session.record_user_response(aurora::bc::attention_guidance::UserResponse::ShiftedTo(
        "ConflictTrace".into(),
    ));

    let mut view = ViewState::new(
        "Detected frequency: 2.500 Hz | Decision: Hold".into(),
        session,
    );
    view.add_conflict(ConflictCard {
        conflict_type: "FrameMismatch".into(),
        reason: "Embodied vs Individual conflict".into(),
        frame_a: "Embodied".into(),
        frame_b: "Individual".into(),
        acknowledged: false,
    });

    let renderer = AuroraRenderer;
    let html = renderer.render_html(&view);
    assert!(html.contains("Attention Sovereignty Index"));
    assert!(html.contains("ASI"));
    assert!(html.contains("reminder"));
}

#[test]
fn html_report_includes_conflict_panel_when_conflict_present() {
    let session = AttentionSession::new("test_render");
    let mut view = ViewState::new(
        "Detected frequency: 2.500 Hz | Decision: Hold".into(),
        session,
    );
    view.add_conflict(ConflictCard {
        conflict_type: "FrameMismatch".into(),
        reason: "Embodied vs Individual conflict".into(),
        frame_a: "Embodied".into(),
        frame_b: "Individual".into(),
        acknowledged: false,
    });

    let renderer = AuroraRenderer;
    let html = renderer.render_html(&view);
    assert!(html.contains("FrameMismatch"));
}

#[test]
fn html_report_shows_no_conflict_when_empty() {
    let session = AttentionSession::new("test_render");
    let view = ViewState::new(
        "Detected frequency: 2.500 Hz | Decision: True".into(),
        session,
    );

    let renderer = AuroraRenderer;
    let html = renderer.render_html(&view);
    assert!(html.contains("No conflict detected"));
}
