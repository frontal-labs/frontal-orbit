# ROLE
You are a senior Bazel platform engineer. Your job: turn the repository at
<REPO_ROOT> into a production-grade, polyglot **Bzlmod** monorepo that mirrors a
reference architecture: hermetic, incremental, reproducible builds for the
languages <LANGUAGES> (subset of: go, rust, typescript, python), with a clean
infrastructure library, dev container, CI, pre-commit, Makefile, scripts, and docs.
# HARD INPUTS (resolve before writing any file)
- <REPO_ROOT>: absolute path to the target repo.
- <MODULE_NAME>: Bzlmod module name (e.g. "acme"). Must be a valid label: lowercase, no spaces.
- <LANGUAGES>: ordered list, e.g. ["go","rust","typescript","python"].
- <BAZEL_VERSION>: pin, e.g. "7.4.0".
- <VERSIONS>: per language toolchain versions
    go: "1.22", rust: "1.80.0", node: "22" (>=22.6 for --experimental-strip-types),
    python: "3.11".
- <RULESET_VERSIONS>: rules_go, rules_rust, rules_python, rules_nodejs, rules_shell,
  bazel_skylib — take the latest mutually-compatible versions from
  https://registry.bazel.build and RECORD them in MODULE.bazel. After writing,
  run `bazel mod tidy` (non-fatal) and adjust pins so the resolved graph matches
  (silences "root module requires X but got Y" warnings).
# GUIDING PRINCIPLES (do not violate)
1. Bzlmod only. NO WORKSPACE file. MODULE.bazel is the single source of truth.
2. Keep ALL infra in a `bazel/` package; it must contain NO application source.
3. The custom TypeScript rule is a VENDORED/DERIVED rule set, so it lives under
   `third_party/bazel_rules/rules_typescript/` (NOT inside `tools/`, which is
   reserved for dev/CLI tooling). Its label is
   `//third_party/bazel_rules/rules_typescript:typescript.bzl`.
4. One canonical root-level directory per language (<lang>/). Shared, language-agnostic
   code lives in a `runtime/` (or `core/`) directory, not inside a language tree.
5. Every app = `<name>` binary + `<name>_lib` + optional `<name>_test`, expressed via
   `*_app()` macros so BUILD files stay declarative.
6. Root `BUILD` exposes `//:<lang>_app` aliases and an `//:all_apps` filegroup.
7. Toolchains registered centrally in MODULE.bazel; `bazel/toolchains` only holds
   `select()` helpers.
8. `.bazelrc` uses a `common` stanza + `try-import %workspace%/.bazelrc.project`
   (gitignored) for local overrides. Never put machine-specific flags in `.bazelrc`.
# PHASE 0 — RECONNAISSANCE
- Inventory <REPO_ROOT>: existing go.mod/Cargo.toml/package.json/pyproject.toml,
  entrypoints, test files, license, README.
- Decide the module name and language list. Stop and report if a language has no
  detectable manifest yet (you will still scaffold a minimal tree).
# PHASE 1 — ROOT SCAFFOLD
Create:
- `.bazelversion`  -> "<BAZEL_VERSION>\n"
- `.bazelrc`       -> common flags (--verbose_failures, --color=yes, --keep_going,
                     build --incompatible_strict_action_env, test --test_output=errors,
                     coverage --combined_report=lcov) and LAST line:
                     `try-import %workspace%/.bazelrc.project`
- `.bazelrc.project` -> commented examples for output_base/remote_cache (gitignored).
- `.bazelignore`  -> ignore .idea, .vscode, __pycache__, node_modules, target/,
                     rust/target, bazel-bin/-out/-testlogs/-<module>/, coverage/.
- `BUILD`         -> `package(default_visibility=["//visibility:public"])` + one
                     `alias(name="<lang>_app", actual="//<lang>:<lang>_app")` per
                     language + an `//:all_apps` filegroup aggregating them.
# PHASE 2 — MODULE.bazel (CRITICAL, encode these exact rules + GOTCHAS)
module(name = "<MODULE_NAME>", version = "0.0.1")
bazel_dep for each ruleset in <RULESET_VERSIONS>.
Rust (only if "rust" in <LANGUAGES>):
  rust = use_extension("@rules_rust//rust:extensions.bzl", "rust")
  rust.toolchain(edition = "2021", versions = ["<rust>"])
  use_repo(rust, "rust_toolchains"); register_toolchains("@rust_toolchains//:all")
  crate = use_extension("@rules_rust//crate_universe:extension.bzl", "crate")
  crate.from_cargo(name = "crates", manifests = ["//rust:Cargo.toml",
                     "//tools/proto-gen:Cargo.toml"])
  use_repo(crate, "crates")
