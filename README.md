<picture>
  <source srcset="./assets/banner.jpg" media="(prefers-color-scheme: dark)">
  <img src="./assets/banner.jpg" alt="Frontal Banner">
</picture>

# Frontal Orbit

A high-performance Rust rewrite of the Orbit CLI agent harness. Built for speed, safety, and native tool execution.

For a task-oriented guide with copy/paste examples, see [`./USAGE.md`](./USAGE.md).

## Quick Start

```bash
# Install the CLI with Homebrew
brew install --HEAD ./homebrew/orbit.rb

# Inspect available commands
orbit --help

# Run the interactive REPL
orbit --model claude-opus-4-6

# One-shot prompt
orbit prompt "explain this codebase"

# JSON output for automation
orbit --output-format json prompt "summarize crates/cli/src/main.rs"
```

If you are developing from source instead of installing the CLI, use `cargo build --workspace` and run `cargo run -p orbit-cli -- ...`.

## Configuration

### API Credentials

Set your API credentials:

```bash
export ORBIT_API_KEY="sk-ant-..."
# Or use Frontal's OpenAI-compatible API gateway
export FRONTAL_API_KEY="frontal-..."
export FRONTAL_BASE_URL="https://api.frontal.ai/v1"
# Or use an Anthropic proxy
export ORBIT_BASE_URL="https://your-proxy.com"
```

### Core Configuration

Orbit now uses a centralized configuration system located at `config/project.json`. This file contains:

- **Project settings**: name, version, description
- **Runtime configuration**: default AI provider, timeouts, concurrency limits
- **Feature flags**: telemetry, plugins, caching, metrics, tracing
- **UI settings**: theme, colors, progress bars
- **Service configuration**: database, Redis, memory settings
- **Sandbox settings**: Docker configuration, execution limits

#### Configuration File Locations

The system looks for `project.json` in this order:

1. `$ORBIT_CONFIG_HOME/project.json` - Custom config directory
2. `$ORBIT_HOME/project.json` - Orbit home directory  
3. `~/.orbit/project.json` - User's home directory
4. `config/project.json` - Project-local configuration

#### Example Configuration

```json
{
  "project": {
    "name": "Orbit",
    "version": "0.1.0",
    "description": "AI-powered development environment and CLI tool"
  },
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
    "enable_metrics": true
  }
}
```

#### Using Configuration in Code

```rust
use orbit_core::config::ProjectConfig;

// Load configuration with fallback to defaults
let config = ProjectConfig::load_or_default();

// Access configuration values
println!("Default provider: {}", config.runtime.default_provider);
println!("Telemetry enabled: {}", config.features.enable_telemetry);
```

For more details, see the [Configuration guide](./CONFIGURATION.md).

## Mock parity harness

The workspace now includes a deterministic Anthropic-compatible mock service and a clean-environment CLI harness for end-to-end parity checks.

```bash

# Run the scripted clean-environment harness
./scripts/run_mock_parity_harness.sh

# Or start the mock service manually for ad hoc CLI runs
cargo run -p mock-anthropic-service -- --bind 127.0.0.1:0
```

Harness coverage:

- `streaming_text`
- `read_file_roundtrip`
- `grep_chunk_assembly`
- `write_file_allowed`
- `write_file_denied`
- `multi_tool_turn_roundtrip`
- `bash_stdout_roundtrip`
- `bash_permission_prompt_approved`
- `bash_permission_prompt_denied`
- `plugin_tool_roundtrip`

Primary artifacts:

- `crates/mock-anthropic-service/` — reusable mock Anthropic-compatible service
- `crates/cli/tests/mock_parity_harness.rs` — clean-env CLI harness
- `scripts/run_mock_parity_harness.sh` — reproducible wrapper
- `scripts/run_mock_parity_diff.py` — scenario checklist + PARITY mapping runner
- `mock_parity_scenarios.json` — scenario-to-PARITY manifest

## Features

