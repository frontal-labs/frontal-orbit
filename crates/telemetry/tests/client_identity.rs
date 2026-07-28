use orbit_telemetry::ClientIdentity;

#[test]
fn new_creates_identity() {
    let id = ClientIdentity::new("test-app", "1.0.0");
    assert_eq!(id.app_name, "test-app");
    assert_eq!(id.app_version, "1.0.0");
    assert_eq!(id.runtime, "rust");
}

#[test]
fn with_runtime_overrides_default() {
    let id = ClientIdentity::new("test-app", "1.0.0").with_runtime("node");
    assert_eq!(id.runtime, "node");
}

#[test]
fn user_agent_format() {
    let id = ClientIdentity::new("my-app", "2.1.0");
    assert_eq!(id.user_agent(), "my-app/2.1.0");
}

#[test]
fn default_uses_crate_version() {
    let id = ClientIdentity::default();
    assert_eq!(id.app_name, "claude-code");
    assert!(!id.app_version.is_empty());
    assert_eq!(id.runtime, "rust");
}

#[test]
fn default_user_agent() {
    let id = ClientIdentity::default();
    let ua = id.user_agent();
    assert!(ua.starts_with("claude-code/"));
}

#[test]
fn clone_and_debug() {
    let id = ClientIdentity::new("app", "1.0");
    let cloned = id.clone();
    assert_eq!(id, cloned);
    let debug = format!("{id:?}");
    assert!(!debug.is_empty());
}

#[test]
fn serde_roundtrip() {
    let id = ClientIdentity::new("my-app", "3.0.0").with_runtime("python");
    let json = serde_json::to_string(&id).unwrap();
    let deserialized: ClientIdentity = serde_json::from_str(&json).unwrap();
    assert_eq!(id, deserialized);
}
