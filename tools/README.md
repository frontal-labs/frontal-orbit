# Developer Tools (`//tools`)

This directory is the canonical home for **frontal-orbit's repo-level developer
tooling**. It is distinct from `crates/tools/` (the *agent's* runtime tool
system: bash/read/write/…). Nothing here ships in the `orbit` binary.

Each subdirectory is a real, runnable tool wired into the build, the `Makefile`,
pre-commit, and `.orbit.json`. `scripts/*.sh` are thin delegators that call into
`tools/`; prefer editing logic here, not in `scripts/`.

## The 12 tools

| Tool | Kind | Purpose |
|------|------|---------|
| [`doctor`](../../docs/TOOLS.md#doctor) | Rust | Environment / toolchain health check |
| [`coverage`](../../docs/TOOLS.md#coverage) | Mixed | Run `bazel coverage //...` and summarize lcov |
| [`benchmark`](../../docs/TOOLS.md#benchmark) | Rust | Build / CI timing harness |
| [`cache`](../../docs/TOOLS.md#cache) | Rust | Inspect and prune Bazel / Cargo caches |
| [`codegen`](../../docs/TOOLS.md#codegen) | Rust | Generate Rust boilerplate from names / JSON schemas |
| [`fuzz`](../../docs/TOOLS.md#fuzz) | Rust | Discover, scaffold, and run fuzz targets |
| [`generators`](../../docs/TOOLS.md#generators) | Rust | Scaffold crates from templates |
| [`remote`](../../docs/TOOLS.md#remote) | Rust | Generate / validate Bazel remote-cache config |
| [`telemetry`](../../docs/TOOLS.md#telemetry) | Rust | Wrap a command and emit a telemetry event |
| [`templates`](../../docs/TOOLS.md#templates) | Lib | Shared `{var}` templating library (no CLI) |
| [`version`](../../docs/TOOLS.md#version) | Rust | Bump monorepo version across root files |
| [`workspace`](../../docs/TOOLS.md#workspace) | Rust | Validate and maintain the workspace member list |

See [`docs/TOOLS.md`](../../docs/TOOLS.md) for the full per-tool reference.

## Repo-wide helper scripts

Alongside the 12 tool crates, `tools/` also hosts repo-wide helper scripts
used by the lint/format Bazel macros (`tools/lint.bzl`):

- `format.sh` — monorepo formatter (Biome for TS/JS/JSON/YAML + `cargo fmt`)
- `lint.sh` — monorepo linter (Biome + `cargo clippy`)
- `sdk_rust.sh` / `sdk_ts.sh` — run the Rust / TypeScript SDK toolchains
- `lint.bzl` — Bazel macros (`biome_lint`, `rust_lint`, `rust_format`) that
  wrap the above as `sh_test` targets so `bazel test //tools:...` and
  `bazel test //sdk:...` run the same checks as CI.

These are exported labels (`//tools:lint.sh`, `//tools:format.sh`, …) and are
distinct from the 12 tool crates above.

## Quickstart

```bash
# via make
make doctor
make workspace check
make version show
make bench build

# or directly via cargo
cargo run -q -p tools-doctor -- --root .
cargo run -q -p tools-workspace -- list

# or via Bazel (shells out to cargo; see note below)
bazel run //tools/doctor:doctor -- --root .
```

## Conventions

A Rust tool under `tools/<name>/` contains:

```
tools/<name>/
  Cargo.toml      # name = "tools-<name>", [lints] workspace = true
  src/main.rs     # clap-based CLI
  BUILD.bazel     # sh_binary wrapping //tools:cargo_run.sh
  README.md       # optional; the canonical docs live in docs/TOOLS.md
```

- **Bazel.** Each tool is exposed as `//tools/<name>:<name>`, an `sh_binary`
  that execs `cargo run -p tools-<name>` from the workspace checkout (see
  `tools/cargo_run.sh`). This keeps `bazel build //...` hermetic and green
  without `rules_rust`; the migration path to native `rust_binary` is tracked in
  `docs/bazel/ROADMAP.md`.
- **Lints.** New tools inherit the workspace lints (`unsafe_code = "forbid"`,
  clippy pedantic). Keep `cargo clippy --workspace -- -D warnings` clean.
- **No `unsafe`.** The workspace forbids `unsafe`; the pre-commit `deny-unsafe`
  hook scans `tools/` as well as `crates/`.

## Adding a new tool

1. `mkdir tools/<name>` and add `Cargo.toml` (`name = "tools-<name>"`),
   `src/main.rs`, and `BUILD.bazel` (copy an existing one and set `args`).
2. Add `"tools/*"` is already a workspace glob, so the new crate is picked up
   automatically by `cargo`. (If you used an explicit path, add it to the root
   `Cargo.toml` `members`.)
3. Add `scripts/<name>.sh` delegating to `cargo run -q -p tools-<name>` and a
   `make <name>` target in `Makefile`.
4. Register it under the `devtools` key in `.orbit.json`.
5. Document it in `docs/TOOLS.md`.

## `third_party/tools/`

Vendored, non-crate tooling dependencies (mirrors the `third_party/` conventions
in `docs/bazel/ARCHITECTURE.md`). Currently a placeholder.
