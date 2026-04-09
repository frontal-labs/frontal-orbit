//! Example demonstrating how to use the core configuration system
//!
//! This example shows how to load and use the ProjectConfig from the orbit-core crate
//! and the ConfigurationManager from the orbit-runtime crate.

use orbit_core::config::ProjectConfig;
use orbit_runtime::ConfigurationManager;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Orbit Core Configuration Example ===\n");

    // Example 1: Load core configuration directly
    println!("1. Loading core configuration directly:");
    let core_config = ProjectConfig::load_or_default();

    println!("  Project Information:");
    println!("    Name: {}", core_config.project.name);
    println!("    Version: {}", core_config.project.version);
    println!("    Description: {}", core_config.project.description);

    println!("  Runtime Configuration:");
    println!(
        "    Default Provider: {}",
        core_config.runtime.default_provider
    );
    println!(
        "    Max Concurrent Requests: {}",
        core_config.runtime.max_concurrent_requests
    );
    println!(
        "    Request Timeout: {}s",
        core_config.runtime.request_timeout_seconds
    );
    println!(
        "    Permission Mode: {}",
        core_config.runtime.permission_mode
    );
    println!("    Log Level: {}", core_config.runtime.log_level);

    println!("  Feature Flags:");
    println!(
        "    Telemetry Enabled: {}",
        core_config.features.enable_telemetry
    );
    println!(
        "    Plugins Enabled: {}",
        core_config.features.enable_plugins
    );
    println!(
        "    Caching Enabled: {}",
        core_config.features.enable_caching
    );
    println!(
        "    Metrics Enabled: {}",
        core_config.features.enable_metrics
    );
    println!(
        "    Tracing Enabled: {}",
        core_config.features.enable_tracing
    );
    println!(
        "    Hot Reload Enabled: {}",
        core_config.features.enable_hot_reload
    );

    println!("  UI Configuration:");
    println!("    Theme: {}", core_config.ui.theme);
    println!("    Colors Enabled: {}", core_config.ui.enable_colors);
    println!(
        "    Progress Bars Enabled: {}",
        core_config.ui.show_progress_bars
    );
    println!(
        "    Confirm Dangerous Operations: {}",
        core_config.ui.confirm_dangerous_operations
    );

    println!("  Path Configuration:");
    println!("    Config Home: {}", core_config.paths.config_home);
    println!("    Home: {}", core_config.paths.home);
    println!("    Cache Directory: {}", core_config.paths.cache_dir);
    println!("    Logs Directory: {}", core_config.paths.logs_dir);

    println!();

    // Example 2: Use the ConfigurationManager (bridges core and runtime configs)
    println!("2. Using ConfigurationManager:");
    match ConfigurationManager::load() {
        Ok(config_manager) => {
            println!("  Successfully loaded both core and runtime configurations");

            // Access core configuration through the manager
            println!("  Core Config Access:");
            println!(
                "    Default Provider: {}",
                config_manager.default_provider()
            );
            println!(
                "    Max Concurrent Requests: {}",
                config_manager.max_concurrent_requests()
            );
            println!(
                "    Request Timeout: {}s",
                config_manager.request_timeout_seconds()
            );
            println!("    Permission Mode: {}", config_manager.permission_mode());
            println!("    Log Level: {}", config_manager.log_level());

            // Provider-specific methods
            println!("  Provider Configuration:");
            for provider in ["anthropic", "openai", "xai"] {
                if config_manager.is_provider_enabled(provider) {
                    if let Some(model) = config_manager.default_model(provider) {
                        println!("    {}: enabled (default model: {})", provider, model);
                    } else {
                        println!("    {}: enabled (no default model)", provider);
                    }
                } else {
                    println!("    {}: disabled", provider);
                }
            }

            // Feature flag methods
            println!("  Feature Flags:");
            println!("    Telemetry: {}", config_manager.is_telemetry_enabled());
            println!("    Plugins: {}", config_manager.are_plugins_enabled());
            println!("    Caching: {}", config_manager.is_caching_enabled());
            println!("    Metrics: {}", config_manager.are_metrics_enabled());

            // UI methods
            println!("  UI Settings:");
            println!("    Theme: {}", config_manager.ui_theme());
            println!("    Colors: {}", config_manager.are_ui_colors_enabled());

            // Path methods
            println!("  Paths:");
            println!("    Cache Directory: {}", config_manager.cache_dir());
            println!("    Logs Directory: {}", config_manager.logs_dir());

            // Service configuration
            println!("  Service Configuration:");
            let services = config_manager.service_config();
            println!(
                "    Database Connection Pool Size: {}",
                services.database.connection_pool_size
            );
            println!(
                "    Database Connection Timeout: {}s",
                services.database.connection_timeout_seconds
            );
            println!(
                "    Redis Connection Pool Size: {}",
                services.redis.connection_pool_size
            );
            println!(
                "    Memory Cache Size: {} MB",
                services.memory.cache_size_mb
            );

            // Sandbox configuration
            println!("  Sandbox Configuration:");
            let sandbox = config_manager.sandbox_config();
            println!("    Docker Enabled: {}", sandbox.enable_docker);
            println!("    Docker Image: {}", sandbox.docker_image);
            println!("    Default Shell: {}", sandbox.default_shell);
            println!(
                "    Max Execution Time: {}s",
                sandbox.max_execution_time_seconds
            );
        }
        Err(e) => {
            println!("  Failed to load ConfigurationManager: {}", e);
            println!("  This is expected if runtime configuration files are not present");
        }
    }

    println!();

    // Example 3: Demonstrate provider-specific configuration
    println!("3. Provider-specific Configuration:");
    for provider in ["anthropic", "openai", "xai"] {
        println!("  {} Provider:", provider);
        println!("    Enabled: {}", core_config.is_provider_enabled(provider));

        if let Some(model) = core_config.get_default_model(provider) {
            println!("    Default Model: {}", model);
        }

        if let Some(provider_config) = core_config.get_provider_config(provider) {
            println!("    Provider Config: enabled={}", provider_config.enabled);
        }
    }

    println!();

    // Example 4: Configuration validation
    println!("4. Configuration Validation:");

    // Validate that at least one provider is enabled
    let enabled_providers = ["anthropic", "openai", "xai"]
        .iter()
        .filter(|&&provider| core_config.is_provider_enabled(provider))
        .count();

    if enabled_providers == 0 {
        println!("  Warning: No AI providers are enabled");
    } else {
        println!("  {} AI provider(s) enabled", enabled_providers);
    }

    // Validate reasonable timeout values
    if core_config.runtime.request_timeout_seconds == 0 {
        println!("  Warning: Request timeout is set to 0 seconds");
    } else if core_config.runtime.request_timeout_seconds > 300 {
        println!(
            "  Warning: Request timeout is very high ({}s)",
            core_config.runtime.request_timeout_seconds
        );
    } else {
        println!(
            "  Request timeout looks reasonable ({}s)",
            core_config.runtime.request_timeout_seconds
        );
    }

    // Validate concurrent requests
    if core_config.runtime.max_concurrent_requests == 0 {
        println!("  Warning: Max concurrent requests is set to 0");
    } else if core_config.runtime.max_concurrent_requests > 50 {
        println!(
            "  Warning: Max concurrent requests is very high ({})",
            core_config.runtime.max_concurrent_requests
        );
    } else {
        println!(
            "  Max concurrent requests looks reasonable ({})",
            core_config.runtime.max_concurrent_requests
        );
    }

    println!("\n=== Example Complete ===");
    Ok(())
}
