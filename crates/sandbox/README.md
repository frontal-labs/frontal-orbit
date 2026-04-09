# Orbit Sandbox

Sandboxing and isolation capabilities for secure code execution environments.

## Overview

This crate provides sandboxing functionality to isolate code execution and restrict access to system resources. It implements configurable security policies for filesystem access, network isolation, and namespace restrictions.

## Features

- Filesystem isolation with multiple modes (off, workspace-only, allow-list)
- Network isolation capabilities
- Namespace restrictions for enhanced security
- Configurable mount points and access controls
- Container environment detection and handling

## Key Components

- **SandboxConfig**: Configuration structure for sandbox settings
- **SandboxRequest**: Runtime sandbox configuration
- **FilesystemIsolationMode**: Enumeration of filesystem isolation levels
- **ContainerEnvironment**: Detection of containerized execution environments

## Dependencies

- `serde` for serialization/deserialization of configuration

## Current Status

This crate provides core sandboxing infrastructure and is included in workspace build/test gates.
