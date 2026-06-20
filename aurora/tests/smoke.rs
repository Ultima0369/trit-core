//! Smoke tests for the Aurora crate skeleton.

#[test]
fn crate_exports_version() {
    assert_eq!(aurora::version(), "0.1.0");
}
