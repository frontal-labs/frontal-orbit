# `third_party/tools/`

Vendored, non-crate tooling dependencies used by the `//tools` developer-tooling
suite. This mirrors the conventions described in
[`docs/bazel/ARCHITECTURE.md`](../../docs/bazel/ARCHITECTURE.md) for the broader
`third_party/` tree:

- Place vendored tooling artifacts here rather than in `tools/<name>/`.
- When adding a non-module dependency, extend `//third_party:repos.bzl`
  (`non_module_deps`) and **pin a sha256 or commit**.
- Keep this directory free of application source — it is infrastructure only.

Currently a placeholder; the `//tools` suite builds its Rust tools via the
workspace `crates/*` + `tools/*` members and wraps them in Bazel with
`//tools:cargo_run.sh`.
