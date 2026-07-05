//! SQLite data layer for Aurora.
//!
//! Manages the local SQLite database at `~/.aurora/data/aurora.db`.
//! Provides schema migration, connection pooling (single connection for M1),
//! and typed accessors for each BC's data.
//!
//! # Schema overview
//!
//! ```sql
//! -- Communication metadata (read-only, imported from external sources)
//! CREATE TABLE communication_events (
//!     id INTEGER PRIMARY KEY,
//!     contact_id TEXT NOT NULL,
//!     source TEXT NOT NULL,        -- "mail", "calendar", "manual"
//!     event_type TEXT NOT NULL,    -- "email_sent", "email_received", "meeting", etc.
//!     timestamp TEXT NOT NULL,     -- ISO 8601
//!     duration_minutes REAL,
//!     metadata_json TEXT           -- source-specific fields as JSON
//! );
//!
//! -- Relationship annotations (user-managed)
//! CREATE TABLE contacts (
//!     id TEXT PRIMARY KEY,
//!     name TEXT NOT NULL,
//!     relation_label TEXT NOT NULL,
//!     notes TEXT DEFAULT '',
//!     deleted INTEGER DEFAULT 0,
//!     created_at TEXT NOT NULL,
//!     updated_at TEXT NOT NULL
//! );
//!
//! CREATE TABLE frame_annotations (
//!     id INTEGER PRIMARY KEY,
//!     contact_id TEXT NOT NULL REFERENCES contacts(id),
//!     frame TEXT NOT NULL,
//!     annotation TEXT NOT NULL,
//!     phase REAL NOT NULL CHECK(phase >= 0.0 AND phase <= 1.0),
//!     created_at TEXT NOT NULL,
//!     UNIQUE(contact_id, frame)
//! );
//!
//! CREATE TABLE annotation_history (
//!     id INTEGER PRIMARY KEY,
//!     contact_id TEXT NOT NULL REFERENCES contacts(id),
//!     field TEXT NOT NULL,
//!     old_value TEXT,
//!     new_value TEXT NOT NULL,
//!     timestamp TEXT NOT NULL
//! );
//!
//! -- Audit log (append-only)
//! CREATE TABLE audit_log (
//!     id INTEGER PRIMARY KEY,
//!     timestamp TEXT NOT NULL,
//!     event_type TEXT NOT NULL,
//!     session_id TEXT NOT NULL,
//!     domain TEXT,
//!     description TEXT NOT NULL,
//!     snapshot_json TEXT,
//!     override_json TEXT
//! );
//! ```

pub mod annotation_store;
pub mod audit_log;
pub mod migrate;
pub mod schema;

use rusqlite::Connection;
use std::path::{Path, PathBuf};

/// Manages the SQLite connection and provides schema initialization.
///
/// Cloning opens a new connection to the same database file. WAL mode
/// allows multiple concurrent connections safely.
pub struct Database {
    conn: Connection,
    path: PathBuf,
}

impl Clone for Database {
    fn clone(&self) -> Self {
        // Open a fresh connection to the same database file.
        // WAL mode allows multiple concurrent readers/writers.
        //
        // NOTE: Clone for in-memory databases is UNSUPPORTED because each
        // SQLite :memory: connection is independent — cloning would create
        // a new empty database and silently lose all data. Callers must use
        // `&Database` borrowing instead (see SqliteAuditLog::new).
        if self.path == *":memory:" {
            panic!(
                "Database::clone() is not supported for in-memory databases. \
                 Use &Database borrowing (e.g. SqliteAuditLog::new(&db)) instead."
            );
        }
        Database::open(&self.path)
            .unwrap_or_else(|_| panic!("failed to clone database connection to {:?}", self.path))
    }
}

impl Database {
    /// Open (or create) the database at the given path and run migrations.
    pub fn open(path: &Path) -> Result<Self, DbError> {
        let conn = Connection::open(path)?;
        // Enable WAL mode for better concurrent read performance
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let mut db = Self {
            conn,
            path: path.to_path_buf(),
        };
        db.migrate()?;
        Ok(db)
    }

    /// Open an in-memory database for testing.
    pub fn open_in_memory() -> Result<Self, DbError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        let mut db = Self {
            conn,
            path: PathBuf::from(":memory:"),
        };
        db.migrate()?;
        Ok(db)
    }

    /// Run schema migrations.
    fn migrate(&mut self) -> Result<(), DbError> {
        migrate::run(&self.conn)
    }

    /// Access the underlying connection.
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Mutable access to the connection.
    pub fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    /// Path to the database file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Database error type.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Migration error: {0}")]
    Migration(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_in_memory_creates_tables() {
        let db = Database::open_in_memory().unwrap();
        // Verify tables exist by querying sqlite_master
        let mut stmt = db
            .conn()
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap();
        let tables: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"contacts".to_string()));
        assert!(tables.contains(&"frame_annotations".to_string()));
        assert!(tables.contains(&"annotation_history".to_string()));
        assert!(tables.contains(&"audit_log".to_string()));
        assert!(tables.contains(&"communication_events".to_string()));
    }

    #[test]
    fn foreign_keys_are_enforced() {
        let db = Database::open_in_memory().unwrap();
        // Inserting a frame_annotation without a contact should fail
        let result = db.conn().execute(
            "INSERT INTO frame_annotations (contact_id, frame, annotation, phase, created_at)
             VALUES ('nonexistent', 'Embodied', 'test', 0.5, '2026-01-01T00:00:00Z')",
            [],
        );
        assert!(result.is_err());
    }

    #[test]
    fn phase_check_constraint_enforced() {
        let db = Database::open_in_memory().unwrap();
        // First create a contact
        db.conn()
            .execute(
                "INSERT INTO contacts (id, name, relation_label, created_at, updated_at)
                 VALUES ('c1', 'Test', 'friend', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
                [],
            )
            .unwrap();

        // Phase > 1.0 should fail
        let result = db.conn().execute(
            "INSERT INTO frame_annotations (contact_id, frame, annotation, phase, created_at)
             VALUES ('c1', 'Embodied', 'test', 1.5, '2026-01-01T00:00:00Z')",
            [],
        );
        assert!(result.is_err());

        // Phase < 0.0 should fail
        let result = db.conn().execute(
            "INSERT INTO frame_annotations (contact_id, frame, annotation, phase, created_at)
             VALUES ('c1', 'Embodied', 'test', -0.1, '2026-01-01T00:00:00Z')",
            [],
        );
        assert!(result.is_err());

        // Phase in [0, 1] should succeed
        let result = db.conn().execute(
            "INSERT INTO frame_annotations (contact_id, frame, annotation, phase, created_at)
             VALUES ('c1', 'Embodied', 'test', 0.5, '2026-01-01T00:00:00Z')",
            [],
        );
        assert!(result.is_ok());
    }
}
