use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum FilesystemIsolationMode {
    Off,
    #[default]
    WorkspaceOnly,
    AllowList,
}

impl FilesystemIsolationMode {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::WorkspaceOnly => "workspace-only",
            Self::AllowList => "allow-list",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SandboxConfig {
    pub enabled: Option<bool>,
    pub namespace_restrictions: Option<bool>,
    pub network_isolation: Option<bool>,
    pub filesystem_mode: Option<FilesystemIsolationMode>,
    pub allowed_mounts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SandboxRequest {
    pub enabled: bool,
    pub namespace_restrictions: bool,
    pub network_isolation: bool,
    pub filesystem_mode: FilesystemIsolationMode,
    pub allowed_mounts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ContainerEnvironment {
    pub in_container: bool,
    pub markers: Vec<String>,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SandboxStatus {
    pub enabled: bool,
    pub requested: SandboxRequest,
    pub supported: bool,
    pub active: bool,
    pub namespace_supported: bool,
    pub namespace_active: bool,
    pub network_supported: bool,
    pub network_active: bool,
    pub filesystem_mode: FilesystemIsolationMode,
    pub filesystem_active: bool,
    pub allowed_mounts: Vec<String>,
    pub in_container: bool,
    pub container_markers: Vec<String>,
    pub fallback_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxDetectionInputs<'a> {
    pub env_pairs: Vec<(String, String)>,
    pub dockerenv_exists: bool,
    pub containerenv_exists: bool,
    pub proc_1_cgroup: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxCommand {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

pub type LinuxSandboxCommand = SandboxCommand;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxBackend {
    LinuxUnshare,
    MacOsSandboxExec,
    WindowsNative,
    DockerRuntime,
}

impl SandboxConfig {
    #[must_use]
    pub fn resolve_request(
        &self,
        enabled_override: Option<bool>,
        namespace_override: Option<bool>,
        network_override: Option<bool>,
        filesystem_mode_override: Option<FilesystemIsolationMode>,
        allowed_mounts_override: Option<Vec<String>>,
    ) -> SandboxRequest {
        SandboxRequest {
            enabled: enabled_override.unwrap_or(self.enabled.unwrap_or(true)),
            namespace_restrictions: namespace_override
                .unwrap_or(self.namespace_restrictions.unwrap_or(true)),
            network_isolation: network_override.unwrap_or(self.network_isolation.unwrap_or(false)),
            filesystem_mode: filesystem_mode_override
                .or(self.filesystem_mode)
                .unwrap_or_default(),
            allowed_mounts: allowed_mounts_override.unwrap_or_else(|| self.allowed_mounts.clone()),
        }
    }
}

#[must_use]
pub fn detect_container_environment() -> ContainerEnvironment {
    let proc_1_cgroup = fs::read_to_string("/proc/1/cgroup").ok();
    detect_container_environment_from(SandboxDetectionInputs {
        env_pairs: env::vars().collect(),
        dockerenv_exists: Path::new("/.dockerenv").exists(),
        containerenv_exists: Path::new("/run/.containerenv").exists(),
        proc_1_cgroup: proc_1_cgroup.as_deref(),
    })
}

#[must_use]
pub fn detect_container_environment_from(
    inputs: SandboxDetectionInputs<'_>,
) -> ContainerEnvironment {
    let mut markers = Vec::new();
    if inputs.dockerenv_exists {
        markers.push("/.dockerenv".to_string());
    }
    if inputs.containerenv_exists {
        markers.push("/run/.containerenv".to_string());
    }
    for (key, value) in inputs.env_pairs {
        let normalized = key.to_ascii_lowercase();
        if matches!(
            normalized.as_str(),
            "container" | "docker" | "podman" | "kubernetes_service_host"
        ) && !value.is_empty()
        {
            markers.push(format!("env:{key}={value}"));
        }
    }
    if let Some(cgroup) = inputs.proc_1_cgroup {
        for needle in ["docker", "containerd", "kubepods", "podman", "libpod"] {
            if cgroup.contains(needle) {
                markers.push(format!("/proc/1/cgroup:{needle}"));
            }
        }
    }
    markers.sort();
    markers.dedup();
    ContainerEnvironment {
        in_container: !markers.is_empty(),
        markers,
    }
}

#[must_use]
pub fn resolve_sandbox_status(config: &SandboxConfig, cwd: &Path) -> SandboxStatus {
    let request = config.resolve_request(None, None, None, None, None);
    resolve_sandbox_status_for_request(&request, cwd)
}

#[must_use]
pub fn resolve_sandbox_status_for_request(request: &SandboxRequest, cwd: &Path) -> SandboxStatus {
    let container = detect_container_environment();
    let capabilities = detect_backend_capabilities(container.in_container);
    let filesystem_active =
        request.enabled && request.filesystem_mode != FilesystemIsolationMode::Off;
    let mut fallback_reasons = Vec::new();

    if request.enabled && request.namespace_restrictions && !capabilities.namespace_supported {
        fallback_reasons.push(capabilities.namespace_reason.to_string());
    }
    if request.enabled && request.network_isolation && !capabilities.network_supported {
        fallback_reasons.push(capabilities.network_reason.to_string());
    }
    if request.enabled
        && request.filesystem_mode == FilesystemIsolationMode::AllowList
        && request.allowed_mounts.is_empty()
    {
        fallback_reasons
            .push("filesystem allow-list requested without configured mounts".to_string());
    }

    let active = request.enabled
        && (!request.namespace_restrictions || capabilities.namespace_supported)
        && (!request.network_isolation || capabilities.network_supported);

    let allowed_mounts = normalize_mounts(&request.allowed_mounts, cwd);

    SandboxStatus {
        enabled: request.enabled,
        requested: request.clone(),
        supported: capabilities.namespace_supported || capabilities.filesystem_supported,
        active,
        namespace_supported: capabilities.namespace_supported,
        namespace_active: request.enabled
            && request.namespace_restrictions
            && capabilities.namespace_supported,
        network_supported: capabilities.network_supported,
        network_active: request.enabled
            && request.network_isolation
            && capabilities.network_supported,
        filesystem_mode: request.filesystem_mode,
        filesystem_active,
        allowed_mounts,
        in_container: container.in_container,
        container_markers: container.markers,
        fallback_reason: (!fallback_reasons.is_empty()).then(|| fallback_reasons.join("; ")),
    }
}

#[must_use]
pub fn build_sandbox_command(
    command: &str,
    cwd: &Path,
    status: &SandboxStatus,
) -> Option<SandboxCommand> {
    build_linux_sandbox_command(command, cwd, status)
        .or_else(|| build_macos_sandbox_command(command, cwd, status))
        .or_else(|| build_windows_sandbox_command(command, cwd, status))
        .or_else(|| build_docker_sandbox_command(command, cwd, status))
}

#[must_use]
pub fn build_linux_sandbox_command(
    command: &str,
    cwd: &Path,
    status: &SandboxStatus,
) -> Option<SandboxCommand> {
    if !cfg!(target_os = "linux")
        || !status.enabled
        || (!status.namespace_active && !status.network_active)
    {
        return None;
    }

    let mut args = vec![
        "--user".to_string(),
        "--map-root-user".to_string(),
        "--mount".to_string(),
        "--ipc".to_string(),
        "--pid".to_string(),
        "--uts".to_string(),
        "--fork".to_string(),
    ];
    if status.network_active {
        args.push("--net".to_string());
    }
    args.push("sh".to_string());
    args.push("-lc".to_string());
    args.push(command.to_string());

    Some(SandboxCommand {
        program: "unshare".to_string(),
        args,
        env: sandbox_env(cwd, status),
    })
}

#[must_use]
pub fn build_macos_sandbox_command(
    command: &str,
    cwd: &Path,
    status: &SandboxStatus,
) -> Option<SandboxCommand> {
    if !cfg!(target_os = "macos")
        || !status.enabled
        || (!status.namespace_active && !status.network_active && !status.filesystem_active)
        || !sandbox_exec_works()
    {
        return None;
    }

    let profile = build_macos_sandbox_profile(status, cwd);
    let args = vec![
        "-p".to_string(),
        profile,
        "sh".to_string(),
        "-lc".to_string(),
        command.to_string(),
    ];

    Some(SandboxCommand {
        program: "sandbox-exec".to_string(),
        args,
        env: sandbox_env(cwd, status),
    })
}

#[must_use]
pub fn build_windows_sandbox_command(
    command: &str,
    cwd: &Path,
    status: &SandboxStatus,
) -> Option<SandboxCommand> {
    if !cfg!(target_os = "windows")
        || !status.enabled
        || (status.requested.namespace_restrictions || status.requested.network_isolation)
    {
        return None;
    }

    Some(SandboxCommand {
        program: "cmd".to_string(),
        args: vec!["/C".to_string(), command.to_string()],
        env: sandbox_env(cwd, status),
    })
}

#[must_use]
pub fn build_docker_sandbox_command(
    command: &str,
    cwd: &Path,
    status: &SandboxStatus,
) -> Option<SandboxCommand> {
    if !status.enabled
        || (!status.requested.namespace_restrictions && !status.requested.network_isolation)
        || status.in_container
        || !docker_backend_enabled()
        || !docker_available()
    {
        return None;
    }

    let image =
        env::var("ORBIT_SANDBOX_DOCKER_IMAGE").unwrap_or_else(|_| "busybox:1.36".to_string());
    let mut args = vec![
        "run".to_string(),
        "--rm".to_string(),
        "-i".to_string(),
        "--workdir".to_string(),
        "/workspace".to_string(),
        "-v".to_string(),
        format!("{}:/workspace", cwd.display()),
    ];
    if status.network_active {
        args.push("--network".to_string());
        args.push("none".to_string());
    }
    args.push(image);
    args.push("sh".to_string());
    args.push("-lc".to_string());
    args.push(command.to_string());

    Some(SandboxCommand {
        program: "docker".to_string(),
        args,
        env: Vec::new(),
    })
}

fn build_macos_sandbox_profile(status: &SandboxStatus, cwd: &Path) -> String {
    let mut profile = vec!["(version 1)".to_string(), "(allow default)".to_string()];
    if status.network_active {
        profile.push("(deny network*)".to_string());
    }
    if status.filesystem_active {
        profile.push("(deny file-write*)".to_string());
        let writable_paths = match status.filesystem_mode {
            FilesystemIsolationMode::Off => Vec::new(),
            FilesystemIsolationMode::WorkspaceOnly => {
                let mut paths = vec![cwd.to_path_buf()];
                paths.extend(status.allowed_mounts.iter().map(PathBuf::from));
                paths
            }
            FilesystemIsolationMode::AllowList => {
                status.allowed_mounts.iter().map(PathBuf::from).collect()
            }
        };
        for writable in writable_paths {
            profile.push(format!(
                "(allow file-write* (subpath \"{}\"))",
                sandbox_profile_literal(&writable.display().to_string())
            ));
        }
    }
    profile.join(" ")
}

fn sandbox_profile_literal(path: &str) -> String {
    // Reject null bytes and other control characters that could
    // terminate the profile early or inject arbitrary directives.
    if path.contains('\0') || path.contains('\n') || path.contains('\r') {
        return String::new();
    }
    path.replace('\\', "\\\\").replace('"', "\\\"")
}

fn sandbox_env(cwd: &Path, status: &SandboxStatus) -> Vec<(String, String)> {
    let sandbox_home = cwd.join(".sandbox-home");
    let sandbox_tmp = cwd.join(".sandbox-tmp");
    let mut env_out = vec![
        ("HOME".to_string(), sandbox_home.display().to_string()),
        ("TMPDIR".to_string(), sandbox_tmp.display().to_string()),
        (
            "ORBIT_SANDBOX_FILESYSTEM_MODE".to_string(),
            status.filesystem_mode.as_str().to_string(),
        ),
        (
            "ORBIT_SANDBOX_ALLOWED_MOUNTS".to_string(),
            status.allowed_mounts.join(":"),
        ),
    ];
    if let Ok(path) = env::var("PATH") {
        env_out.push(("PATH".to_string(), path));
    }
    env_out
}

fn normalize_mounts(mounts: &[String], cwd: &Path) -> Vec<String> {
    let cwd = cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf());
    mounts
        .iter()
        .map(|mount| {
            let path = PathBuf::from(mount);
            let resolved = if path.is_absolute() {
                path.canonicalize().unwrap_or(path)
            } else {
                let joined = cwd.join(&path);
                joined.canonicalize().unwrap_or(joined)
            };
            // Reject paths that escape the cwd workspace boundary
            if !resolved.starts_with(&cwd) {
                // Skip this mount silently rather than allowing traversal
                return String::new();
            }
            resolved.display().to_string()
        })
        .filter(|p| !p.is_empty())
        .collect()
}

fn command_exists(command: &str) -> bool {
    env::var_os("PATH")
        .is_some_and(|paths| env::split_paths(&paths).any(|path| path.join(command).exists()))
}

fn docker_available() -> bool {
    command_exists("docker")
}

fn docker_backend_enabled() -> bool {
    env::var("ORBIT_SANDBOX_ENABLE_DOCKER")
        .ok()
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
}

fn detect_backend_capabilities(in_container: bool) -> BackendCapabilities {
    let platform = current_platform();
    let unshare_ok = platform == Platform::Linux && unshare_user_namespace_works();
    let sandbox_exec_ok = platform == Platform::MacOs && sandbox_exec_works();
    let docker_ok = docker_available() && docker_backend_enabled();
    backend_capabilities(
        platform,
        in_container,
        unshare_ok,
        sandbox_exec_ok,
        docker_ok,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Platform {
    Linux,
    MacOs,
    Windows,
    Other,
}

fn current_platform() -> Platform {
    if cfg!(target_os = "linux") {
        Platform::Linux
    } else if cfg!(target_os = "macos") {
        Platform::MacOs
    } else if cfg!(target_os = "windows") {
        Platform::Windows
    } else {
        Platform::Other
    }
}

#[derive(Debug, Clone, Copy)]
struct BackendCapabilities {
    namespace_supported: bool,
    network_supported: bool,
    filesystem_supported: bool,
    namespace_reason: &'static str,
    network_reason: &'static str,
}

#[allow(clippy::fn_params_excessive_bools)]
fn backend_capabilities(
    platform: Platform,
    in_container: bool,
    unshare_ok: bool,
    sandbox_exec_ok: bool,
    docker_ok: bool,
) -> BackendCapabilities {
    if in_container {
        return BackendCapabilities {
            namespace_supported: true,
            network_supported: true,
            filesystem_supported: true,
            namespace_reason: "",
            network_reason: "",
        };
    }

    match platform {
        Platform::Linux if unshare_ok => BackendCapabilities {
            namespace_supported: true,
            network_supported: true,
            filesystem_supported: true,
            namespace_reason: "",
            network_reason: "",
        },
        Platform::MacOs if sandbox_exec_ok => BackendCapabilities {
            namespace_supported: true,
            network_supported: true,
            filesystem_supported: true,
            namespace_reason: "",
            network_reason: "",
        },
        Platform::Windows if docker_ok => BackendCapabilities {
            namespace_supported: true,
            network_supported: true,
            filesystem_supported: true,
            namespace_reason: "",
            network_reason: "",
        },
        Platform::Linux => BackendCapabilities {
            namespace_supported: false,
            network_supported: false,
            filesystem_supported: true,
            namespace_reason: "namespace isolation unavailable (requires Linux with `unshare`)",
            network_reason: "network isolation unavailable (requires Linux with `unshare`)",
        },
        Platform::MacOs => BackendCapabilities {
            namespace_supported: false,
            network_supported: false,
            filesystem_supported: true,
            namespace_reason: "namespace isolation unavailable (requires macOS with `sandbox-exec`)",
            network_reason: "network isolation unavailable (requires macOS with `sandbox-exec`)",
        },
        Platform::Windows => BackendCapabilities {
            namespace_supported: false,
            network_supported: false,
            filesystem_supported: true,
            namespace_reason:
                "namespace isolation unavailable (requires Docker Desktop or Windows container backend)",
            network_reason:
                "network isolation unavailable (requires Docker Desktop or Windows container backend)",
        },
        Platform::Other => BackendCapabilities {
            namespace_supported: false,
            network_supported: false,
            filesystem_supported: true,
            namespace_reason: "namespace isolation unavailable on this platform",
            network_reason: "network isolation unavailable on this platform",
        },
    }
}

/// Check whether `unshare --user` actually works on this system.
/// On some CI environments (e.g. GitHub Actions), the binary exists but
/// user namespaces are restricted, causing silent failures.
fn unshare_user_namespace_works() -> bool {
    use std::sync::OnceLock;
    static RESULT: OnceLock<bool> = OnceLock::new();
    *RESULT.get_or_init(|| {
        if !command_exists("unshare") {
            return false;
        }
        std::process::Command::new("unshare")
            .args(["--user", "--map-root-user", "true"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    })
}

fn sandbox_exec_works() -> bool {
    use std::sync::OnceLock;
    static RESULT: OnceLock<bool> = OnceLock::new();
    *RESULT.get_or_init(|| {
        if !command_exists("sandbox-exec") {
            return false;
        }
        std::process::Command::new("sandbox-exec")
            .args(["-p", "(version 1) (allow default)", "true"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    })
}

#[cfg(test)]
mod tests {
    use super::{
        backend_capabilities, build_linux_sandbox_command, build_macos_sandbox_profile,
        detect_container_environment_from, sandbox_profile_literal, FilesystemIsolationMode,
        Platform, SandboxConfig, SandboxDetectionInputs,
    };
    use std::path::Path;

    #[test]
    fn detects_container_markers_from_multiple_sources() {
        let detected = detect_container_environment_from(SandboxDetectionInputs {
            env_pairs: vec![("container".to_string(), "docker".to_string())],
            dockerenv_exists: true,
            containerenv_exists: false,
            proc_1_cgroup: Some("12:memory:/docker/abc"),
        });

        assert!(detected.in_container);
        assert!(detected
            .markers
            .iter()
            .any(|marker| marker == "/.dockerenv"));
        assert!(detected
            .markers
            .iter()
            .any(|marker| marker == "env:container=docker"));
        assert!(detected
            .markers
            .iter()
            .any(|marker| marker == "/proc/1/cgroup:docker"));
    }

    #[test]
    fn resolves_request_with_overrides() {
        let config = SandboxConfig {
            enabled: Some(true),
            namespace_restrictions: Some(true),
            network_isolation: Some(false),
            filesystem_mode: Some(FilesystemIsolationMode::WorkspaceOnly),
            allowed_mounts: vec!["logs".to_string()],
        };

        let request = config.resolve_request(
            Some(true),
            Some(false),
            Some(true),
            Some(FilesystemIsolationMode::AllowList),
            Some(vec!["tmp".to_string()]),
        );

        assert!(request.enabled);
        assert!(!request.namespace_restrictions);
        assert!(request.network_isolation);
        assert_eq!(request.filesystem_mode, FilesystemIsolationMode::AllowList);
        assert_eq!(request.allowed_mounts, vec!["tmp"]);
    }

    #[test]
    fn builds_linux_launcher_with_network_flag_when_requested() {
        let config = SandboxConfig::default();
        let status = super::resolve_sandbox_status_for_request(
            &config.resolve_request(
                Some(true),
                Some(true),
                Some(true),
                Some(FilesystemIsolationMode::WorkspaceOnly),
                None,
            ),
            Path::new("/workspace"),
        );

        if let Some(launcher) =
            build_linux_sandbox_command("printf hi", Path::new("/workspace"), &status)
        {
            assert_eq!(launcher.program, "unshare");
            assert!(launcher.args.iter().any(|arg| arg == "--mount"));
            assert!(launcher.args.iter().any(|arg| arg == "--net") == status.network_active);
        }
    }

    #[test]
    fn macos_profile_literal_escapes_quotes_and_backslashes() {
        let escaped = sandbox_profile_literal(r#"/tmp/"hello"\world"#);
        assert_eq!(escaped, r#"/tmp/\"hello\"\\world"#);
    }

    #[test]
    fn macos_profile_workspace_mode_denies_writes_then_allows_workspace() {
        let status = super::resolve_sandbox_status_for_request(
            &SandboxConfig::default().resolve_request(
                Some(true),
                Some(true),
                Some(false),
                Some(FilesystemIsolationMode::WorkspaceOnly),
                None,
            ),
            Path::new("/workspace"),
        );
        let profile = build_macos_sandbox_profile(&status, Path::new("/workspace"));
        assert!(profile.contains("(deny file-write*)"));
        assert!(profile.contains("(allow file-write* (subpath \"/workspace\"))"));
    }

    #[test]
    fn backend_supports_linux_when_unshare_works() {
        let caps = backend_capabilities(Platform::Linux, false, true, false, false);
        assert!(caps.namespace_supported);
        assert!(caps.network_supported);
    }

    #[test]
    fn backend_supports_macos_when_sandbox_exec_works() {
        let caps = backend_capabilities(Platform::MacOs, false, false, true, false);
        assert!(caps.namespace_supported);
        assert!(caps.network_supported);
    }

    #[test]
    fn backend_supports_windows_with_docker_runtime() {
        let caps = backend_capabilities(Platform::Windows, false, false, false, true);
        assert!(caps.namespace_supported);
        assert!(caps.network_supported);
    }

    #[test]
    fn backend_supports_container_even_without_native_backends() {
        let caps = backend_capabilities(Platform::Other, true, false, false, false);
        assert!(caps.namespace_supported);
        assert!(caps.network_supported);
        assert!(caps.filesystem_supported);
    }
}
