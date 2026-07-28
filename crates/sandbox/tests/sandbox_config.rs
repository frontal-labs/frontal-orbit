use orbit_sandbox::{FilesystemIsolationMode, SandboxConfig, SandboxRequest};

#[test]
fn resolve_request_with_all_overrides() {
    let config = SandboxConfig::default();
    let request = config.resolve_request(
        Some(true),
        Some(true),
        Some(true),
        Some(FilesystemIsolationMode::AllowList),
        Some(vec!["/allowed/path".to_string()]),
    );
    assert!(request.enabled);
    assert!(request.namespace_restrictions);
    assert!(request.network_isolation);
    assert_eq!(request.filesystem_mode, FilesystemIsolationMode::AllowList);
    assert_eq!(request.allowed_mounts, vec!["/allowed/path"]);
}

#[test]
fn resolve_request_with_false_overrides() {
    let config = SandboxConfig::default();
    let request = config.resolve_request(
        Some(false),
        Some(false),
        Some(false),
        Some(FilesystemIsolationMode::Off),
        Some(vec![]),
    );
    assert!(!request.enabled);
    assert!(!request.namespace_restrictions);
    assert!(!request.network_isolation);
    assert_eq!(request.filesystem_mode, FilesystemIsolationMode::Off);
    assert!(request.allowed_mounts.is_empty());
}

#[test]
fn resolve_request_with_no_overrides_uses_config_defaults() {
    let config = SandboxConfig::default();
    let request = config.resolve_request(None, None, None, None, None);
    assert!(request.enabled);
    assert!(request.namespace_restrictions);
    assert!(!request.network_isolation);
    assert_eq!(
        request.filesystem_mode,
        FilesystemIsolationMode::WorkspaceOnly
    );
    assert!(request.allowed_mounts.is_empty());
}

#[test]
fn resolve_request_partial_overrides_use_config_values() {
    let config = SandboxConfig {
        enabled: Some(false),
        namespace_restrictions: Some(true),
        network_isolation: Some(true),
        filesystem_mode: Some(FilesystemIsolationMode::AllowList),
        allowed_mounts: vec!["data".to_string()],
    };
    let request = config.resolve_request(None, Some(false), None, None, None);
    assert!(!request.enabled);
    assert!(!request.namespace_restrictions);
    assert!(request.network_isolation);
    assert_eq!(request.filesystem_mode, FilesystemIsolationMode::AllowList);
    assert_eq!(request.allowed_mounts, vec!["data"]);
}

#[test]
fn resolve_request_none_overrides_fall_through_to_config() {
    let config = SandboxConfig {
        enabled: Some(true),
        namespace_restrictions: Some(false),
        network_isolation: None,
        filesystem_mode: None,
        allowed_mounts: vec!["mounts".to_string()],
    };
    let request = config.resolve_request(None, None, None, None, None);
    assert!(request.enabled);
    assert!(!request.namespace_restrictions);
    assert!(!request.network_isolation);
    assert_eq!(
        request.filesystem_mode,
        FilesystemIsolationMode::WorkspaceOnly
    );
    assert_eq!(request.allowed_mounts, vec!["mounts"]);
}

#[test]
fn default_config_resolves_to_sensible_defaults() {
    let config = SandboxConfig::default();
    assert!(config.enabled.is_none());
    assert!(config.namespace_restrictions.is_none());
    assert!(config.network_isolation.is_none());
    assert!(config.filesystem_mode.is_none());
    assert!(config.allowed_mounts.is_empty());
}

#[test]
fn sandbox_request_defaults() {
    let request = SandboxRequest::default();
    assert!(!request.enabled);
    assert!(!request.namespace_restrictions);
    assert!(!request.network_isolation);
    assert_eq!(request.filesystem_mode, FilesystemIsolationMode::default());
    assert!(request.allowed_mounts.is_empty());
}

#[test]
fn resolve_request_override_precedence() {
    let config = SandboxConfig {
        enabled: Some(true),
        network_isolation: Some(false),
        ..SandboxConfig::default()
    };
    let request = config.resolve_request(Some(false), None, Some(true), None, None);
    assert!(!request.enabled, "override should override config");
    assert!(request.network_isolation, "override should override config");
}

#[test]
fn filesystem_isolation_mode_as_str() {
    assert_eq!(FilesystemIsolationMode::Off.as_str(), "off");
    assert_eq!(
        FilesystemIsolationMode::WorkspaceOnly.as_str(),
        "workspace-only"
    );
    assert_eq!(FilesystemIsolationMode::AllowList.as_str(), "allow-list");
}

#[test]
fn filesystem_isolation_mode_default() {
    assert_eq!(
        FilesystemIsolationMode::default(),
        FilesystemIsolationMode::WorkspaceOnly
    );
}

#[test]
fn config_clone_and_equality() {
    let config = SandboxConfig {
        enabled: Some(true),
        namespace_restrictions: Some(false),
        network_isolation: Some(true),
        filesystem_mode: Some(FilesystemIsolationMode::AllowList),
        allowed_mounts: vec!["/mnt/data".to_string(), "/mnt/logs".to_string()],
    };
    let cloned = config.clone();
    assert_eq!(config, cloned);
}

#[test]
fn request_default_and_equality() {
    let request = SandboxRequest {
        enabled: true,
        namespace_restrictions: false,
        network_isolation: true,
        filesystem_mode: FilesystemIsolationMode::Off,
        allowed_mounts: vec![],
    };
    assert!(request.enabled);
    assert!(!request.namespace_restrictions);
    assert!(request.network_isolation);
    assert_eq!(request.filesystem_mode, FilesystemIsolationMode::Off);
    assert!(request.allowed_mounts.is_empty());
}

#[test]
fn config_partial_defaults() {
    let config = SandboxConfig::default();
    assert!(config.enabled.is_none());
    assert!(config.allowed_mounts.is_empty());
}