Node.js / TypeScript (only if "typescript" in <LANGUAGES>):
  node = use_extension("@rules_nodejs//nodejs:extensions.bzl", "node")
  node.toolchain(name = "nodejs")
  use_repo(node, "nodejs_toolchains")
  register_toolchains("@nodejs_toolchains//:all")
  GOTCHA: rules_nodejs 6.x ships ONLY the hermetic node *toolchain*. The old
  nodejs_binary/nodejs_library macros were REMOVED (moved to aspect_rules_js).
  Run TS via Node's built-in `--experimental-strip-types` (Node >=22.6) using the
  custom `typescript_binary` rule in //third_party/bazel_rules/rules_typescript
  (Phase 4). Do NOT expect `npm install` in Bazel.
Proto (only if you actually use proto_library / per-language proto rules):
  bazel_dep(name = "rules_proto", version = "<resolved>")  # keep if referenced
  GOTCHA: do NOT call use_extension("@rules_proto//proto:extensions.bzl", "proto") —
  that file does NOT exist in rules_proto 6.x/7.x. The proto toolchain is
  AUTO-REGISTERED by the protobuf module (com_google_protobuf) under bzlmod. Just
  declare bazel_dep; rely on auto-registration. If you only keep raw .proto in a
  filegroup, you do not even need rules_proto.
third_party (optional): declare the non_module_deps extension from //third_party:repos.bzl.
# PHASE 3 — bazel/ INFRASTRUCTURE LIBRARY
Create each subpackage with a BUILD.bazel (`package(default_visibility=["//visibility:public"])`)
and the Starlark below. Keep helpers pure/side-effect-free.
- bazel/BUILD.bazel : docstring only (marks package, lists subpackages).
- bazel/defs/common.bzl : repo_root_label(), label_name(), with_prefix() helpers.
- bazel/defs/<lang>/<lang>_defs.bzl : constants (DEFAULT_GO_VERSION, DEFAULT_EDITION,
  NODE_MIN_VERSION) + helpers (go_importpath, crate_name, ts_entry_label).
- bazel/macros/language_macros.bzl : go_app/rust_app/python_app/typescript_app(name, …)
  wrappers that emit *_lib + binary + optional *_test with consistent naming.
  Load typescript_binary from
  //third_party/bazel_rules/rules_typescript:typescript.bzl.
- bazel/toolchains/toolchains.bzl : toolchain_for(toolchain_type, fallback=None) -> select().
- bazel/platforms/platforms.bzl : host_platform() + target_platforms() (linux_x86_64,
  linux_arm64, macos_arm64) using //bazel/constraints values.
- bazel/constraints/BUILD.bazel : constraint_setting/constraint_value for os/arch.
- bazel/config/BUILD.bazel : bool_flag(asan/coverage/strict) + matching config_setting
  rules (usable in select()).
- bazel/aspects/{lint_aspect,coverage_aspect}.bzl : SCAFFOLD aspects (return []), clearly
  documented as placeholders to be wired to golangci-lint/clippy/ruff/eslint/buildifier later.
- bazel/transitions/transitions.bzl : platform_transition forcing
  //command_line_option:platforms to //bazel/platforms:linux_x86_64.
- bazel/extensions/module_extensions.bzl : SCAFFOLD module_extensions returning
  ctx.extension_meta(repos=[]) (placeholders).
- bazel/bzlmod/overrides.bzl : local_override()/patch_module()/apply_overrides() helper
  structs (no Bazel API calls; consumed by MODULE.bazel comments).
- bazel/ci/{ci_build.sh,ci_test.sh,ci_lint.sh,BUILD.bazel} : thin shell sh_test targets
  wrapping `bazel build/test` and pre-commit (tags=["manual"]).
- bazel/templates/ : BUILD.template + per-language main.*.template + BUILD.tmpl files
  (exports_files) used by a //tools/scaffold tool. The typescript template MUST load
  from //third_party/bazel_rules/rules_typescript:typescript.bzl.
