# Bazel Monorepo Architecture

This document describes the **Bazel build foundation** for Frontal Orbit. It is
deliberately decoupled from application source: all build infrastructure lives
under `bazel/` and `third_party/`, and `MODULE.bazel` is the single source of
truth (no `WORKSPACE` file — Bzlmod only).

## Layout

| Path                    | Purpose                                                      |
|-------------------------|--------------------------------------------------------------|
| `MODULE.bazel`          | Module name, ruleset `bazel_dep`s, Bzlmod extensions.        |
| `.bazelversion`         | Pins the Bazel version (`7.4.0`).                            |
| `.bazelrc`              | Common flags + `try-import %workspace%/.bazelrc.project`.    |
| `.bazelrc.project`      | Gitignored, machine-local overrides only.                    |
| `.bazelignore`          | Patterns Bazel must ignore (IDE, target/, node_modules/…).   |
| `BUILD`                 | Root package; exposes `//:buildifier` alias.                |
| `bazel/`                | Reusable Starlark infrastructure (no app source).           |
| `third_party/`          | Non-registry deps, vendored rule sets, override conventions. |
| `.devcontainer/`        | Dev container (Bazel via apt keyring + language features).   |
| `scripts/` + `Makefile` | Thin wrappers around `bazel` / `pre-commit`.                |

## `bazel/` infrastructure library

`bazel/` is a pure helper package — it contains **no application code**. Its
subpackages:

| Package                 | Contents                                                     |
|-------------------------|-------------------------------------------------------------|
| `bazel/defs/`           | Pure helper functions (`common.bzl`) and language defs.     |
| `bazel/toolchains/`     | `toolchain_for()` select() helpers.                         |
| `bazel/platforms/`      | Host + target platform definitions.                         |
| `bazel/constraints/`    | OS/arch `constraint_setting` / `constraint_value`.          |
| `bazel/config/`         | Build flags (`asan`/`coverage`/`strict`) + `config_setting`.|
| `bazel/aspects/`        | SCAFFOLD `lint`/`coverage` aspects (return `[]`).          |
| `bazel/transitions/`    | Configuration transitions (e.g. force `linux_x86_64`).      |
| `bazel/extensions/`     | SCAFFOLD module extension (declares no repos).              |
| `bazel/bzlmod/`         | `overrides.bzl` helper structs consumed by `MODULE.bazel`.  |
| `bazel/ci/`             | `sh_test` wrappers around `bazel build/test` + pre-commit.  |

## Design decisions

1. **Bzlmod only.** No `WORKSPACE`. `MODULE.bazel` pins every ruleset.
2. **No application code under `bazel/`.** `bazel/` is infrastructure only.
3. **Central toolchain registration.** Toolchains are registered in
   `MODULE.bazel`; `bazel/toolchains` only offers `select()` helpers.
4. **Machine-local flags stay out of `.bazelrc`.** Local overrides go in the
   gitignored `.bazelrc.project`.
5. **`bazel mod tidy` is non-fatal** in bootstrap — network / cargo-env
   failures must not block development.
6. **Scaffolds are labeled placeholders.** Aspects, extensions, and
   transitions return no providers/repos until wired to real tooling.

## Extending the monorepo

- Add a new ruleset: `bazel_dep` in `MODULE.bazel`, record the version, then
  run `bazel mod tidy`.
- Add a vendored rule set: place it under `third_party/bazel_rules/`, keep its
  `BUILD` exporting the `.bzl`, and load it from consuming `BUILD` files.
- Add a non-module dep: extend `//third_party:repos.bzl` (`non_module_deps`)
  and **pin a sha256 or commit**.
