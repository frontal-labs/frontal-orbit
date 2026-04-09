# Plugin System Guide

This guide covers the Orbit plugin system, including how to use, develop, and distribute plugins.

## Overview

The Orbit plugin system allows extending the CLI with custom tools, commands, and functionality. Plugins can:

- Add new tools to the tool ecosystem
- Define custom slash commands
- Provide new AI providers
- Extend the UI and runtime behavior
- Integrate with external services

## Plugin Architecture

### Plugin Structure

A plugin is a directory with the following structure:

```
my-plugin/
  plugin.json          # Plugin manifest
  README.md            # Plugin documentation
  src/                 # Source code (if applicable)
  lib/                 # Compiled libraries
  config/              # Configuration files
  tests/               # Test files
```

### Plugin Manifest

The `plugin.json` file defines plugin metadata:

```json
{
  "name": "my-plugin",
  "version": "1.0.0",
  "description": "A custom plugin for Orbit",
  "author": {
    "name": "Your Name",
    "email": "your.email@example.com"
  },
  "license": "MIT",
  "homepage": "https://github.com/yourname/my-plugin",
  "repository": "https://github.com/yourname/my-plugin.git",
  "keywords": ["orbit", "plugin", "tools"],
  "orbit_version": ">=0.1.0",
  "type": "tool",
  "entry_point": "lib/libmy_plugin.so",
  "dependencies": [],
  "permissions": [
    "network",
    "file_read",
    "file_write"
  ],
  "tools": [
    {
      "name": "my_tool",
      "description": "Custom tool description",
      "schema": "schemas/tool.json"
    }
  ],
  "commands": [
    {
      "name": "my_command",
      "description": "Custom slash command",
      "handler": "handle_my_command"
    }
  ],
  "config": {
    "schema": "schemas/config.json",
    "default": "config/default.json"
  }
}
```

## Using Plugins

### Installing Plugins

```bash
# Install from local directory
orbit plugin install /path/to/my-plugin

# Install from Git repository
orbit plugin install https://github.com/username/my-plugin.git

# Install from plugin registry
orbit plugin install my-plugin

# Install specific version
orbit plugin install my-plugin@1.2.3
```

### Managing Plugins

```bash
# List installed plugins
orbit plugin list

# Show plugin details
orbit plugin show my-plugin

# Enable/disable plugin
orbit plugin enable my-plugin
orbit plugin disable my-plugin

# Update plugin
orbit plugin update my-plugin

# Uninstall plugin
orbit plugin uninstall my-plugin

# Check for updates
orbit plugin check-updates
```

### Plugin Configuration

```bash
# Configure plugin
orbit plugin config my-plugin --set key=value

# Show plugin config
orbit plugin config my-plugin --show

# Reset plugin config
orbit plugin config my-plugin --reset
```

## Built-in Plugins

### Filesystem Plugin

Provides enhanced file system operations:

```bash
# Install filesystem plugin
orbit plugin install filesystem

# Use filesystem tools
/filesystem/watch /path/to/directory
/filesystem/sync /source /destination
/filesystem/backup /path/to/backup
```

### Database Plugin

Database connectivity and operations:

```bash
# Install database plugin
orbit plugin install database

# Connect to database
/database connect postgresql://user:pass@localhost/db

# Run queries
/database query "SELECT * FROM users"
/database schema show
```

### Cloud Plugin

Cloud service integrations:

```bash
# Install cloud plugin
orbit plugin install cloud

# AWS operations
/cloud aws s3 list-buckets
/cloud aws ec2 list-instances

# Google Cloud operations
/cloud gcp storage list-buckets
/cloud gcp compute list-instances
```

## Developing Plugins

### Plugin Types

#### Tool Plugins
Add new tools to the tool ecosystem:

```rust
// src/lib.rs
use orbit_tools::{Tool, ToolResult, ToolContext};

pub struct MyTool;

impl Tool for MyTool {
    fn name(&self) -> &str {
        "my_tool"
    }

    fn description(&self) -> &str {
        "A custom tool for specific functionality"
    }

    fn execute(&self, ctx: &ToolContext, args: serde_json::Value) -> ToolResult {
        // Tool implementation
        Ok(serde_json::json!({"result": "success"}))
    }
}

// Plugin entry point
#[no_mangle]
pub extern "C" fn get_tools() -> Vec<Box<dyn Tool>> {
    vec![Box::new(MyTool)]
}
```

#### Command Plugins
Add custom slash commands:

