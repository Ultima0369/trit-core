//! RelationshipAnnotation BC — user-managed relationship profiles and frame annotations.
//!
//! # Aggregate root
//! [`ContactProfile`] — a person the user interacts with, annotated with frames.
//!
//! # Port
//! [`AnnotationStore`] trait — the single interface for relationship data.

use crate::bc::BcError;
use std::collections::HashMap;

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

fn chrono_now() -> String {
    chrono::Utc::now().to_rfc3339()
}

// ── Port trait ────────────────────────────────────────────────────────────

/// The single interface for relationship annotation storage.
///
/// M0: in-memory HashMap. M1: SQLite-backed.
///
/// # Owned vs reference methods
///
/// `get_contact` and `list_contacts` return references — suitable for
/// in-memory stores. For SQLite-backed stores, use the `_owned` variants
/// (`get_contact_owned`, `list_contacts_owned`) which return owned values.
/// The reference methods default to calling the owned variants and
/// panicking with a clear message directing callers to use `_owned`.
pub trait AnnotationStore {
    /// Get a contact by ID (reference variant).
    ///
    /// In-memory implementations return a reference. SQLite-backed
    /// implementations panic — use `get_contact_owned` instead.
    fn get_contact(&self, id: &str) -> Result<&ContactProfile, BcError> {
        let _ = id;
        unimplemented!("use get_contact_owned() — this store cannot return references")
    }

    /// List all non-deleted contacts (reference variant).
    ///
    /// In-memory implementations return references. SQLite-backed
    /// implementations panic — use `list_contacts_owned` instead.
    fn list_contacts(&self) -> Vec<&ContactProfile> {
        unimplemented!("use list_contacts_owned() — this store cannot return references")
    }

    /// Get a contact by ID (owned variant).
    ///
    /// This is the primary method for SQLite-backed stores. In-memory
    /// implementations delegate to `get_contact` and clone.
    fn get_contact_owned(&self, id: &str) -> Result<ContactProfile, BcError> {
        self.get_contact(id).cloned()
    }

    /// List all non-deleted contacts (owned variant).
    ///
    /// This is the primary method for SQLite-backed stores. In-memory
    /// implementations delegate to `list_contacts` and clone.
    fn list_contacts_owned(&self) -> Result<Vec<ContactProfile>, BcError> {
        Ok(self.list_contacts().into_iter().cloned().collect())
    }

    /// Create a new contact.
    fn create_contact(&mut self, profile: ContactProfile) -> Result<(), BcError>;

    /// Update a contact's frame annotation.
    fn update_annotation(&mut self, id: &str, annotation: FrameAnnotation) -> Result<(), BcError>;

    /// Soft-delete a contact.
    fn delete_contact(&mut self, id: &str) -> Result<(), BcError>;

    /// Number of contacts (including deleted).
    fn count(&self) -> usize;
}

// ── M0 implementation (in-memory) ─────────────────────────────────────────

/// In-memory annotation store for M0.
pub struct InMemoryAnnotationStore {
    contacts: HashMap<String, ContactProfile>,
}

impl InMemoryAnnotationStore {
    pub fn new() -> Self {
        Self {
            contacts: HashMap::new(),
        }
    }
}

impl Default for InMemoryAnnotationStore {
    fn default() -> Self {
        Self::new()
    }
}

impl AnnotationStore for InMemoryAnnotationStore {
    fn get_contact(&self, id: &str) -> Result<&ContactProfile, BcError> {
        self.contacts.get(id).ok_or_else(|| BcError::NotFound {
            entity: "ContactProfile".into(),
            id: id.into(),
        })
    }

    fn list_contacts(&self) -> Vec<&ContactProfile> {
        self.contacts.values().filter(|c| !c.deleted).collect()
    }

    fn create_contact(&mut self, profile: ContactProfile) -> Result<(), BcError> {
        if self.contacts.contains_key(&profile.id) {
            return Err(BcError::InvalidInput {
                field: "id".into(),
                reason: format!("contact {} already exists", profile.id),
            });
        }
        self.contacts.insert(profile.id.clone(), profile);
        Ok(())
    }

