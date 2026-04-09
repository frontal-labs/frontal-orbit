# Orbit Integrations

External integration modules providing MCP interoperability and native IDE integration for the Orbit ecosystem.

## Overview

The `orbit-integrations` crate serves as the bridge between Orbit and external systems, including Model Context Protocol (MCP) servers and various IDE environments. It provides the infrastructure needed for seamless integration with popular development tools and external services.

## Features

### MCP Integration
- **MCP Server Management**: Configuration, lifecycle, and stdio management
- **Tool Bridge**: Bidirectional tool execution between Orbit and MCP servers
- **Protocol Handling**: Full MCP protocol implementation with error handling
- **Server Discovery**: Automatic detection and connection to MCP servers
- **Resource Sharing**: Resource access and sharing between systems

### IDE Integration
- **Multi-Editor Support**: VS Code, Cursor, Windsurf, Antigravity, and more
- **Configuration Management**: Per-editor configuration files and settings
- **Extension Packaging**: `.vsix` package generation and distribution
- **Editor Launching**: Automatic editor detection and launching
- **Workspace Integration**: Deep workspace-aware functionality

## Key Components

### MCP System
- `McpServer` - Server lifecycle and communication management
- `McpToolBridge` - Tool execution bridge between systems
- `McpConfig` - Server configuration and connection settings
- `McpProtocol` - Protocol implementation and message handling

### IDE System
- `IdeManager` - Multi-editor support and management
- `IdeConfig` - Per-editor configuration handling
- `ExtensionBuilder` - `.vsix` package creation and management
- `WorkspaceIntegration` - Workspace-aware IDE features

## Current Status

This crate is active in the workspace and provides comprehensive integration capabilities:

### MCP Features
- MCP-related configuration, lifecycle, stdio management, and tool bridge functionality
- Full protocol compliance with MCP specification
- Resource and tool sharing capabilities
- Error handling and recovery mechanisms

### IDE Features
- `/ide` target parsing and command handling
- Workspace-local config persistence and management
- Per-editor config wiring (`.vscode/orbit.json` / `.cursor/orbit.json` / `.antigravity/orbit.json` / `.windsurf/orbit.json`)
- `.vsix` packaging from `extensions/orbit-ide`
- Extension install via editor CLI
- Editor launching and detection

## Usage

### MCP Integration
```rust
use orbit_integrations::{McpServer, McpConfig};

let config = McpConfig::new("server-name", "/path/to/server");
let server = McpServer::start(config)?;
server.connect_tools()?;
```

### IDE Integration
```rust
use orbit_integrations::{IdeManager, IdeConfig};

let manager = IdeManager::new();
let ide = manager.detect_ide()?;
let config = IdeConfig::load_for_workspace(&ide)?;
manager.launch_editor(&config)?;
```

## Supported IDEs

- **VS Code**: Full integration with marketplace and settings
- **Cursor**: Native support with custom configuration
- **Windsurf**: Deep workspace integration
- **Antigravity**: Experimental support
- **Other editors**: Extensible framework for new editors

## Configuration Files

The crate manages several configuration files:
- `.vscode/orbit.json` - VS Code specific settings
- `.cursor/orbit.json` - Cursor specific settings
- `.windsurf/orbit.json` - Windsurf specific settings
- `.antigravity/orbit.json` - Antigravity specific settings

## MCP Protocol Support

Full MCP protocol implementation including:
- Tool discovery and execution
- Resource access and management
- Message passing and error handling
- Server lifecycle management

## Integration Architecture

The integration system follows a modular architecture:
1. **Protocol Layer**: MCP protocol implementation
2. **Management Layer**: Server and IDE lifecycle management
3. **Configuration Layer**: Settings and configuration handling
4. **Bridge Layer**: Communication between systems

## Testing

Comprehensive test coverage includes:
- MCP protocol compliance tests
- IDE integration validation
- Configuration loading and saving
- Extension packaging verification

Run tests with:
```bash
cargo test -p orbit-integrations
```

## Dependencies

- MCP protocol libraries
- IDE-specific SDKs and APIs
- Configuration management libraries
- Extension packaging tools

## Future Development

Planned enhancements:
- Additional IDE support
- Advanced MCP features
- Cloud-based configuration
- Real-time synchronization
