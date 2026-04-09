# Configuration Guide

This guide covers all configuration options for the Orbit CLI, including environment variables, config files, and runtime settings.

## Configuration Precedence

Settings are applied in the following order (highest to lowest priority):

1. Command-line flags
2. Environment variables
3. Project config file (`.orbit.json`)
4. User config file (`~/.orbit/config.json`)
5. Default values

## Environment Variables

### Required Variables

```bash
# Anthropic API (primary provider)
export ANTHROPIC_API_KEY="sk-ant-..."
export ANTHROPIC_BASE_URL="https://api.anthropic.com"  # optional

# OpenAI-compatible API
export OPENAI_API_KEY="sk-..."
export OPENAI_BASE_URL="https://api.openai.com/v1"  # optional

# xAI API
export XAI_API_KEY="xai-..."
export XAI_BASE_URL="https://api.x.ai/v1"  # optional

# Frontal API gateway
export FRONTAL_API_KEY="frontal-..."
export FRONTAL_BASE_URL="https://api.frontal.ai/v1"
```

### Optional Variables

```bash
# General settings
export ORBIT_LOG_LEVEL="info"  # debug, info, warn, error
export ORBIT_CONFIG_DIR="$HOME/.orbit"
export ORBIT_DATA_DIR="$HOME/.orbit/data"

# Provider selection
export ORBIT_DEFAULT_PROVIDER="anthropic"  # anthropic, openai, xai
export ORBIT_DEFAULT_MODEL="claude-opus-4-6"

# Permission settings
export ORBIT_PERMISSION_MODE="danger-full-access"  # danger-full-access, safe-mode, ask-permissions
export ORBIT_ALLOWED_TOOLS="bash,read,write,edit,grep"

# Session settings
export ORBIT_SESSION_DIR="$HOME/.orbit/sessions"
export ORBIT_AUTO_SAVE_SESSIONS="true"
export ORBIT_MAX_SESSIONS="100"

# MCP settings
export ORBIT_MCP_SERVERS_DIR="$HOME/.orbit/mcp-servers"
export ORBIT_MCP_TIMEOUT="30"
```

## Config File Format

The `.orbit.json` config file uses JSON format with the following structure:

```json
{
  "version": "1.0",
  "providers": {
    "anthropic": {
      "api_key": "${ANTHROPIC_API_KEY}",
      "base_url": "https://api.anthropic.com",
      "default_model": "claude-opus-4-6"
    },
    "openai": {
      "api_key": "${OPENAI_API_KEY}",
      "base_url": "https://api.openai.com/v1",
      "default_model": "gpt-4"
    },
    "xai": {
      "api_key": "${XAI_API_KEY}",
      "base_url": "https://api.x.ai/v1",
      "default_model": "grok-beta"
    }
  },
  "runtime": {
    "default_provider": "anthropic",
    "default_model": "claude-opus-4-6",
    "permission_mode": "danger-full-access",
    "allowed_tools": ["bash", "read", "write", "edit", "grep", "glob", "web_search", "web_fetch"],
    "max_tokens": 4096,
    "temperature": 0.7,
    "timeout": 300
  },
  "session": {
    "auto_save": true,
    "max_sessions": 100,
    "session_dir": "${ORBIT_SESSION_DIR}",
    "resume_last_session": false
  },
  "mcp": {
    "servers_dir": "${ORBIT_MCP_SERVERS_DIR}",
    "timeout": 30,
    "auto_start": [],
    "enabled": true
  },
  "plugins": {
    "plugins_dir": "${ORBIT_CONFIG_DIR}/plugins",
    "auto_load": [],
    "enabled": true
  },
  "ui": {
    "output_format": "text",  # text, json
    "color_output": true,
    "show_thinking": false,
    "show_tool_calls": true,
    "stream_output": true
  },
  "telemetry": {
    "enabled": false,
    "endpoint": "",
    "sample_rate": 0.1
  }
}
```

## Command-Line Flags

### Global Flags

```bash
orbit [OPTIONS] [COMMAND]

Options:
  -m, --model <MODEL>                 AI model to use
  -p, --provider <PROVIDER>           AI provider (anthropic, openai, xai)
  -o, --output-format <FORMAT>        Output format [text|json]
  -P, --permission-mode <MODE>        Permission mode
      --dangerously-skip-permissions  Skip all permission checks
      --allowed-tools <TOOLS>         Comma-separated list of allowed tools
      --resume <SESSION>              Resume session
      --config <FILE>                 Config file path
      --version, -V                   Show version
      --help, -h                      Show help
```

### Model Selection

```bash
# Full model names
orbit --model claude-opus-4-6
orbit --model claude-sonnet-4-6
orbit --model claude-haiku-4-5-20251213

# Model aliases
orbit --model opus      # claude-opus-4-6
orbit --model sonnet     # claude-sonnet-4-6
orbit --model haiku      # claude-haiku-4-5-20251213

# OpenAI models
orbit --provider openai --model gpt-4
orbit --provider openai --model gpt-4-turbo

# xAI models
orbit --provider xai --model grok-beta
```

## Permission Modes

### danger-full-access
- All tools are allowed without confirmation
- Recommended for trusted environments and automation
- Default mode for CLI usage

### safe-mode
- Only safe tools are allowed (read, grep, web_search, web_fetch)
- Destructive tools require explicit approval
- Recommended for untrusted codebases

### ask-permissions
- Prompt for approval on every tool use
- Most secure but interactive
- Recommended for learning and debugging

## Tool Configuration

### Built-in Tools

