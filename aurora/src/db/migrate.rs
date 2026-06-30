//! Schema migration runner.
//!
//! Uses a `schema_version` table to track which migrations have been applied.
//! Migrations are idempotent (`CREATE TABLE IF NOT EXISTS`).

use super::DbError;
use rusqlite::Connection;

/// Run all pending migrations.
pub fn run(conn: &Connection) -> Result<(), DbError> {
    // Ensure version tracking table exists
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )?;

    let current: i32 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |row| row.get(0),
    )?;

    if current < super::schema::SCHEMA_VERSION {
        apply_v1(conn)?;
    }

    Ok(())
}

fn apply_v1(conn: &Connection) -> Result<(), DbError> {
    // Wrap in a transaction so a partial failure leaves the DB unchanged.
    // execute_batch runs every statement in the string (CREATE TABLE *and*
    // its trailing CREATE INDEX statements) — conn.execute() would silently
    // run only the first and skip all indexes.
    conn.execute_batch("BEGIN")?;
    for sql in super::schema::ALL_TABLES {
        conn.execute_batch(sql)?;
    }
    conn.execute(
        "INSERT INTO schema_version (version) VALUES (?1)",
        [super::schema::SCHEMA_VERSION],
    )?;
    conn.execute_batch("COMMIT")?;
    Ok(())
}
