# Orbit Usage

This guide covers the current Rust workspace at the repository root and the `orbit` CLI binary. If you are brand new, make the doctor health check your first run: start `orbit`, then run `/doctor`.

## Quick-start health check

Run this before prompts, sessions, or automation:

```bash
cargo build --workspace
./target/debug/orbit
# first command inside the REPL
/doctor
```

`/doctor` is the built-in setup and preflight diagnostic. Once you have a saved session, you can rerun it with `./target/debug/orbit --resume latest /doctor`.

## Prerequisites

- Rust toolchain with `cargo`
- One of:
  - `ANTHROPIC_API_KEY` for direct API access
  - `ORBIT_CONFIG_HOME` for OAuth-based auth
- Optional: `ANTHROPIC_BASE_URL` when targeting a proxy or local service

## Install / build the workspace

```bash
cargo build --workspace
```

The CLI binary is available at `target/debug/orbit` after a debug build. Make the doctor check above your first post-build step.

## Quick start

### First-run doctor check

```bash
./target/debug/orbit
/doctor
```

### Interactive REPL

```bash
./target/debug/orbit
```

### One-shot prompt

```bash
./target/debug/orbit prompt "summarize this repository"
```

### Shorthand prompt mode

```bash
./target/debug/orbit "explain crates/runtime/src/lib.rs"
```

### JSON output for scripting

```bash
./target/debug/orbit --output-format json prompt "status"
```

## Model and permission controls

```bash
./target/debug/orbit --model sonnet prompt "review this diff"
./target/debug/orbit --permission-mode read-only prompt "summarize Cargo.toml"
./target/debug/orbit --permission-mode workspace-write prompt "update README.md"
./target/debug/orbit --allowedTools read,glob "inspect the runtime crate"
```

Supported permission modes:

- `read-only`
- `workspace-write`
- `danger-full-access`

Model aliases currently supported by the CLI:

- `opus` → `claude-opus-4-6`
- `sonnet` → `claude-sonnet-4-6`
- `haiku` → `claude-haiku-4-5-20251213`

## Authentication

### API key

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

### OAuth

```bash
./target/debug/orbit login
./target/debug/orbit logout
```

## Common operational commands

```bash
./target/debug/orbit status
./target/debug/orbit sandbox
./target/debug/orbit agents
./target/debug/orbit mcp
./target/debug/orbit skills
./target/debug/orbit system-prompt --cwd .. --date 2026-04-04
```

## Session management

REPL turns are persisted under `.orbit/sessions/` in the current workspace.

```bash
./target/debug/orbit --resume latest
./target/debug/orbit --resume latest /status /diff
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
- `commands`
- `compat-harness`
- `mock-anthropic-service`
- `plugins`
- `runtime`
- `cli` (package in `crates/cli/`)
- `telemetry`
- `tools`
