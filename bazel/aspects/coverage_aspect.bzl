# Coverage aspect — wired to Bazel's native coverage instrumentation, with
# extension points for llvm-cov (Rust) and istanbul (TypeScript).
#
# Marks a target's declared sources as instrumented so that
# `bazel coverage //...` collects them. This is a real, runnable coverage
# hook: it returns `coverage_common.instrumented_files()` for any target that
# declares `srcs`.
#
# Apply on demand, e.g.:
#   bazel coverage //some:target --aspects=//bazel/aspects:coverage_aspect.bzl%coverage_aspect
#
# Extension points (documented, not yet implemented):
#   - Rust llvm-cov: when the target is a rust_* rule, attach a
#     `coverage_common.instrumented_files()` provider that includes the
#     `.rlib` / `.so` outputs so llvm-cov can generate lcov.
#   - TypeScript istanbul: when the target is a typescript_binary, attach a
#     provider that points at the transpiled `.js` outputs for nyc/istanbul.

def _coverage_aspect_impl(target, ctx):
    srcs = []
    if hasattr(ctx.rule, "attr") and hasattr(ctx.rule.attr, "srcs"):
        srcs = ctx.rule.files.srcs

    providers = [coverage_common.instrumented_files(files = depset(srcs))]

    # Extension point: llvm-cov for Rust targets.
    # if "rust" in ctx.rule.kind:
    #     providers.append(coverage_common.instrumented_files(
    #         files = depset(target.files.to_list()),
    #     ))

    # Extension point: istanbul for TypeScript targets.
    # if ctx.rule.kind == "typescript_binary":
    #     providers.append(coverage_common.instrumented_files(
    #         files = depset(target.files.to_list()),
    #     ))

    return providers

coverage_aspect = aspect(
    implementation = _coverage_aspect_impl,
    doc = "Marks a target's sources as instrumented for `bazel coverage`. Extension points for llvm-cov and istanbul are documented in the implementation.",
)
