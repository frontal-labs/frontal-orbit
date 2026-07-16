# Third-party dependency conventions

This directory centralizes everything that is *not* first-party application
code but is also not a normal Bazel module registry dependency.

## Layout

| Path               | Purpose                                                        |
|--------------------|----------------------------------------------------------------|
| `repos.bzl`        | `non_module_deps` Bzlmod module extension (placeholder).        |
| `bazel_rules/`     | Vendored / derived Bazel rule sets (reserved for future use).   |
| `manifests/`       | Locked manifests / checksums for vendored sources.             |
| `patches/`         | Patch files applied to third-party modules.                    |
| `overrides/`       | Local or single-version overrides (documented, not applied).   |
| `archives/`        | Vendored source archives (with sha256).                        |
| `libraries/`       | Prebuilt third-party libraries consumed via `data`/`deps`.     |
| `tools/`           | Third-party CLI tooling wrapped for hermetic use.              |

## Bzlmod mechanisms

The `non_module_deps` extension in `repos.bzl` documents the three Bzlmod
override mechanisms. Every added entry MUST pin a `sha256` or `commit` for
reproducibility:

1. `local_path_override` — point a module at a local checkout (dev only).
2. `single_version_override` — force a version, apply `patches`, or drop a dep.
3. `archive_override` — replace a module's source with a vendored archive.

Helper structs for emitting these live in `//bazel/bzlmod:overrides.bzl`.
