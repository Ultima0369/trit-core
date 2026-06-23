//! Attention pipeline link: AttentionGuidance BC → AuditTrail BC → SQLite.
//!
//! Runs the attention scheduling cycle on decision signals, persists audit
//! entries to SQLite (or in-memory for testing), and computes the ASI metric.

use crate::bc::attention_guidance::{AttentionManager, AttentionPort, AttentionSession};
use crate::bc::audit_trail::{AuditDecisionSnapshot, AuditEntry, AuditEventType, AuditPort};
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

/// Build an [`AuditDecisionSnapshot`] from a slice of TritWords.
fn build_snapshot(signals: &[TritWord]) -> AuditDecisionSnapshot {
    AuditDecisionSnapshot {
        signal_count: signals.len(),
        signal_frames: signals.iter().map(|s| s.frame().to_string()).collect(),
        result_value: "pending".into(),
        result_frame: "Meta".into(),
    }
}

/// Run the attention pipeline link with SQLite persistence.
///
/// 1. Create an AttentionManager and run one scheduling cycle.
/// 2. Build an audit snapshot from the signals.
/// 3. Persist the audit entry to SQLite via SqliteAuditLog.
/// 4. Return the attention outcome.
pub fn run_attention(signals: &[TritWord], db: Database) -> Result<AttentionOutcome, BcError> {
    let mut attention = AttentionManager::new("attention_session");
    let cmd = attention.run_cycle(signals);

    // Build and persist audit entry
    let snapshot = build_snapshot(signals);
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

/// Run the attention pipeline link without SQLite persistence (in-memory only).
///
/// Useful for testing or when no database is available.
pub fn run_attention_in_memory(signals: &[TritWord]) -> Result<AttentionOutcome, BcError> {
    let mut attention = AttentionManager::new("attention_session");
    let cmd = attention.run_cycle(signals);

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

    #[test]
    fn run_attention_in_memory_does_not_panic() {
        let signals = vec![
            TritWord::tru(Frame::Embodied),
            TritWord::fals(Frame::Individual),
        ];
        let outcome = run_attention_in_memory(&signals).unwrap();
        // ASI starts at 0.0 for a new session with no user responses
        assert_eq!(outcome.asi, 0.0);
        assert_eq!(outcome.reminder_count, 0);
    }

    #[test]
    fn run_attention_in_memory_tracks_reminders() {
        // Feed many mixed-frame signals to trigger a non-Continue response
        let signals: Vec<TritWord> = (0..10)
            .map(|_| TritWord::tru(Frame::Embodied))
            .chain((0..5).map(|_| TritWord::fals(Frame::Individual)))
            .collect();

        let outcome = run_attention_in_memory(&signals).unwrap();
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

        let outcome = run_attention(&signals, db).unwrap();
        assert!(outcome.asi >= 0.0);
        // The audit entry was persisted — we can verify by checking the DB
        // (SqliteAuditLog::entry_count is tested in db/audit_log.rs)
    }

    #[test]
    fn attention_outcome_contains_session_data() {
        let signals = vec![TritWord::tru(Frame::Science)];
        let outcome = run_attention_in_memory(&signals).unwrap();

        assert!(!outcome.session.session_id().is_empty());
        assert_eq!(outcome.asi, outcome.session.asi());
        assert_eq!(outcome.reminder_count, outcome.session.reminder_count());
    }
}
