# Tooling macros for linting and formatting the monorepo.
#
# These wrap the language-agnostic linters (Biome for TS/JS/JSON/YAML and
# rustfmt/clippy for Rust) as Bazel `sh_test` targets so `bazel test //tools:...`
# and per-SDK `bazel test //sdk/...` exercise the same checks CI runs.

def biome_lint(name, target = ".", data = [], tags = None, **kwargs):
    """Run `biome check` over a path as a Bazel test target."""
    native.sh_test(
        name = name,
        srcs = ["//tools:lint.sh"],
        data = data + ["//tools:lint.sh"],
        args = [target],
        tags = tags or ["lint", "biome", "typescript"],
        **kwargs
    )

def rust_lint(name, crate = None, data = [], tags = None, **kwargs):
    """Run `cargo clippy` (optionally scoped to a crate) as a Bazel test target."""
    crate_arg = ("-p " + crate) if crate else "--workspace"
    native.sh_test(
        name = name,
        srcs = ["//tools:lint.sh"],
        data = data,
        args = [crate_arg],
        tags = tags or ["lint", "rust"],
        **kwargs
    )

def rust_format(name, tags = None, **kwargs):
    """Verify Rust formatting with `cargo fmt --check` as a Bazel test target."""
    native.sh_test(
        name = name,
        srcs = ["//tools:format.sh"],
        tags = tags or ["fmt", "rust"],
        **kwargs
    )
