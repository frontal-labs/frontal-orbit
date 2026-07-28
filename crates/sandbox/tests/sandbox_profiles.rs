use orbit_sandbox::{
    build_macos_sandbox_command, FilesystemIsolationMode, SandboxRequest, SandboxStatus,
};
use std::path::Path;

fn make_status(
    filesystem_mode: FilesystemIsolationMode,
    network_active: bool,
    filesystem_active: bool,
    allowed_mounts: Vec<String>,
) -> SandboxStatus {
    SandboxStatus {
        enabled: true,
        requested: SandboxRequest {
            enabled: true,
            namespace_restrictions: true,
            network_isolation: network_active,
            filesystem_mode,
            allowed_mounts: allowed_mounts.clone(),
        },
        supported: true,
        active: true,
        namespace_supported: true,
        namespace_active: true,
        network_supported: true,
        network_active,
        filesystem_mode,
        filesystem_active,
        allowed_mounts,
        in_container: false,
        container_markers: vec![],
        fallback_reason: None,
    }
}

#[test]
fn profile_with_off_filesystem_mode() {
    let status = make_status(FilesystemIsolationMode::Off, false, false, vec![]);
    let result = build_macos_sandbox_command("echo test", Path::new("/workspace"), &status);

    #[cfg(target_os = "macos")]
    {
        if let Some(cmd) = result {
            let profile = &cmd.args[1];
            assert!(profile.contains("(allow default)"));
            assert!(!profile.contains("(deny file-write*)"));
        }
    }
}

#[test]
fn profile_with_workspace_only_and_allowlist_mounts() {
    let status = make_status(
        FilesystemIsolationMode::WorkspaceOnly,
        false,
        true,
        vec!["/extra/mount".to_string()],
    );
    let result = build_macos_sandbox_command("echo test", Path::new("/workspace"), &status);

    #[cfg(target_os = "macos")]
    {
        if let Some(cmd) = result {
            let profile = &cmd.args[1];
            assert!(profile.contains("(deny file-write*)"));
            assert!(profile.contains("(allow file-write* (subpath \"/workspace\"))"));
            assert!(profile.contains("(allow file-write* (subpath \"/extra/mount\"))"));
        }
    }
}

#[test]
fn profile_with_allow_list_mode() {
    let status = make_status(
        FilesystemIsolationMode::AllowList,
        false,
        true,
        vec!["/data".to_string(), "/logs".to_string()],
    );
    let result = build_macos_sandbox_command("echo test", Path::new("/workspace"), &status);

    #[cfg(target_os = "macos")]
    {
        if let Some(cmd) = result {
            let profile = &cmd.args[1];
            assert!(profile.contains("(deny file-write*)"));
            assert!(profile.contains("(allow file-write* (subpath \"/data\"))"));
            assert!(profile.contains("(allow file-write* (subpath \"/logs\"))"));
            assert!(!profile.contains("/workspace"));
        }
    }
}

#[test]
fn profile_with_network_isolation() {
    let status = make_status(FilesystemIsolationMode::Off, true, false, vec![]);
    let result = build_macos_sandbox_command("echo test", Path::new("/workspace"), &status);

    #[cfg(target_os = "macos")]
    {
        if let Some(cmd) = result {
            let profile = &cmd.args[1];
            assert!(profile.contains("(deny network*)"));
            assert!(!profile.contains("(deny file-write*)"));
        }
    }
}

#[test]
fn profile_with_both_network_and_filesystem_isolation() {
    let status = make_status(FilesystemIsolationMode::WorkspaceOnly, true, true, vec![]);
    let result = build_macos_sandbox_command("echo test", Path::new("/workspace"), &status);

    #[cfg(target_os = "macos")]
    {
        if let Some(cmd) = result {
            let profile = &cmd.args[1];
            assert!(profile.contains("(deny network*)"));
            assert!(profile.contains("(deny file-write*)"));
            assert!(profile.contains("(allow file-write* (subpath \"/workspace\"))"));
        }
    }
}

#[test]
fn profile_starts_with_version_and_allow_default() {
    let status = make_status(FilesystemIsolationMode::WorkspaceOnly, false, true, vec![]);
    let result = build_macos_sandbox_command("echo test", Path::new("/workspace"), &status);

    #[cfg(target_os = "macos")]
    {
        if let Some(cmd) = result {
            let profile = &cmd.args[1];
            assert!(profile.starts_with("(version 1) (allow default)"));
        }
    }
}

#[test]
fn profile_handles_paths_with_special_characters() {
    let status = make_status(
        FilesystemIsolationMode::WorkspaceOnly,
        false,
        true,
        vec!["/path/with\"quotes".to_string()],
    );
    let result = build_macos_sandbox_command("echo test", Path::new("/workspace"), &status);

    #[cfg(target_os = "macos")]
    {
        if let Some(cmd) = result {
            let profile = &cmd.args[1];
            // The quote should be escaped in the sandbox profile
            assert!(profile.contains("with\\\"quotes"));
        }
    }
}

#[test]
fn profile_with_no_writable_mounts_in_workspace_only() {
    let status = make_status(FilesystemIsolationMode::WorkspaceOnly, false, true, vec![]);
    let result = build_macos_sandbox_command("echo test", Path::new("/workspace"), &status);

    #[cfg(target_os = "macos")]
    {
        if let Some(cmd) = result {
            let profile = &cmd.args[1];
            assert!(profile.contains("(allow file-write* (subpath \"/workspace\"))"));
        }
    }
}

#[test]
fn profile_with_allow_list_and_no_mounts_grants_no_writes() {
    let status = make_status(FilesystemIsolationMode::AllowList, false, true, vec![]);
    let result = build_macos_sandbox_command("echo test", Path::new("/workspace"), &status);

    #[cfg(target_os = "macos")]
    {
        if let Some(cmd) = result {
            let profile = &cmd.args[1];
            assert!(profile.contains("(deny file-write*)"));
            // No (allow file-write*) entries since there are no mounts
            let allow_count = profile.matches("(allow file-write*").count();
            assert_eq!(allow_count, 0);
        }
    }
}
