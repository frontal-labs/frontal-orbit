# Frontal Orbit

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Detected stack
- Languages: Rust.
- Frameworks: none detected from the supported starter markers.

## Verification
- Run Rust verification from the repo root: `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`

## Repository shape
- `crates/` contains the Rust workspace crates and active CLI/runtime implementation.
- `tools/` contains the repo-level developer tooling suite (doctor, coverage, benchmark, cache, codegen, fuzz, generators, remote, telemetry, templates, version, workspace). Each tool is a cargo workspace member; `scripts/` are thin delegators and `Makefile` exposes `make <tool>` targets. See `docs/TOOLS.md` for the full reference.
- `docs/` and top-level Markdown files track behavior, parity, and operational guidance.

## Dev tooling
- Run a tool with `make <tool>` (e.g. `make doctor`, `make workspace`, `make version`) or directly via `cargo run -p tools-<name>`.
- Adding a new Rust tool: create `tools/<name>/` with a `Cargo.toml` (`name = "tools-<name>"`), `src/main.rs`, and a `BUILD.bazel` `sh_binary` wrapping `//tools:cargo_run.sh` (see `tools/BUILD.bazel`). Register it in root `Cargo.toml` members and in `.orbit.json` `devtools`.

## Working agreement
- Prefer small, reviewable changes and keep generated bootstrap files aligned with actual repo workflows.
- Keep shared defaults in `.orbit.json`; reserve `.orbit/settings.local.json` for machine-local overrides.
- Do not overwrite existing `ORBIT.md` content automatically; update it intentionally when repo workflows change.
