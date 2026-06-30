//! Encrypted configuration storage.
//!
//! Uses Windows DPAPI for user-level encryption of API keys and
//! provider settings. Configuration is stored at `%APPDATA%\aurora\config.enc`.
//!
//! # Security
//!
//! - API keys are encrypted on disk via DPAPI (AES-256 under the hood)
//! - Decrypted keys exist only in memory, inside a `Mutex<Option<...>>`
//! - `ConfigStore` does NOT implement `Debug` (prevents log leakage)
//! - DPAPI binds to the current Windows user — different user = cannot decrypt
//! - DPAPI binds to the current machine — copy to another machine = cannot decrypt

pub mod dpapi;
pub mod store;

pub use store::ConfigStore;
