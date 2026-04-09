# Core Configuration Integration

This document summarizes the integration of the new core configuration system into the Orbit codebase.

## Overview

We've successfully implemented a new configuration system using the `orbit-core` crate that provides:

- **Type-safe JSON configuration** with Rust structs
- **Feature flags and internal settings** (no secrets)
- **Multiple configuration file locations** with fallback support
- **Integration with existing runtime configuration**
- **Gradual migration path** from the old system

## Key Components

### 1. Core Configuration (`orbit-core`)

- **Location**: `crates/core/src/config.rs`
- **Config file**: `config/project.json`
- **Features**:
  - Project metadata (name, version, description)
  - Runtime configuration (providers, timeouts, concurrency)
  - Feature flags (telemetry, plugins, caching, metrics)
  - UI configuration (theme, colors, progress bars)
  - Path configuration (cache, logs, config directories)
  - Service configuration (database, Redis, memory)
  - Sandbox configuration (Docker settings)
  - Experimental features

### 2. Configuration Bridge (`orbit-runtime`)

- **Location**: `crates/runtime/src/core_config.rs`
- **Purpose**: Bridges core configuration with existing runtime configuration
- **Features**:
  - `ConfigurationManager` that loads both core and runtime configs
  - Convenience methods for accessing common configuration values
  - Backward compatibility with existing runtime system

### 3. CLI Integration (`orbit-cli`)

- **Updated**: `crates/cli/src/main.rs`
- **Features**:
  - Added `ConfigurationManager` import
  - Enhanced doctor report with core configuration information
  - Added `check_core_config_health()` diagnostic check

### 4. API Integration (`orbit-api`)

- **Updated**: `crates/api/src/main.rs`
- **Features**:
  - Demonstrates loading core configuration on startup
  - Prints key configuration values

## Configuration File Structure

The `config/project.json` file contains:

```json
{
  "project": {
    "name": "Orbit",
    "version": "0.1.0",
    "description": "AI-powered development environment and CLI tool"
  },
  "runtime": {
    "default_provider": "anthropic",
    "providers": {
      "anthropic": { "enabled": true, "default_model": "claude-3-5-sonnet-20241022" },
      "openai": { "enabled": true, "default_model": "gpt-4" },
      "xai": { "enabled": true, "default_model": "grok-beta" }
    },
    "permission_mode": "permissive",
    "log_level": "info",
    "max_concurrent_requests": 10,
    "request_timeout_seconds": 30
  },
  "features": {
    "auto_compaction_threshold": 100,
    "enable_telemetry": true,
    "enable_plugins": true,
    "enable_caching": true,
    "enable_metrics": true,
    "enable_tracing": false,
    "enable_hot_reload": false,
    "max_file_size_mb": 100,
    "max_memory_usage_mb": 2048
  },
  "ui": {
    "theme": "default",
    "enable_colors": true,
    "show_progress_bars": true,
    "confirm_dangerous_operations": true
  },
  "paths": {
    "config_home": "~/.orbit",
    "home": "~/.orbit",
    "cache_dir": "~/.orbit/cache",
    "logs_dir": "~/.orbit/logs"
  },
  "services": {
    "database": {
      "connection_pool_size": 10,
      "connection_timeout_seconds": 30,
      "max_connections": 20
    },
    "redis": {
      "connection_pool_size": 10,
      "connection_timeout_seconds": 10
    },
    "memory": {
      "cache_size_mb": 512,
      "namespace": "default"
    }
  },
  "sandbox": {
    "enable_docker": false,
    "docker_image": "busybox:1.36",
    "default_shell": "/bin/bash",
    "max_execution_time_seconds": 300
  },
  "experimental": {
    "enable_new_features": false,
    "beta_features": []
  }
}
```

## Usage Examples

### Direct Core Configuration Usage

```rust
use orbit_core::config::ProjectConfig;

// Load configuration with fallback to defaults
let config = ProjectConfig::load_or_default();

// Access configuration values
println!("Default provider: {}", config.runtime.default_provider);
println!("Telemetry enabled: {}", config.features.enable_telemetry);

// Provider-specific methods
if config.is_provider_enabled("anthropic") {
    let model = config.get_default_model("anthropic").unwrap();
    println!("Using Anthropic with model: {}", model);
}
```

### Using ConfigurationManager

```rust
use orbit_runtime::ConfigurationManager;

// Load both core and runtime configurations
let config_manager = ConfigurationManager::load()?;

// Access core configuration through convenience methods
let provider = config_manager.default_provider();
let max_requests = config_manager.max_concurrent_requests();
let timeout = config_manager.request_timeout_seconds();

// Check feature flags
if config_manager.is_telemetry_enabled() {
    // Initialize telemetry
}

// Access service configuration
let db_pool_size = config_manager.service_config().database.connection_pool_size;
```

### CLI Doctor Report

The CLI `orbit doctor` command now includes core configuration information:

```
Core Configuration
  Default provider: anthropic
  Max concurrent requests: 10
  Request timeout: 30s
  Telemetry enabled: true
  Plugins enabled: true
  Caching enabled: true
  Metrics enabled: true
  UI theme: default
  anthropic model: claude-3-5-sonnet-20241022
  openai model: gpt-4
  xai model: grok-beta
```

## Configuration File Locations

The system looks for `project.json` in the following locations (in order):

1. `$ORBIT_CONFIG_HOME/project.json` - Custom config directory
2. `$ORBIT_HOME/project.json` - Orbit home directory  
3. `~/.orbit/project.json` - User's home directory
4. `config/project.json` - Project-local configuration

If no configuration file is found, the system falls back to sensible defaults.

## Benefits

1. **Type Safety**: All configuration values are strongly typed with Rust structs
2. **No Secrets**: Configuration contains only internal settings and feature flags
3. **Version Control Safe**: Can be committed to version control without security concerns
4. **Backward Compatible**: Existing runtime configuration system continues to work
5. **Gradual Migration**: Can adopt new configuration system incrementally
6. **Comprehensive**: Covers all major aspects of system configuration
7. **Testable**: Full test coverage with comprehensive examples

## Testing

Run the example to see the configuration system in action:

```bash
cargo run -p orbit-core --example core_config_example
```

Run tests for the core configuration:

```bash
cargo test -p orbit-core
cargo test -p orbit-runtime core_config
```

## Future Enhancements

1. **Configuration Validation**: Add runtime validation of configuration values
2. **Hot Reload**: Support for reloading configuration without restart
3. **Environment Overrides**: Allow environment variables to override config values
4. **Configuration Schema**: JSON schema for validation and IDE support
5. **Migration Tools**: Tools to migrate from old configuration format
6. **CLI Commands**: Add CLI commands for managing configuration

## Migration Path

For existing code using the runtime configuration system:

1. **Immediate**: Use `ConfigurationManager` to access both systems
2. **Gradual**: Replace runtime config access with core config methods
3. **Complete**: Migrate entirely to core configuration system

The `ConfigurationManager` provides a smooth transition path while maintaining backward compatibility.
