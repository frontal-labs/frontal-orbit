<picture>
  <source srcset="./assets/banner.jpg" media="(prefers-color-scheme: dark)">
  <img src="./assets/banner.jpg" alt="Frontal Banner">
</picture>

# Frontal Orbit

Orbit is the public Rust implementation of the `orbit` CLI agent harness.
The canonical implementation now lives at the repository root, and the current source of truth for this repository is **frontal-labs/orbit-code**.

> [!IMPORTANT]
> Start with [`USAGE.md`](./USAGE.md) for build, auth, CLI, session, and parity-harness workflows. Make `orbit doctor` your first health check after building, use [`RUST-README.md`](./RUST-README.md) for crate-level details, read [`PARITY.md`](./PARITY.md) for the current Rust-port checkpoint, and see [`docs/container.md`](./docs/container.md) for the container-first workflow.

## Current repository shape

- **`Cargo.toml` + `crates/`** — canonical Rust workspace and the `orbit` CLI binary
- **`USAGE.md`** — task-oriented usage guide for the current product surface
- **`PARITY.md`** — Rust-port parity status and migration notes
- **`ROADMAP.md`** — active roadmap and cleanup backlog
- **`PHILOSOPHY.md`** — project intent and system-design framing

## Quick start

```bash
cargo build --workspace
./target/debug/orbit --help
./target/debug/orbit prompt "summarize this repository"
```

Authenticate with either an API key or the built-in OAuth flow:

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
# or
./target/debug/orbit login
```

Run the workspace test suite:

```bash
cargo test --workspace
```

## Documentation map

- [`USAGE.md`](./USAGE.md) — quick commands, auth, sessions, config, parity harness
- [`RUST-README.md`](./RUST-README.md) — crate map, CLI surface, features, workspace layout
- [`PARITY.md`](./PARITY.md) — parity status for the Rust port
- [`MOCK_PARITY_HARNESS.md`](./MOCK_PARITY_HARNESS.md) — deterministic mock-service harness details
- [`ROADMAP.md`](./ROADMAP.md) — active roadmap and open cleanup work
- [`PHILOSOPHY.md`](./PHILOSOPHY.md) — why the project exists and how it is operated
- [`DEVELOPMENT.md`](./DEVELOPMENT.md) — dev environment and local infra workflow
- [`CHANGELOG.md`](./CHANGELOG.md) — versioned change history
- [`RELEASE.md`](./RELEASE.md) — release notes and rollout checklist
- [`TUI-ENHANCEMENT-PLAN.md`](./TUI-ENHANCEMENT-PLAN.md) — TUI architecture and phased plan
- [`LICENSE.md`](./LICENSE.md) — license and attribution notice
- [`CODE_OF_CONDUCT.md`](./CODE_OF_CONDUCT.md) — community participation standards
