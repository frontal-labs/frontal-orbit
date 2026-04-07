# ORBIT.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Detected stack
- Languages: Rust.
- Frameworks: none detected from the supported starter markers.

## Verification
- Run Rust verification from the repo root: `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`

## Repository shape
- `crates/` contains the Rust workspace crates and active CLI/runtime implementation.
- `docs/` and top-level Markdown files track behavior, parity, and operational guidance.

## Working agreement
- Prefer small, reviewable changes and keep generated bootstrap files aligned with actual repo workflows.
- Keep shared defaults in `.orbit.json`; reserve `.orbit/settings.local.json` for machine-local overrides.
- Do not overwrite existing `ORBIT.md` content automatically; update it intentionally when repo workflows change.
