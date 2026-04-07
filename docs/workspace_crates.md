# Workspace Crates

The workspace is defined in `Cargo.toml` with `members = ["crates/*"]`.

## Package map

| Path | Package | Purpose |
|---|---|---|
| `crates/orbit-cli` | `orbit-cli` | Main CLI binary package (`orbit`) and REPL entry point. |
| `crates/runtime` | `orbit-runtime` | Runtime/session model, config loading, permissions, MCP, orchestration utilities. |
| `crates/api` | `orbit-api` | Provider-facing client and request/response types. |
| `crates/tools` | `orbit-tools` | Tool registry and execution surfaces used by runtime/CLI. |
| `crates/commands` | `orbit-commands` | Slash command parsing/handling and related command metadata. |
| `crates/plugins` | `orbit-plugins` | Plugin manifests, lifecycle hooks, and plugin manager support. |
| `crates/telemetry` | `orbit-telemetry` | Telemetry models and session/event tracking types. |
| `crates/compat-harness` | `orbit-compat-harness` | Compatibility/parity extraction support. |
| `crates/mock-anthropic-service` | `orbit-mock-anthropic-service` | Deterministic local mock provider service for tests/harness runs. |
| `crates/agents` | `orbit-agents` | Workspace package (currently minimal scaffold). |
| `crates/api-client` | `orbit-api-client` | Workspace package (currently minimal scaffold). |
| `crates/brain` | `orbit-brain` | Workspace package (currently minimal scaffold). |
| `crates/integratins` | `orbit-integratins` | Workspace package (currently minimal scaffold). |
| `crates/memory` | `orbit-memory` | Workspace package (currently minimal scaffold). |
| `crates/server` | `orbit-server` | Workspace package (currently minimal scaffold). |

## Internal dependency flow (high level)

- `orbit-cli` depends on: `orbit-api`, `orbit-commands`, `orbit-compat-harness`, `orbit-runtime`, `orbit-plugins`, `orbit-tools`.
- `orbit-tools` depends on: `orbit-api`, `orbit-commands`, `orbit-plugins`, `orbit-runtime`.
- `orbit-api` depends on: `orbit-runtime`, `orbit-telemetry`.
- `orbit-runtime` depends on: `orbit-plugins`, `orbit-telemetry`.
- `orbit-compat-harness` depends on: `orbit-commands`, `orbit-tools`, `orbit-runtime`.

## Lints and workspace policy

Workspace-wide Rust and Clippy lint settings are configured in the root `Cargo.toml` and shared by member crates via `[lints] workspace = true`.
