# Orbit Providers

AI provider client implementations and abstractions for the Orbit ecosystem.

## Overview

This crate provides unified client implementations for multiple AI providers including Anthropic, OpenAI-compatible APIs, xAI, Frontal, AWS Bedrock, Microsoft Azure, and Ollama. It handles authentication, streaming responses, prompt caching, and provider-specific protocol differences through a common interface.

## Features

- Multi-provider support (Anthropic, OpenAI, xAI, Frontal, Bedrock, Azure, Ollama)
- Streaming response handling with Server-Sent Events
- OAuth token management and refresh
- Intelligent prompt caching with configurable policies
- Provider detection and model alias resolution
- Comprehensive error handling and type safety
- Telemetry integration for request tracking

## Key Components

- **ProviderClient**: Unified client interface for all AI providers
- **AnthropicClient**: Native Anthropic API implementation
- **OpenAiCompatClient**: OpenAI-compatible API client
- **PromptCache**: Intelligent caching system for prompt optimization
- **SSE Parser**: Handles streaming responses from providers
- **OAuth Management**: Secure token handling and refresh logic

## Dependencies

- `reqwest` for HTTP client functionality
- `tokio` for async runtime
- `serde` for serialization/deserialization
- `orbit-runtime` for core runtime integration
- `orbit-telemetry` for analytics and monitoring

## Current Status

This crate provides the core AI provider infrastructure and is included in workspace build/test gates.
