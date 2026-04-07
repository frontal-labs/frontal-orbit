# Orbit API

Core API client library for interacting with various AI providers in the Orbit ecosystem.

## Overview

This crate provides a unified interface for communicating with different AI providers including Anthropic, OpenAI-compatible APIs, and xAI. It handles authentication, request/response formatting, streaming responses, and prompt caching.

## Features

- Multi-provider support (Anthropic, OpenAI-compatible, xAI)
- Streaming response handling with Server-Sent Events
- OAuth token management and refresh
- Prompt caching with configurable cache policies
- Comprehensive error handling and type safety
- Telemetry integration for request tracking

## Key Components

- **ProviderClient**: Unified client interface for all AI providers
- **PromptCache**: Intelligent caching system for prompt optimization
- **SSE Parser**: Handles streaming responses from providers
- **OAuth Management**: Secure token handling and refresh logic

## Dependencies

- `reqwest` for HTTP client functionality
- `tokio` for async runtime
- `serde` for serialization/deserialization
- `orbit-runtime` for core runtime integration
- `orbit-telemetry` for analytics and monitoring
