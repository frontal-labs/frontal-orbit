# Configuration Guide

This guide covers the Orbit configuration system, including the new core configuration and legacy runtime configuration.

## Overview

Orbit uses a hierarchical configuration system:

1. **Core Configuration** (`config/project.json`) - New type-safe configuration system
2. **Runtime Configuration** - Legacy configuration system (still supported)
3. **Environment Variables** - Override configuration file settings
4. **Command-line Flags** - Override all other settings

## Core Configuration

The core configuration system provides type-safe, version-controlled configuration without secrets.

### Configuration File Structure

The main configuration file is `config/project.json`:

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
      "anthropic": {
        "enabled": true,
        "default_model": "claude-3-5-sonnet-20241022"
      },
      "openai": {
        "enabled": true,
        "default_model": "gpt-4"
      },
      "xai": {
        "enabled": true,
        "default_model": "grok-beta"
      }
    },
    "permission_mode": "permissive",
    "log_level": "info",
    "max_concurrent_requests": 10,
    "request_timeout_seconds": 30
  },
  "paths": {
    "config_home": "~/.orbit",
    "home": "~/.orbit",
    "cache_dir": "~/.orbit/cache",
    "logs_dir": "~/.orbit/logs"
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

### Configuration Sections

#### Project
- `name`: Project name
- `version`: Project version
- `description`: Project description

#### Runtime
- `default_provider`: Default AI provider (`anthropic`, `openai`, `xai`)
- `providers`: Provider-specific configurations
- `permission_mode`: Permission mode (`permissive`, `read-only`, `restricted`)
- `log_level`: Logging level (`debug`, `info`, `warn`, `error`)
- `max_concurrent_requests`: Maximum concurrent API requests
- `request_timeout_seconds`: Request timeout in seconds

#### Paths
- `config_home`: Configuration directory
- `home`: Orbit home directory
- `cache_dir`: Cache directory
- `logs_dir`: Logs directory

#### Features
- `enable_telemetry`: Enable telemetry collection
- `enable_plugins`: Enable plugin system
- `enable_caching`: Enable response caching
- `enable_metrics`: Enable metrics collection
- `enable_tracing`: Enable distributed tracing
- `enable_hot_reload`: Enable hot reload for development
- `auto_compaction_threshold`: Auto-compaction threshold
- `max_file_size_mb`: Maximum file size for operations
- `max_memory_usage_mb`: Maximum memory usage

#### UI
- `theme`: UI theme (`default`, `dark`, `light`)
- `enable_colors`: Enable colored output
- `show_progress_bars`: Show progress bars
- `confirm_dangerous_operations`: Confirm dangerous operations

#### Services
- `database`: Database connection settings
- `redis`: Redis connection settings
- `memory`: Memory cache settings

#### Sandbox
- `enable_docker`: Enable Docker sandbox
- `docker_image`: Default Docker image
- `default_shell`: Default shell for sandbox
- `max_execution_time_seconds`: Maximum execution time

#### Experimental
- `enable_new_features`: Enable experimental features
- `beta_features`: List of beta features to enable

### Configuration File Locations

The system looks for `project.json` in this order:

1. `$ORBIT_CONFIG_HOME/project.json` - Custom config directory
2. `$ORBIT_HOME/project.json` - Orbit home directory  
3. `~/.orbit/project.json` - User's home directory
4. `config/project.json` - Project-local configuration

### Using Core Configuration in Code

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

// Access nested configuration
let db_pool_size = config.services.database.connection_pool_size;
let cache_size = config.services.memory.cache_size_mb;
```

### Using ConfigurationManager

```rust
use orbit_runtime::ConfigurationManager;

// Load both core and runtime configurations
let manager = ConfigurationManager::load()?;

// Access core configuration through convenience methods
let provider = manager.default_provider();
let max_requests = manager.max_concurrent_requests();
let timeout = manager.request_timeout_seconds();

// Check feature flags
if manager.is_telemetry_enabled() {
    // Initialize telemetry
}

// Access service configuration
let services = manager.service_config();
println!("DB pool size: {}", services.database.connection_pool_size);
```

## Environment Variables

Environment variables override configuration file settings:

### Core Configuration Overrides
```bash
# Runtime settings
export ORBIT_DEFAULT_PROVIDER="openai"
export ORBIT_PERMISSION_MODE="restricted"
export ORBIT_LOG_LEVEL="debug"

# Feature flags
export ORBIT_ENABLE_TELEMETRY="false"
export ORBIT_ENABLE_PLUGINS="true"
export ORBIT_ENABLE_CACHING="true"

# Paths
export ORBIT_CONFIG_HOME="/custom/config/path"
export ORBIT_HOME="/custom/orbit/home"
export ORBIT_CACHE_DIR="/custom/cache"
export ORBIT_LOGS_DIR="/custom/logs"
```

### API Provider Variables
```bash
# Anthropic
export ANTHROPIC_API_KEY="sk-ant-..."
export ANTHROPIC_BASE_URL="https://api.anthropic.com"

# OpenAI
export OPENAI_API_KEY="sk-..."
export OPENAI_BASE_URL="https://api.openai.com/v1"

# xAI
export XAI_API_KEY="xai-..."
export XAI_BASE_URL="https://api.x.ai/v1"

# Frontal
export FRONTAL_API_KEY="frontal-..."
export FRONTAL_BASE_URL="https://api.frontal.ai/v1"
```

### Service Variables
```bash
# Database
export DATABASE_URL="postgresql://user:pass@localhost:5432/db"

# Redis
export REDIS_URL="redis://localhost:6379"

# Memory/Pinecone
export ORBIT_MEMORY_PINECONE_URL="https://index.pinecone.io"
export ORBIT_MEMORY_PINECONE_API_KEY="..."
```

## Legacy Runtime Configuration

The legacy runtime configuration system is still supported for backward compatibility.

### Runtime Configuration Files
- `~/.orbit/settings.json` - User settings
- `.orbit/settings.json` - Project settings
- `.orbit/local-settings.json` - Local overrides

### Migration Path

1. **Immediate**: Use `ConfigurationManager` to access both systems
2. **Gradual**: Replace runtime config access with core config methods
3. **Complete**: Migrate entirely to core configuration system

## Configuration Validation

### Doctor Command

Check your configuration with the doctor command:

```bash
orbit
/doctor
```

The doctor report includes:
- Core configuration status
- Runtime configuration status
- Authentication status
- Workspace health
- Sandbox status
- System information

### Configuration Validation

```rust
use orbit_core::config::ProjectConfig;

let config = ProjectConfig::load_or_default();

// Validate configuration
if config.runtime.max_concurrent_requests == 0 {
    eprintln!("Warning: Max concurrent requests cannot be 0");
}

if config.runtime.request_timeout_seconds > 300 {
    eprintln!("Warning: Request timeout is very high");
}

// Validate at least one provider is enabled
let enabled_providers = ["anthropic", "openai", "xai"]
    .iter()
    .filter(|&&provider| config.is_provider_enabled(provider))
    .count();

if enabled_providers == 0 {
    eprintln!("Error: No AI providers are enabled");
}
```

## Examples

### Development Configuration

```json
{
  "runtime": {
    "default_provider": "anthropic",
    "log_level": "debug",
    "permission_mode": "permissive"
  },
  "features": {
    "enable_telemetry": false,
    "enable_tracing": true,
    "enable_hot_reload": true
  },
  "experimental": {
    "enable_new_features": true,
    "beta_features": ["new_ui", "enhanced_sandbox"]
  }
}
```

### Production Configuration

```json
{
  "runtime": {
    "default_provider": "anthropic",
    "log_level": "info",
    "permission_mode": "restricted",
    "max_concurrent_requests": 5,
    "request_timeout_seconds": 60
  },
  "features": {
    "enable_telemetry": true,
    "enable_tracing": true,
    "enable_hot_reload": false
  },
  "services": {
    "database": {
      "connection_pool_size": 20,
      "max_connections": 50
    }
  }
}
```

### Minimal Configuration

```json
{
  "runtime": {
    "default_provider": "anthropic"
  },
  "features": {
    "enable_telemetry": false,
    "enable_plugins": false,
    "enable_caching": false,
    "enable_metrics": false
  }
}
```

## Troubleshooting

### Configuration Not Loading

1. Check file permissions on `config/project.json`
2. Verify JSON syntax with a linter
3. Check configuration file locations
4. Run `orbit /doctor` for diagnostic information

### Provider Not Working

1. Verify API keys are set correctly
2. Check provider is enabled in configuration
3. Verify default model is set for the provider
4. Check network connectivity

### Performance Issues

1. Reduce `max_concurrent_requests` if rate limited
2. Increase `request_timeout_seconds` for slow models
3. Enable caching for repeated requests
4. Disable unnecessary features

### Development Issues

1. Use development configuration file
2. Enable debug logging
3. Enable tracing for detailed diagnostics
4. Use hot reload for faster iteration

## Best Practices

1. **Version Control**: Commit `config/project.json` to version control
2. **Environment Specific**: Use different configs for dev/staging/prod
3. **Security**: Never commit secrets; use environment variables
4. **Validation**: Validate configuration on startup
5. **Documentation**: Document custom configuration options
6. **Testing**: Test configuration changes in isolation
7. **Monitoring**: Monitor configuration changes and their effects

For more detailed examples and integration guides, see [`./CORE_CONFIG_INTEGRATION.md`](./CORE_CONFIG_INTEGRATION.md).
