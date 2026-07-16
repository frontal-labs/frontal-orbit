# Orbit Providers

AI provider client implementations and abstractions for the Orbit ecosystem.

## Overview

This crate provides unified client implementations for multiple AI providers including Anthropic, OpenAI-compatible APIs, xAI, Frontal, AWS Bedrock, Microsoft Azure, and Ollama. It handles authentication, streaming responses, prompt caching, and provider-specific protocol differences through a common interface.

## Features

- **Multi-provider Support**: Anthropic, OpenAI, xAI, Frontal, Bedrock, Azure, Ollama
- **Streaming Responses**: Real-time response handling with Server-Sent Events
- **Authentication**: OAuth token management, API key handling, and refresh logic
- **Prompt Caching**: Intelligent caching with configurable policies for optimization
- **Provider Detection**: Automatic provider detection and model alias resolution
- **Error Handling**: Comprehensive error types and recovery mechanisms
- **Telemetry**: Request tracking, performance monitoring, and analytics
- **Type Safety**: Strongly typed request/response structures
- **Protocol Abstraction**: Unified interface despite provider differences

## Supported Providers

### Anthropic
- Native Claude API integration
- Full streaming support
- Message history management
- Tool use and function calling
- Context window optimization

### OpenAI-Compatible
- OpenAI GPT models
- xAI Grok models
- Frontal API gateway
- Custom OpenAI-compatible endpoints
- Standardized request/response handling

### Cloud Providers
- AWS Bedrock with various model families
- Microsoft Azure OpenAI Service
- Enterprise authentication and security
- Region-specific endpoints
- Custom model deployments

### Local Models
- Ollama integration for local models
- Custom model hosting
- Private deployment support
- On-premises security

## Key Components

### ProviderClient
- Unified client interface for all AI providers
- Provider-agnostic API methods
- Automatic provider selection
- Request routing and load balancing

### Specific Clients
- **AnthropicClient**: Native Anthropic API implementation
- **OpenAiCompatClient**: OpenAI-compatible API client
- **BedrockClient**: AWS Bedrock integration
- **AzureClient**: Microsoft Azure OpenAI
- **OllamaClient**: Local model integration

### Supporting Infrastructure
- **PromptCache**: Intelligent caching system for prompt optimization
- **SSEParser**: Handles streaming responses from providers
- **OAuthManager**: Secure token handling and refresh logic
- **ModelRegistry**: Model discovery and alias resolution
- **RateLimiter**: Request throttling and quota management

## Authentication

### API Keys
- Environment variable configuration
- Secure key storage and rotation
- Multi-provider key management
- Key validation and testing

### OAuth Flow
- Token acquisition and refresh
- Secure token storage
- Automatic token renewal
- Error handling and retry logic

### Enterprise Authentication
- AWS IAM integration for Bedrock
- Azure AD integration for Azure OpenAI
- Custom authentication providers
- SSO and enterprise SAML support

## Configuration

### Environment Variables
```bash
# Anthropic
export ORBIT_API_KEY="sk-ant-..."
export ORBIT_BASE_URL="https://api.anthropic.com"

# OpenAI
export OPENAI_API_KEY="sk-..."
export OPENAI_BASE_URL="https://api.openai.com/v1"

# xAI
export XAI_API_KEY="xai-..."
export XAI_BASE_URL="https://api.x.ai/v1"

# Frontal
export FRONTAL_API_KEY="frontal-..."
export FRONTAL_BASE_URL="https://ai.frontal.dev/v1"

# AWS Bedrock
export AWS_ACCESS_KEY_ID="..."
export AWS_SECRET_ACCESS_KEY="..."
export AWS_REGION="us-east-1"

# Azure OpenAI
export AZURE_OPENAI_API_KEY="..."
export AZURE_OPENAI_ENDPOINT="https://your-resource.openai.azure.com/"

# Ollama
export OLLAMA_BASE_URL="http://localhost:11434"
```

### Provider Selection
- CLI flag: `--provider anthropic|openai|xai|frontal|bedrock|azure|ollama`
- Configuration file settings
- Runtime provider switching
- Automatic provider detection

## Usage

```rust
use orbit_providers::{ProviderClient, ProviderConfig};

// Create provider client
let config = ProviderConfig::from_env()?;
let client = ProviderClient::new(config)?;

// Send request to specific provider
let response = client
    .with_provider("anthropic")
    .chat_completion(request)
    .await?;

// Streaming response
let mut stream = client
    .with_provider("openai")
    .stream_chat_completion(request)
    .await?;

while let Some(chunk) = stream.next().await {
    println!("{}", chunk.content);
}
```

## Performance Optimization

### Prompt Caching
- Intelligent cache key generation
- Configurable cache policies
- Cache invalidation strategies
- Performance monitoring

### Request Optimization
- Context window management
- Token counting and estimation
- Batch request support
- Request deduplication

### Network Optimization
- Connection pooling
- Request retry logic
- Timeout configuration
- Circuit breaker patterns

## Error Handling

### Provider Errors
- Rate limiting and quota exceeded
- Authentication failures
- Model availability issues
- Network connectivity problems

### Recovery Strategies
- Automatic retry with exponential backoff
- Provider failover
- Graceful degradation
- Error reporting and telemetry

## Dependencies

- `reqwest` for HTTP client functionality
- `tokio` for async runtime
- `serde` for serialization/deserialization
- `serde_json` for JSON handling
- `thiserror` for error types
- `tracing` for structured logging
- `orbit-runtime` for core runtime integration
- `orbit-telemetry` for analytics and monitoring

## Testing

Comprehensive test coverage includes:
- Unit tests for each provider client
- Integration tests with mock services
- Error handling validation
- Performance benchmarks
- Authentication flow testing

Run tests with:
```bash
cargo test -p orbit-providers
```

## Current Status

This crate provides the core AI provider infrastructure and is included in workspace build/test gates. It supports all major AI providers and provides a unified interface for the Orbit ecosystem.

## Future Development

Planned enhancements:
- Additional provider support
- Advanced caching strategies
- Performance optimizations
- Enhanced authentication methods
- Real-time provider health monitoring
