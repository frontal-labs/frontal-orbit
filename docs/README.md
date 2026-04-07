# Orbit Docs

This directory contains focused, implementation-aligned docs for the Rust workspace.

## Documents

- [`cli-reference.md`](./cli-reference.md) - top-level CLI usage, flags, and command examples.
- [`workspace-crates.md`](./workspace-crates.md) - crate map for the `orbit-*` workspace packages.
- [`development-and-testing.md`](./development-and-testing.md) - local development loops, checks, and parity harness workflows.
- [`container.md`](./container.md) - Docker/Podman build and run workflows.

## Fast path

```bash
cargo build --workspace
./target/debug/orbit --help
./target/debug/orbit doctor
cargo test --workspace
```

If you are new to the repository, start with `cli-reference.md` and `workspace-crates.md`.
