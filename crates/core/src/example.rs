//! Example usage of the `ProjectConfig`
//!
//! This module demonstrates how to load and use the project configuration.

use crate::config::ProjectConfig;

/// Example function showing how to load and use configuration
pub fn load_and_use_config() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from default location
    let config = ProjectConfig::load_or_default();

    println!(
        "Project: {} v{}",
        config.project.name, config.project.version
    );
    println!("Description: {}", config.project.description);

    // Access runtime configuration
    println!("Default provider: {}", config.runtime.default_provider);
    println!("Permission mode: {}", config.runtime.permission_mode);
    println!("Log level: {}", config.runtime.log_level);

    // Check provider configurations
    for provider in ["anthropic", "openai", "xai"] {
        if config.is_provider_enabled(provider) {
            let model = config.get_default_model(provider).unwrap_or_default();
            println!("{provider}: enabled (default model: {model})");
        } else {
            println!("{provider}: disabled");
        }
    }

    // Access path configurations
    println!("Config home: {}", config.paths.config_home);
    println!("Orbit home: {}", config.paths.home);

    // Access service configurations
    println!(
        "Database connection pool size: {}",
        config.services.database.connection_pool_size
    );
    println!(
        "Redis connection pool size: {}",
        config.services.redis.connection_pool_size
    );
    println!(
        "Memory cache size: {} MB",
        config.services.memory.cache_size_mb
    );

    // Access feature flags
    println!("Telemetry enabled: {}", config.features.enable_telemetry);
    println!("Plugins enabled: {}", config.features.enable_plugins);
    println!(
        "Auto-compaction threshold: {}",
        config.features.auto_compaction_threshold
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example_usage() {
        // This test demonstrates the example usage
        load_and_use_config().expect("Failed to load and use config");
    }
}
