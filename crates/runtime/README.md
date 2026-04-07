# Orbit Runtime

Core runtime primitives and session management for the Orbit ecosystem.

## Overview

This crate provides the foundational runtime components that power the Orbit CLI and supporting services. It handles session persistence, permission evaluation, prompt assembly, MCP plumbing, tool operations, and the core conversation loop.

## Features

- Session management and persistence
- Permission evaluation and enforcement
- MCP (Model Context Protocol) integration
- Tool execution and file operations
- Conversation flow control
- Configuration management
- Plugin lifecycle management
- OAuth authentication
- Bash command execution and validation
- LSP client integration
- Telemetry and usage tracking

## Key Components

- **Session**: Core session state and persistence
- **Permissions**: Security and access control system
- **MCP Integration**: Protocol handling for external tools
- **File Operations**: Secure file system interactions
- **Bash Execution**: Command execution with validation
- **Configuration**: Runtime configuration management
- **Plugin Lifecycle**: Plugin loading and management
- **Conversation**: AI conversation flow and context

## Dependencies

- `tokio` for async runtime
- `serde` for serialization
- `regex` for pattern matching
- `glob` for file pattern handling
- `walkdir` for file system traversal
- `sha2` for cryptographic operations
- `orbit-plugins` for plugin integration
- `orbit-telemetry` for analytics

## Architecture

The runtime serves as the central nervous system of Orbit, coordinating between:
- AI provider interactions via the API layer
- Plugin system for extensibility
- Tool execution and file operations
- Security and permission enforcement
- Session state and conversation management
