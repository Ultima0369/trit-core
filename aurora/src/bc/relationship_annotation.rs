//! RelationshipAnnotation BC — user-managed relationship profiles and frame annotations.
//!
//! # Aggregate root
//! [`ContactProfile`] — a person the user interacts with, annotated with frames.

use crate::bc::BcError;

// ── Entities ──────────────────────────────────────────────────────────────

/// The type of relationship between the user and a contact.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RelationLabel {
    Colleague,
    Friend,
    Family,
    Partner,
    Self_,
    Other(String),
}

impl RelationLabel {
    /// Human-readable label.
    pub fn as_str(&self) -> &str {
        match self {
            RelationLabel::Colleague => "colleague",
            RelationLabel::Friend => "friend",
            RelationLabel::Family => "family",
            RelationLabel::Partner => "partner",
            RelationLabel::Self_ => "self",
            RelationLabel::Other(_) => "other",
        }
    }
}

// ── Deserialization types ───────────────────────────────────────────────────

/// JSON-deserializable contact input format.
///
/// This is the intermediate representation used when loading contacts
/// from a JSON data source. It is converted to [`ContactProfile`] via
/// the `From` trait, which handles `RelationLabel` parsing and
/// `FrameAnnotation` validation.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ContactInput {
    pub id: String,
    pub name: String,
    pub relation_label: String,
    #[serde(default)]
    pub annotations: Vec<FrameAnnotationInput>,
    #[serde(default)]
    pub notes: String,
}

/// JSON-deserializable frame annotation input.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct FrameAnnotationInput {
    pub frame: String,
    #[serde(default)]
    pub annotation: String,
    pub phase: f64,
}

/// A user's annotation of how a contact relates to a specific Frame.
#[derive(Debug, Clone)]
pub struct FrameAnnotation {
    /// The frame being annotated.
    pub frame: String,
    /// Free-text annotation (e.g. "相处后感到消耗").
    pub annotation: String,
    /// Phase [0.0, 1.0] — how strongly this frame applies.
    pub phase: f64,
}

impl FrameAnnotation {
    /// Create a new frame annotation. Phase must be in [0.0, 1.0].
    pub fn new(frame: String, annotation: String, phase: f64) -> Result<Self, BcError> {
        if !(0.0..=1.0).contains(&phase) {
            return Err(BcError::InvalidInput {
                field: "phase".into(),
                reason: format!("must be in [0.0, 1.0], got {phase}"),
            });
        }
        Ok(Self {
            frame,
            annotation,
            phase,
        })
    }
}

/// A single change to a contact's annotations, recorded for history.
#[derive(Debug, Clone)]
pub struct AnnotationChange {
    /// When the change was made (ISO 8601).
    pub timestamp: String,
    /// What field was changed.
    pub field: String,
    /// The previous value (None if this was creation).
    pub old_value: Option<String>,
    /// The new value.
    pub new_value: String,
}

// ── Aggregate root ────────────────────────────────────────────────────────

/// A contact profile — the aggregate root for relationship annotations.
///
/// Each contact has a unique ID, a relation label, frame annotations,
/// and an immutable history of changes.
#[derive(Debug, Clone)]
pub struct ContactProfile {
    /// Unique identifier (user-assigned or generated).
    pub id: String,
    /// Display name.
    pub name: String,
    /// Relationship type.
    pub relation_label: RelationLabel,
    /// Frame annotations for this contact.
    pub frames: Vec<FrameAnnotation>,
    /// Annotation history (append-only).
    pub history: Vec<AnnotationChange>,
    /// Whether this contact is soft-deleted.
    pub deleted: bool,
    /// Free-text notes.
    pub notes: String,
}

impl ContactProfile {
    /// Create a new contact profile.
    pub fn new(id: String, name: String, relation_label: RelationLabel) -> Self {
        Self {
            id,
            name,
            relation_label,
            frames: Vec::new(),
            history: Vec::new(),
            deleted: false,
            notes: String::new(),
        }
    }

