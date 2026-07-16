use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{env, fs};

/// Main project configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub project: ProjectInfo,
    pub runtime: RuntimeConfig,
    pub paths: PathConfig,
    pub sandbox: SandboxConfig,
    pub services: ServiceConfig,
    pub development: DevelopmentConfig,
    pub features: FeatureConfig,
    pub ui: UiConfig,
    pub experimental: ExperimentalConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub name: String,
    pub version: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub default_provider: String,
    pub providers: ProviderConfig,
    pub permission_mode: String,
    pub log_level: String,
    pub max_concurrent_requests: u32,
    pub request_timeout_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub anthropic: ProviderDetails,
    pub openai: ProviderDetails,
    pub xai: ProviderDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderDetails {
    pub enabled: bool,
    pub default_model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathConfig {
    pub config_home: String,
    pub home: String,
    pub codex_home: String,
    pub sandbox_home: String,
    pub cache_dir: String,
    pub logs_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub enable_docker: bool,
    pub docker_image: String,
    pub default_shell: String,
    pub max_execution_time_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub memory: MemoryConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub connection_pool_size: u32,
    pub connection_timeout_seconds: u32,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub connection_pool_size: u32,
    pub connection_timeout_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub cache_size_mb: u32,
    pub namespace: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevelopmentConfig {
    pub mock_parity_report_path: String,
    pub hosted_task_file: String,
    pub enable_debug_mode: bool,
    pub enable_test_endpoints: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct FeatureConfig {
    pub auto_compaction_threshold: u32,
    pub enable_telemetry: bool,
    pub enable_plugins: bool,
    pub enable_caching: bool,
    pub enable_metrics: bool,
    pub enable_tracing: bool,
    pub enable_hot_reload: bool,
    pub max_file_size_mb: u32,
    pub max_memory_usage_mb: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub theme: String,
    pub enable_colors: bool,
    pub show_progress_bars: bool,
    pub confirm_dangerous_operations: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentalConfig {
    pub enable_new_features: bool,
    pub beta_features: Vec<String>,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        ProjectConfig {
            project: ProjectInfo {
                name: "Orbit".to_string(),
                version: "0.1.0".to_string(),
                description: "AI-powered development environment and CLI tool".to_string(),
            },
            runtime: RuntimeConfig {
                default_provider: "anthropic".to_string(),
                providers: ProviderConfig {
                    anthropic: ProviderDetails {
                        enabled: true,
                        default_model: "claude-3-5-sonnet-20241022".to_string(),
                    },
                    openai: ProviderDetails {
                        enabled: true,
                        default_model: "gpt-4".to_string(),
                    },
                    xai: ProviderDetails {
                        enabled: true,
                        default_model: "grok-beta".to_string(),
                    },
                },
                permission_mode: "permissive".to_string(),
                log_level: "info".to_string(),
                max_concurrent_requests: 10,
                request_timeout_seconds: 30,
            },
            paths: PathConfig {
                config_home: "~/.orbit".to_string(),
                home: "~/.orbit".to_string(),
                codex_home: "~/.codex".to_string(),
                sandbox_home: "/workspace/.sandbox-home".to_string(),
                cache_dir: "~/.orbit/cache".to_string(),
                logs_dir: "~/.orbit/logs".to_string(),
            },
            sandbox: SandboxConfig {
                enable_docker: false,
                docker_image: "busybox:1.36".to_string(),
                default_shell: "/bin/bash".to_string(),
                max_execution_time_seconds: 300,
            },
            services: ServiceConfig {
                database: DatabaseConfig {
                    connection_pool_size: 10,
                    connection_timeout_seconds: 30,
                    max_connections: 20,
                },
                redis: RedisConfig {
                    connection_pool_size: 10,
                    connection_timeout_seconds: 10,
                },
                memory: MemoryConfig {
                    cache_size_mb: 512,
                    namespace: "default".to_string(),
                },
            },
            development: DevelopmentConfig {
                mock_parity_report_path: "/tmp/mock_parity_report.json".to_string(),
                hosted_task_file: "/tmp/orbit_hosted_task.json".to_string(),
                enable_debug_mode: false,
                enable_test_endpoints: false,
            },
            features: FeatureConfig {
                auto_compaction_threshold: 100,
                enable_telemetry: true,
                enable_plugins: true,
                enable_caching: true,
                enable_metrics: true,
                enable_tracing: false,
                enable_hot_reload: false,
                max_file_size_mb: 100,
                max_memory_usage_mb: 2048,
            },
            ui: UiConfig {
                theme: "default".to_string(),
                enable_colors: true,
                show_progress_bars: true,
                confirm_dangerous_operations: true,
            },
            experimental: ExperimentalConfig {
                enable_new_features: false,
                beta_features: vec![],
            },
        }
    }
}

impl ProjectConfig {
    /// Load configuration from the default config file location
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = Self::get_config_path()?;
        Self::load_from_path(&config_path)
    }

    /// Load configuration from a specific file path
    pub fn load_from_path(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        if !path.exists() {
            return Err(format!("Config file not found: {}", path.display()).into());
        }

        let content = fs::read_to_string(path)?;
        let config: ProjectConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Load configuration with fallback to defaults
    #[must_use]
    pub fn load_or_default() -> Self {
        if let Ok(config) = Self::load() {
            config
        } else {
            eprintln!("Warning: Could not load config file, using defaults");
            ProjectConfig::default()
        }
    }

    /// Get the default configuration file path
    pub fn get_config_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
        // Check for ORBIT_CONFIG_HOME environment variable first
        if let Ok(config_home) = env::var("ORBIT_CONFIG_HOME") {
            let config_path = PathBuf::from(config_home).join("project.json");
            if config_path.exists() {
                return Ok(config_path);
            }
        }

        // Check for ORBIT_HOME environment variable
        if let Ok(orbit_home) = env::var("ORBIT_HOME") {
            let config_path = PathBuf::from(orbit_home).join("project.json");
            if config_path.exists() {
                return Ok(config_path);
            }
        }

        // Check user's home directory
        if let Ok(home_dir) = env::var("HOME") {
            let config_path = PathBuf::from(home_dir).join(".orbit").join("project.json");
            if config_path.exists() {
                return Ok(config_path);
            }
        }

        // Fallback to project-local config
        // Try to find the project root by looking for Cargo.toml
        let mut current_dir = env::current_dir()?;
        loop {
            let cargo_toml = current_dir.join("Cargo.toml");
            let config_path = current_dir.join("config").join("project.json");

            if cargo_toml.exists() && config_path.exists() {
                return Ok(config_path);
            }

            // Move up to parent directory
            match current_dir.parent() {
                Some(parent) => current_dir = parent.to_path_buf(),
                None => break,
            }
        }

        Err("No configuration file found".into())
    }

    /// Save configuration to the default location
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::get_config_path()?;
        self.save_to_path(&config_path)
    }

    /// Save configuration to a specific file path
    pub fn save_to_path(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Get a specific provider configuration
    #[must_use]
    pub fn get_provider_config(&self, provider: &str) -> Option<&ProviderDetails> {
        match provider {
            "anthropic" => Some(&self.runtime.providers.anthropic),
            "openai" => Some(&self.runtime.providers.openai),
            "xai" => Some(&self.runtime.providers.xai),
            _ => None,
        }
    }

    /// Check if a provider is enabled
    #[must_use]
    pub fn is_provider_enabled(&self, provider: &str) -> bool {
        self.get_provider_config(provider)
            .is_some_and(|details| details.enabled)
    }

    /// Get the default model for a provider
    #[must_use]
    pub fn get_default_model(&self, provider: &str) -> Option<String> {
        self.get_provider_config(provider)
            .map(|details| details.default_model.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ProjectConfig::default();
        assert_eq!(config.project.name, "Orbit");
        assert_eq!(config.runtime.default_provider, "anthropic");
        assert!(config.runtime.providers.anthropic.enabled);
    }

    #[test]
    fn test_config_serialization() {
        let config = ProjectConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: ProjectConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.project.name, parsed.project.name);
    }

    #[test]
    fn test_save_and_load() {
        let original_config = ProjectConfig::default();
        let json_str = serde_json::to_string(&original_config).unwrap();
        let parsed: ProjectConfig = serde_json::from_str(&json_str).unwrap();
        assert_eq!(original_config.project.name, parsed.project.name);
    }

    #[test]
    fn test_provider_config() {
        let config = ProjectConfig::default();
        assert!(config.is_provider_enabled("anthropic"));
        assert_eq!(
            config.get_default_model("anthropic"),
            Some("claude-3-5-sonnet-20241022".to_string())
        );
        assert!(!config.is_provider_enabled("unknown"));
    }
}
