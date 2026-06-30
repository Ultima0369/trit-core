//! SQLite-backed annotation store for the RelationshipAnnotation BC.
//!
//! Stores contacts, frame annotations, and annotation history in SQLite tables.

use crate::bc::relationship_annotation::{
    AnnotationChange, ContactProfile, FrameAnnotation, RelationLabel,
};
use crate::bc::BcError;
use crate::db::Database;

/// SQLite-backed annotation store.
pub struct SqliteAnnotationStore {
    db: Database,
}

impl SqliteAnnotationStore {
    /// Create a new SQLite-backed store from an existing Database.
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Create an in-memory store for testing.
    pub fn new_in_memory() -> Result<Self, BcError> {
        let db = Database::open_in_memory().map_err(|e| BcError::Domain {
            bc: "RelationshipAnnotation".into(),
            message: e.to_string(),
        })?;
        Ok(Self { db })
    }
}

impl SqliteAnnotationStore {
    /// Get a contact by ID (owned).
    pub fn get_contact_owned(&self, id: &str) -> Result<ContactProfile, BcError> {
        let conn = self.db.conn();

        let (name, relation_label_str, notes, deleted): (String, String, String, i32) = conn
            .query_row(
                "SELECT name, relation_label, notes, deleted FROM contacts WHERE id = ?1",
                rusqlite::params![id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .map_err(|_| BcError::NotFound {
                entity: "ContactProfile".into(),
                id: id.into(),
            })?;

        let relation_label = parse_relation_label(&relation_label_str);

        // Load frame annotations
        let mut stmt = conn
            .prepare("SELECT frame, annotation, phase FROM frame_annotations WHERE contact_id = ?1")
            .map_err(|e| BcError::Domain {
                bc: "RelationshipAnnotation".into(),
                message: e.to_string(),
            })?;

        let frames: Vec<FrameAnnotation> = stmt
            .query_map(rusqlite::params![id], |row| {
                Ok(FrameAnnotation {
                    frame: row.get(0)?,
                    annotation: row.get(1)?,
                    phase: row.get(2)?,
                })
            })
            .map_err(|e| BcError::Domain {
                bc: "RelationshipAnnotation".into(),
                message: e.to_string(),
            })?
            .filter_map(|r| r.ok())
            .collect();

        // Load history
        let mut stmt = conn
            .prepare("SELECT field, old_value, new_value, timestamp FROM annotation_history WHERE contact_id = ?1 ORDER BY id")
            .map_err(|e| BcError::Domain {
                bc: "RelationshipAnnotation".into(),
                message: e.to_string(),
            })?;

        let history: Vec<AnnotationChange> = stmt
            .query_map(rusqlite::params![id], |row| {
                Ok(AnnotationChange {
                    field: row.get(0)?,
                    old_value: row.get(1)?,
                    new_value: row.get(2)?,
                    timestamp: row.get(3)?,
                })
            })
            .map_err(|e| BcError::Domain {
                bc: "RelationshipAnnotation".into(),
                message: e.to_string(),
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(ContactProfile {
            id: id.to_string(),
            name,
            relation_label,
            frames,
            history,
            deleted: deleted != 0,
            notes,
        })
    }

    /// List all non-deleted contacts (owned).
    pub fn list_contacts_owned(&self) -> Result<Vec<ContactProfile>, BcError> {
        let conn = self.db.conn();
        let mut stmt = conn
            .prepare("SELECT id FROM contacts WHERE deleted = 0")
            .map_err(|e| BcError::Domain {
                bc: "RelationshipAnnotation".into(),
                message: e.to_string(),
            })?;

        let ids: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| BcError::Domain {
                bc: "RelationshipAnnotation".into(),
                message: e.to_string(),
            })?
            .filter_map(|r| r.ok())
            .collect();

        ids.iter().map(|id| self.get_contact_owned(id)).collect()
    }

    /// Create a new contact.
    pub fn create_contact(&mut self, profile: ContactProfile) -> Result<(), BcError> {
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO contacts (id, name, relation_label, notes, deleted, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                profile.id,
                profile.name,
                profile.relation_label.as_str(),
                profile.notes,
                profile.deleted as i32,
                chrono::Utc::now().to_rfc3339(),
                chrono::Utc::now().to_rfc3339(),
            ],
        )
        .map_err(|e| BcError::Domain {
            bc: "RelationshipAnnotation".into(),
            message: e.to_string(),
        })?;

        // Insert frame annotations
        for ann in &profile.frames {
            conn.execute(
                "INSERT INTO frame_annotations (contact_id, frame, annotation, phase, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    profile.id,
                    ann.frame,
                    ann.annotation,
                    ann.phase,
                    chrono::Utc::now().to_rfc3339(),
                ],
            )
            .map_err(|e| BcError::Domain {
                bc: "RelationshipAnnotation".into(),
                message: e.to_string(),
            })?;
        }

        Ok(())
    }

    /// Update a contact's frame annotation.
    pub fn update_annotation(
        &mut self,
        id: &str,
        annotation: FrameAnnotation,
    ) -> Result<(), BcError> {
        let conn = self.db.conn();
        let now = chrono::Utc::now().to_rfc3339();

        // Check contact exists and is not deleted
        let deleted: i32 = conn
            .query_row(
                "SELECT deleted FROM contacts WHERE id = ?1",
                rusqlite::params![id],
                |row| row.get(0),
            )
            .map_err(|_| BcError::NotFound {
                entity: "ContactProfile".into(),
                id: id.into(),
            })?;

        if deleted != 0 {
            return Err(BcError::InvalidState {
                current: "deleted".into(),
                required: "active".into(),
            });
        }

        // Get old annotation value for history
        let old_value: Option<String> = conn
            .query_row(
                "SELECT annotation FROM frame_annotations WHERE contact_id = ?1 AND frame = ?2",
                rusqlite::params![id, annotation.frame],
                |row| row.get(0),
            )
            .ok();

        // Upsert frame annotation
        conn.execute(
            "INSERT INTO frame_annotations (contact_id, frame, annotation, phase, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(contact_id, frame) DO UPDATE SET
                annotation = excluded.annotation,
                phase = excluded.phase",
            rusqlite::params![
                id,
                annotation.frame,
                annotation.annotation,
                annotation.phase,
                now
            ],
        )
        .map_err(|e| BcError::Domain {
            bc: "RelationshipAnnotation".into(),
            message: e.to_string(),
        })?;

        // Record history
        conn.execute(
            "INSERT INTO annotation_history (contact_id, field, old_value, new_value, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                id,
                format!("frame.{}", annotation.frame),
                old_value,
                annotation.annotation,
                now,
            ],
        )
        .map_err(|e| BcError::Domain {
            bc: "RelationshipAnnotation".into(),
            message: e.to_string(),
        })?;

        // Update contact's updated_at
        conn.execute(
            "UPDATE contacts SET updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, id],
        )
        .map_err(|e| BcError::Domain {
            bc: "RelationshipAnnotation".into(),
            message: e.to_string(),
        })?;

        Ok(())
    }

