//! JSON fallback data source tests.

use aurora::ingest::{DataSource, IngestManager, json_fallback::JsonFallbackSource};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct TestContact {
    name: String,
    emails_per_week: f64,
    relation_label: String,
}

#[test]
fn json_fallback_loads_contacts_from_file() {
    // Write a temp JSON file
    let dir = std::env::temp_dir().join("aurora_test_ingest");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("contacts.json");
    std::fs::write(
        &path,
        r#"[
            {"name": "张三", "emails_per_week": 12.0, "relation_label": "colleague"},
            {"name": "李四", "emails_per_week": 3.0, "relation_label": "friend"}
        ]"#,
    )
    .unwrap();

    let source = JsonFallbackSource::new(&path).unwrap();
    let contacts: Vec<TestContact> = source.load().unwrap();

    assert_eq!(contacts.len(), 2);
    assert_eq!(contacts[0].name, "张三");
    assert_eq!(contacts[0].emails_per_week, 12.0);
    assert_eq!(contacts[1].relation_label, "friend");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn json_fallback_returns_error_for_missing_file() {
    let source = JsonFallbackSource::new(std::path::Path::new("/nonexistent/aurora_contacts.json"));
    assert!(source.is_err());
}

#[test]
fn ingest_manager_falls_back_to_json_when_mail_unavailable() {
    let dir = std::env::temp_dir().join("aurora_test_ingest_mgr");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("contacts.json");
    std::fs::write(
        &path,
        r#"[
            {"name": "王五", "emails_per_week": 8.0, "relation_label": "family"}
        ]"#,
    )
    .unwrap();

    let manager = IngestManager::with_json_fallback(&path).unwrap();
    let count = manager.contact_count();
    assert_eq!(count, 1);

    std::fs::remove_dir_all(&dir).ok();
}
