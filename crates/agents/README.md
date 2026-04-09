# Orbit Agents

Agent-related models and interfaces for the Orbit workspace.

## Current Status

This crate is part of the active workspace and compiles/tests successfully.

It now re-exports the workspace observability surface for agent runtimes,
including:

- `orbit-observability` run/span APIs for AI agent instrumentation
- Sentry-style error reporting configuration and capture primitives
- adapters for forwarding agent observations into `orbit-telemetry`
