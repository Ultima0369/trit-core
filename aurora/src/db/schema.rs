//! SQL schema constants for Aurora.
//!
//! Each constant is a `CREATE TABLE IF NOT EXISTS` statement.
//! Schema version is tracked in a `schema_version` pragma table.

/// Current schema version. Increment when adding migrations.
pub const SCHEMA_VERSION: i32 = 1;

/// Communication events — imported from mail/calendar/manual sources.
pub const CREATE_COMMUNICATION_EVENTS: &str = "
CREATE TABLE IF NOT EXISTS communication_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    contact_id TEXT NOT NULL,
    source TEXT NOT NULL,
    event_type TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    duration_minutes REAL,
    metadata_json TEXT
);
CREATE INDEX IF NOT EXISTS idx_comm_events_contact ON communication_events(contact_id);
CREATE INDEX IF NOT EXISTS idx_comm_events_timestamp ON communication_events(timestamp);
";

/// Contacts — user-managed relationship profiles.
pub const CREATE_CONTACTS: &str = "
CREATE TABLE IF NOT EXISTS contacts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    relation_label TEXT NOT NULL,
    notes TEXT DEFAULT '',
    deleted INTEGER DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
";

/// Frame annotations — per-contact, per-frame annotations.
pub const CREATE_FRAME_ANNOTATIONS: &str = "
CREATE TABLE IF NOT EXISTS frame_annotations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    contact_id TEXT NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
    frame TEXT NOT NULL,
    annotation TEXT NOT NULL,
    phase REAL NOT NULL CHECK(phase >= 0.0 AND phase <= 1.0),
    created_at TEXT NOT NULL,
    UNIQUE(contact_id, frame)
);
CREATE INDEX IF NOT EXISTS idx_frame_ann_contact ON frame_annotations(contact_id);
";

/// Annotation history — append-only record of changes.
pub const CREATE_ANNOTATION_HISTORY: &str = "
CREATE TABLE IF NOT EXISTS annotation_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    contact_id TEXT NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
    field TEXT NOT NULL,
    old_value TEXT,
    new_value TEXT NOT NULL,
    timestamp TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_ann_history_contact ON annotation_history(contact_id);
";

/// Audit log — append-only, all BCs write here.
pub const CREATE_AUDIT_LOG: &str = "
CREATE TABLE IF NOT EXISTS audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    event_type TEXT NOT NULL,
    session_id TEXT NOT NULL,
    domain TEXT,
    description TEXT NOT NULL,
    snapshot_json TEXT,
    override_json TEXT
);
CREATE INDEX IF NOT EXISTS idx_audit_session ON audit_log(session_id);
CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_log(timestamp);
CREATE INDEX IF NOT EXISTS idx_audit_event_type ON audit_log(event_type);
";

/// All CREATE statements in dependency order.
pub const ALL_TABLES: &[&str] = &[
    CREATE_CONTACTS,
    CREATE_COMMUNICATION_EVENTS,
    CREATE_FRAME_ANNOTATIONS,
    CREATE_ANNOTATION_HISTORY,
    CREATE_AUDIT_LOG,
];