| Feature | Status |
|---------|--------|
| Anthropic / OpenAI-compatible provider flows + streaming (OpenAI, xAI, Frontal, Bedrock, Azure) |  |
| Environment-variable auth (Anthropic/OpenAI/xAI/Frontal/Bedrock/Azure/Ollama) |  |
| Interactive REPL (rustyline) |  |
| Tool system (bash, read, write, edit, grep, glob) |  |
| Web tools (search, fetch) |  |
| Sub-agent / agent surfaces |  |
| Todo tracking |  |
| Notebook editing |  |
| ORBIT.md / project memory |  |
| Config file hierarchy (`.orbit.json` + merged config sections) |  |
| Permission system |  |
| MCP server lifecycle + inspection |  |
| Session persistence + resume |  |
| Cost / usage / stats surfaces |  |
| Git integration |  |
| Markdown terminal rendering (ANSI) |  |
| Model aliases (opus/sonnet/haiku) |  |
| Provider flag support (anthropic, openai, xai) |  |
| Direct CLI subcommands (`status`, `sandbox`, `agents`, `mcp`, `skills`, `doctor`) |  |
| Slash commands (including `/skills`, `/agents`, `/mcp`, `/doctor`, `/plugin`, `/subagent`) |  |
| Hooks (`/hooks`, config-backed lifecycle hooks) |  |
| Plugin management surfaces |  |
| Skills inventory / install surfaces |  |
| Machine-readable JSON output across core CLI surfaces |  |
| GitHub integration (PRs, issues, check runs) |  |
| IDE integration (VS Code, Cursor, Windsurf, Antigravity) |  |
| Embedding and semantic memory |  |
| Event system and messaging |  |
| Training and style adaptation |  |
| Webhook processing |  |
| Repository lifecycle management |  |
| Sandbox and isolation |  |
| Observability and monitoring |  |
| Orchestration and workflow management |  |

## Model Aliases

Short names resolve to the latest model versions:

| Alias | Resolves To |
|-------|------------|
| `opus` | `claude-opus-4-6` |
| `sonnet` | `claude-sonnet-4-6` |
| `haiku` | `claude-haiku-4-5-20251213` |

## CLI Flags and Commands

Representative current surface:

```text
orbit [OPTIONS] [COMMAND]

Flags:
  --model MODEL
  --output-format text|json
  --permission-mode MODE
  --dangerously-skip-permissions
  --allowedTools TOOLS
  --resume [SESSION.jsonl|session-id|latest]
  --version, -V

Top-level commands:
  prompt <text>
  help
  version
  status
  sandbox
  dump-manifests
  bootstrap-plan
  agents
  mcp
  skills
  system-prompt
  init
```

The command surface is moving quickly. For the canonical live help text, run:

```bash
cargo run -p orbit-cli -- --help
```

## Slash Commands (REPL)

Tab completion expands slash commands, model aliases, permission modes, and recent session IDs.

The REPL now exposes a much broader surface than the original minimal shell:

- session / visibility: `/help`, `/status`, `/sandbox`, `/cost`, `/resume`, `/session`, `/version`, `/usage`, `/stats`
- workspace / git: `/compact`, `/clear`, `/config`, `/memory`, `/init`, `/diff`, `/commit`, `/pr`, `/issue`, `/export`, `/hooks`, `/files`, `/branch`, `/release-notes`, `/add-dir`
- discovery / debugging: `/mcp`, `/agents`, `/skills`, `/doctor`, `/tasks`, `/context`, `/desktop`, `/ide`
- automation / analysis: `/review`, `/advisor`, `/insights`, `/security-review`, `/subagent`, `/team`, `/telemetry`, `/providers`, `/cron`, and more
- plugin management: `/plugin` (with aliases `/plugins`, `/marketplace`)

Notable orbit-first surfaces now available directly in slash form:
- `/skills [list|install <path>|help]`
- `/agents [list|help]`
- `/mcp [list|show <server>|help]`
- `/doctor`
- `/plugin [list|install <path>|enable <name>|disable <name>|uninstall <id>|update <id>]`
- `/subagent [list|steer <target> <msg>|kill <id>]`

See [`./USAGE.md`](./USAGE.md) for usage examples and run `cargo run -p orbit-cli -- --help` for the live canonical command list.

## Workspace Layout

