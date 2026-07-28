use orbit_plugins::{PluginPermission, PluginToolPermission};

#[test]
fn permission_as_str() {
    assert_eq!(PluginPermission::Read.as_str(), "read");
    assert_eq!(PluginPermission::Write.as_str(), "write");
    assert_eq!(PluginPermission::Execute.as_str(), "execute");
}

#[test]
fn permission_as_ref_str() {
    let read: &str = PluginPermission::Read.as_ref();
    assert_eq!(read, "read");
    let write: &str = PluginPermission::Write.as_ref();
    assert_eq!(write, "write");
    let execute: &str = PluginPermission::Execute.as_ref();
    assert_eq!(execute, "execute");
}

#[test]
fn permission_serde_roundtrip() {
    for perm in [
        PluginPermission::Read,
        PluginPermission::Write,
        PluginPermission::Execute,
    ] {
        let json = serde_json::to_value(perm).unwrap();
        let deserialized: PluginPermission = serde_json::from_value(json).unwrap();
        assert_eq!(perm, deserialized);
    }
}

#[test]
fn permission_serde_lowercase() {
    assert_eq!(
        serde_json::to_value(PluginPermission::Read).unwrap(),
        "read"
    );
    assert_eq!(
        serde_json::to_value(PluginPermission::Write).unwrap(),
        "write"
    );
    assert_eq!(
        serde_json::to_value(PluginPermission::Execute).unwrap(),
        "execute"
    );
}

#[test]
fn permission_ordering() {
    assert!(PluginPermission::Read < PluginPermission::Write);
    assert!(PluginPermission::Write < PluginPermission::Execute);
    assert!(PluginPermission::Read < PluginPermission::Execute);
}

#[test]
fn tool_permission_as_str() {
    assert_eq!(PluginToolPermission::ReadOnly.as_str(), "read-only");
    assert_eq!(
        PluginToolPermission::WorkspaceWrite.as_str(),
        "workspace-write"
    );
    assert_eq!(
        PluginToolPermission::DangerFullAccess.as_str(),
        "danger-full-access"
    );
}

#[test]
fn tool_permission_serde_roundtrip() {
    for perm in [
        PluginToolPermission::ReadOnly,
        PluginToolPermission::WorkspaceWrite,
        PluginToolPermission::DangerFullAccess,
    ] {
        let json = serde_json::to_value(perm).unwrap();
        let deserialized: PluginToolPermission = serde_json::from_value(json).unwrap();
        assert_eq!(perm, deserialized);
    }
}

#[test]
fn tool_permission_serde_kebab_case() {
    assert_eq!(
        serde_json::to_value(PluginToolPermission::ReadOnly).unwrap(),
        "read-only"
    );
    assert_eq!(
        serde_json::to_value(PluginToolPermission::WorkspaceWrite).unwrap(),
        "workspace-write"
    );
    assert_eq!(
        serde_json::to_value(PluginToolPermission::DangerFullAccess).unwrap(),
        "danger-full-access"
    );
}

#[test]
fn tool_permission_ordering() {
    assert!(PluginToolPermission::ReadOnly < PluginToolPermission::WorkspaceWrite);
    assert!(PluginToolPermission::WorkspaceWrite < PluginToolPermission::DangerFullAccess);
    assert!(PluginToolPermission::ReadOnly < PluginToolPermission::DangerFullAccess);
}

#[test]
fn permission_serde_deserializes_valid_strings() {
    assert_eq!(
        serde_json::from_str::<PluginPermission>("\"read\"").unwrap(),
        PluginPermission::Read
    );
    assert_eq!(
        serde_json::from_str::<PluginPermission>("\"write\"").unwrap(),
        PluginPermission::Write
    );
    assert_eq!(
        serde_json::from_str::<PluginPermission>("\"execute\"").unwrap(),
        PluginPermission::Execute
    );
}

#[test]
fn permission_serde_rejects_invalid_strings() {
    assert!(serde_json::from_str::<PluginPermission>("\"admin\"").is_err());
    assert!(serde_json::from_str::<PluginPermission>("\"READ\"").is_err());
}

#[test]
fn tool_permission_serde_deserializes_valid_strings() {
    assert_eq!(
        serde_json::from_str::<PluginToolPermission>("\"read-only\"").unwrap(),
        PluginToolPermission::ReadOnly
    );
    assert_eq!(
        serde_json::from_str::<PluginToolPermission>("\"workspace-write\"").unwrap(),
        PluginToolPermission::WorkspaceWrite
    );
    assert_eq!(
        serde_json::from_str::<PluginToolPermission>("\"danger-full-access\"").unwrap(),
        PluginToolPermission::DangerFullAccess
    );
}

#[test]
fn tool_permission_serde_rejects_invalid_strings() {
    assert!(serde_json::from_str::<PluginToolPermission>("\"admin\"").is_err());
    assert!(serde_json::from_str::<PluginToolPermission>("\"readonly\"").is_err());
}
