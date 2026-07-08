//! Attention session tests — ASI tracking and user response recording.
//!
//! Migrated from old attention module to new bc::attention_guidance.

use aurora::bc::attention_guidance::{AttentionSession, UserResponse};
use trit_core::core::{Frame, TritWord};

#[test]
fn new_session_starts_with_zero_asi() {
    let session = AttentionSession::new("test_session");
    assert_eq!(session.asi(), 0.0);
    assert_eq!(session.reminder_count(), 0);
    assert_eq!(session.user_active_shift_count(), 0);
}

#[test]
fn user_shift_response_increases_active_count_and_asi() {
    let mut session = AttentionSession::new("test_session");
    let _word = TritWord::tru(Frame::Embodied);

    // Record a reminder + user active shift
    session.record_reminder("ShiftTo", "ConflictTrace", "跨 Frame 冲突");
    session.record_user_response(UserResponse::ShiftedTo("ConflictTrace".into()));

    assert_eq!(session.reminder_count(), 1);
    assert_eq!(session.user_active_shift_count(), 1);
    // ASI = (1 active shift) / (1 reminder) * 1.0 = 1.0
    assert!(session.asi() > 0.0);
}

#[test]
fn user_ignore_does_not_increase_asi() {
    let mut session = AttentionSession::new("test_session");

    session.record_reminder("ShiftTo", "ConflictTrace", "跨 Frame 冲突");
    session.record_user_response(UserResponse::Ignored);

    assert_eq!(session.reminder_count(), 1);
    assert_eq!(session.user_active_shift_count(), 0);
    assert_eq!(session.asi(), 0.0);
}

#[test]
fn user_override_hold_increases_active_count() {
    let mut session = AttentionSession::new("test_session");

    session.record_reminder("HoldCurrent", "Meta", "冲突悬置");
    session.record_user_response(UserResponse::OverrodeHold {
        chosen_frame: "Individual".into(),
    });

    assert_eq!(session.user_active_shift_count(), 1);
    assert!(session.asi() > 0.0);
}

#[test]
fn multiple_reminders_with_mixed_responses() {
    let mut session = AttentionSession::new("test_session");

    // Reminder 1: user shifts
    session.record_reminder("ShiftTo", "ConflictTrace", "冲突1");
    session.record_user_response(UserResponse::ShiftedTo("ConflictTrace".into()));

    // Reminder 2: user ignores
    session.record_reminder("ShiftTo", "Body", "冲突2");
    session.record_user_response(UserResponse::Ignored);

    // Reminder 3: user overrides hold
    session.record_reminder("HoldCurrent", "Meta", "冲突3");
    session.record_user_response(UserResponse::OverrodeHold {
        chosen_frame: "Individual".into(),
    });

    assert_eq!(session.reminder_count(), 3);
    assert_eq!(session.user_active_shift_count(), 2);
    // ASI = 2/3 * 1.0 = 0.666...
    assert!(session.asi() > 0.5 && session.asi() < 0.7);
}