    /// Add or update a frame annotation, recording the change in history.
    pub fn annotate_frame(&mut self, annotation: FrameAnnotation) {
        let old = self.frames.iter().position(|f| f.frame == annotation.frame);
        let new_value = annotation.annotation.clone();
        let frame_name = annotation.frame.clone();

        let change = match old {
            Some(idx) => {
                let old_ann = self.frames[idx].annotation.clone();
                self.frames[idx] = annotation;
                AnnotationChange {
                    timestamp: chrono_now(),
                    field: format!("frame.{frame_name}"),
                    old_value: Some(old_ann),
                    new_value,
                }
            }
            None => {
                self.frames.push(annotation);
                AnnotationChange {
                    timestamp: chrono_now(),
                    field: format!("frame.{frame_name}"),
                    old_value: None,
                    new_value,
                }
            }
        };

        self.history.push(change);
    }

    /// Soft-delete this contact (history is preserved).
    pub fn delete(&mut self) {
        self.deleted = true;
        self.history.push(AnnotationChange {
            timestamp: chrono_now(),
            field: "deleted".into(),
            old_value: Some("false".into()),
            new_value: "true".into(),
        });
    }

    /// Number of frame annotations.
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Number of history entries.
    pub fn history_len(&self) -> usize {
        self.history.len()
    }
}

impl From<ContactInput> for ContactProfile {
    fn from(input: ContactInput) -> Self {
        let relation_label = match input.relation_label.as_str() {
            "colleague" => RelationLabel::Colleague,
            "friend" => RelationLabel::Friend,
            "family" => RelationLabel::Family,
            "partner" => RelationLabel::Partner,
            "self" => RelationLabel::Self_,
            other => RelationLabel::Other(other.to_string()),
        };
        let mut profile = ContactProfile::new(input.id, input.name, relation_label);
        profile.notes = input.notes;
        for ann in input.annotations {
            if let Ok(fa) = FrameAnnotation::new(ann.frame, ann.annotation, ann.phase) {
                profile.annotate_frame(fa);
            }
        }
        profile
    }
}

