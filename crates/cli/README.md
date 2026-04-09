# Orbit CLI

Main command-line interface for the Orbit ecosystem, providing comprehensive AI-powered development workflows.

## Overview

The Orbit CLI is the primary user interface for interacting with the Orbit AI assistant system. It provides interactive REPL mode, one-shot command execution, comprehensive tool integration, and multi-provider support for AI-powered development workflows.

## Features

- **Interactive REPL**: Full-featured REPL with readline support, tab completion, and command history
- **One-shot Commands**: Direct command execution for automation and scripting
- **Multi-Provider Support**: Anthropic, OpenAI, xAI, Frontal, Bedrock, Azure, and Ollama
- **Rich Output Rendering**: Markdown rendering, syntax highlighting, and ANSI formatting
- **Plugin System**: Extensible architecture with plugin management and installation
- **Permission Management**: Configurable permission modes for security and safety
- **Session Persistence**: Save and resume conversations across sessions
- **Slash Commands**: 50+ built-in commands for system management and automation
- **Tool Integration**: Comprehensive tool system for file operations, web access, and more
- **Compatibility Testing**: Built-in testing and validation tools
- **JSON Output**: Machine-readable output for automation and integration

## Key Components

- **Interactive Mode**: Full-featured REPL with command history, completion, and slash commands
- **Command Processing**: Unified command parsing, argument handling, and execution
- **Output Rendering**: Rich formatting for AI responses, code, and structured data
- **Provider Management**: Multi-provider routing, authentication, and streaming
- **Plugin Integration**: Extensible architecture with install/enable/disable workflows
- **Session Management**: Persistence, resumption, and state management
- **Permission System**: Configurable access controls and safety policies
- **Tool Execution**: Built-in tools for file operations, web access, and system integration

## Dependencies

- `orbit-api` for AI provider integration and HTTP services
- `orbit-commands` for slash command system and registry
- `orbit-compat-harness` for testing and compatibility validation
- `orbit-runtime` for core functionality and session management
- `orbit-plugins` for extensibility and plugin management
- `orbit-tools` for tool integration and execution
- `orbit-providers` for multi-provider AI client support
- `crossterm` for terminal handling and cross-platform support
- `rustyline` for readline functionality and completion
- `pulldown-cmark` for markdown parsing and rendering
- `syntect` for syntax highlighting and code formatting

## Usage

The main binary is named `orbit` and can be used in multiple ways:

### Interactive Mode
```bash
# Start interactive REPL
orbit

# Start with specific model
orbit --model claude-opus-4-6

# Start with specific permissions
orbit --permission-mode workspace-write
```

### One-shot Commands
```bash
# Simple prompt
orbit prompt "explain this codebase"

# With specific provider
orbit --provider anthropic prompt "your question"
orbit --provider openai prompt "your question"
orbit --provider xai prompt "your question"

# With JSON output for automation
orbit --output-format json prompt "summarize crates/cli/src/main.rs"

# With specific permissions
orbit --permission-mode read-only prompt "analyze this file"
```

### Direct Subcommands
```bash
# Check system status
orbit status

# List available agents
orbit agents

# Check MCP servers
orbit mcp

# Run system diagnostics
orbit doctor

# Show sandbox information
orbit sandbox
```

### Provider Selection
```bash
# Force specific provider
orbit --provider anthropic prompt "your question"
orbit --provider openai prompt "your question"
orbit --provider xai prompt "your question"

# With model aliases
orbit --provider anthropic --model opus prompt "complex task"
orbit --provider openai --model gpt-4 prompt "your question"
```

### Session Management
```bash
# Resume latest session
orbit --resume latest

# Resume specific session
orbit --resume session-123

# Resume and run command
orbit --resume latest /status
```

## Configuration

The CLI supports multiple configuration methods:

### Environment Variables
```bash
export ANTHROPIC_API_KEY="sk-ant-..."
export OPENAI_API_KEY="sk-..."
export XAI_API_KEY="xai-..."
export FRONTAL_API_KEY="frontal-..."
export FRONTAL_BASE_URL="https://api.frontal.ai/v1"
```

### Configuration Files
- `~/.orbit.json` - Global user configuration
- `~/.config/orbit/settings.json` - System configuration
- `.orbit.json` - Workspace configuration
- `.orbit/settings.json` - Workspace settings
- `.orbit/settings.local.json` - Local workspace overrides

## Slash Commands

The REPL provides 50+ slash commands organized by category:

### Session Management
- `/help`, `/status`, `/sandbox`, `/cost`, `/resume`, `/session`, `/version`, `/usage`, `/stats`

### Workspace & Git
- `/compact`, `/clear`, `/config`, `/memory`, `/init`, `/diff`, `/commit`, `/pr`, `/issue`, `/export`, `/hooks`, `/files`, `/branch`, `/release-notes`, `/add-dir`

### Discovery & Debugging
- `/mcp`, `/agents`, `/skills`, `/doctor`, `/tasks`, `/context`, `/desktop`, `/ide`

### Automation & Analysis
- `/review`, `/advisor`, `/insights`, `/security-review`, `/subagent`, `/team`, `/telemetry`, `/providers`, `/cron`

### Plugin Management
- `/plugin`, `/plugins`, `/marketplace` - Install, enable, disable, update plugins

## Permission Modes

- `read-only` - Safe mode with read access only
- `workspace-write` - Write access within workspace bounds
- `danger-full-access` - Full system access (use with caution)

## Model Aliases

- `opus` - `claude-opus-4-6`
- `sonnet` - `claude-sonnet-4-6`
- `haiku` - `claude-haiku-4-5-20251213`

## Development

For development from source:

```bash
# Build the workspace
cargo build --workspace

# Run the CLI
cargo run -p orbit-cli -- [args]

# Run tests
cargo test -p orbit-cli
```

## Integration

The CLI integrates with:
- `orbit-runtime` for session management and core functionality
- `orbit-providers` for multi-provider AI client support
- `orbit-tools` for comprehensive tool integration
- `orbit-plugins` for extensibility and customization
- `orbit-memory` for semantic memory and context management