```rust
use orbit_commands::{Command, CommandResult, CommandContext};

pub struct MyCommand;

impl Command for MyCommand {
    fn name(&self) -> &str {
        "my_command"
    }

    fn description(&self) -> &str {
        "Custom slash command"
    }

    fn execute(&self, ctx: &CommandContext, args: Vec<String>) -> CommandResult {
        // Command implementation
        Ok("Command executed successfully".to_string())
    }
}
```

#### Provider Plugins
Add new AI providers:

```rust
use orbit_providers::{Provider, ProviderConfig, CompletionResult};

pub struct MyProvider;

impl Provider for MyProvider {
    fn name(&self) -> &str {
        "my_provider"
    }

    fn complete(&self, config: &ProviderConfig, prompt: &str) -> CompletionResult {
        // Provider implementation
        Ok("Response from my provider".to_string())
    }
}
```

### Plugin Development Setup

1. **Create plugin directory**
   ```bash
   mkdir my-plugin
   cd my-plugin
   ```

2. **Initialize plugin**
   ```bash
   orbit plugin init --type tool --name my-plugin
   ```

3. **Write plugin code**
   ```rust
   // src/lib.rs
   use orbit_tools::prelude::*;

   #[derive(Debug)]
   pub struct MyCustomTool;

   impl Tool for MyCustomTool {
       fn name(&self) -> &'static str {
           "my_custom_tool"
       }

       fn description(&self) -> &'static str {
           "Does something custom"
       }

       fn schema(&self) -> serde_json::Value {
           serde_json::json!({
               "type": "object",
               "properties": {
                   "input": {
                       "type": "string",
                       "description": "Input for the tool"
                   }
               },
               "required": ["input"]
           })
       }

       fn execute(&self, ctx: &ToolContext, args: serde_json::Value) -> ToolResult {
           let input = args["input"].as_str().ok_or("Missing input")?;
           
           // Your tool logic here
           let result = format!("Processed: {}", input);
           
           Ok(serde_json::json!({
               "result": result,
               "timestamp": chrono::Utc::now().to_rfc3339()
           }))
       }
   }

   // Export the tool
   #[no_mangle]
   pub extern "C" fn get_tools() -> Vec<Box<dyn Tool>> {
       vec![Box::new(MyCustomTool)]
   }
   ```

4. **Create plugin manifest**
   ```json
   {
     "name": "my-custom-tool",
     "version": "0.1.0",
     "description": "A custom tool plugin",
     "type": "tool",
     "entry_point": "target/libmy_custom_tool.so",
     "orbit_version": ">=0.1.0",
     "permissions": ["network", "file_read"]
   }
   ```

5. **Build plugin**
   ```bash
   cargo build --release
   ```

6. **Test plugin**
   ```bash
   orbit plugin install .
   orbit plugin test my-custom-tool
   ```

### Plugin Testing

#### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_tool() {
        let tool = MyCustomTool;
        let ctx = ToolContext::mock();
        let args = serde_json::json!({"input": "test"});
        
        let result = tool.execute(&ctx, args).unwrap();
        assert_eq!(result["result"], "Processed: test");
    }
}
```

#### Integration Tests

```bash
# Test plugin installation
orbit plugin test-install my-plugin

# Test plugin functionality
orbit plugin test my-plugin

# Run plugin in test mode
orbit --plugin-test ./my-plugin prompt "test my tool"
```

### Plugin Distribution

#### Publishing to Registry

```bash
# Build plugin for distribution
orbit plugin build --release

# Publish to registry
orbit plugin publish

# Publish specific version
orbit plugin publish --version 1.2.3

# Publish as beta
orbit plugin publish --tag beta
```

#### GitHub Distribution

```bash
# Create GitHub release
git tag v1.2.3
git push origin v1.2.3

# Install from GitHub
orbit plugin install https://github.com/username/my-plugin.git
```

#### Local Distribution

```bash
# Create plugin package
orbit plugin package --output my-plugin-1.2.3.tar.gz

