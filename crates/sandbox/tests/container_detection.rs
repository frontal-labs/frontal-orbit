use orbit_sandbox::{detect_container_environment_from, SandboxDetectionInputs};

#[test]
fn not_in_container_when_no_markers() {
    let result = detect_container_environment_from(SandboxDetectionInputs {
        env_pairs: vec![],
        dockerenv_exists: false,
        containerenv_exists: false,
        proc_1_cgroup: None,
    });
    assert!(!result.in_container);
    assert!(result.markers.is_empty());
}

#[test]
fn detects_dockerenv_file() {
    let result = detect_container_environment_from(SandboxDetectionInputs {
        env_pairs: vec![],
        dockerenv_exists: true,
        containerenv_exists: false,
        proc_1_cgroup: None,
    });
    assert!(result.in_container);
    assert_eq!(result.markers, vec!["/.dockerenv"]);
}

#[test]
fn detects_containerenv_file() {
    let result = detect_container_environment_from(SandboxDetectionInputs {
        env_pairs: vec![],
        dockerenv_exists: false,
        containerenv_exists: true,
        proc_1_cgroup: None,
    });
    assert!(result.in_container);
    assert_eq!(result.markers, vec!["/run/.containerenv"]);
}

#[test]
fn detects_container_env_var() {
    let result = detect_container_environment_from(SandboxDetectionInputs {
        env_pairs: vec![("container".to_string(), "docker".to_string())],
        dockerenv_exists: false,
        containerenv_exists: false,
        proc_1_cgroup: None,
    });
    assert!(result.in_container);
    assert_eq!(result.markers, vec!["env:container=docker"]);
}

#[test]
fn detects_docker_env_var() {
    let result = detect_container_environment_from(SandboxDetectionInputs {
        env_pairs: vec![("DOCKER".to_string(), "true".to_string())],
        dockerenv_exists: false,
        containerenv_exists: false,
        proc_1_cgroup: None,
    });
    assert!(result.in_container);
    assert!(result.markers.iter().any(|m| m == "env:DOCKER=true"));
}

#[test]
fn detects_podman_env_var() {
    let result = detect_container_environment_from(SandboxDetectionInputs {
        env_pairs: vec![("PODMAN".to_string(), "1".to_string())],
        dockerenv_exists: false,
        containerenv_exists: false,
        proc_1_cgroup: None,
    });
    assert!(result.in_container);
    assert!(result.markers.iter().any(|m| m == "env:PODMAN=1"));
}

#[test]
fn detects_kubernetes_service_host() {
    let result = detect_container_environment_from(SandboxDetectionInputs {
        env_pairs: vec![(
            "KUBERNETES_SERVICE_HOST".to_string(),
            "10.0.0.1".to_string(),
        )],
        dockerenv_exists: false,
        containerenv_exists: false,
        proc_1_cgroup: None,
    });
    assert!(result.in_container);
}

#[test]
fn detects_docker_cgroup() {
    let result = detect_container_environment_from(SandboxDetectionInputs {
        env_pairs: vec![],
        dockerenv_exists: false,
        containerenv_exists: false,
        proc_1_cgroup: Some("12:memory:/docker/abc123"),
    });
    assert!(result.in_container);
    assert!(result.markers.iter().any(|m| m == "/proc/1/cgroup:docker"));
}

#[test]
fn detects_containerd_cgroup() {
    let result = detect_container_environment_from(SandboxDetectionInputs {
        env_pairs: vec![],
        dockerenv_exists: false,
        containerenv_exists: false,
        proc_1_cgroup: Some("1:name=systemd:/kubepods/besteffort/pod123/containerd://abc"),
    });
    assert!(result.in_container);
    assert!(result
        .markers
        .iter()
        .any(|m| m == "/proc/1/cgroup:containerd"));
    assert!(result
        .markers
        .iter()
        .any(|m| m == "/proc/1/cgroup:kubepods"));
}