fn chrono_now() -> String {
    chrono::Utc::now().to_rfc3339()
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::annotation_store::SqliteAnnotationStore;

    fn make_profile(id: &str, name: &str) -> ContactProfile {
        ContactProfile::new(id.into(), name.into(), RelationLabel::Friend)
    }

    #[test]
    fn frame_annotation_rejects_invalid_phase() {
        assert!(FrameAnnotation::new("Embodied".into(), "test".into(), 1.5).is_err());
        assert!(FrameAnnotation::new("Embodied".into(), "test".into(), -0.1).is_err());
    }

    #[test]
    fn frame_annotation_accepts_boundary_phases() {
        assert!(FrameAnnotation::new("Embodied".into(), "test".into(), 0.0).is_ok());
        assert!(FrameAnnotation::new("Embodied".into(), "test".into(), 1.0).is_ok());
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
    fn annotate_frame_adds_history() {
        let mut store = SqliteAnnotationStore::new_in_memory().unwrap();
        store.create_contact(make_profile("c1", "Alice")).unwrap();

        let ann = FrameAnnotation::new("Embodied".into(), "高频联系".into(), 0.8).unwrap();
        store.update_annotation("c1", ann).unwrap();

        let contact = store.get_contact_owned("c1").unwrap();
        assert_eq!(contact.frame_count(), 1);
        assert_eq!(contact.history_len(), 1);
    }

    #[test]
    fn annotate_same_frame_replaces_and_records() {
        let mut store = SqliteAnnotationStore::new_in_memory().unwrap();
        store.create_contact(make_profile("c1", "Alice")).unwrap();

        let ann1 = FrameAnnotation::new("Embodied".into(), "高频".into(), 0.8).unwrap();
        store.update_annotation("c1", ann1).unwrap();

        let ann2 = FrameAnnotation::new("Embodied".into(), "中频".into(), 0.5).unwrap();
        store.update_annotation("c1", ann2).unwrap();

        let contact = store.get_contact_owned("c1").unwrap();
        assert_eq!(contact.frame_count(), 1); // same frame, replaced
        assert_eq!(contact.history_len(), 2); // two changes recorded
        assert_eq!(contact.frames[0].annotation, "中频");
    }

    #[test]
    fn soft_delete_preserves_data() {
        let mut store = SqliteAnnotationStore::new_in_memory().unwrap();
        store.create_contact(make_profile("c1", "Alice")).unwrap();
        store.delete_contact("c1").unwrap();

        // Deleted contact still exists in store
        assert_eq!(store.count(), 1);
        // But not in listing
        assert!(store.list_contacts_owned().unwrap().is_empty());
        // And is marked deleted
        let contact = store.get_contact_owned("c1").unwrap();
        assert!(contact.deleted);
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
    fn get_nonexistent_contact_fails() {
        let store = SqliteAnnotationStore::new_in_memory().unwrap();
        assert!(store.get_contact_owned("nonexistent").is_err());
    }

    // ── ContactInput deserialization ────────────────────────────────────

    #[test]
    fn contact_input_deserializes_minimal() {
        let json = r#"{"id":"c1","name":"Alice","relation_label":"friend"}"#;
        let input: ContactInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.id, "c1");
        assert_eq!(input.name, "Alice");
        assert_eq!(input.relation_label, "friend");
        assert!(input.annotations.is_empty());
        assert!(input.notes.is_empty());
    }

    #[test]
    fn contact_input_deserializes_with_annotations() {
        let json = r#"{
            "id": "c1",
            "name": "Alice",
            "relation_label": "colleague",
            "annotations": [
                {"frame": "Embodied", "annotation": "高频", "phase": 0.8}
            ],
            "notes": "test note"
        }"#;
        let input: ContactInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.annotations.len(), 1);
        assert_eq!(input.annotations[0].frame, "Embodied");
        assert_eq!(input.annotations[0].phase, 0.8);
        assert_eq!(input.notes, "test note");
    }

    #[test]
    fn contact_input_converts_to_profile() {
        let input = ContactInput {
            id: "c1".into(),
            name: "Alice".into(),
            relation_label: "friend".into(),
            annotations: vec![FrameAnnotationInput {
                frame: "Embodied".into(),
                annotation: "高频".into(),
                phase: 0.8,
            }],
            notes: "test".into(),
        };
        let profile = ContactProfile::from(input);
        assert_eq!(profile.name, "Alice");
        assert_eq!(profile.relation_label, RelationLabel::Friend);
        assert_eq!(profile.frame_count(), 1);
        assert_eq!(profile.frames[0].phase, 0.8);
        assert_eq!(profile.notes, "test");
    }

    #[test]
    fn contact_input_skips_invalid_phase_in_conversion() {
        let input = ContactInput {
            id: "c1".into(),
            name: "Alice".into(),
            relation_label: "friend".into(),
            annotations: vec![FrameAnnotationInput {
                frame: "Embodied".into(),
                annotation: "bad phase".into(),
                phase: 1.5, // invalid — FrameAnnotation::new will reject
            }],
            notes: String::new(),
        };
        let profile = ContactProfile::from(input);
        // Invalid phase annotation is silently skipped (FrameAnnotation::new returns Err)
        assert_eq!(profile.frame_count(), 0);
    }

    #[test]
    fn contact_input_unknown_relation_label_becomes_other() {
        let input = ContactInput {
            id: "c1".into(),
            name: "Alice".into(),
            relation_label: "mentor".into(),
            annotations: vec![],
            notes: String::new(),
        };
        let profile = ContactProfile::from(input);
        assert_eq!(
            profile.relation_label,
            RelationLabel::Other("mentor".into())
        );
    }
}
