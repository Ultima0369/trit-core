//! AuditTrail BC — immutable audit log of all decisions and user actions.
//!
//! # Aggregate root
//! [`AuditLog`] — append-only sequence of audit entries.
//!
//! # Port
//! [`AuditPort`] trait — the single interface for audit logging.
//!
//! # Design
//! - Append-only: entries can be added but never removed or modified.
//! - Queryable: filter by time range, domain, session, or entry type.
//! - Serializable: all entries are JSON-serializable.

use crate::bc::BcError;
use std::fmt;

// ── Entities ──────────────────────────────────────────────────────────────

/// Types of auditable events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditEventType {
    Decision,
    UserOverride,
    AnnotationChange,
    DataExport,
    ConfigChange,
}

impl fmt::Display for AuditEventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuditEventType::Decision => write!(f, "decision"),
            AuditEventType::UserOverride => write!(f, "user_override"),
            AuditEventType::AnnotationChange => write!(f, "annotation_change"),
            AuditEventType::DataExport => write!(f, "data_export"),
            AuditEventType::ConfigChange => write!(f, "config_change"),
        }
    }
}

/// A snapshot of a decision for audit purposes.
#[derive(Debug, Clone)]
pub struct AuditDecisionSnapshot {
    /// Number of input signals.
    pub signal_count: usize,
    /// Frame of each input signal (debug string).
    pub signal_frames: Vec<String>,
    /// The final decision value.
    pub result_value: String,
    /// The final decision frame.
    pub result_frame: String,
}

/// A record of a user overriding a system output.
#[derive(Debug, Clone)]
pub struct OverrideRecord {
    /// What was overridden (e.g. "Hold", "ShiftTo").
    pub overridden: String,
    /// What the user chose instead.
    pub chosen: String,
}

/// A single entry in the audit log.
#[derive(Debug, Clone)]
pub struct AuditEntry {
    /// ISO 8601 timestamp.
    pub timestamp: String,
    /// Type of event.
    pub event_type: AuditEventType,
    /// Session ID this entry belongs to.
    pub session_id: String,
    /// Domain of the decision (for Decision events).
    pub domain: Option<String>,
    /// Decision snapshot (for Decision events).
    pub decision_snapshot: Option<AuditDecisionSnapshot>,
    /// Override record (for UserOverride events).
    pub override_record: Option<OverrideRecord>,
    /// Free-text description.
    pub description: String,
}

impl AuditEntry {
    /// Create a new audit entry with timestamp.
    pub fn new(event_type: AuditEventType, session_id: String, description: String) -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339(),
            event_type,
            session_id,
            domain: None,
            decision_snapshot: None,
            override_record: None,
            description,
        }
    }

    /// Attach a decision snapshot to this entry.
    pub fn with_decision_snapshot(mut self, snapshot: AuditDecisionSnapshot) -> Self {
        self.decision_snapshot = Some(snapshot);
        self
    }

    /// Attach an override record to this entry.
    pub fn with_override(mut self, record: OverrideRecord) -> Self {
        self.override_record = Some(record);
        self
    }

    /// Set the domain for this entry.
    pub fn with_domain(mut self, domain: String) -> Self {
        self.domain = Some(domain);
        self
    }
}

/// Filter criteria for querying audit entries.
#[derive(Debug, Clone, Default)]
pub struct AuditFilter {
    /// Filter by session ID.
    pub session_id: Option<String>,
    /// Filter by event type.
    pub event_type: Option<AuditEventType>,
    /// Filter entries after this timestamp (inclusive, ISO 8601).
    pub after: Option<String>,
    /// Maximum number of entries to return (None = unlimited).
    pub limit: Option<usize>,
}

impl AuditFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_session(mut self, id: String) -> Self {
        self.session_id = Some(id);
        self
    }

    pub fn with_event_type(mut self, t: AuditEventType) -> Self {
        self.event_type = Some(t);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Check if an entry matches this filter.
    pub fn matches(&self, entry: &AuditEntry) -> bool {
        if let Some(ref sid) = self.session_id {
            if &entry.session_id != sid {
                return false;
            }
        }
        if let Some(ref et) = self.event_type {
            if entry.event_type != *et {
                return false;
            }
        }
        if let Some(ref after) = self.after {
            if entry.timestamp.as_str() < after.as_str() {
                return false;
            }
        }
        true
    }
}

// ── Aggregate root ────────────────────────────────────────────────────────

/// An append-only audit log — the aggregate root.
///
/// Entries can only be added, never removed or modified.
#[derive(Debug, Clone)]
pub struct AuditLog {
    entries: Vec<AuditEntry>,
}

impl AuditLog {
    /// Create a new empty audit log.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Append an entry to the log. Returns the entry index.
    pub fn append(&mut self, entry: AuditEntry) -> usize {
        let idx = self.entries.len();
        self.entries.push(entry);
        idx
    }

