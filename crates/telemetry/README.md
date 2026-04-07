# Orbit Telemetry

Telemetry and analytics system for monitoring Orbit usage and performance.

## Overview

This crate provides comprehensive telemetry capabilities for tracking usage patterns, performance metrics, and system health across the Orbit ecosystem. It supports multiple output formats and can be configured for different monitoring needs.

## Features

- Event tracking and analytics
- Session tracing and profiling
- JSONL and memory-based telemetry sinks
- Client identity management
- Request profiling for AI providers
- Configurable telemetry output

## Key Components

- **TelemetrySink**: Configurable output destinations
- **SessionTracer**: Session-level tracking
- **AnalyticsEvent**: Standardized event format
- **JsonlTelemetrySink**: File-based logging
- **MemoryTelemetrySink**: In-memory tracking

## Dependencies

- `serde` for event serialization
- `serde_json` for JSON-based telemetry

## Usage

Telemetry can be configured to track various aspects of Orbit usage, from individual API calls to session-level patterns, providing insights for optimization and debugging.
