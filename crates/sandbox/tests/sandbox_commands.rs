use orbit_sandbox::{
    build_docker_sandbox_command, build_linux_sandbox_command, build_macos_sandbox_command,
    build_sandbox_command, build_windows_sandbox_command, FilesystemIsolationMode, SandboxCommand,
    SandboxRequest, SandboxStatus,
};
use std::path::Path;

fn disabled_status() -> SandboxStatus {
    SandboxStatus {
        enabled: false,
        requested: SandboxRequest::default(),
        supported: false,
        active: false,
        namespace_supported: false,
        namespace_active: false,
        network_supported: false,
        network_active: false,
        filesystem_mode: FilesystemIsolationMode::Off,
        filesystem_active: false,
        allowed_mounts: vec![],
        in_container: false,
        container_markers: vec![],
        fallback_reason: None,
    }
}

fn enabled_status(
    namespace_active: bool,
    network_active: bool,
    filesystem_active: bool,
) -> SandboxStatus {
    SandboxStatus {
        enabled: true,
        requested: SandboxRequest {
            enabled: true,
            namespace_restrictions: namespace_active,
            network_isolation: network_active,
            filesystem_mode: if filesystem_active {
                FilesystemIsolationMode::WorkspaceOnly
            } else {
                FilesystemIsolationMode::Off
            },
            allowed_mounts: vec![],
        },
        supported: true,
        active: true,
        namespace_supported: true,
        namespace_active,
        network_supported: true,
        network_active,
        filesystem_mode: if filesystem_active {
            FilesystemIsolationMode::WorkspaceOnly
        } else {
            FilesystemIsolationMode::Off
        },
        filesystem_active,
        allowed_mounts: vec![],
        in_container: false,
        container_markers: vec![],
        fallback_reason: None,
    }
}

#[test]
fn all_command_builders_return_none_when_disabled() {
    let status = disabled_status();
    let cwd = Path::new("/workspace");

    assert!(build_linux_sandbox_command("echo hi", cwd, &status).is_none());
    assert!(build_macos_sandbox_command("echo hi", cwd, &status).is_none());
    assert!(build_windows_sandbox_command("echo hi", cwd, &status).is_none());
    assert!(build_docker_sandbox_command("echo hi", cwd, &status).is_none());
    assert!(build_sandbox_command("echo hi", cwd, &status).is_none());
}

#[test]
fn linux_command_returns_none_on_non_linux() {
    let status = enabled_status(true, false, false);
    let result = build_linux_sandbox_command("echo hi", Path::new("/workspace"), &status);
    if cfg!(target_os = "linux") {
        // On Linux, this should return Some when namespaces are supported
        // but we can't test unshare availability here
    } else {
        assert!(
            result.is_none(),
            "linux command should be None on non-Linux"
        );
    }
}

#[test]
fn windows_command_returns_none_on_non_windows() {
    let status = enabled_status(false, false, false);
    let result = build_windows_sandbox_command("echo hi", Path::new("/workspace"), &status);
    if cfg!(target_os = "windows") {
        // On Windows, this may return Some depending on flags
    } else {
        assert!(
            result.is_none(),
            "windows command should be None on non-Windows"
        );
    }
}

#[test]
fn docker_command_returns_none_when_not_enabled() {
    let status = enabled_status(true, true, false);
    let result = build_docker_sandbox_command("echo hi", Path::new("/workspace"), &status);
    assert!(
        result.is_none(),
        "docker should not be available without env flag"
    );
}

#[test]
fn docker_command_returns_none_in_container() {
    let mut status = enabled_status(true, true, false);
    status.in_container = true;
    let result = build_docker_sandbox_command("echo hi", Path::new("/workspace"), &status);
    assert!(
        result.is_none(),
        "docker backend should not be used in container"
    );
}

#[test]
fn build_sandbox_command_tries_all_backends() {
    let status = disabled_status();
    // When all backends return None, build_sandbox_command returns None
    assert!(build_sandbox_command("echo hi", Path::new("/workspace"), &status).is_none());
}

#[test]
fn sandbox_command_struct_accessors() {
    let cmd = SandboxCommand {
        program: "test".to_string(),
        args: vec!["-a".to_string(), "value".to_string()],
        env: vec![("KEY".to_string(), "VAL".to_string())],
    };
    assert_eq!(cmd.program, "test");
    assert_eq!(cmd.args, vec!["-a", "value"]);
    assert_eq!(cmd.env, vec![("KEY".to_string(), "VAL".to_string())]);
}

#[test]
fn macos_command_includes_expected_args_when_available() {
    let status = enabled_status(true, true, true);
    let result = build_macos_sandbox_command("echo test", Path::new("/workspace"), &status);

    #[cfg(target_os = "macos")]
    {
        // sandbox-exec should be available on macOS
        if let Some(cmd) = result {
            assert_eq!(cmd.program, "sandbox-exec");
            assert_eq!(cmd.args[0], "-p");
            let profile = &cmd.args[1];
            assert!(profile.contains("(deny network*)"));
            assert!(profile.contains("(deny file-write*)"));
            assert_eq!(cmd.args[2], "sh");
            assert_eq!(cmd.args[3], "-lc");
            assert_eq!(cmd.args[4], "echo test");
            assert!(!cmd.env.is_empty());
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        assert!(
            result.is_none(),
            "macos command should be None on non-macOS"
        );
    }
}

#[test]
fn linux_command_structure_when_available() {
    let status = enabled_status(true, true, false);
    let result = build_linux_sandbox_command("echo test", Path::new("/workspace"), &status);

    #[cfg(target_os = "linux")]
    {
        if let Some(cmd) = result {
            assert_eq!(cmd.program, "unshare");
            assert!(cmd.args.iter().any(|a| a == "--mount"));
            assert!(cmd.args.iter().any(|a| a == "--net"));
            assert!(cmd.args.iter().any(|a| a == "--fork"));
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        assert!(result.is_none());
    }
}

#[test]
fn macos_command_without_network_active_omits_network_denial() {
    let status = enabled_status(true, false, true);
    let result = build_macos_sandbox_command("echo test", Path::new("/workspace"), &status);

    #[cfg(target_os = "macos")]
    {
        if let Some(cmd) = result {
            let profile = &cmd.args[1];
            assert!(!profile.contains("(deny network*)"));
            assert!(profile.contains("(deny file-write*)"));
        }
    }
}

#[test]
fn macos_command_without_filesystem_active_omits_file_denial() {
    let status = enabled_status(true, true, false);
    let result = build_macos_sandbox_command("echo test", Path::new("/workspace"), &status);

    #[cfg(target_os = "macos")]
    {
        if let Some(cmd) = result {
            let profile = &cmd.args[1];
            assert!(!profile.contains("(deny file-write*)"));
            assert!(profile.contains("(deny network*)"));
        }
    }
}
