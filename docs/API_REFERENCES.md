# API Reference

This comprehensive reference covers the Orbit API for programmatic integration and automation.

## Table of Contents

- [Overview](#overview)
- [Authentication](#authentication)
- [REST API](#rest-api)
- [WebSocket API](#websocket-api)
- [Rust API](#rust-api)
  - [Configuration API](#configuration-api)
- [Python API](#python-api)
- [JavaScript API](#javascript-api)
- [Error Handling](#error-handling)
- [Rate Limiting](#rate-limiting)
- [Examples](#examples)

## Overview

The Orbit API provides multiple ways to integrate Orbit's capabilities into your applications:

- **REST API**: HTTP-based API for web integration
- **WebSocket API**: Real-time streaming and events
- **Rust API**: Native Rust library integration
- **Python API**: Python bindings for Orbit functionality
- **JavaScript API**: Node.js and browser support

### Base URLs

- **Production**: `https://api.orbit.ai/v1`
- **Development**: `http://localhost:8080/v1`
- **Staging**: `https://staging-api.orbit.ai/v1`

## Authentication

### API Key Authentication

All API requests require authentication using an API key:

```bash
# Using environment variable
export ORBIT_API_KEY="your-api-key-here"
export ORBIT_SERVER_API_KEY="your-hosted-server-api-key"

# Using header
curl -H "Authorization: Bearer your-api-key-here" \
     https://api.orbit.ai/v1/completions
```

For self-hosted `orbit-server` deployments, set `ORBIT_SERVER_API_KEY` on the server and
have connectors or other clients present that same shared secret as `ORBIT_API_KEY` or
the `x-api-key` header when calling hosted control-plane routes.

### Token Types

| Token Type | Description | Usage |
|------------|-------------|--------|
| **Session Token** | Temporary session token | Interactive sessions |
| **API Key** | Permanent API key | Server applications |
| **Plugin Token** | Plugin-specific token | Plugin authentication |

### Token Management

```bash
# Generate session token
orbit auth token --type session --ttl 1h

# Generate API key
orbit auth token --type api --name "My App"

# List tokens
orbit auth token list

# Revoke token
orbit auth token revoke <TOKEN_ID>
```

## REST API

### Endpoints

#### Completions

Create a completion request.

```http
POST /v1/completions
Content-Type: application/json
Authorization: Bearer <API_KEY>

{
  "model": "claude-sonnet-4-6",
  "provider": "anthropic",
  "messages": [
    {
      "role": "user",
      "content": "What files are in the current directory?"
    }
  ],
  "tools": ["read", "grep"],
  "permission_mode": "safe-mode",
  "stream": false,
  "max_tokens": 4096,
  "temperature": 0.7
}
```

**Response:**
```json
{
  "id": "req_123456",
  "object": "completion",
  "created": 1704067200,
  "model": "claude-sonnet-4-6",
  "provider": "anthropic",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "I can see the following files in the current directory...",
        "tool_calls": [
          {
            "type": "function",
            "function": {
              "name": "read",
              "arguments": "{\"path\": \".\"}"
            }
          }
        ]
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 25,
    "completion_tokens": 150,
    "total_tokens": 175
  }
}
```

#### Streaming Completions

Stream completion responses in real-time.

```http
POST /v1/completions
Content-Type: application/json
Authorization: Bearer <API_KEY>

{
  "model": "claude-sonnet-4-6",
  "messages": [
    {
      "role": "user",
      "content": "Explain this codebase"
    }
  ],
  "stream": true
}
```

**Streaming Response:**
```
data: {"id": "req_123456", "type": "completion_start", ...}
data: {"type": "content_delta", "delta": {"content": "I"}}
data: {"type": "content_delta", "delta": {"content": " can"}}
data: {"type": "completion_end", ...}
```

#### Tools

Execute tools directly.

```http
POST /v1/tools/execute
Content-Type: application/json
Authorization: Bearer <API_KEY>

{
  "tool": "read",
  "arguments": {
    "path": "/path/to/file.txt"
  },
  "permission_mode": "safe-mode"
}
```

**Response:**
```json
{
  "success": true,
  "result": {
    "content": "File content here...",
    "metadata": {
      "size": 1024,
      "modified": "2024-01-01T12:00:00Z"
    }
  },
  "tool": "read",
  "execution_time": 0.05
}
```

#### Sessions

Manage conversation sessions.

```http
# Create session
POST /v1/sessions
{
  "model": "claude-sonnet-4-6",
  "permission_mode": "safe-mode",
  "metadata": {
    "name": "Development Session"
  }
}

# Get session
GET /v1/sessions/{session_id}

# List sessions
GET /v1/sessions

# Delete session
DELETE /v1/sessions/{session_id}
```

#### Configuration

Get and update configuration.

```http
# Get configuration
GET /v1/config

# Update configuration
PATCH /v1/config
{
  "runtime": {
    "default_model": "claude-opus-5"
  }
}
```

#### Status

Get system status and health.

```http
GET /v1/status
```

**Response:**
```json
{
  "status": "healthy",
  "version": "1.0.0",
  "uptime": 86400,
  "components": {
    "api": "healthy",
    "database": "healthy",
    "mcp": "healthy"
  },
  "metrics": {
    "requests_per_minute": 45,
    "active_sessions": 12,
    "memory_usage": "512MB"
  }
}
```

## WebSocket API

### Connection

Connect to the WebSocket API for real-time communication:

```javascript
const ws = new WebSocket('wss://api.orbit.ai/v1/ws');

ws.onopen = function() {
  // Authenticate
  ws.send(JSON.stringify({
    type: 'auth',
    token: 'your-api-key'
  }));
};

ws.onmessage = function(event) {
  const data = JSON.parse(event.data);
  console.log('Received:', data);
};
```

### Message Types

#### Authentication

```json
{
  "type": "auth",
  "token": "your-api-key"
}
```

#### Completion Request

```json
{
  "type": "completion",
  "request_id": "req_123",
  "model": "claude-sonnet-4-6",
  "messages": [
    {
      "role": "user",
      "content": "Hello, world!"
    }
  ],
  "stream": true
}
```

#### Completion Response

```json
{
  "type": "completion_response",
  "request_id": "req_123",
  "content": "Hello! How can I help you today?",
  "finish_reason": "stop"
}
```

#### Tool Execution

```json
{
  "type": "tool_call",
  "tool": "read",
  "arguments": {
    "path": "/tmp/test.txt"
  }
}
```

#### Tool Result

```json
{
  "type": "tool_result",
  "tool": "read",
  "result": {
    "content": "File content",
    "success": true
  }
}
```

## Rust API

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
orbit-api = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
```

### Basic Usage

```rust
use orbit_api::{Client, Config, CompletionRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client
    let config = Config::builder()
        .api_key("your-api-key")
        .base_url("https://api.orbit.ai/v1")
        .build()?;
    
    let client = Client::new(config);
    
    // Create completion request
    let request = CompletionRequest::builder()
        .model("claude-sonnet-4-6")
        .message("user", "What files are in the current directory?")
        .tools(&["read", "grep"])
        .permission_mode(PermissionMode::Safe)
        .build()?;
    
    // Send request
    let response = client.completions().create(request).await?;
    
    println!("Response: {}", response.content);
    
    Ok(())
}
```

### Configuration API

The Orbit configuration system provides type-safe configuration management:

```rust
use orbit_core::config::ProjectConfig;
use orbit_runtime::ConfigurationManager;

// Load core configuration
let config = ProjectConfig::load_or_default();

// Access configuration values
println!("Default provider: {}", config.runtime.default_provider);
println!("Telemetry enabled: {}", config.features.enable_telemetry);

// Provider configuration
if config.is_provider_enabled("anthropic") {
    let model = config.get_default_model("anthropic").unwrap();
    println!("Using Anthropic with model: {}", model);
}

// Use ConfigurationManager for bridge functionality
let manager = ConfigurationManager::load()?;
let provider = manager.default_provider();
let max_requests = manager.max_concurrent_requests();

// Feature flags
if manager.is_telemetry_enabled() {
    // Initialize telemetry
}

// Service configuration
let services = manager.service_config();
println!("Database pool size: {}", services.database.connection_pool_size);
```

### Configuration Structs

#### ProjectConfig
```rust
pub struct ProjectConfig {
    pub project: ProjectInfo,
    pub runtime: RuntimeConfig,
    pub paths: PathConfig,
    pub features: FeatureConfig,
    pub ui: UiConfig,
    pub services: ServiceConfig,
    pub sandbox: SandboxConfig,
    pub experimental: ExperimentalConfig,
}
```

#### RuntimeConfig
```rust
pub struct RuntimeConfig {
    pub default_provider: String,
    pub providers: ProviderConfig,
    pub permission_mode: String,
    pub log_level: String,
    pub max_concurrent_requests: u32,
    pub request_timeout_seconds: u32,
}
```

#### FeatureConfig
```rust
pub struct FeatureConfig {
    pub auto_compaction_threshold: u32,
    pub enable_telemetry: bool,
    pub enable_plugins: bool,
    pub enable_caching: bool,
    pub enable_metrics: bool,
    pub enable_tracing: bool,
    pub enable_hot_reload: bool,
    pub max_file_size_mb: u32,
    pub max_memory_usage_mb: u32,
}
```

### Configuration Methods

#### ProjectConfig Methods
```rust
impl ProjectConfig {
    // Load configuration with fallback to defaults
    pub fn load_or_default() -> Self;
    
    // Load from specific path
    pub fn load_from_path(path: &PathBuf) -> Result<Self, Error>;
    
    // Provider methods
    pub fn is_provider_enabled(&self, provider: &str) -> bool;
    pub fn get_default_model(&self, provider: &str) -> Option<String>;
    pub fn get_provider_config(&self, provider: &str) -> Option<&ProviderDetails>;
    
    // Save configuration
    pub fn save(&self) -> Result<(), Error>;
    pub fn save_to_path(&self, path: &PathBuf) -> Result<(), Error>;
}
```

#### ConfigurationManager Methods
```rust
impl ConfigurationManager {
    // Load both core and runtime configurations
    pub fn load() -> Result<Self, Error>;
    pub fn load_with_cwd(cwd: impl AsRef<Path>) -> Result<Self, Error>;
    
    // Core configuration accessors
    pub fn default_provider(&self) -> &str;
    pub fn max_concurrent_requests(&self) -> u32;
    pub fn request_timeout_seconds(&self) -> u32;
    pub fn permission_mode(&self) -> &str;
    pub fn log_level(&self) -> &str;
    
    // Feature flag accessors
    pub fn is_telemetry_enabled(&self) -> bool;
    pub fn are_plugins_enabled(&self) -> bool;
    pub fn is_caching_enabled(&self) -> bool;
    pub fn are_metrics_enabled(&self) -> bool;
    
    // Provider methods
    pub fn is_provider_enabled(&self, provider: &str) -> bool;
    pub fn default_model(&self, provider: &str) -> Option<String>;
    
    // Configuration access
    pub fn core(&self) -> &ProjectConfig;
    pub fn runtime(&self) -> &RuntimeConfig;
    pub fn service_config(&self) -> &ServiceConfig;
    pub fn sandbox_config(&self) -> &SandboxConfig;
    pub fn feature_config(&self) -> &FeatureConfig;
}
```

### Streaming

```rust
use orbit_api::{Client, CompletionRequest};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?;
    
    let request = CompletionRequest::builder()
        .model("claude-sonnet-4-6")
        .message("user", "Explain this codebase")
        .stream(true)
        .build()?;
    
    let mut stream = client.completions().create_stream(request).await?;
    
    while let Some(chunk) = stream.next().await {
        match chunk? {
            Chunk::Content(delta) => print!("{}", delta),
            Chunk::ToolCall(call) => println!("Tool call: {:?}", call),
            Chunk::End => break,
        }
    }
    
    Ok(())
}
```

### Tool Execution

```rust
use orbit_api::{Client, ToolRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?;
    
    let request = ToolRequest::builder()
        .tool("read")
        .arg("path", "/tmp/test.txt")
        .permission_mode(PermissionMode::Safe)
        .build()?;
    
    let result = client.tools().execute(request).await?;
    
    println!("Tool result: {}", result.content);
    
    Ok(())
}
```

## Python API

### Installation

```bash
pip install orbit-api
```

### Basic Usage

```python
from orbit_api import Client, CompletionRequest

# Create client
client = Client(api_key="your-api-key")

# Create completion request
request = CompletionRequest(
    model="claude-sonnet-4-6",
    messages=[
        {"role": "user", "content": "What files are in the current directory?"}
    ],
    tools=["read", "grep"],
    permission_mode="safe-mode"
)

# Send request
response = client.completions.create(request)
print(response.content)
```

### Streaming

```python
from orbit_api import Client, CompletionRequest

client = Client(api_key="your-api-key")

request = CompletionRequest(
    model="claude-sonnet-4-6",
    messages=[{"role": "user", "content": "Explain this codebase"}],
    stream=True
)

for chunk in client.completions.create_stream(request):
    if chunk.type == "content":
        print(chunk.content, end="")
    elif chunk.type == "tool_call":
        print(f"\nTool call: {chunk.tool}")
```

### Async Usage

```python
import asyncio
from orbit_api import AsyncClient

async def main():
    client = AsyncClient(api_key="your-api-key")
    
    request = CompletionRequest(
        model="claude-sonnet-4-6",
        messages=[{"role": "user", "content": "Hello, world!"}]
    )
    
    response = await client.completions.create(request)
    print(response.content)

asyncio.run(main())
```

## JavaScript API

### Installation

```bash
npm install @orbit/api
```

### Node.js Usage

```javascript
const { OrbitClient } = require('@orbit/api');

// Create client
const client = new OrbitClient({
  apiKey: 'your-api-key',
  baseURL: 'https://api.orbit.ai/v1'
});

// Create completion
async function createCompletion() {
  const request = {
    model: 'claude-sonnet-4-6',
    messages: [
      { role: 'user', content: 'What files are in the current directory?' }
    ],
    tools: ['read', 'grep'],
    permissionMode: 'safe-mode'
  };
  
  const response = await client.completions.create(request);
  console.log(response.content);
}

createCompletion();
```

### Browser Usage

```html
<!DOCTYPE html>
<html>
<head>
  <script src="https://cdn.jsdelivr.net/npm/@orbit/api"></script>
</head>
<body>
  <script>
    const client = new OrbitClient({
      apiKey: 'your-api-key'
    });
    
    client.completions.create({
      model: 'claude-sonnet-4-6',
      messages: [{ role: 'user', content: 'Hello!' }]
    }).then(response => {
      console.log(response.content);
    });
  </script>
</body>
</html>
```

### Streaming

```javascript
const { OrbitClient } = require('@orbit/api');

const client = new OrbitClient({ apiKey: 'your-api-key' });

async function streamCompletion() {
  const request = {
    model: 'claude-sonnet-4-6',
    messages: [{ role: 'user', content: 'Tell me a story' }],
    stream: true
  };
  
  const stream = await client.completions.createStream(request);
  
  for await (const chunk of stream) {
    if (chunk.type === 'content') {
      process.stdout.write(chunk.content);
    }
  }
}

streamCompletion();
```

## Error Handling

### Error Types

| Error Code | Description | HTTP Status |
|------------|-------------|-------------|
| `invalid_request` | Invalid request parameters | 400 |
| `authentication_error` | Authentication failed | 401 |
| `permission_denied` | Insufficient permissions | 403 |
| `not_found` | Resource not found | 404 |
| `rate_limited` | Rate limit exceeded | 429 |
| `server_error` | Internal server error | 500 |
| `service_unavailable` | Service temporarily unavailable | 503 |

### Error Response Format

```json
{
  "error": {
    "type": "invalid_request",
    "message": "The model 'invalid-model' does not exist",
    "code": "model_not_found",
    "param": "model",
    "request_id": "req_123456"
  }
}
```

### Handling Errors

#### Rust

```rust
use orbit_api::{Client, Error, CompletionRequest};

#[tokio::main]
async fn main() {
    let client = Client::from_env().unwrap();
    
    let request = CompletionRequest::builder()
        .model("claude-sonnet-4-6")
        .message("user", "Hello")
        .build()
        .unwrap();
    
    match client.completions().create(request).await {
        Ok(response) => println!("Success: {}", response.content),
        Err(Error::InvalidRequest(e)) => eprintln!("Invalid request: {}", e),
        Err(Error::Authentication(e)) => eprintln!("Auth failed: {}", e),
        Err(e) => eprintln!("Other error: {}", e),
    }
}
```

#### Python

```python
from orbit_api import Client, CompletionRequest, OrbitError

client = Client(api_key="your-api-key")

try:
    request = CompletionRequest(
        model="claude-sonnet-4-6",
        messages=[{"role": "user", "content": "Hello"}]
    )
    response = client.completions.create(request)
    print(response.content)
except OrbitError.InvalidRequest as e:
    print(f"Invalid request: {e}")
except OrbitError.Authentication as e:
    print(f"Authentication failed: {e}")
except OrbitError as e:
    print(f"Error: {e}")
```

#### JavaScript

```javascript
const { OrbitClient, OrbitError } = require('@orbit/api');

const client = new OrbitClient({ apiKey: 'your-api-key' });

try {
  const response = await client.completions.create({
    model: 'claude-sonnet-4-6',
    messages: [{ role: 'user', content: 'Hello' }]
  });
  console.log(response.content);
} catch (error) {
  if (error instanceof OrbitError.InvalidRequest) {
    console.error('Invalid request:', error.message);
  } else if (error instanceof OrbitError.Authentication) {
    console.error('Authentication failed:', error.message);
  } else {
    console.error('Error:', error.message);
  }
}
```

## Rate Limiting

### Limits

| Endpoint | Rate Limit | Burst Limit |
|----------|------------|-------------|
| Completions | 60 requests/minute | 10 requests |
| Tool Execution | 120 requests/minute | 20 requests |
| Sessions | 30 requests/minute | 5 requests |
| Configuration | 10 requests/minute | 2 requests |

### Rate Limit Headers

```http
X-RateLimit-Limit: 60
X-RateLimit-Remaining: 45
X-RateLimit-Reset: 1704067260
X-RateLimit-Retry-After: 30
```

### Handling Rate Limits

#### Automatic Retry

```python
from orbit_api import Client, CompletionRequest
import time

client = Client(api_key="your-api-key")

def create_completion_with_retry(request, max_retries=3):
    for attempt in range(max_retries):
        try:
            return client.completions.create(request)
        except OrbitError.RateLimited as e:
            if attempt < max_retries - 1:
                wait_time = e.retry_after or 30
                print(f"Rate limited. Waiting {wait_time} seconds...")
                time.sleep(wait_time)
            else:
                raise
```

#### Exponential Backoff

```javascript
const { OrbitClient } = require('@orbit/api');

async function createCompletionWithRetry(request, maxRetries = 3) {
  const client = new OrbitClient({ apiKey: 'your-api-key' });
  
  for (let attempt = 0; attempt < maxRetries; attempt++) {
    try {
      return await client.completions.create(request);
    } catch (error) {
      if (error.type === 'rate_limited' && attempt < maxRetries - 1) {
        const waitTime = Math.min(1000 * Math.pow(2, attempt), 30000);
        console.log(`Rate limited. Waiting ${waitTime}ms...`);
        await new Promise(resolve => setTimeout(resolve, waitTime));
      } else {
        throw error;
      }
    }
  }
}
```

## Examples

### Web Application

```javascript
// server.js
const express = require('express');
const { OrbitClient } = require('@orbit/api');

const app = express();
const client = new OrbitClient({
  apiKey: process.env.ORBIT_API_KEY
});

app.post('/api/chat', async (req, res) => {
  try {
    const { message } = req.body;
    
    const response = await client.completions.create({
      model: 'claude-sonnet-4-6',
      messages: [{ role: 'user', content: message }],
      permissionMode: 'safe-mode'
    });
    
    res.json({ response: response.content });
  } catch (error) {
    res.status(500).json({ error: error.message });
  }
});

app.listen(3000, () => {
  console.log('Server running on port 3000');
});
```

### CLI Tool

```python
#!/usr/bin/env python3
# orbit-cli.py

import argparse
import sys
from orbit_api import Client, CompletionRequest

def main():
    parser = argparse.ArgumentParser(description='Orbit CLI Tool')
    parser.add_argument('prompt', help='Prompt to send')
    parser.add_argument('--model', default='claude-sonnet-4-6', help='Model to use')
    parser.add_argument('--stream', action='store_true', help='Stream response')
    
    args = parser.parse_args()
    
    client = Client.from_env()
    
    request = CompletionRequest(
        model=args.model,
        messages=[{'role': 'user', 'content': args.prompt}],
        stream=args.stream
    )
    
    if args.stream:
        for chunk in client.completions.create_stream(request):
            if chunk.type == 'content':
                print(chunk.content, end='', flush=True)
        print()
    else:
        response = client.completions.create(request)
        print(response.content)

if __name__ == '__main__':
    main()
```

### Data Processing Pipeline

```rust
use orbit_api::{Client, CompletionRequest, ToolRequest};
use futures::StreamExt;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?;
    
    // Read all source files
    let files = std::fs::read_dir("src/")?;
    
    for file in files {
        let file = file?;
        let path = file.path();
        
        // Read file content
        let read_request = ToolRequest::builder()
            .tool("read")
            .arg("path", path.to_str().unwrap())
            .build()?;
        
        let result = client.tools().execute(read_request).await?;
        
        // Analyze with AI
        let analysis_request = CompletionRequest::builder()
            .model("claude-sonnet-4-6")
            .message("user", &format!("Analyze this code:\n{}", result.content))
            .build()?;
        
        let analysis = client.completions().create(analysis_request).await?;
        
        println!("File: {:?}", path);
        println!("Analysis: {}\n", analysis.content);
    }
    
    Ok(())
}
```

### Real-time Chat Interface

```html
<!DOCTYPE html>
<html>
<head>
  <title>Orbit Chat</title>
  <script src="https://cdn.jsdelivr.net/npm/@orbit/api"></script>
  <style>
    .chat-container { max-width: 800px; margin: 0 auto; }
    .message { margin: 10px 0; padding: 10px; border-radius: 5px; }
    .user { background: #e3f2fd; text-align: right; }
    .assistant { background: #f3e5f5; }
    .input-container { display: flex; gap: 10px; }
    #message-input { flex: 1; padding: 10px; }
    #send-button { padding: 10px 20px; }
  </style>
</head>
<body>
  <div class="chat-container" id="chat"></div>
  <div class="input-container">
    <input type="text" id="message-input" placeholder="Type your message...">
    <button id="send-button">Send</button>
  </div>

  <script>
    const client = new OrbitClient({ apiKey: 'your-api-key' });
    const chatContainer = document.getElementById('chat');
    const messageInput = document.getElementById('message-input');
    const sendButton = document.getElementById('send-button');

    function addMessage(content, isUser) {
      const messageDiv = document.createElement('div');
      messageDiv.className = `message ${isUser ? 'user' : 'assistant'}`;
      messageDiv.textContent = content;
      chatContainer.appendChild(messageDiv);
      chatContainer.scrollTop = chatContainer.scrollHeight;
    }

    async function sendMessage() {
      const message = messageInput.value.trim();
      if (!message) return;

      addMessage(message, true);
      messageInput.value = '';

      try {
        const request = {
          model: 'claude-sonnet-4-6',
          messages: [{ role: 'user', content: message }],
          stream: true
        };

        let assistantMessage = '';
        const stream = await client.completions.createStream(request);
        
        for await (const chunk of stream) {
          if (chunk.type === 'content') {
            assistantMessage += chunk.content;
            addMessage(assistantMessage, false);
          }
        }
      } catch (error) {
        addMessage(`Error: ${error.message}`, false);
      }
    }

    sendButton.addEventListener('click', sendMessage);
    messageInput.addEventListener('keypress', (e) => {
      if (e.key === 'Enter') sendMessage();
    });
  </script>
</body>
</html>
```

This API reference provides comprehensive coverage of all Orbit API endpoints and integration methods for building powerful applications.
