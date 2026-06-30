use aurora::config::ConfigStore;
use std::fs;

/// Helper: create a ConfigStore pointed at a temp directory.
fn temp_config_store() -> ConfigStore {
    let dir = std::env::temp_dir().join(format!("aurora_test_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.enc");
    ConfigStore::at_path(&path)
}

#[test]
fn missing_key_returns_none() {
    let store = temp_config_store();
    let result = store.get_api_key("nonexistent").unwrap();
    assert!(result.is_none());
}

#[test]
fn set_and_get_api_key_roundtrip() {
    let mut store = temp_config_store();
    store
        .set_api_key("test-provider", "sk-test-key-12345")
        .unwrap();
    let key = store.get_api_key("test-provider").unwrap();
    assert_eq!(key.as_deref(), Some("sk-test-key-12345"));
}

#[test]
fn remove_api_key_clears_it() {
    let mut store = temp_config_store();
    store
        .set_api_key("test-provider", "sk-test-key-12345")
        .unwrap();
    store.remove_api_key("test-provider").unwrap();
    let key = store.get_api_key("test-provider").unwrap();
    assert!(key.is_none());
}

#[test]
fn multiple_keys_independent() {
    let mut store = temp_config_store();
    store.set_api_key("provider-a", "key-a").unwrap();
    store.set_api_key("provider-b", "key-b").unwrap();

    assert_eq!(
        store.get_api_key("provider-a").unwrap().as_deref(),
        Some("key-a")
    );
    assert_eq!(
        store.get_api_key("provider-b").unwrap().as_deref(),
        Some("key-b")
    );

    store.remove_api_key("provider-a").unwrap();
    assert!(store.get_api_key("provider-a").unwrap().is_none());
    assert_eq!(
        store.get_api_key("provider-b").unwrap().as_deref(),
        Some("key-b")
    );
}

#[test]
fn overwrite_existing_key() {
    let mut store = temp_config_store();
    store.set_api_key("provider", "old-key").unwrap();
    store.set_api_key("provider", "new-key").unwrap();
    assert_eq!(
        store.get_api_key("provider").unwrap().as_deref(),
        Some("new-key")
    );
}

#[test]
fn local_model_path_default_none() {
    let store = temp_config_store();
    assert!(store.local_model_path().unwrap().is_none());
}

#[test]
fn set_and_get_local_model_path() {
    let mut store = temp_config_store();
    store
        .set_local_model_path("http://localhost:11434")
        .unwrap();
    assert_eq!(
        store.local_model_path().unwrap().as_deref(),
        Some("http://localhost:11434")
    );
}

#[test]
fn cloud_model_default_none() {
    let store = temp_config_store();
    assert!(store.cloud_model().unwrap().is_none());
}

#[test]
fn set_and_get_cloud_model() {
    let mut store = temp_config_store();
    store.set_cloud_model("claude-opus-4-8").unwrap();
    assert_eq!(
        store.cloud_model().unwrap().as_deref(),
        Some("claude-opus-4-8")
    );
}

#[test]
fn new_config_file_does_not_exist_initially() {
    let dir = std::env::temp_dir().join(format!("aurora_test_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.enc");
    assert!(!path.exists());
    let mut store = ConfigStore::at_path(&path);
    let _ = store.get_api_key("anything").unwrap();
    store.set_api_key("test", "value").unwrap();
    assert!(path.exists());
}