#[test]
fn detects_kubepods_cgroup() {
    let result = detect_container_environment_from(SandboxDetectionInputs {
        env_pairs: vec![],
        dockerenv_exists: false,
        containerenv_exists: false,
        proc_1_cgroup: Some("1:name=systemd:/kubepods/besteffort/pod123/abc"),
    });
    assert!(result.in_container);
    assert!(result
        .markers
        .iter()
        .any(|m| m == "/proc/1/cgroup:kubepods"));
}

#[test]
fn detects_podman_cgroup() {
    let result = detect_container_environment_from(SandboxDetectionInputs {
        env_pairs: vec![],
        dockerenv_exists: false,
        containerenv_exists: false,
        proc_1_cgroup: Some("0::/machine.slice/libpod-abc.scope"),
    });
    assert!(result.in_container);
    assert!(result.markers.iter().any(|m| m == "/proc/1/cgroup:libpod"));
}

#[test]
fn ignores_unrelated_env_vars() {
    let result = detect_container_environment_from(SandboxDetectionInputs {
        env_pairs: vec![
            ("PATH".to_string(), "/usr/bin".to_string()),
            ("HOME".to_string(), "/root".to_string()),
            ("USER".to_string(), "admin".to_string()),
        ],
        dockerenv_exists: false,
        containerenv_exists: false,
        proc_1_cgroup: None,
    });
    assert!(!result.in_container);
    assert!(result.markers.is_empty());
}

#[test]
fn multiple_markers_are_deduplicated() {
    let result = detect_container_environment_from(SandboxDetectionInputs {
        env_pairs: vec![("container".to_string(), "docker".to_string())],
        dockerenv_exists: true,
        containerenv_exists: true,
        proc_1_cgroup: Some("12:memory:/docker/abc"),
    });
    assert!(result.in_container);
    assert!(result.markers.contains(&"/.dockerenv".to_string()));
    assert!(result.markers.contains(&"/run/.containerenv".to_string()));
    assert!(result.markers.contains(&"env:container=docker".to_string()));
    assert!(result
        .markers
        .contains(&"/proc/1/cgroup:docker".to_string()));
    assert_eq!(result.markers.len(), 4);
}

#[test]
fn empty_env_pairs_does_not_affect_detection() {
    let result = detect_container_environment_from(SandboxDetectionInputs {
        env_pairs: vec![],
        dockerenv_exists: false,
        containerenv_exists: false,
        proc_1_cgroup: None,
    });
    assert!(!result.in_container);
}

#[test]
fn env_var_with_empty_value_is_ignored() {
    let result = detect_container_environment_from(SandboxDetectionInputs {
        env_pairs: vec![("container".to_string(), String::new())],
        dockerenv_exists: false,
        containerenv_exists: false,
        proc_1_cgroup: None,
    });
    assert!(!result.in_container);
}

#[test]
fn env_var_check_is_case_insensitive() {
    let result = detect_container_environment_from(SandboxDetectionInputs {
        env_pairs: vec![("Container".to_string(), "podman".to_string())],
        dockerenv_exists: false,
        containerenv_exists: false,
        proc_1_cgroup: None,
    });
    assert!(result.in_container);
    assert!(result.markers.iter().any(|m| m == "env:Container=podman"));
}

#[test]
fn empty_cgroup_does_not_cause_detection() {
    let result = detect_container_environment_from(SandboxDetectionInputs {
        env_pairs: vec![],
        dockerenv_exists: false,
        containerenv_exists: false,
        proc_1_cgroup: Some(""),
    });
    assert!(!result.in_container);
}

#[test]
fn non_matching_cgroup_does_not_cause_detection() {
    let result = detect_container_environment_from(SandboxDetectionInputs {
        env_pairs: vec![],
        dockerenv_exists: false,
        containerenv_exists: false,
        proc_1_cgroup: Some("0::/system.slice/sshd.service"),
    });
    assert!(!result.in_container);
}