    /// Number of entries in the log.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the log is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate over all entries (oldest first).
    pub fn iter(&self) -> impl Iterator<Item = &AuditEntry> {
        self.entries.iter()
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

// ── Port trait ────────────────────────────────────────────────────────────

/// The single interface for audit logging.
///
/// All BCs write to the audit log through this port.
///
/// # Owned vs reference methods
///
/// `query` returns references — suitable for in-memory stores. For SQLite-backed
/// stores, use `query_owned` which returns owned values. The reference method
/// defaults to calling `query_owned` and panicking with a clear message.
pub trait AuditPort {
    /// Record a new audit entry.
    fn record(&mut self, entry: AuditEntry) -> Result<(), BcError>;

    /// Query audit entries matching a filter (reference variant).
    ///
    /// In-memory implementations return references. SQLite-backed
    /// implementations panic — use `query_owned` instead.
    fn query(&self, filter: &AuditFilter) -> Vec<&AuditEntry> {
        let _ = filter;
        unimplemented!("use query_owned() — this store cannot return references")
    }

    /// Query audit entries matching a filter (owned variant).
    ///
    /// This is the primary method for SQLite-backed stores. In-memory
    /// implementations delegate to `query` and clone.
    fn query_owned(&self, filter: &AuditFilter) -> Result<Vec<AuditEntry>, BcError> {
        Ok(self.query(filter).into_iter().cloned().collect())
    }

    /// Total number of entries.
    fn entry_count(&self) -> usize;
}

// ── M0 implementation (in-memory) ─────────────────────────────────────────

/// In-memory audit log implementation for M0.
pub struct InMemoryAuditLog {
    log: AuditLog,
}

impl InMemoryAuditLog {
    pub fn new() -> Self {
        Self {
            log: AuditLog::new(),
        }
    }
}

impl Default for InMemoryAuditLog {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditPort for InMemoryAuditLog {
    fn record(&mut self, entry: AuditEntry) -> Result<(), BcError> {
        self.log.append(entry);
        Ok(())
    }

    fn query(&self, filter: &AuditFilter) -> Vec<&AuditEntry> {
        let mut results: Vec<&AuditEntry> = self.log.iter().filter(|e| filter.matches(e)).collect();

        if let Some(limit) = filter.limit {
            results.truncate(limit);
        }
        results
    }

    fn entry_count(&self) -> usize {
        self.log.len()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_entry_increases_count() {
        let mut log = InMemoryAuditLog::new();
        let entry = AuditEntry::new(
            AuditEventType::Decision,
            "s1".into(),
            "test decision".into(),
        );
        log.record(entry).unwrap();
        assert_eq!(log.entry_count(), 1);
    }

    #[test]
    fn entries_are_append_only() {
        let mut log = InMemoryAuditLog::new();

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

        assert_eq!(log.entry_count(), 2);
        let all: Vec<_> = log.query(&AuditFilter::new()).into_iter().collect();
        assert_eq!(all[0].description, "first");
        assert_eq!(all[1].description, "second");
    }

    #[test]
    fn filter_by_session() {
        let mut log = InMemoryAuditLog::new();

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
        let results = log.query(&filter);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].description, "session 1");
    }

    #[test]
    fn filter_by_event_type() {
        let mut log = InMemoryAuditLog::new();

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
        let results = log.query(&filter);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].description, "config");
    }

    #[test]
    fn filter_with_limit() {
        let mut log = InMemoryAuditLog::new();

        for i in 0..5 {
            log.record(AuditEntry::new(
                AuditEventType::Decision,
                "s1".into(),
                format!("entry {i}"),
            ))
            .unwrap();
        }

        let filter = AuditFilter::new().with_limit(2);
        let results = log.query(&filter);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn entry_with_decision_snapshot() {
        let snapshot = AuditDecisionSnapshot {
            signal_count: 2,
            signal_frames: vec!["Embodied".into(), "Individual".into()],
            result_value: "Hold".into(),
            result_frame: "Meta".into(),
        };

        let entry = AuditEntry::new(AuditEventType::Decision, "s1".into(), "conflict".into())
            .with_decision_snapshot(snapshot)
            .with_domain("Relational".into());

        assert!(entry.decision_snapshot.is_some());
        assert_eq!(entry.domain.as_deref(), Some("Relational"));
        assert_eq!(
            entry.decision_snapshot.as_ref().unwrap().signal_frames[0],
            "Embodied"
        );
    }

    #[test]
    fn entry_with_override() {
        let entry = AuditEntry::new(
            AuditEventType::UserOverride,
            "s1".into(),
            "user overrode Hold".into(),
        )
        .with_override(OverrideRecord {
            overridden: "Hold".into(),
            chosen: "Individual".into(),
        });

        assert!(entry.override_record.is_some());
        assert_eq!(entry.override_record.as_ref().unwrap().overridden, "Hold");
    }

    #[test]
    fn audit_event_type_display() {
        assert_eq!(AuditEventType::Decision.to_string(), "decision");
        assert_eq!(AuditEventType::UserOverride.to_string(), "user_override");
        assert_eq!(
            AuditEventType::AnnotationChange.to_string(),
            "annotation_change"
        );
        assert_eq!(AuditEventType::DataExport.to_string(), "data_export");
        assert_eq!(AuditEventType::ConfigChange.to_string(), "config_change");
    }
}
