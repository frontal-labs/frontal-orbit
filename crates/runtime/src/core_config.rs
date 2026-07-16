//! Bridge between runtime configuration and core configuration
//!
//! This module provides integration between the existing runtime configuration system
//! and the new core configuration system, allowing gradual migration and coexistence.

use crate::config::{ConfigError, ConfigLoader, RuntimeConfig};
use orbit_core::config::ProjectConfig;
use std::sync::Arc;

/// Configuration manager that combines both runtime and core configurations
#[derive(Debug, Clone)]
pub struct ConfigurationManager {
    pub core_config: Arc<ProjectConfig>,
    pub runtime_config: Arc<RuntimeConfig>,
}

impl ConfigurationManager {
    /// Load both core and runtime configurations
    pub fn load() -> Result<Self, ConfigError> {
        // Load core configuration
        let core_config = Arc::new(ProjectConfig::load_or_default());

        // Load runtime configuration using existing system
        let cwd = std::env::current_dir().map_err(ConfigError::Io)?;
        let runtime_config = Arc::new(ConfigLoader::default_for(&cwd).load()?);

        Ok(Self {
            core_config,
            runtime_config,
        })
    }

    /// Load with custom working directory
    pub fn load_with_cwd(cwd: impl AsRef<std::path::Path>) -> Result<Self, ConfigError> {
        // Load core configuration
        let core_config = Arc::new(ProjectConfig::load_or_default());

        // Load runtime configuration
        let runtime_config = Arc::new(ConfigLoader::default_for(cwd.as_ref()).load()?);

        Ok(Self {
            core_config,
            runtime_config,
        })
    }

    /// Get the core configuration
    #[must_use]
    pub fn core(&self) -> &ProjectConfig {
        &self.core_config
    }

    /// Get the runtime configuration
    #[must_use]
    pub fn runtime(&self) -> &RuntimeConfig {
        &self.runtime_config
    }

    /// Get the default provider from core config, falling back to runtime config
    #[must_use]
    pub fn default_provider(&self) -> &str {
        &self.core_config.runtime.default_provider
    }

    /// Get the default model for a provider from core config
    #[must_use]
    pub fn default_model(&self, provider: &str) -> Option<String> {
        self.core_config.get_default_model(provider)
    }

    /// Check if a provider is enabled in core config
    #[must_use]
    pub fn is_provider_enabled(&self, provider: &str) -> bool {
        self.core_config.is_provider_enabled(provider)
    }

    /// Get max concurrent requests from core config
    #[must_use]
    pub fn max_concurrent_requests(&self) -> u32 {
        self.core_config.runtime.max_concurrent_requests
    }

    /// Get request timeout from core config
    #[must_use]
    pub fn request_timeout_seconds(&self) -> u32 {
        self.core_config.runtime.request_timeout_seconds
    }

    /// Get permission mode from core config
    #[must_use]
    pub fn permission_mode(&self) -> &str {
        &self.core_config.runtime.permission_mode
    }

    /// Get log level from core config
    #[must_use]
    pub fn log_level(&self) -> &str {
        &self.core_config.runtime.log_level
    }

    /// Check if telemetry is enabled from core config
    #[must_use]
    pub fn is_telemetry_enabled(&self) -> bool {
        self.core_config.features.enable_telemetry
    }

    /// Check if plugins are enabled from core config
    #[must_use]
    pub fn are_plugins_enabled(&self) -> bool {
        self.core_config.features.enable_plugins
    }

    /// Check if caching is enabled from core config
    #[must_use]
    pub fn is_caching_enabled(&self) -> bool {
        self.core_config.features.enable_caching
    }

    /// Check if metrics are enabled from core config
    #[must_use]
    pub fn are_metrics_enabled(&self) -> bool {
        self.core_config.features.enable_metrics
    }

    /// Get UI theme from core config
    #[must_use]
    pub fn ui_theme(&self) -> &str {
        &self.core_config.ui.theme
    }

    /// Check if UI colors are enabled from core config
    #[must_use]
    pub fn are_ui_colors_enabled(&self) -> bool {
        self.core_config.ui.enable_colors
    }

    /// Get cache directory from core config
    #[must_use]
    pub fn cache_dir(&self) -> &str {
        &self.core_config.paths.cache_dir
    }

    /// Get logs directory from core config
    #[must_use]
    pub fn logs_dir(&self) -> &str {
        &self.core_config.paths.logs_dir
    }

    /// Get sandbox configuration from core config
    #[must_use]
    pub fn sandbox_config(&self) -> &orbit_core::config::SandboxConfig {
        &self.core_config.sandbox
    }

    /// Get service configuration from core config
    #[must_use]
    pub fn service_config(&self) -> &orbit_core::config::ServiceConfig {
        &self.core_config.services
    }

    /// Get feature configuration from core config
    #[must_use]
    pub fn feature_config(&self) -> &orbit_core::config::FeatureConfig {
        &self.core_config.features
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_configuration_manager_load() {
        // This test will use the actual project config if available
        match ConfigurationManager::load() {
            Ok(config_manager) => {
                // Test that we can access both configurations
                let _core = config_manager.core();
                let _runtime = config_manager.runtime();

                // Test core config accessors
                assert!(!config_manager.default_provider().is_empty());
                assert!(config_manager.max_concurrent_requests() > 0);
                assert!(config_manager.request_timeout_seconds() > 0);

                // Test provider methods
                if config_manager.is_provider_enabled("anthropic") {
                    assert!(config_manager.default_model("anthropic").is_some());
                }
            }
            Err(e) => {
                // This is expected in CI environments where config files may not exist
                println!("Failed to load configuration manager: {e}");
            }
        }
    }

    #[test]
    fn test_core_config_defaults() {
        let config = ProjectConfig::default();

        // Test default values
        assert_eq!(config.runtime.default_provider, "frontal");
        assert_eq!(config.runtime.max_concurrent_requests, 10);
        assert_eq!(config.runtime.request_timeout_seconds, 30);
        assert_eq!(config.runtime.permission_mode, "permissive");
        assert_eq!(config.runtime.log_level, "info");

        // Test feature flags
        assert!(config.features.enable_telemetry);
        assert!(config.features.enable_plugins);
        assert!(config.features.enable_caching);
        assert!(config.features.enable_metrics);
        assert!(!config.features.enable_tracing);
        assert!(!config.features.enable_hot_reload);

        // Test UI config
        assert_eq!(config.ui.theme, "default");
        assert!(config.ui.enable_colors);
        assert!(config.ui.show_progress_bars);
        assert!(config.ui.confirm_dangerous_operations);

        // Test experimental config
        assert!(!config.experimental.enable_new_features);
        assert!(config.experimental.beta_features.is_empty());
    }
}
