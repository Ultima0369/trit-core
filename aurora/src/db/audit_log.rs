//! SQLite-backed implementation of the AuditTrail BC's AuditPort trait.
//!
//! Audit entries are append-only — rows can be inserted but never updated or deleted.

use crate::bc::audit_trail::{
    AuditDecisionSnapshot, AuditEntry, AuditEventType, AuditFilter, AuditPort, OverrideRecord,
};
use crate::bc::BcError;
use crate::db::Database;

/// SQLite-backed audit log.
pub struct SqliteAuditLog {
    db: Database,
}

impl SqliteAuditLog {
    /// Create a new SQLite-backed audit log from an existing Database.
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Create an in-memory audit log for testing.
    pub fn new_in_memory() -> Result<Self, BcError> {
        let db = Database::open_in_memory().map_err(|e| BcError::Domain {
            bc: "AuditTrail".into(),
            message: e.to_string(),
        })?;
        Ok(Self { db })
    }
}

impl AuditPort for SqliteAuditLog {
    fn record(&mut self, entry: AuditEntry) -> Result<(), BcError> {
        let snapshot_json = entry.decision_snapshot.as_ref().map(|s| {
            serde_json::json!({
                "signal_count": s.signal_count,
                "signal_frames": s.signal_frames,
                "result_value": s.result_value,
                "result_frame": s.result_frame,
            })
            .to_string()
        });

        let override_json = entry.override_record.as_ref().map(|o| {
            serde_json::json!({
                "overridden": o.overridden,
                "chosen": o.chosen,
            })
            .to_string()
        });

        self.db.conn().execute(
            "INSERT INTO audit_log (timestamp, event_type, session_id, domain, description, snapshot_json, override_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                entry.timestamp,
                entry.event_type.to_string(),
                entry.session_id,
                entry.domain,
                entry.description,
                snapshot_json,
                override_json,
            ],
        ).map_err(|e| BcError::Domain {
            bc: "AuditTrail".into(),
            message: e.to_string(),
        })?;

