use crate::percept::ConfigError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Decrypted in-memory configuration — never written to disk as plaintext.
///
/// # Security
///
/// `Debug` is manually implemented to mask `api_keys` — a derived `Debug`
/// would print API keys in plaintext to logs and error messages.
#[derive(Clone, Default, Serialize, Deserialize)]
struct DecryptedConfig {
    #[serde(default)]
    api_keys: HashMap<String, String>,
    #[serde(default)]
    local_model_path: Option<String>,
    #[serde(default)]
    cloud_model: Option<String>,
}

impl std::fmt::Debug for DecryptedConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DecryptedConfig")
            .field("api_keys", &format!("[{} keys]", self.api_keys.len()))
            .field("local_model_path", &self.local_model_path)
            .field("cloud_model", &self.cloud_model)
            .finish()
    }
}

/// Encrypted configuration store backed by Windows DPAPI.
///
/// # Security
///
/// - API keys are encrypted on disk via DPAPI
/// - Decrypted keys exist only in memory
/// - This struct intentionally does NOT implement `Debug`
/// - API key values are never logged
pub struct ConfigStore {
    path: PathBuf,
    config: DecryptedConfig,
}

impl ConfigStore {
    /// Open the config store at the default path: `%APPDATA%\aurora\config.enc`.
    pub fn open() -> Result<Self, ConfigError> {
        let path = Self::default_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let config = Self::load(&path)?;
        Ok(Self { path, config })
    }

    /// Open the config store at a specific path (for testing).
    ///
    /// If the file exists but fails to decrypt/parse, logs a warning and falls
    /// back to default — never silently loses a corrupted config without trace.
    pub fn at_path(path: &std::path::Path) -> Self {
        let config = Self::load(path).unwrap_or_else(|e| {
            tracing::warn!(error = %e, path = ?path, "config load failed, using default");
            DecryptedConfig::default()
        });
        Self {
            path: path.to_path_buf(),
            config,
        }
    }

    fn default_path() -> Result<PathBuf, ConfigError> {
        #[cfg(target_os = "windows")]
        {
            let appdata = std::env::var("APPDATA").unwrap_or_else(|_| {
                let home = std::env::var("USERPROFILE").unwrap_or_default();
                format!("{home}\\AppData\\Roaming")
            });
            Ok(PathBuf::from(appdata).join("aurora").join("config.enc"))
        }
        #[cfg(not(target_os = "windows"))]
        {
            let home = std::env::var("HOME").unwrap_or_default();
            Ok(PathBuf::from(home).join(".aurora").join("config.enc"))
        }
    }

    pub fn set_api_key(&mut self, provider: &str, key: &str) -> Result<(), ConfigError> {
        self.config
            .api_keys
            .insert(provider.to_string(), key.to_string());
        self.save_encrypted()
    }

    pub fn get_api_key(&self, provider: &str) -> Result<Option<String>, ConfigError> {
        Ok(self.config.api_keys.get(provider).cloned())
    }

    pub fn remove_api_key(&mut self, provider: &str) -> Result<(), ConfigError> {
        self.config.api_keys.remove(provider);
        self.save_encrypted()
    }

    pub fn local_model_path(&self) -> Result<Option<String>, ConfigError> {
        Ok(self.config.local_model_path.clone())
    }

    pub fn set_local_model_path(&mut self, path: &str) -> Result<(), ConfigError> {
        self.config.local_model_path = Some(path.to_string());
        self.save_encrypted()
    }

    pub fn cloud_model(&self) -> Result<Option<String>, ConfigError> {
        Ok(self.config.cloud_model.clone())
    }

    pub fn set_cloud_model(&mut self, model: &str) -> Result<(), ConfigError> {
        self.config.cloud_model = Some(model.to_string());
        self.save_encrypted()
    }

    /// Load (and decrypt) the config at `path`, or default if it doesn't exist.
    fn load(path: &std::path::Path) -> Result<DecryptedConfig, ConfigError> {
        if !path.exists() {
            return Ok(DecryptedConfig::default());
        }
        let encrypted = fs::read(path)?;
        let plain = crate::config::dpapi::decrypt(&encrypted)?;
        let config: DecryptedConfig = serde_json::from_slice(&plain)?;
        Ok(config)
    }

    fn save_encrypted(&self) -> Result<(), ConfigError> {
        let plain = serde_json::to_vec(&self.config)?;
        let encrypted = crate::config::dpapi::encrypt(&plain)?;
        // Atomic write: config.enc corruption would lose all keys/settings.
        let tmp = self.path.with_extension("tmp");
        fs::write(&tmp, &encrypted)?;
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}
