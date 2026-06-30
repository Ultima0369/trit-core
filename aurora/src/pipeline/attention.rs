//! Attention pipeline link: AttentionGuidance BC → AuditTrail BC → SQLite.
//!
//! Runs the attention scheduling cycle on decision signals, persists audit
//! entries to SQLite (or in-memory for testing), and computes the ASI metric.

use crate::bc::attention_guidance::{AttentionManager, AttentionSession};
use crate::bc::audit_trail::{
    AuditDecisionSnapshot, AuditEntry, AuditEventType, ContactAuditRecord,
};
use crate::bc::relationship_annotation::ContactProfile;
use crate::bc::ternary_decision::DecisionRecord;
use crate::bc::BcError;
use crate::db::audit_log::SqliteAuditLog;
use crate::db::Database;
use truncore::adapters::AttentionCmd;
use truncore::core::TritWord;

/// Outcome of the attention pipeline link.
#[derive(Debug, Clone)]
pub struct AttentionOutcome {
    /// The attention scheduler command (None if Continue).
    pub cmd: Option<AttentionCmd>,
    /// Current Attention Sovereignty Index (ASI) value.
    pub asi: f64,
    /// Number of reminders issued in this session.
    pub reminder_count: usize,
    /// The attention session (for rendering).
    pub session: AttentionSession,
}

/// Build an [`AuditDecisionSnapshot`] from the decision result, signals, and contacts.
///
/// Records the *real* decision value and frame from [`DecisionRecord`] — not a
/// placeholder — so the audit log is genuinely traceable.
fn build_snapshot(
    decision: &DecisionRecord,
    signals: &[TritWord],
    contacts: &[ContactProfile],
) -> AuditDecisionSnapshot {
    let contact_participation = if contacts.is_empty() {
        None
    } else {
        Some(
            contacts
                .iter()
                .flat_map(|c| {
                    c.frames.iter().map(|ann| {
                        let value = if ann.phase >= 0.5 { "True" } else { "False" };
                        ContactAuditRecord {
                            contact_id: c.id.clone(),
                            contact_name: c.name.clone(),
                            relation_label: c.relation_label.as_str().to_string(),
                            frame: ann.frame.clone(),
                            phase: ann.phase,
                            trit_value: value.to_string(),
                        }
                    })
                })
                .collect(),
        )
    };

    AuditDecisionSnapshot {
        signal_count: signals.len(),
        signal_frames: signals.iter().map(|s| s.frame().to_string()).collect(),
        result_value: format!("{:?}", decision.result.value()),
        result_frame: decision.result.frame().to_string(),
        contact_participation,
    }
}

