# AGENTS.md

This file provides comprehensive guidance to AI assistants (including Antigravity, Claude Code, and Orbit itself) when working with code in this repository. Frontal Orbit is a high-performance Rust rewrite of the Orbit agent harness, designed for safety, speed, and deep tool integration.

## Stack Overview
- **Core Engine**: Rust 2021 (Workspace of 20+ crates).
- **Primary Frameworks**: Axum (server/webhooks), Serde (JSON), Reqwest (API clients), Tower/Tracing (observability).
- **Secondary Languages**: Python (parity scripts), JavaScript/TS (IDE/Slack extensions), Ruby (Homebrew), Terraform (infra).
- **Design Patterns**: Content-block-based messaging, stateful sessions with compaction, rule-based permission escalation, and lifecycle hooks.

## Repository Architecture
The workspace is split into specialized crates to ensure maintainability and separation of concerns:

- `crates/api`: Public API facade re-exporting provider/model generic types.
- `crates/runtime`: The heart of Orbit. Contains the `ConversationRuntime`, `Session` management, `PermissionPolicy`, and `ConfigLoader`.
- `crates/tools`: Implementation of all built-in tools (Bash, Read, Write, Web, etc.).
- `crates/cli`: Entry point for the `orbit` binary, REPL logic, and CLI-specific rendering.
- `crates/commands`: Shared registry for slash commands (e.g., `/compact`, `/status`).
- `crates/providers`: Client implementations for Anthropic, OpenAI, xAI, Azure, etc.
- `crates/memory`: Semantic memory, knowledge graphs, and vector storage.
- `crates/plugins`: Marketplace and registry logic for external capability extensions.
- `crates/sandbox`: Filesystem and process isolation primitives.
- `crates/mock-anthropic-service`: A deterministic local mock for testing parity without API costs.

## The Conversation Runtime
Understanding the `ConversationRuntime` in `crates/runtime/src/conversation.rs` is critical for extending agent behavior.

### The Turn Loop
A single "turn" (triggered by user input) follows this lifecycle:
1. **Pre-turn Hooks**: Lifecycle hooks execute to modify input or set constraints.
2. **Model Request**: Assembles the system prompt + conversation history into an `ApiRequest`.
3. **Iteration Loop**: If the assistant returns `ToolUse` blocks:
   - **Permission Check**: `PermissionPolicy` evaluates if the current mode allows the tool + input.
   - **Hook Interception**: `PreToolUse` hooks can modify parameters or cancel the tool.
   - **Execution**: `ToolExecutor` runs the tool.
   - **Post-hook**: `PostToolUse` logic processes the result.
   - **Feedback**: Results are added to history, and the model is called again (up to `max_iterations`).
4. **Auto-Compaction**: If token usage exceeds the threshold, the session is compacted.
5. **Post-turn Hooks**: Final stats and telemetry are recorded.

## Security & Permission System
Orbit uses a hierarchical permission system defined in `crates/runtime/src/permissions.rs`.

### Permission Modes
- `ReadOnly`: Only tools with `PermissionMode::ReadOnly` are allowed (e.g., `read_file`, `glob_search`).
- `WorkspaceWrite`: Allows file modifications in the workspace.
- `DangerFullAccess`: Required for `bash` and `Agent` handoffs.
- `Prompt`: Forces a user prompt for *every* tool call regardless of its default level.

### Rule-Based Overrides
Users can define specific rules in configuration:
- `deny("bash(rm -rf:*)")`: Blocks dangerous commands globally.
- `allow("bash(git:*)")`: Safely allows git operations even in restricted modes.
- `ask("write_file(Cargo.toml)")`: Forces approval for sensitive files.

## The Toolbelt
All tools are defined in `crates/tools/src/lib.rs`. Key tools include:
- `bash`: Runs shell commands (sandboxed by default).
- `edit_file`: Performs precise string replacement to minimize diff sizes.
- `grep_search` / `glob_search`: High-speed workspace discovery.
- `WebSearch` / `WebFetch`: Real-time information gathering.
- `Agent`: Handoff to specialized sub-agents.
- `Skill`: Execution of local "skill" scripts (custom JS/Shell prompts).

## Slash Commands (REPL Shortcuts)
Slash commands are the primary way to interact with the runtime's state. See `crates/commands/src/lib.rs` for the full registry.

- **Status & Health**: `/doctor`, `/status`, `/usage`, `/cost`, `/sandbox`.
- **Context Control**: `/compact`, `/clear`, `/resume`, `/pin`, `/files`.
- **Intelligence**: `/ultraplan` (deep reasoning), `/review` (code audit), `/bughunter`, `/advisor`.
- **Extension**: `/plugin`, `/skills`, `/mcp`, `/subagent`.
- **Git/Ops**: `/diff`, `/commit`, `/pr`, `/issue`, `/log`, `/release-notes`.

## Verification & Development
When contributing or debugging, follow these workflows:

1. **Health Check**: Run `orbit doctor` or `/doctor` early and often.
2. **Rust Quality**:
   - `cargo fmt`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test --workspace`
3. **Parity Testing**: Use `./scripts/run_mock_parity_harness.sh` to ensure model response parsing and tool execution remain deterministic across providers.
4. **Mocking**: Run `cargo run -p mock-anthropic-service` for locally hosted, cost-free development.

## Working Agreement
- **Commits**: Strictly follow [Conventional Commits](https://www.conventionalcommits.org/).
- **Changes**: Prefer small, atomic PRs. Each crate has its own `README.md` explaining its specific invariant rules.
- **Config**: Shared defaults belong in `config/project.json`. Local overrides go in `.orbit/settings.local.json`.
- **Documentation**: Keep `AGENTS.md` and crate-level READMEs up-to-date with architectural changes.
- **Parity**: Any new tool or slash command must be added to the parity harness manifest if it consumes model input.
