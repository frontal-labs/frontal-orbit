# Orbit Plugins

Plugin system for extending Orbit functionality with custom commands and tools.

## Overview

This crate provides the core plugin infrastructure that allows developers to extend the Orbit CLI with custom commands, tools, and integrations. It defines the plugin interface, management system, and communication protocols.

## Features

- Plugin registration and discovery
- Command and tool extension points
- Plugin lifecycle management
- Serialization interface for plugin data
- Error handling and validation

## Key Components

- **PluginManager**: Central registry and lifecycle manager
- **PluginError**: Comprehensive error handling for plugin operations
- **PluginSummary**: Metadata and capability information
- Serialization support for plugin communication

## Dependencies

- `serde` for plugin data serialization
- `serde_json` for JSON-based plugin communication

## Usage

Plugins can be developed to extend Orbit with custom functionality, integrating seamlessly with the command system and runtime environment.