| Tool | Description | Safe Mode | Permissions |
|------|-------------|-----------|-------------|
| bash | Execute shell commands | No | Full system access |
| read | Read file contents | Yes | File read access |
| write | Write/create files | No | File write access |
| edit | Edit existing files | No | File write access |
| grep | Search file contents | Yes | File read access |
| glob | Search file patterns | Yes | File read access |
| web_search | Search the web | Yes | Web access |
| web_fetch | Fetch web content | Yes | Web access |
| agent | Launch sub-agents | No | Full access |

### Tool Restrictions

```bash
# Allow specific tools only
orbit --allowed-tools "read,grep,web_search"

# Disable dangerous tools
orbit --allowed-tools "read,write,edit,grep,glob,web_search,web_fetch"

# Custom tool restrictions in config
{
  "runtime": {
    "allowed_tools": ["read", "grep", "web_search"],
    "tool_restrictions": {
      "bash": {
        "allowed_commands": ["ls", "cat", "grep"],
        "blocked_commands": ["rm", "sudo", "chmod"]
      }
    }
  }
}
```

## Session Configuration

### Session Persistence

```bash
# Enable session persistence
orbit --session auto-save

# Resume last session
orbit --resume latest

# Resume specific session
orbit --resume session-123.jsonl

# Export session
orbit session export --format json --output session.json
```

### Session Settings

```json
{
  "session": {
    "auto_save": true,
    "max_sessions": 100,
    "session_dir": "~/.orbit/sessions",
    "compression": "gzip",
    "encryption": false,
    "metadata": {
      "save_system_info": true,
      "save_environment": false,
      "save_git_state": true
    }
  }
}
```

## MCP Configuration

### Server Configuration

```json
{
  "mcp": {
    "servers": {
      "filesystem": {
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/allowed/files"],
        "env": {
          "NODE_ENV": "production"
        },
        "timeout": 30,
        "auto_start": true
      },
      "github": {
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-github"],
        "env": {
          "GITHUB_PERSONAL_ACCESS_TOKEN": "${GITHUB_TOKEN}"
        },
        "timeout": 60,
        "auto_start": false
      }
    }
  }
}
```

### MCP Settings

```bash
# List available MCP servers
orbit mcp list

# Start specific server
orbit mcp start filesystem

# Configure MCP server
orbit mcp config filesystem --timeout 60 --auto-start
```

## Plugin Configuration

### Plugin Discovery

```json
{
  "plugins": {
    "plugins_dir": "~/.orbit/plugins",
    "auto_load": ["plugin-name"],
    "registry_url": "https://plugins.orbit.ai",
    "update_check_interval": "24h",
    "trusted_sources": ["https://github.com", "https://plugins.orbit.ai"]
  }
}
```

### Plugin Settings

```bash
# Install plugin
orbit plugin install /path/to/plugin

# Enable/disable plugin
orbit plugin enable plugin-name
orbit plugin disable plugin-name

# List plugins
orbit plugin list

# Update plugin
orbit plugin update plugin-name
```

## UI Configuration

### Output Formatting

```json
{
  "ui": {
    "output_format": "text",
    "color_output": true,
    "show_thinking": false,
    "show_tool_calls": true,
    "stream_output": true,
    "terminal_width": 80,
    "markdown_rendering": true,
    "syntax_highlighting": true
  }
}
```

### Display Options

```bash
# JSON output
orbit --output-format json prompt "summarize this file"

# Disable colors
orbit --color=false prompt "explain this"

# Show tool calls
orbit --show-tool-calls prompt "list files"
```

## Telemetry Configuration

### Usage Tracking

```json
{
  "telemetry": {
    "enabled": false,
    "endpoint": "https://telemetry.orbit.ai/v1/events",
    "sample_rate": 0.1,
    "batch_size": 10,
    "flush_interval": "60s",
    "events": [
      "session_start",
      "session_end",
      "tool_use",
      "error",
      "command_completion"
    ]
  }
}
```

### Privacy Settings

```bash
# Disable telemetry
export ORBIT_TELEMETRY_ENABLED=false

# Set sample rate
export ORBIT_TELEMETRY_SAMPLE_RATE=0.1

# Custom endpoint
export ORBIT_TELEMETRY_ENDPOINT="https://my-telemetry.example.com"
```

## Advanced Configuration

### Custom Prompts

```json
{
  "prompts": {
    "system": "You are Orbit, a helpful AI assistant...",
    "user_context": "Current working directory: {cwd}\nGit branch: {branch}",
    "tool_use_template": "Using tool: {tool} with args: {args}"
  }
}
```

### Performance Tuning

```json
{
  "performance": {
    "max_concurrent_requests": 5,
    "request_timeout": 300,
    "retry_attempts": 3,
    "retry_delay": "1s",
    "cache_size": "100MB",
    "compression": true
  }
}
```

### Security Settings

```json
{
  "security": {
    "encrypt_sessions": false,
    "encrypt_config": false,
    "api_key_rotation": false,
    "audit_logging": false,
    "sandbox_mode": false
  }
}
```

## Configuration Validation

### Check Configuration

```bash
# Validate current configuration
orbit config validate

# Show effective configuration
orbit config show

# Show specific section
orbit config show providers
orbit config show runtime
```

### Common Issues

1. **API key not found**: Ensure environment variables are set
2. **Config file not found**: Check file path and permissions
3. **Invalid JSON**: Validate JSON syntax
4. **Permission denied**: Check file permissions for config directory

## Migration Guide

### From Environment Variables

If you're currently using only environment variables, you can migrate to a config file:

```bash
# Generate config from current environment
orbit config init --from-env

# This creates .orbit.json with current settings
```

### Version Upgrades

When upgrading Orbit versions:

1. Backup current config: `cp .orbit.json .orbit.json.backup`
2. Run config validation: `orbit config validate`
3. Update deprecated settings as needed
4. Test with `--dry-run` flag before applying changes
