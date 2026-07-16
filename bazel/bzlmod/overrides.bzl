# Bzlmod override helper structs.
#
# These are pure data helpers (no Bazel API calls). They document the three
# Bzlmod override mechanisms and are consumed by MODULE.bazel comments / a
# future non_module_deps extension. Every added entry MUST pin a sha256 or
# commit for reproducibility.

def local_override(module_name, path):
    """Describe a local_path_override for a module."""
    return struct(kind = "local_path_override", module_name = module_name, path = path)

def patch_module(module_name, version, patches = [], patch_strip = 0):
    """Describe a module with patches applied via single_version_override."""
    return struct(
        kind = "single_version_override",
        module_name = module_name,
        version = version,
        patches = patches,
        patch_strip = patch_strip,
    )

def apply_overrides(_overrides):
    """Render override structs into a human-readable description.

    This is a no-op helper for docs/auditing; actual overrides are expressed
    directly in MODULE.bazel.
    """
    return [_o.kind + ":" + _o.module_name for _o in _overrides]
