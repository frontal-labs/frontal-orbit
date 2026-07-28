use std::path::PathBuf;

use orbit_core::config::{
    PathConfig,
    ProjectConfig, ProviderDetails,
};

#[test]
fn default_config_has_expected_values() {
    let config = ProjectConfig::default();
    assert_eq!(config.project.name, "Orbit");
    assert_eq!(config.project.version, "0.1.0");
    assert_eq!(config.runtime.default_provider, "frontal");
    assert_eq!(config.runtime.permission_mode, "permissive");
    assert_eq!(config.runtime.max_concurrent_requests, 10);
    assert_eq!(config.runtime.request_timeout_seconds, 30);
}

#[test]
fn default_config_providers_are_all_enabled() {
    let config = ProjectConfig::default();
    assert!(config.is_provider_enabled("anthropic"));
    assert!(config.is_provider_enabled("openai"));
    assert!(config.is_provider_enabled("xai"));
    assert!(config.is_provider_enabled("frontal"));
}

#[test]
fn unknown_provider_is_not_enabled() {
    let config = ProjectConfig::default();
    assert!(!config.is_provider_enabled("unknown"));
    assert!(config.get_provider_config("unknown").is_none());
}

#[test]
fn default_model_per_provider() {
    let config = ProjectConfig::default();
    assert_eq!(
        config.get_default_model("anthropic"),
        Some("claude-3-5-sonnet-20241022".to_string())
    );
    assert_eq!(
        config.get_default_model("openai"),
        Some("gpt-4".to_string())
    );
    assert_eq!(
        config.get_default_model("xai"),
        Some("grok-beta".to_string())
    );
    assert_eq!(
        config.get_default_model("frontal"),
        Some("claude-3-5-sonnet-20241022".to_string())
    );
    assert_eq!(config.get_default_model("unknown"), None);
}

#[test]
fn config_serialize_deserialize_roundtrip() {
    let config = ProjectConfig::default();
    let json = serde_json::to_string(&config).unwrap();
    let parsed: ProjectConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(config.project.name, parsed.project.name);
    assert_eq!(config.project.version, parsed.project.version);
    assert_eq!(
        config.runtime.default_provider,
        parsed.runtime.default_provider
    );
    assert_eq!(config.paths.config_home, parsed.paths.config_home);
    assert_eq!(config.sandbox.enable_docker, parsed.sandbox.enable_docker);
    assert_eq!(
        config.features.enable_telemetry,
        parsed.features.enable_telemetry
    );
}

#[test]
fn config_serialize_minimal_json() {
    let json = r#"{
        "project": { "name": "Test", "version": "1.0", "description": "test" },
        "runtime": {
            "default_provider": "test",
            "providers": {
                "anthropic": { "enabled": false, "default_model": "m1" },
                "openai": { "enabled": false, "default_model": "m2" },
                "xai": { "enabled": false, "default_model": "m3" },
                "frontal": { "enabled": false, "default_model": "m4" }
            },
            "permission_mode": "strict",
            "log_level": "debug",
            "max_concurrent_requests": 5,
            "request_timeout_seconds": 60
        },
        "paths": {
            "config_home": "/tmp/.orbit", "home": "/tmp/.orbit",
            "codex_home": "/tmp/.codex", "sandbox_home": "/tmp/sandbox",
            "cache_dir": "/tmp/.orbit/cache", "logs_dir": "/tmp/.orbit/logs"
        },
        "sandbox": {
            "enable_docker": true, "docker_image": "ubuntu:latest",
            "default_shell": "/bin/zsh", "max_execution_time_seconds": 600
        },
        "services": {
            "database": { "connection_pool_size": 5, "connection_timeout_seconds": 15, "max_connections": 10 },
            "redis": { "connection_pool_size": 5, "connection_timeout_seconds": 15 },
            "memory": { "cache_size_mb": 256, "namespace": "test" }
        },
        "development": {
            "mock_parity_report_path": "/tmp/report.json",
            "hosted_task_file": "/tmp/task.json",
            "enable_debug_mode": true,
            "enable_test_endpoints": true
        },
        "features": {
            "auto_compaction_threshold": 50, "enable_telemetry": false,
            "enable_plugins": false, "enable_caching": false,
            "enable_metrics": false, "enable_tracing": true,
            "enable_hot_reload": true, "max_file_size_mb": 50, "max_memory_usage_mb": 1024
        },
        "ui": {
            "theme": "dark", "enable_colors": false,
            "show_progress_bars": false, "confirm_dangerous_operations": false
        },
        "experimental": {
            "enable_new_features": true,
            "beta_features": ["feature-x", "feature-y"]
        }
    }"#;
    let config: ProjectConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.project.name, "Test");
    assert_eq!(config.runtime.default_provider, "test");
    assert!(!config.features.enable_telemetry);
    assert!(config.sandbox.enable_docker);
    assert_eq!(config.ui.theme, "dark");
    assert_eq!(config.experimental.beta_features.len(), 2);
}

#[test]
fn provider_config_customization() {
    let mut config = ProjectConfig::default();
    config.runtime.providers.anthropic = ProviderDetails {
        enabled: false,
        default_model: "custom-model".to_string(),
    };
    assert!(!config.is_provider_enabled("anthropic"));
    assert_eq!(
        config.get_default_model("anthropic"),
        Some("custom-model".to_string())
    );
}

