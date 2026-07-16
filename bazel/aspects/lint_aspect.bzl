# Lint aspect — wired to a real linter (buildifier), with extension points for
# clippy and eslint.
#
# For every Starlark file (`.bzl`, `.bazel`, `BUILD`, `WORKSPACE`,
# `MODULE.bazel`) in a target's sources, runs `buildifier --lint=warn
# --mode=check`. The action fails the build when buildifier reports a lint
# warning.
#
# Apply on demand, e.g.:
#   bazel build //some:target --aspects=//bazel/aspects:lint_aspect.bzl%lint_aspect
#
# Extension points (documented, not yet implemented):
#   - Rust clippy: invoke `@rules_rust//rust:defs.bzl` `rust_clippy` as an
#     action when the target is a rust_* rule and `tags` contains "lint".
#   - ESLint: invoke `@aspect_rules_js//js:defs.bzl` `eslint` as an action
#     when the target is a js_* / ts_* rule and `tags` contains "lint".

_STARLARK_EXTS = ["bzl", "bazel"]
_STARLARK_NAMES = ["BUILD", "WORKSPACE", "MODULE.bazel", "BUILD.bazel"]

def _is_starlark(f):
    return f.extension in _STARLARK_EXTS or f.basename in _STARLARK_NAMES

def _lint_aspect_impl(target, ctx):
    buildifier = ctx.executable._buildifier
    outputs = []
    srcs = getattr(ctx.rule.files, "srcs", []) if hasattr(ctx.rule, "files") else []
    for src in srcs:
        if not _is_starlark(src):
            continue
        out = ctx.actions.declare_file(src.basename + ".linted")
        ctx.actions.run(
            executable = buildifier,
            arguments = ["--lint=warn", "--mode=check", src.path],
            inputs = [src],
            outputs = [out],
            mnemonic = "BuildifierLint",
            progress_message = "Linting %s with buildifier" % src.path,
        )
        outputs.append(out)

    # Extension point: clippy for Rust targets.
    # if "rust" in ctx.rule.kind and "lint" in (ctx.rule.attr.tags or []):
    #     clippy = ctx.executable._clippy
    #     ...

    # Extension point: eslint for JS/TS targets.
    # if ctx.rule.kind in ("js_library", "js_binary", "js_test"):
    #     eslint = ctx.executable._eslint
    #     ...

    return [OutputGroupInfo(lint = depset(outputs))]

lint_aspect = aspect(
    implementation = _lint_aspect_impl,
    attrs = {
        "_buildifier": attr.label(
            default = "@buildifier_prebuilt//:buildifier",
            executable = True,
            cfg = "exec",
        ),
        # "_clippy": attr.label(default = "@rules_rust//rust/clippy:clippy", executable = True, cfg = "exec"),
        # "_eslint": attr.label(default = "@aspect_rules_js//js:eslint", executable = True, cfg = "exec"),
    },
    doc = "Runs buildifier --lint=warn on Starlark sources of the target. Extension points for clippy and eslint are documented in the implementation.",
)