    fn update_annotation(&mut self, id: &str, annotation: FrameAnnotation) -> Result<(), BcError> {
        let contact = self.contacts.get_mut(id).ok_or_else(|| BcError::NotFound {
            entity: "ContactProfile".into(),
            id: id.into(),
        })?;
        if contact.deleted {
            return Err(BcError::InvalidState {
                current: "deleted".into(),
                required: "active".into(),
            });
        }
        contact.annotate_frame(annotation);
        Ok(())
    }

    fn delete_contact(&mut self, id: &str) -> Result<(), BcError> {
        let contact = self.contacts.get_mut(id).ok_or_else(|| BcError::NotFound {
            entity: "ContactProfile".into(),
            id: id.into(),
        })?;
        contact.delete();
        Ok(())
    }

    fn count(&self) -> usize {
        self.contacts.len()
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
        let mut store = InMemoryAnnotationStore::new();
        store.create_contact(make_profile("c1", "Alice")).unwrap();
        let contact = store.get_contact("c1").unwrap();
        assert_eq!(contact.name, "Alice");
        assert_eq!(store.count(), 1);
    }

    #[test]
    fn duplicate_contact_rejected() {
        let mut store = InMemoryAnnotationStore::new();
        store.create_contact(make_profile("c1", "Alice")).unwrap();
        assert!(store.create_contact(make_profile("c1", "Bob")).is_err());
    }

    #[test]
    fn annotate_frame_adds_history() {
        let mut store = InMemoryAnnotationStore::new();
        store.create_contact(make_profile("c1", "Alice")).unwrap();

        let ann = FrameAnnotation::new("Embodied".into(), "高频联系".into(), 0.8).unwrap();
        store.update_annotation("c1", ann).unwrap();

        let contact = store.get_contact("c1").unwrap();
        assert_eq!(contact.frame_count(), 1);
        assert_eq!(contact.history_len(), 1);
    }

    #[test]
    fn annotate_same_frame_replaces_and_records() {
        let mut store = InMemoryAnnotationStore::new();
        store.create_contact(make_profile("c1", "Alice")).unwrap();

        let ann1 = FrameAnnotation::new("Embodied".into(), "高频".into(), 0.8).unwrap();
        store.update_annotation("c1", ann1).unwrap();

        let ann2 = FrameAnnotation::new("Embodied".into(), "中频".into(), 0.5).unwrap();
        store.update_annotation("c1", ann2).unwrap();

        let contact = store.get_contact("c1").unwrap();
        assert_eq!(contact.frame_count(), 1); // same frame, replaced
        assert_eq!(contact.history_len(), 2); // two changes recorded
        assert_eq!(contact.frames[0].annotation, "中频");
    }

    #[test]
    fn soft_delete_preserves_data() {
        let mut store = InMemoryAnnotationStore::new();
        store.create_contact(make_profile("c1", "Alice")).unwrap();
        store.delete_contact("c1").unwrap();

        // Deleted contact still exists in store
        assert_eq!(store.count(), 1);
        // But not in listing
        assert!(store.list_contacts().is_empty());
        // And is marked deleted
        let contact = store.get_contact("c1").unwrap();
        assert!(contact.deleted);
    }

    #[test]
    fn update_deleted_contact_fails() {
        let mut store = InMemoryAnnotationStore::new();
        store.create_contact(make_profile("c1", "Alice")).unwrap();
        store.delete_contact("c1").unwrap();

        let ann = FrameAnnotation::new("Embodied".into(), "test".into(), 0.5).unwrap();
        assert!(store.update_annotation("c1", ann).is_err());
    }

    #[test]
    fn get_nonexistent_contact_fails() {
        let store = InMemoryAnnotationStore::new();
        assert!(store.get_contact("nonexistent").is_err());
    }
}