# Install from package
orbit plugin install my-plugin-1.2.3.tar.gz
```

## Plugin API Reference

### Tool API

#### Tool Trait

```rust
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn schema(&self) -> serde_json::Value;
    fn execute(&self, ctx: &ToolContext, args: serde_json::Value) -> ToolResult;
    fn permissions(&self) -> Vec<Permission> { vec![] }
}
```

#### Tool Context

```rust
pub struct ToolContext {
    pub working_directory: PathBuf,
    pub environment: HashMap<String, String>,
    pub session_id: String,
    pub user_id: String,
    pub config: Config,
}
```

### Command API

#### Command Trait

```rust
pub trait Command: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn usage(&self) -> &'static str;
    fn execute(&self, ctx: &CommandContext, args: Vec<String>) -> CommandResult;
}
```

### Provider API

#### Provider Trait

```rust
pub trait Provider: Send + Sync {
    fn name(&self) -> &'static str;
    fn configure(&self, config: ProviderConfig) -> Result<(), ProviderError>;
    fn complete(&self, prompt: &str, options: CompletionOptions) -> CompletionResult;
    fn stream(&self, prompt: &str, options: CompletionOptions) -> StreamResult;
}
```

## Plugin Security

### Permissions

Plugins declare required permissions in their manifest:

```json
{
  "permissions": [
    "network",
    "file_read",
    "file_write",
    "execute",
    "environment"
  ]
}
```

### Sandboxing

Plugins run in a sandboxed environment with:

- Limited file system access
- Restricted network access
- Controlled process execution
- Isolated memory space

### Security Best Practices

1. **Validate all inputs** - Never trust user input
2. **Use secure defaults** - Default to safe configurations
3. **Minimize permissions** - Request only necessary permissions
4. **Handle errors gracefully** - Don't expose sensitive information
5. **Update dependencies** - Keep dependencies secure

## Plugin Examples

### File Watcher Plugin

```rust
pub struct FileWatcherTool;

impl Tool for FileWatcherTool {
    fn name(&self) -> &'static str {
        "file_watcher"
    }

    fn description(&self) -> &'static str {
        "Watch files for changes"
    }

    fn execute(&self, ctx: &ToolContext, args: serde_json::Value) -> ToolResult {
        let path = args["path"].as_str().ok_or("Missing path")?;
        let recursive = args["recursive"].as_bool().unwrap_or(false);
        
        // Implementation would watch the file/directory
        Ok(serde_json::json!({
            "watching": path,
            "recursive": recursive,
            "status": "active"
        }))
    }
}
```

### HTTP Client Plugin

```rust
pub struct HttpClientTool;

impl Tool for HttpClientTool {
    fn name(&self) -> &'static str {
        "http_client"
    }

    fn description(&self) -> &'static str {
        "Make HTTP requests"
    }

    fn execute(&self, ctx: &ToolContext, args: serde_json::Value) -> ToolResult {
        let url = args["url"].as_str().ok_or("Missing URL")?;
        let method = args["method"].as_str().unwrap_or("GET");
        
        // Implementation would make HTTP request
        Ok(serde_json::json!({
            "url": url,
            "method": method,
            "status": "success"
        }))
    }
}
```

## Troubleshooting

### Common Issues

1. **Plugin fails to load**
   - Check plugin manifest syntax
   - Verify entry point path
   - Check Orbit version compatibility

2. **Permission denied**
   - Review plugin permissions
   - Check file system permissions
   - Verify sandbox configuration

3. **Plugin not found**
   - Check plugin installation
   - Verify plugin name
   - Check plugin registry

### Debug Mode

```bash
# Enable plugin debug logging
RUST_LOG=orbit_plugins=debug orbit plugin list

# Test plugin in isolation
orbit plugin test my-plugin --debug

# Show plugin diagnostics
orbit plugin doctor my-plugin
```

## Plugin Registry

### Official Registry

The official plugin registry hosts community plugins:

- **URL**: https://plugins.orbit.ai
- **Search**: `orbit plugin search <keyword>`
- **Categories**: Tools, Commands, Providers, Themes

### Community Plugins

Popular community plugins:

- **filesystem**: Enhanced file operations
- **database**: Database connectivity
- **cloud**: Cloud service integrations
- **monitoring**: System monitoring tools
- **automation**: Workflow automation

### Contributing to Registry

1. **Fork the registry repository**
2. **Add your plugin**
3. **Submit pull request**
4. **Wait for review and approval**

## Plugin Roadmap

### Upcoming Features

- **Plugin templates**: Quick-start templates for common plugin types
- **Plugin marketplace**: Built-in marketplace UI
- **Plugin dependencies**: Plugin-to-plugin dependencies
- **Hot reloading**: Reload plugins without restart
- **Plugin signing**: Cryptographic plugin verification

### API Changes

- **v2.0 API**: Breaking changes for improved security
- **Async support**: Full async/await support
- **Streaming tools**: Tools that can stream results
- **Multi-provider**: Plugins that support multiple providers