```text
.
|   Cargo.toml              # Workspace root
|   Cargo.lock
|   crates/
|   |   api/                # Public API facade re-exporting provider/model APIs
|   |   agents/             # Agent management and coordination
|   |   cli/                # Main CLI binary (`orbit`)
|   |   commands/           # Shared slash-command registry + help rendering
|   |   compat-harness/     # TS manifest extraction harness
|   |   core/               # Shared core capabilities and foundational types
|   |   embeddings/         # Embedding primitives for semantic memory
|   |   events/             # Event system and messaging infrastructure
|   |   github/             # GitHub API client integration
|   |   integrations/       # MCP interoperability and IDE integration
|   |   memory/             # Semantic memory and knowledge graph utilities
|   |   mock-anthropic-service/ # Deterministic local Anthropic-compatible mock
|   |   observability/      # Structured observability primitives
|   |   orchestrator/       # Workflow management and resource allocation
|   |   plugins/            # Plugin metadata, manager, install/enable/disable surfaces
|   |   providers/          # Provider clients and routing (Anthropic/OpenAI/xAI/Frontal/...)
|   |   repo/               # Repository lifecycle management
|   |   runtime/            # Session, config, permissions, MCP, prompts, auth/runtime loop
|   |   sandbox/            # Sandboxing and isolation capabilities
|   |   server/             # Hosted control-plane services
|   |   telemetry/          # Session tracing and usage telemetry types
|   |   tools/              # Built-in tools, skill resolution, tool search, agent runtime surfaces
|   |   training/           # Style-learning and adaptation system
|   |   webhooks/           # Webhook receiving and processing
```

### Crate Responsibilities

- **api** - public API facade crate that re-exports provider and model APIs
- **agents** - agent management, coordination, and sub-agent orchestration
- **cli** - REPL, one-shot prompt, direct CLI subcommands, streaming display, tool call rendering, CLI argument parsing
- **commands** - slash command definitions, parsing, help text generation, JSON/text command rendering
- **compat-harness** - extracts tool/prompt manifests from upstream TS source
- **core** - shared types, traits, utilities, error handling, and foundational components
- **embeddings** - embedding primitives, vector operations, and semantic similarity
- **events** - event types, handlers, bus implementation, and messaging infrastructure
- **github** - GitHub API client for PRs, issues, check runs, and repository operations
- **integrations** - MCP server management, IDE integration, and external service bridges
- **memory** - semantic memory, knowledge graphs, and persistent storage abstractions
- **mock-anthropic-service** - deterministic `/v1/messages` mock for CLI parity tests and local harness runs
- **observability** - error reporting, structured logging, and agent-level observability
- **orchestrator** - work item routing, execution planning, lane assignment, and resource management
- **plugins** - plugin metadata, install/enable/disable/update flows, plugin tool definitions, hook integration surfaces
- **providers** - provider clients, routing, SSE streaming, request/response types, env-var auth, request-size/context-window preflight
- **repo** - repository lifecycle, checkout management, and source tree preparation
- **runtime** - `ConversationRuntime`, config loading, session persistence, permission policy, MCP client lifecycle, system prompt assembly, usage tracking
- **sandbox** - filesystem isolation, namespace restrictions, and security policies
- **server** - hosted task APIs, event streaming, Docker lane execution, and policy-driven recovery
- **telemetry** - session trace events, usage analytics, and monitoring infrastructure
- **tools** - tool specs + execution: Bash, ReadFile, WriteFile, EditFile, GlobSearch, GrepSearch, WebSearch, WebFetch, Agent, TodoWrite, NotebookEdit, Skill, ToolSearch, and runtime-facing tool discovery
- **training** - style learning, dataset building, profile training, and code adaptation
- **webhooks** - webhook authentication, event processing, and HTTP endpoint management

## Stats

- **~89K lines** of Rust
- **21 crates** in workspace
- **Binary name:** `orbit`
- **Default model:** `claude-opus-4-6`
- **Default permissions:** `danger-full-access`

## Attribution

Originally based on [ultraworkers/claw-code](https://github.com/ultraworkers/claw-code), a high-performance Rust rewrite of AI agent tooling. This fork has been significantly modified and rebranded as "Frontal Orbit" by Frontal Labs.

## License

See repository root.
