# Third-party (non-module) dependency module extension.
#
# PLACEHOLDER — declares no repositories by default. This is the single home
# for vendored / local / archive overrides in the monorepo. Every added entry
# MUST pin a sha256 or commit for reproducibility (see //third_party:README.md).

def _non_module_deps_impl(_ctx):
    # No repositories are pinned by default. Add entries that declare repos
    # via ctx.module_repo(...) / ctx.use_repo_add / etc. as needed.
    return None

non_module_deps = module_extension(
    implementation = _non_module_deps_impl,
    doc = "Non-module dependency extension; no repos pinned by default.",
)
