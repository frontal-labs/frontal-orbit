# Orbit Mock Anthropic Service

Mock service for simulating Anthropic API responses during testing and development.

## Overview

This crate provides a mock implementation of the Anthropic API service that can be used for testing, development, and demonstration purposes. It simulates the behavior of the real Anthropic API without requiring actual API calls.

## Features

- Mock Anthropic API server
- Realistic response simulation
- Development and testing support
- Standalone binary execution

## Usage

The mock service can be run as a standalone binary:

```bash
cargo run --bin mock-anthropic-service
```

This starts a local server that mimics the Anthropic API behavior for testing purposes.

## Dependencies

- `orbit-api` for API compatibility
- `serde_json` for response formatting
- `tokio` for async server functionality
