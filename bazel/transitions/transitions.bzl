# SCAFFOLD configuration transition.
#
# Forces the //command_line_option:platforms to the linux_x86_64 target
# platform, useful for forcing a reproducible build configuration regardless
# of the host.

def _platform_transition_impl(_settings, _attr):
    return {
        "//command_line_option:platforms": ["//bazel/platforms:linux_x86_64"],
    }

platform_transition = transition(
    implementation = _platform_transition_impl,
    inputs = [],
    outputs = ["//command_line_option:platforms"],
)