    /// Soft-delete a contact.
    pub fn delete_contact(&mut self, id: &str) -> Result<(), BcError> {
        let conn = self.db.conn();
        let now = chrono::Utc::now().to_rfc3339();

        let affected = conn
            .execute(
                "UPDATE contacts SET deleted = 1, updated_at = ?1 WHERE id = ?2 AND deleted = 0",
                rusqlite::params![now, id],
            )
            .map_err(|e| BcError::Domain {
                bc: "RelationshipAnnotation".into(),
                message: e.to_string(),
            })?;

        if affected == 0 {
            return Err(BcError::NotFound {
                entity: "ContactProfile".into(),
                id: id.into(),
            });
        }

        // Record history
        conn.execute(
            "INSERT INTO annotation_history (contact_id, field, old_value, new_value, timestamp)
             VALUES (?1, 'deleted', 'false', 'true', ?2)",
            rusqlite::params![id, now],
        )
        .map_err(|e| BcError::Domain {
            bc: "RelationshipAnnotation".into(),
            message: e.to_string(),
        })?;

        Ok(())
    }

    /// Number of contacts (including deleted).
    pub fn count(&self) -> usize {
        self.db
            .conn()
            .query_row("SELECT COUNT(*) FROM contacts", [], |row| row.get(0))
            .unwrap_or(0)
    }
}

