package(default_visibility = ["//visibility:public"])

exports_files(["MODULE.bazel"])

# Buildifier from @buildifier_prebuilt, exposed for the pre-commit `bazel run` hook.
alias(
    name = "buildifier",
    actual = "@buildifier_prebuilt//:buildifier",
)
