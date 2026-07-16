# Bazel Roadmap

This tracks the planned maturation of the Bazel monorepo foundation. The
scaffold is intentionally conservative: aspects, extensions, transitions, and
per-language `macros` are **placeholders** until wired to real tooling.

## Done

- [x] Bzlmod-only root: `.bazelversion`, `.bazelrc`, `.bazelrc.project`,
      `.bazelignore`, root `BUILD`.
- [x] `MODULE.bazel` with `bazel_skylib`, `rules_shell`, `buildifier_prebuilt`, and the
      `third_party` `non_module_deps` extension.
- [x] `bazel/` infrastructure library (defs, toolchains, platforms,
      constraints, config, aspects, transitions, extensions, bzlmod, ci).
- [x] `third_party/` conventions (`repos.bzl`, `README`, `patches/`,
      `overrides/`, `archives/`, `manifests/`, `libraries/`, `tools/`).
- [x] Dev container, CI (`ci.yml`, `lint.yml`), pre-commit, Makefile, scripts.
- [x] Docs under `docs/bazel/`.
- [x] Per-language `*_app()` macros and demo trees (`rust/`, `typescript/`).
- [x] Vendored `typescript_binary` rule under `third_party/bazel_rules/rules_typescript`.
- [x] `lint_aspect` wired to buildifier (real), with documented extension points
      for clippy and eslint.
- [x] `coverage_aspect` wired to `coverage_common.instrumented_files()` (real),
      with documented extension points for llvm-cov and istanbul.
- [x] Remote cache configuration documented in `.bazelrc.project`.
- [x] CI hardened: pinned `setup-bazel`, added `make ci` step.

## Next

- [ ] Implement clippy wiring in `lint_aspect` (requires `rust_clippy` target).
- [ ] Implement eslint wiring in `lint_aspect` (requires `aspect_rules_js`).
- [ ] Implement llvm-cov wiring in `coverage_aspect` for Rust.
- [ ] Implement istanbul wiring in `coverage_aspect` for TypeScript.
- [ ] Validate `bazel coverage //... --combined_report=lcov` end-to-end.
- [ ] Add per-language `*_app()` macros and demo trees for Go and Python
      (requires `rules_go` and `rules_python` in `MODULE.bazel`).
- [ ] Add a remote cache / RBE configuration behind `.bazelrc.project`.