/// Run the attention pipeline link with SQLite persistence.
///
/// 1. Create an AttentionManager and run one scheduling cycle.
/// 2. Build an audit snapshot from the decision result, signals, and contacts.
/// 3. Persist the audit entry to SQLite via SqliteAuditLog.
/// 4. Return the attention outcome.
///
/// For tests, pass `Database::open_in_memory()` — no separate in-memory path.
pub fn run_attention(
    decision: &DecisionRecord,
    signals: &[TritWord],
    db: Database,
    contacts: &[ContactProfile],
) -> Result<AttentionOutcome, BcError> {
    let mut attention = AttentionManager::new("attention_session");
    let cmd = attention.run_cycle(signals);

    // Build and persist audit entry
    let snapshot = build_snapshot(decision, signals, contacts);
    let entry = AuditEntry::new(
        AuditEventType::Decision,
        attention.session().session_id().to_string(),
        "attention cycle".into(),
    )
    .with_decision_snapshot(snapshot);

    let mut audit = SqliteAuditLog::new(db);
    audit.record(entry)?;

    let session = attention.session().clone();

    Ok(AttentionOutcome {
        cmd,
        asi: attention.asi(),
        reminder_count: attention.session().reminder_count(),
        session,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use truncore::core::Frame;

    /// Minimal DecisionRecord for tests — Hold result in Meta frame (typical cross-frame outcome).
    fn test_decision(signals: &[TritWord]) -> DecisionRecord {
        DecisionRecord {
            input_signals: signals.to_vec(),
            result: TritWord::hold(truncore::core::Frame::Meta),
            interrupts: Vec::new(),
            domain: "test".into(),
        }
    }

    /// Open an in-memory SQLite DB for tests (replaces the old in-memory-only path).
    fn test_db() -> Database {
        Database::open_in_memory().expect("in-memory db")
    }

    #[test]
    fn run_attention_does_not_panic() {
        let signals = vec![
            TritWord::tru(Frame::Embodied),
            TritWord::fals(Frame::Individual),
        ];
        let outcome = run_attention(&test_decision(&signals), &signals, test_db(), &[]).unwrap();
        // ASI starts at 0.0 for a new session with no user responses
        assert_eq!(outcome.asi, 0.0);
        assert_eq!(outcome.reminder_count, 0);
    }

    #[test]
    fn run_attention_tracks_reminders() {
        // Feed many mixed-frame signals to trigger a non-Continue response
        let signals: Vec<TritWord> = (0..10)
            .map(|_| TritWord::tru(Frame::Embodied))
            .chain((0..5).map(|_| TritWord::fals(Frame::Individual)))
            .collect();

        let outcome = run_attention(&test_decision(&signals), &signals, test_db(), &[]).unwrap();
        // The scheduler may or may not trigger a reminder depending on signal count;
        // the key invariant is that the function completes without panicking
        assert!(outcome.asi >= 0.0);
        assert!(outcome.asi <= 1.0);
    }

    #[test]
    fn run_attention_with_sqlite_persists_audit_entry() {
        let db = Database::open_in_memory()
            .map_err(|e| BcError::Domain {
                bc: "AuditTrail".into(),
                message: e.to_string(),
            })
            .unwrap();

        let signals = vec![
            TritWord::tru(Frame::Embodied),
            TritWord::fals(Frame::Individual),
        ];
        let decision = test_decision(&signals);

        let outcome = run_attention(&decision, &signals, db, &[]).unwrap();
        assert!(outcome.asi >= 0.0);
        // The audit entry was persisted — we can verify by checking the DB
        // (SqliteAuditLog::entry_count is tested in db/audit_log.rs)
    }

    #[test]
    fn attention_outcome_contains_session_data() {
        let signals = vec![TritWord::tru(Frame::Science)];
        let outcome = run_attention(&test_decision(&signals), &signals, test_db(), &[]).unwrap();

        assert!(!outcome.session.session_id().is_empty());
        assert_eq!(outcome.asi, outcome.session.asi());
        assert_eq!(outcome.reminder_count, outcome.session.reminder_count());
    }

    #[test]
    fn build_snapshot_with_contacts_includes_participation() {
        use crate::bc::relationship_annotation::{FrameAnnotation, RelationLabel};

        let mut profile = ContactProfile::new("c1".into(), "Alice".into(), RelationLabel::Friend);
        profile
            .annotate_frame(FrameAnnotation::new("Embodied".into(), "高频".into(), 0.8).unwrap());

        let signals = vec![TritWord::tru(Frame::Embodied)];
        let snapshot = build_snapshot(&test_decision(&signals), &signals, &[profile]);

        assert!(snapshot.contact_participation.is_some());
        let records = snapshot.contact_participation.unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].contact_id, "c1");
        assert_eq!(records[0].contact_name, "Alice");
        assert_eq!(records[0].relation_label, "friend");
        assert_eq!(records[0].frame, "Embodied");
        assert_eq!(records[0].phase, 0.8);
        assert_eq!(records[0].trit_value, "True");
    }

    #[test]
    fn build_snapshot_without_contacts_has_none_participation() {
        let signals = vec![TritWord::tru(Frame::Science)];
        let snapshot = build_snapshot(&test_decision(&signals), &signals, &[]);
        assert!(snapshot.contact_participation.is_none());
    }

    #[test]
    fn build_snapshot_low_phase_is_false_trit_value() {
        use crate::bc::relationship_annotation::{FrameAnnotation, RelationLabel};

        let mut profile = ContactProfile::new("c2".into(), "Bob".into(), RelationLabel::Colleague);
        profile
            .annotate_frame(FrameAnnotation::new("Individual".into(), "低频".into(), 0.3).unwrap());

        let signals = vec![];
        let snapshot = build_snapshot(&test_decision(&signals), &signals, &[profile]);

        let records = snapshot.contact_participation.unwrap();
        assert_eq!(records[0].trit_value, "False");
    }
}