fn parse_relation_label(s: &str) -> RelationLabel {
    match s {
        "colleague" => RelationLabel::Colleague,
        "friend" => RelationLabel::Friend,
        "family" => RelationLabel::Family,
        "partner" => RelationLabel::Partner,
        "self" => RelationLabel::Self_,
        other => RelationLabel::Other(other.to_string()),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_profile(id: &str, name: &str) -> ContactProfile {
        ContactProfile::new(id.into(), name.into(), RelationLabel::Friend)
    }

    #[test]
    fn create_and_retrieve_contact() {
        let mut store = SqliteAnnotationStore::new_in_memory().unwrap();
        store.create_contact(make_profile("c1", "Alice")).unwrap();

        let contact = store.get_contact_owned("c1").unwrap();
        assert_eq!(contact.name, "Alice");
        assert_eq!(store.count(), 1);
    }

    #[test]
    fn duplicate_contact_rejected() {
        let mut store = SqliteAnnotationStore::new_in_memory().unwrap();
        store.create_contact(make_profile("c1", "Alice")).unwrap();
        assert!(store.create_contact(make_profile("c1", "Bob")).is_err());
    }

    #[test]
    fn annotate_frame_persists() {
        let mut store = SqliteAnnotationStore::new_in_memory().unwrap();
        store.create_contact(make_profile("c1", "Alice")).unwrap();

        let ann = FrameAnnotation::new("Embodied".into(), "高频联系".into(), 0.8).unwrap();
        store.update_annotation("c1", ann).unwrap();

        let contact = store.get_contact_owned("c1").unwrap();
        assert_eq!(contact.frame_count(), 1);
        assert_eq!(contact.history_len(), 1);
        assert_eq!(contact.frames[0].annotation, "高频联系");
    }

    #[test]
    fn annotate_same_frame_replaces() {
        let mut store = SqliteAnnotationStore::new_in_memory().unwrap();
        store.create_contact(make_profile("c1", "Alice")).unwrap();

        store
            .update_annotation(
                "c1",
                FrameAnnotation::new("Embodied".into(), "高频".into(), 0.8).unwrap(),
            )
            .unwrap();

        store
            .update_annotation(
                "c1",
                FrameAnnotation::new("Embodied".into(), "中频".into(), 0.5).unwrap(),
            )
            .unwrap();

        let contact = store.get_contact_owned("c1").unwrap();
        assert_eq!(contact.frame_count(), 1);
        assert_eq!(contact.history_len(), 2);
        assert_eq!(contact.frames[0].annotation, "中频");
    }

    #[test]
    fn soft_delete_preserves_data() {
        let mut store = SqliteAnnotationStore::new_in_memory().unwrap();
        store.create_contact(make_profile("c1", "Alice")).unwrap();
        store.delete_contact("c1").unwrap();

        assert_eq!(store.count(), 1); // still in DB
        let contact = store.get_contact_owned("c1").unwrap();
        assert!(contact.deleted);
        assert_eq!(contact.history_len(), 1); // deletion recorded
    }

    #[test]
    fn update_deleted_contact_fails() {
        let mut store = SqliteAnnotationStore::new_in_memory().unwrap();
        store.create_contact(make_profile("c1", "Alice")).unwrap();
        store.delete_contact("c1").unwrap();

        let ann = FrameAnnotation::new("Embodied".into(), "test".into(), 0.5).unwrap();
        assert!(store.update_annotation("c1", ann).is_err());
    }

    #[test]
    fn list_contacts_excludes_deleted() {
        let mut store = SqliteAnnotationStore::new_in_memory().unwrap();
        store.create_contact(make_profile("c1", "Alice")).unwrap();
        store.create_contact(make_profile("c2", "Bob")).unwrap();
        store.delete_contact("c1").unwrap();

        let contacts = store.list_contacts_owned().unwrap();
        assert_eq!(contacts.len(), 1);
        assert_eq!(contacts[0].name, "Bob");
    }

    #[test]
    fn multiple_frame_annotations() {
        let mut store = SqliteAnnotationStore::new_in_memory().unwrap();
        store.create_contact(make_profile("c1", "Alice")).unwrap();

        store
            .update_annotation(
                "c1",
                FrameAnnotation::new("Embodied".into(), "高频".into(), 0.8).unwrap(),
            )
            .unwrap();
        store
            .update_annotation(
                "c1",
                FrameAnnotation::new("Individual".into(), "消耗".into(), 0.3).unwrap(),
            )
            .unwrap();
        store
            .update_annotation(
                "c1",
                FrameAnnotation::new("Relational".into(), "依赖".into(), 0.7).unwrap(),
            )
            .unwrap();

        let contact = store.get_contact_owned("c1").unwrap();
        assert_eq!(contact.frame_count(), 3);
        assert_eq!(contact.history_len(), 3);
    }
}