#[test]
fn path_config_construction() {
    let paths = PathConfig {
        config_home: "/custom/.orbit".to_string(),
        home: "/custom/.orbit".to_string(),
        codex_home: "/custom/.codex".to_string(),
        sandbox_home: "/custom/sandbox".to_string(),
        cache_dir: "/custom/.orbit/cache".to_string(),
        logs_dir: "/custom/.orbit/logs".to_string(),
    };
    assert_eq!(paths.config_home, "/custom/.orbit");
    assert_eq!(paths.cache_dir, "/custom/.orbit/cache");
}

#[test]
fn load_from_nonexistent_path_returns_error() {
    let path = PathBuf::from("/nonexistent/path/project.json");
    let result = ProjectConfig::load_from_path(&path);
    assert!(result.is_err());
}

#[test]
fn save_and_load_roundtrip() {
    let dir = std::env::temp_dir().join("orbit_core_test_save_load");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("project.json");
    let config = ProjectConfig::default();
    config.save_to_path(&path).unwrap();
    let loaded = ProjectConfig::load_from_path(&path).unwrap();
    assert_eq!(config.project.name, loaded.project.name);
    assert_eq!(
        config.runtime.default_provider,
        loaded.runtime.default_provider
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_creates_parent_directories() {
    let dir = std::env::temp_dir()
        .join("orbit_core_test_parents")
        .join("nested")
        .join("deep");
    let path = dir.join("project.json");
    let config = ProjectConfig::default();
    config.save_to_path(&path).unwrap();
    assert!(path.exists());
    let remove_dir = std::env::temp_dir().join("orbit_core_test_parents");
    let _ = std::fs::remove_dir_all(&remove_dir);
}

#[test]
fn load_or_default_falls_back_when_no_config() {
    let config = ProjectConfig::load_or_default();
    assert_eq!(config.project.name, "Orbit");
}

#[test]
fn default_paths_are_absolute_or_tilde_prefixed() {
    let config = ProjectConfig::default();
    assert!(config.paths.config_home.starts_with('~'));
    assert!(config.paths.cache_dir.starts_with('~'));
    assert!(config.paths.logs_dir.starts_with('~'));
}

#[test]
fn sandbox_config_defaults() {
    let config = ProjectConfig::default();
    assert!(!config.sandbox.enable_docker);
    assert_eq!(config.sandbox.default_shell, "/bin/bash");
    assert_eq!(config.sandbox.max_execution_time_seconds, 300);
}

#[test]
fn feature_config_defaults() {
    let config = ProjectConfig::default();
    assert!(config.features.enable_telemetry);
    assert!(config.features.enable_plugins);
    assert!(!config.features.enable_tracing);
    assert_eq!(config.features.max_file_size_mb, 100);
}

#[test]
fn experimental_config_defaults() {
    let config = ProjectConfig::default();
    assert!(!config.experimental.enable_new_features);
    assert!(config.experimental.beta_features.is_empty());
}

#[test]
fn ui_config_defaults() {
    let config = ProjectConfig::default();
    assert_eq!(config.ui.theme, "default");
    assert!(config.ui.enable_colors);
    assert!(config.ui.show_progress_bars);
    assert!(config.ui.confirm_dangerous_operations);
}

#[test]
fn service_config_defaults() {
    let config = ProjectConfig::default();
    assert_eq!(config.services.database.connection_pool_size, 10);
    assert_eq!(config.services.redis.connection_pool_size, 10);
    assert_eq!(config.services.memory.cache_size_mb, 512);
    assert_eq!(config.services.memory.namespace, "default");
}

#[test]
fn development_config_defaults() {
    let config = ProjectConfig::default();
    assert!(!config.development.enable_debug_mode);
    assert!(!config.development.enable_test_endpoints);
}

#[test]
fn project_config_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<ProjectConfig>();
}

#[test]
fn project_config_is_sync() {
    fn assert_sync<T: Sync>() {}
    assert_sync::<ProjectConfig>();
}

#[test]
fn provider_details_serde() {
    let details = ProviderDetails {
        enabled: true,
        default_model: "gpt-4o".to_string(),
    };
    let json = serde_json::to_string(&details).unwrap();
    let parsed: ProviderDetails = serde_json::from_str(&json).unwrap();
    assert!(parsed.enabled);
    assert_eq!(parsed.default_model, "gpt-4o");
}

#[test]
fn config_can_be_partially_updated() {
    let mut config = ProjectConfig::default();
    config.project.name = "Custom Project".to_string();
    config.runtime.log_level = "trace".to_string();
    config.sandbox.enable_docker = true;
    assert_eq!(config.project.name, "Custom Project");
    assert_eq!(config.runtime.log_level, "trace");
    assert!(config.sandbox.enable_docker);
}

#[test]
fn get_provider_config_returns_correct_details() {
    let config = ProjectConfig::default();
    let anthropic = config.get_provider_config("anthropic").unwrap();
    assert!(anthropic.enabled);
    assert_eq!(anthropic.default_model, "claude-3-5-sonnet-20241022");
}
