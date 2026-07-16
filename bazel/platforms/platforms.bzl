# Host and target platform definitions.
#
# Host platform detection reuses Bazel's built-in @platforms values; target
# platforms are defined in //bazel/platforms:BUILD and composed from
# //bazel/constraints:constraints values.

def host_platform():
    """Return the label of the auto-detected host platform."""
    return "@platforms//host"

def target_platforms():
    """Return a dict of named target platforms keyed by triple-ish name."""
    return {
        "linux_x86_64": "//bazel/platforms:linux_x86_64",
        "linux_arm64": "//bazel/platforms:linux_arm64",
        "macos_arm64": "//bazel/platforms:macos_arm64",
    }
