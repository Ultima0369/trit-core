//! Aurora application directory management.
//!
//! Creates and manages `~/.aurora/` with correct permissions:
//! - `data/` — SQLite database
//! - `config/` — user configuration
//! - `audit/` — exported audit logs
//!
//! On Unix, the directory is created with mode 0700 (owner-only access).
//! On Windows, the directory inherits parent permissions.

use std::fs;
use std::path::{Path, PathBuf};

/// Manages the Aurora application directory at `~/.aurora/`.
pub struct AuroraDir {
    root: PathBuf,
}

impl AuroraDir {
    /// Create or open the Aurora directory at `~/.aurora/`.
    ///
    /// Creates subdirectories if they don't exist.
    pub fn init() -> Result<Self, DirError> {
        let home = dirs_next().ok_or(DirError::NoHomeDir)?;
        let root = home.join(".aurora");

        // Create root directory if needed
        fs::create_dir_all(&root)?;

        // Set restrictive permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
        }

        // Create subdirectories
        fs::create_dir_all(root.join("data"))?;
        fs::create_dir_all(root.join("config"))?;
        fs::create_dir_all(root.join("audit"))?;

        Ok(Self { root })
    }

    /// Create an AuroraDir rooted at a custom path (for testing).
    pub fn at(path: &Path) -> Result<Self, DirError> {
        fs::create_dir_all(path.join("data"))?;
        fs::create_dir_all(path.join("config"))?;
        fs::create_dir_all(path.join("audit"))?;
        Ok(Self {
            root: path.to_path_buf(),
        })
    }

    /// Path to the root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Path to the SQLite database file.
    pub fn db_path(&self) -> PathBuf {
        self.root.join("data").join("aurora.db")
    }

    /// Path to the data directory.
    pub fn data_dir(&self) -> PathBuf {
        self.root.join("data")
    }

    /// Path to the config directory.
    pub fn config_dir(&self) -> PathBuf {
        self.root.join("config")
    }

    /// Path to the audit export directory.
    pub fn audit_dir(&self) -> PathBuf {
        self.root.join("audit")
    }
}

/// Errors from directory initialization.
#[derive(Debug, thiserror::Error)]
pub enum DirError {
    #[error("cannot determine home directory")]
    NoHomeDir,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Find the user's home directory.
fn dirs_next() -> Option<PathBuf> {
    // Try HOME first (Unix), then USERPROFILE (Windows)
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
        .map(PathBuf::from)
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_path_creates_subdirectories() {
        let tmp = std::env::temp_dir().join("aurora_test_dir");
        // Clean up from previous runs
        let _ = fs::remove_dir_all(&tmp);

        let aurora = AuroraDir::at(&tmp).unwrap();

        assert!(aurora.data_dir().exists());
        assert!(aurora.config_dir().exists());
        assert!(aurora.audit_dir().exists());
        assert!(aurora.db_path().ends_with("aurora.db"));

        // Clean up
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn db_path_is_in_data_dir() {
        let tmp = std::env::temp_dir().join("aurora_test_db_path");
        let _ = fs::remove_dir_all(&tmp);

        let aurora = AuroraDir::at(&tmp).unwrap();
        let db_path = aurora.db_path();
        assert!(db_path.starts_with(aurora.data_dir()));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn repeated_init_is_idempotent() {
        let tmp = std::env::temp_dir().join("aurora_test_idempotent");
        let _ = fs::remove_dir_all(&tmp);

        AuroraDir::at(&tmp).unwrap();
        // Second init should succeed without errors
        AuroraDir::at(&tmp).unwrap();

        let _ = fs::remove_dir_all(&tmp);
    }
}