# PHASE 4 — PER-LANGUAGE TREES + CUSTOM RULES
For each language create <lang>/ with BUILD using the *_app() macro and a minimal
entrypoint (main.go / src/main.rs + src/lib.rs / src/index.ts / main.py + src/pkg).
Wire dependencies through the language's native manifest (go.mod, Cargo.toml,
package.json, pyproject.toml) so rules_go/crate_universe/rules_python pick them up.
third_party/bazel_rules/rules_typescript/ — the custom rule. COPY THIS CORRECTLY
(these are the two bugs that break naive implementations):
  third_party/bazel_rules/rules_typescript/BUILD:
      package(default_visibility = ["//visibility:public"])
      # Custom TypeScript rules built on the rules_nodejs hermetic toolchain.
      exports_files(["typescript.bzl"])
  third_party/bazel_rules/rules_typescript/typescript.bzl:
      def _typescript_binary_impl(ctx):
          toolchain = ctx.toolchains["@rules_nodejs//nodejs:toolchain_type"]
          node_info = toolchain.nodeinfo
          entry_point = ctx.file.entry_point
          launcher = ctx.actions.declare_file(ctx.label.name)   # NOT name+".sh"
          node_path = node_info.target_tool_path
          node_runfiles_path = node_path[len("external/"):] if node_path.startswith("external/") else node_path
          node_args = "--experimental-strip-types" if entry_point.short_path.endswith(".ts") else ""
          ctx.actions.write(output = launcher, content = """#!/usr/bin/env bash
          set -euo pipefail
          RUNFILES_DIR="${{RUNFILES_DIR:-}}"
          if [[ -z "$RUNFILES_DIR" ]]; then
            if [[ -d "$0.runfiles" ]]; then RUNFILES_DIR="$0.runfiles"
            else RUNFILES_DIR="$(cd "$(dirname "$0")/.." && pwd)"; fi
          fi
          NODE="$RUNFILES_DIR/{node_runfiles_path}"
          [[ -x "$NODE" ]] || NODE="{node_path}"
          exec "$NODE" {node_args} "$RUNFILES_DIR/{entry}" "$@"
          """.format(node_runfiles_path=node_runfiles_path, node_path=node_path,
                     node_args=node_args, entry=ctx.workspace_name+"/"+entry_point.short_path),
          is_executable = True)
          return [DefaultInfo(executable = launcher,
                              files = depset([launcher]),            # expose as default output
                              runfiles = ctx.runfiles(
                                  files = [entry_point] + ctx.files.srcs + ctx.files.data
                                           + node_info.tool_files))]
      typescript_binary = rule(implementation = _typescript_binary_impl, executable = True,
          attrs = { "entry_point": attr.label(allow_single_file=[".ts",".mts",".cts",".js",".mjs",".cjs"], mandatory=True),
                    "srcs": attr.label_list(allow_files=[".ts",".mts",".cts",".js",".mjs",".cjs",".json"]),
                    "data": attr.label_list(allow_files=True) },
          toolchains = ["@rules_nodejs//nodejs:toolchain_type"])
GOTCHA (bzlmod runfiles): the main repo's runfiles root is `_main`, NOT the module
name. So a smoke test MUST NOT hardcode `<MODULE_NAME>/...`. Instead:
  sh_test(name="<lang>_test", srcs=["smoke_test.sh"], data=[":<lang>_app"],
          args = ["$(rootpath :<lang>_app")])   # use $(rootpath), NOT $(execpath)
  and smoke_test.sh: BIN="${1:-${TEST_SRCDIR:-}/_main/<lang>/<lang>_app}".
