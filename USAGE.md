# Orbit Usage

This guide covers the current Rust workspace at the repository root and the `orbit` CLI binary. If you are brand new, make the doctor health check your first run: start `orbit`, then run `/doctor`.

## Quick-start health check

Run this before prompts, sessions, or automation:

```bash
brew install --HEAD ./homebrew/orbit.rb
orbit
# first command inside the REPL
/doctor
```

`/doctor` is the built-in setup and preflight diagnostic. Once you have a saved session, you can rerun it with `orbit --resume latest /doctor`.

## Prerequisites

- Homebrew for CLI installation, or a Rust toolchain with `cargo` for source builds
- One of:
  - `ANTHROPIC_API_KEY` for direct API access
  - `OPENAI_API_KEY` for OpenAI
  - `XAI_API_KEY` for xAI
  - `FRONTAL_API_KEY` for Frontal's OpenAI-compatible gateway
  - `BEDROCK_API_KEY` for Bedrock-compatible gateways
  - `AZURE_OPENAI_API_KEY` for Azure OpenAI-compatible gateways
  - or local `OLLAMA_BASE_URL` (defaults to `http://localhost:11434`)
- Optional: `ANTHROPIC_BASE_URL` when targeting a proxy or local service
- Optional: `FRONTAL_BASE_URL` when targeting a custom Frontal gateway URL

## Configuration

Orbit uses a centralized configuration system that allows you to customize behavior without modifying code.

### Configuration File

The main configuration file is `config/project.json`. It contains:

- **Project settings**: name, version, description
- **Runtime configuration**: default AI provider, timeouts, concurrency limits
- **Feature flags**: telemetry, plugins, caching, metrics, tracing
- **UI settings**: theme, colors, progress bars
- **Service configuration**: database, Redis, memory settings
- **Sandbox settings**: Docker configuration, execution limits

### Configuration File Locations

The system looks for `project.json` in this order:

1. `$ORBIT_CONFIG_HOME/project.json` - Custom config directory
2. `$ORBIT_HOME/project.json` - Orbit home directory  
3. `~/.orbit/project.json` - User's home directory
4. `config/project.json` - Project-local configuration

### Common Configuration Options

```json
{
  "runtime": {
    "default_provider": "anthropic",
    "max_concurrent_requests": 10,
    "request_timeout_seconds": 30,
    "permission_mode": "permissive",
    "log_level": "info"
  },
  "features": {
    "enable_telemetry": true,
    "enable_plugins": true,
    "enable_caching": true,
    "enable_metrics": true,
    "enable_tracing": false
  },
  "ui": {
    "theme": "default",
    "enable_colors": true,
    "show_progress_bars": true
  }
}
```

### Checking Your Configuration

Run the doctor command to see your current configuration:

```bash
orbit
/doctor
```

The doctor report now includes a "Core Configuration" section showing:
- Default provider and settings
- Feature flags status
- UI preferences
- Service configurations

### Environment Variables

Environment variables take precedence over configuration file settings:

```bash
# Override default provider
export ORBIT_DEFAULT_PROVIDER="openai"

# Override log level
export ORBIT_LOG_LEVEL="debug"

# Override permission mode
export ORBIT_PERMISSION_MODE="restricted"
```

## Install / build the workspace

```bash
# Install the CLI with Homebrew
brew install --HEAD ./homebrew/orbit.rb

# Or build from source
cargo build --workspace
```

The installed CLI is available as `orbit`. If you build from source instead, the debug binary is available at `target/debug/orbit`. Make the doctor check above your first post-build step.

## Quick start

### First-run doctor check

```bash
orbit
/doctor
```

### Interactive REPL

```bash
orbit
```

### One-shot prompt

```bash
orbit prompt "summarize this repository"
```

### Shorthand prompt mode

```bash
orbit "explain crates/runtime/src/lib.rs"
```

### JSON output for scripting

```bash
orbit --output-format json prompt "status"
```

## Model and permission controls

```bash
orbit --model sonnet prompt "review this diff"
orbit --permission-mode read-only prompt "summarize Cargo.toml"
orbit --permission-mode workspace-write prompt "update README.md"
orbit --allowedTools read,glob "inspect the runtime crate"
```

Supported permission modes:

- `read-only`
- `workspace-write`
- `danger-full-access`

Model aliases currently supported by the CLI:

- `opus` → `claude-opus-4-6`
- `sonnet` → `claude-sonnet-4-6`
- `haiku` → `claude-haiku-4-5-20251213`

## Provider Selection

Use the `--provider` flag to force a specific AI provider:

```bash
# Force Anthropic provider
orbit --provider anthropic prompt "your question"

# Force OpenAI provider
orbit --provider openai prompt "your question"

# Force xAI provider
orbit --provider xai prompt "your question"

# Combine with model aliases
orbit --provider anthropic --model opus prompt "complex task"
orbit --provider openai --model gpt-4 prompt "your question"
```

Supported providers:
- `anthropic` - Claude models via Anthropic API
- `openai` - GPT models via OpenAI API
- `xai` - Grok models via xAI API

## Authentication

### API key

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
# or
export FRONTAL_API_KEY="frontal-..."
```

## Common operational commands

```bash
orbit status
orbit sandbox
orbit agents
orbit mcp
orbit skills
orbit system-prompt --cwd .. --date 2026-04-04
```

## Session management

REPL turns are persisted under `.orbit/sessions/` in the current workspace.

```bash
orbit --resume latest
orbit --resume latest /status /diff
```

Useful interactive commands include `/help`, `/status`, `/cost`, `/config`, `/session`, `/model`, `/permissions`, and `/export`.

## Config file resolution order

Runtime config is loaded in this order, with later entries overriding earlier ones:

1. `~/.orbit.json`
2. `~/.config/orbit/settings.json`
3. `<repo>/.orbit.json`
4. `<repo>/.orbit/settings.json`
5. `<repo>/.orbit/settings.local.json`

## Mock parity harness

The workspace includes a deterministic Anthropic-compatible mock service and parity harness.

```bash
./scripts/run_mock_parity_harness.sh
```

Manual mock service startup:

```bash
cargo run -p mock-anthropic-service -- --bind 127.0.0.1:0
```

## Verification

```bash
cargo test --workspace
```

## Workspace overview

Current Rust crates:

- `api`
- `providers`
- `commands`
- `compat-harness`
- `mock-anthropic-service`
- `plugins`
- `runtime`
- `cli` (package in `crates/cli/`)
- `telemetry`
- `tools`
