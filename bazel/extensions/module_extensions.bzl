# SCAFFOLD module extensions.
#
# PLACEHOLDER — returns no repositories. These exist so the monorepo has a
# stable home for future Bzlmod extensions (e.g. vendored toolchains or
# generated repos) without touching MODULE.bazel structure later.

def _module_ext_impl(_ctx):
    return ctx.extension_meta(repos = [])

module_ext = module_extension(
    implementation = _module_ext_impl,
    doc = "Placeholder module extension; declares no repositories yet.",
)