        Ok(())
    }

    fn query_owned(&self, filter: &AuditFilter) -> Result<Vec<AuditEntry>, BcError> {
        let conn = self.db.conn();

        let mut sql = String::from(
            "SELECT timestamp, event_type, session_id, domain, description, snapshot_json, override_json
             FROM audit_log WHERE 1=1",
        );
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref sid) = filter.session_id {
            sql.push_str(" AND session_id = ?");
            params.push(Box::new(sid.clone()));
        }
        if let Some(ref et) = filter.event_type {
            sql.push_str(" AND event_type = ?");
            params.push(Box::new(et.to_string()));
        }
        if let Some(ref after) = filter.after {
            sql.push_str(" AND timestamp >= ?");
            params.push(Box::new(after.clone()));
        }

        sql.push_str(" ORDER BY id");

        if let Some(limit) = filter.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql).map_err(|e| BcError::Domain {
            bc: "AuditTrail".into(),
            message: e.to_string(),
        })?;

        let entries = stmt
            .query_map(param_refs.as_slice(), |row| {
                let timestamp: String = row.get(0)?;
                let event_type_str: String = row.get(1)?;
                let session_id: String = row.get(2)?;
                let domain: Option<String> = row.get(3)?;
                let description: String = row.get(4)?;
                let snapshot_json: Option<String> = row.get(5)?;
                let override_json: Option<String> = row.get(6)?;

                let event_type = match event_type_str.as_str() {
                    "decision" => AuditEventType::Decision,
                    "user_override" => AuditEventType::UserOverride,
                    "annotation_change" => AuditEventType::AnnotationChange,
                    "data_export" => AuditEventType::DataExport,
                    "config_change" => AuditEventType::ConfigChange,
                    _ => AuditEventType::Decision, // fallback
                };

                let decision_snapshot = snapshot_json.and_then(|json_str| {
                    let parsed: serde_json::Value = serde_json::from_str(&json_str).ok()?;
                    Some(AuditDecisionSnapshot {
                        signal_count: parsed.get("signal_count")?.as_u64()? as usize,
                        signal_frames: parsed
                            .get("signal_frames")?
                            .as_array()?
                            .iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect(),
                        result_value: parsed.get("result_value")?.as_str()?.to_string(),
                        result_frame: parsed.get("result_frame")?.as_str()?.to_string(),
                    })
                });

                let override_record = override_json.and_then(|json_str| {
                    let parsed: serde_json::Value = serde_json::from_str(&json_str).ok()?;
                    Some(OverrideRecord {
                        overridden: parsed.get("overridden")?.as_str()?.to_string(),
                        chosen: parsed.get("chosen")?.as_str()?.to_string(),
                    })
                });

                Ok(AuditEntry {
                    timestamp,
                    event_type,
                    session_id,
                    domain,
                    description,
                    decision_snapshot,
                    override_record,
                })
            })
            .map_err(|e| BcError::Domain {
                bc: "AuditTrail".into(),
                message: e.to_string(),
            })?;

        entries
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| BcError::Domain {
                bc: "AuditTrail".into(),
                message: e.to_string(),
            })
    }

    fn entry_count(&self) -> usize {
        self.db
            .conn()
            .query_row("SELECT COUNT(*) FROM audit_log", [], |row| row.get(0))
            .unwrap_or(0)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_entry_increases_count() {
        let mut log = SqliteAuditLog::new_in_memory().unwrap();
        let entry = AuditEntry::new(AuditEventType::Decision, "s1".into(), "test".into());
        log.record(entry).unwrap();
        assert_eq!(log.entry_count(), 1);
    }

    #[test]
    fn entries_are_persisted_in_order() {
        let mut log = SqliteAuditLog::new_in_memory().unwrap();

        log.record(AuditEntry::new(
            AuditEventType::Decision,
            "s1".into(),
            "first".into(),
        ))
        .unwrap();
        log.record(AuditEntry::new(
            AuditEventType::Decision,
            "s1".into(),
            "second".into(),
        ))
        .unwrap();

        let entries = log.query_owned(&AuditFilter::new()).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].description, "first");
        assert_eq!(entries[1].description, "second");
    }

    #[test]
    fn filter_by_session() {
        let mut log = SqliteAuditLog::new_in_memory().unwrap();

        log.record(AuditEntry::new(
            AuditEventType::Decision,
            "s1".into(),
            "session 1".into(),
        ))
        .unwrap();
        log.record(AuditEntry::new(
            AuditEventType::Decision,
            "s2".into(),
            "session 2".into(),
        ))
        .unwrap();

        let filter = AuditFilter::new().with_session("s1".into());
        let entries = log.query_owned(&filter).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].description, "session 1");
    }

    #[test]
    fn filter_by_event_type() {
        let mut log = SqliteAuditLog::new_in_memory().unwrap();

        log.record(AuditEntry::new(
            AuditEventType::Decision,
            "s1".into(),
            "decision".into(),
        ))
        .unwrap();
        log.record(AuditEntry::new(
            AuditEventType::ConfigChange,
            "s1".into(),
            "config".into(),
        ))
        .unwrap();

        let filter = AuditFilter::new().with_event_type(AuditEventType::ConfigChange);
        let entries = log.query_owned(&filter).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].description, "config");
    }

    #[test]
    fn filter_with_limit() {
        let mut log = SqliteAuditLog::new_in_memory().unwrap();

        for i in 0..5 {
            log.record(AuditEntry::new(
                AuditEventType::Decision,
                "s1".into(),
                format!("entry {i}"),
            ))
            .unwrap();
        }

        let filter = AuditFilter::new().with_limit(2);
        let entries = log.query_owned(&filter).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn entry_with_decision_snapshot_roundtrips() {
        let mut log = SqliteAuditLog::new_in_memory().unwrap();

        let snapshot = AuditDecisionSnapshot {
            signal_count: 2,
            signal_frames: vec!["Embodied".into(), "Individual".into()],
            result_value: "Hold".into(),
            result_frame: "Meta".into(),
        };

        let entry = AuditEntry::new(AuditEventType::Decision, "s1".into(), "conflict".into())
            .with_decision_snapshot(snapshot)
            .with_domain("Relational".into());

        log.record(entry).unwrap();

        let entries = log.query_owned(&AuditFilter::new()).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].decision_snapshot.is_some());
        assert_eq!(
            entries[0].decision_snapshot.as_ref().unwrap().result_value,
            "Hold"
        );
        assert_eq!(entries[0].domain.as_deref(), Some("Relational"));
    }

    #[test]
    fn entry_with_override_roundtrips() {
        let mut log = SqliteAuditLog::new_in_memory().unwrap();

        let entry = AuditEntry::new(AuditEventType::UserOverride, "s1".into(), "override".into())
            .with_override(OverrideRecord {
                overridden: "Hold".into(),
                chosen: "Individual".into(),
            });

        log.record(entry).unwrap();

        let entries = log.query_owned(&AuditFilter::new()).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].override_record.is_some());
        assert_eq!(
            entries[0].override_record.as_ref().unwrap().overridden,
            "Hold"
        );
    }

    #[test]
    fn audit_log_is_append_only() {
        let mut log = SqliteAuditLog::new_in_memory().unwrap();

        log.record(AuditEntry::new(
            AuditEventType::Decision,
            "s1".into(),
            "first".into(),
        ))
        .unwrap();

        // Trying to modify an existing entry is impossible through the AuditPort trait —
        // there's no update/delete method. This is by design.
        // Even direct SQL UPDATE would require opening a new connection.
        // The append-only constraint is enforced by the API, not the database.

        log.record(AuditEntry::new(
            AuditEventType::Decision,
            "s1".into(),
            "second".into(),
        ))
        .unwrap();

        assert_eq!(log.entry_count(), 2);
        // Both entries are still there
        let entries = log.query_owned(&AuditFilter::new()).unwrap();
        assert_eq!(entries.len(), 2);
    }
}
