# Development And Testing

This guide covers the default local workflow for the Rust workspace.

## Prerequisites

- Rust toolchain with `cargo`
- Optional auth env for live provider runs: `ANTHROPIC_API_KEY`

## Standard local loop

```bash
cargo fmt --all
cargo check --workspace
cargo test --workspace
```

For a full binary build:

```bash
cargo build --workspace
./target/debug/orbit --help
```

## Practical sanity checks

```bash
./target/debug/orbit doctor
./target/debug/orbit sandbox
./target/debug/orbit status
```

## Running a one-shot prompt

```bash
./target/debug/orbit prompt "summarize crates/runtime/src/lib.rs"
./target/debug/orbit --output-format json prompt "status"
```

## Parity harness workflow

Repository scripts:

- `scripts/run_mock_parity_harness.sh`
- `scripts/run_mock_parity_diff.py`

Typical run:

```bash
./scripts/run_mock_parity_harness.sh
```

Manual mock service startup:

```bash
cargo run -p orbit-mock-anthropic-service -- --bind 127.0.0.1:0
```

## Config and session files

- Runtime config candidates include `.orbit.json` and `.orbit/settings*.json` in workspace/home locations.
- Sessions are written under `.orbit/sessions/` in the active workspace.

## Common troubleshooting

- Build failures after dependency/layout changes: `cargo clean && cargo build --workspace`
- Command behavior differences: re-run with `--output-format json` to inspect structured responses.
- Environment issues: run `orbit doctor` before deeper debugging.
