//! Example demonstrating how to use the core configuration system
//!
//! This example shows how to load and use the `ProjectConfig` from the orbit-core crate
//! and the `ConfigurationManager` from the orbit-runtime crate.

use orbit_core::config::ProjectConfig;
use orbit_runtime::ConfigurationManager;

fn main() {
    println!("=== Orbit Core Configuration Example ===\n");

    // Example 1: Load core configuration directly
    println!("1. Loading core configuration directly:");
    print_core_config(&ProjectConfig::load_or_default());

    println!();

    // Example 2: Use the ConfigurationManager (bridges core and runtime configs)
    println!("2. Using ConfigurationManager:");
    let Ok(config_manager) = ConfigurationManager::load() else {
        println!("  Failed to load ConfigurationManager");
        println!("  This is expected if runtime configuration files are not present");
        return;
    };

    print_configuration_manager(&config_manager);

    println!();

    // Example 3: Demonstrate provider-specific configuration
    println!("3. Provider-specific Configuration:");
    let core_config = ProjectConfig::load_or_default();
    for provider in ["anthropic", "openai", "xai"] {
        println!("  {provider} Provider:");
        println!("    Enabled: {}", core_config.is_provider_enabled(provider));

        if let Some(model) = core_config.get_default_model(provider) {
            println!("    Default Model: {model}");
        }

        if let Some(provider_config) = core_config.get_provider_config(provider) {
            println!("    Provider Config: enabled={}", provider_config.enabled);
        }
    }

    println!();

    // Example 4: Configuration validation
    println!("4. Configuration Validation:");

    let enabled_providers = ["anthropic", "openai", "xai"]
        .iter()
        .filter(|&&provider| core_config.is_provider_enabled(provider))
        .count();

    if enabled_providers == 0 {
        println!("  Warning: No AI providers are enabled");
    } else {
        println!("  {enabled_providers} AI provider(s) enabled");
    }

    validate_timeout(core_config.runtime.request_timeout_seconds);
    validate_concurrent_requests(core_config.runtime.max_concurrent_requests);

    println!("\n=== Example Complete ===");
}

fn print_core_config(core_config: &ProjectConfig) {
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
}

fn print_configuration_manager(config_manager: &ConfigurationManager) {
    println!("  Successfully loaded both core and runtime configurations");

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

    println!("  Provider Configuration:");
    for provider in ["anthropic", "openai", "xai"] {
        if config_manager.is_provider_enabled(provider) {
            if let Some(model) = config_manager.default_model(provider) {
                println!("    {provider}: enabled (default model: {model})");
            } else {
                println!("    {provider}: enabled (no default model)");
            }
        } else {
            println!("    {provider}: disabled");
        }
    }

    println!("  Feature Flags:");
    println!("    Telemetry: {}", config_manager.is_telemetry_enabled());
    println!("    Plugins: {}", config_manager.are_plugins_enabled());
    println!("    Caching: {}", config_manager.is_caching_enabled());
    println!("    Metrics: {}", config_manager.are_metrics_enabled());

    println!("  UI Settings:");
    println!("    Theme: {}", config_manager.ui_theme());
    println!("    Colors: {}", config_manager.are_ui_colors_enabled());

    println!("  Paths:");
    println!("    Cache Directory: {}", config_manager.cache_dir());
    println!("    Logs Directory: {}", config_manager.logs_dir());

    println!("  Service Configuration:");
    let services = config_manager.service_config();
    println!(
        "    Memory Cache Size: {} MB",
        services.memory.cache_size_mb
    );

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

fn validate_timeout(request_timeout_seconds: u32) {
    if request_timeout_seconds == 0 {
        println!("  Warning: Request timeout is set to 0 seconds");
    } else if request_timeout_seconds > 300 {
        println!("  Warning: Request timeout is very high ({request_timeout_seconds}s)");
    } else {
        println!("  Request timeout looks reasonable ({request_timeout_seconds}s)");
    }
}

fn validate_concurrent_requests(max_concurrent_requests: u32) {
    if max_concurrent_requests == 0 {
        println!("  Warning: Max concurrent requests is set to 0");
    } else if max_concurrent_requests > 50 {
        println!("  Warning: Max concurrent requests is very high ({max_concurrent_requests})");
    } else {
        println!("  Max concurrent requests looks reasonable ({max_concurrent_requests})");
    }
}