# PHASE 5 — third_party/ CONVENTIONS
third_party/{README,BUILD,repos.bzl,patches/,overrides/,archives/,bazel_rules/}.
repos.bzl documents the 3 Bzlmod mechanisms and defines `non_module_deps` module
extension (no repos by default; every added entry pins sha256/commit).
bazel_rules/ holds vendored/derived rule sets — currently rules_typescript (Phase 4).
# PHASE 6 — DEV CONTAINER
.devcontainer/Dockerfile (install bazel via apt keyring), devcontainer.json (features:
git, go, python, node, rust; postCreateCommand -> post-create.sh which runs
`make bootstrap`), post-create.sh.
# PHASE 7 — CI (.github/workflows)
ci.yml: checkout -> setup-bazelisk -> setup go/rust/node/python -> scripts/third_party_check.sh
-> `bazel build //...` -> `bazel test //...`.
lint.yml: install linters + run pre-commit --all-files.
# PHASE 8 — pre-commit
.pre-commit-config.yaml: pre-commit-hooks, markdownlint, editorconfig-checker, actionlint
(.github/workflows/*.yml), hadolint (.devcontainer/Dockerfile), gitleaks, and local hooks:
buildifier (`bazel run //:buildifier`), rustfmt, go vet, prettier, third-party-check.
# PHASE 9 — Makefile + scripts/
Makefile: phony targets build/test/lint/fmt/tidy/bootstrap/doctor/clean/coverage each
calling scripts/<x>.sh with $(ARGS). scripts/: bootstrap.sh (verify bazel, install
pre-commit, `bazel mod tidy || echo` — NON-FATAL), build.sh (//... or //<lang>/...),
test.sh, fmt.sh, lint.sh, tidy.sh, doctor.sh, clean.sh, ci.sh, coverage.sh, plus
third_party_check.sh. Every script: `set -euo pipefail`, resolves REPO_ROOT.
# PHASE 10 — DOCS
docs/ARCHITECTURE.md (structure + design decisions; list
third_party/bazel_rules/rules_typescript as the custom TS rule location),
docs/GETTING-STARTED.md (dev container option + local option), docs/ROADMAP.md,
docs/languages/<LANG>.md per language (toolchain, project layout, common commands,
the runfiles/_main gotcha). README.md and docs/DEVELOPMENT.md must reference
//third_party/bazel_rules/rules_typescript:typescript.bzl.
# PHASE 11 — VERIFICATION (must pass before declaring done)
For each language in <LANGUAGES>:
  bazel build //<lang>:<lang>_app
  bazel run   //:<lang>_app      # prints expected output
  bazel test  //<lang>:<lang>_test
Also: bazel build //:all_apps. If rust is present and `bazel mod tidy` fails due to a
global ~/.cargo/config.toml or host Cargo < edition2024, that is ENVIRONMENTAL — keep
the committed Cargo.lock; do NOT treat it as a repo defect. Report it separately.
# GUARDRAILS / ANTI-PATTERNS
- No `use_extension` on rules_proto proto:extensions.bzl (does not exist).
- No `.sh` suffix on executable launchers; expose via DefaultInfo(files=depset([...])).
- No hardcoded module-name runfiles paths; use $(rootpath) + $1.
- Never put machine-specific flags in .bazelrc (use .bazelrc.project).
- Don't add application code under bazel/.
- The custom TS rule lives under third_party/bazel_rules/, NEVER under tools/.
- Don't run `bazel mod tidy` as a hard gate in bootstrap (network/cargo env failures).
- Keep scaffolds (aspects/extensions/transitions) clearly labeled as placeholders.
# OUTPUT CONTRACT
When finished, report: (a) files created grouped by phase, (b) final MODULE.bazel
snippet, (c) verification results per language (build/run/test PASS/FAIL), (d) any
environmental issues that blocked `mod tidy` (with the exact error), and (e) suggested
next steps to flesh out the scaffold aspects/extensions.
Refactor summary (what changed since the prior prompt):
- Custom rule moved from //tools/typescript:typescript.bzl → //third_party/bazel_rules/rules_typescript:typescript.bzl (Principle 3, Phase 4, Phase 10, and every load(...) in the prompt now point there).
- tools/ is now reserved strictly for dev/CLI tooling; the derived rule set lives under third_party/bazel_rules/ (Phase 5).
- Scaffold BUILD.tmpl and //tools/scaffold template generator reference the new label.
- All audit fixes retained: proto auto-registration, rules_nodejs toolchain-only, _main runfiles root, depset for DefaultInfo.files, $(rootpath) for test args, non-fatal mod tidy.
Verified in-repo: all six TypeScript targets (//typescript:typescript_app, //tools/mcp-server:mcp_server, //tools/docs-gen:docs_gen, //examples/typescript:ts_example, //benchmarks/typescript:ts_bench, //typescript/packages/sdk:sdk_demo) build and //typescript:typescript_test passes from the new location.