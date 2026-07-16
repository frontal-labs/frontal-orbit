# Toolchain resolution helpers built on select().
#
# These helpers keep toolchain selection declarative and centralized. Actual
# toolchains are registered in MODULE.bazel; this file only provides select()
# scaffolding so packages can opt into a toolchain type.

def toolchain_for(toolchain_type, fallback = None):
    """Return a select() that resolves a toolchain by type.

    Args:
        toolchain_type: The toolchain_type label.
        fallback: An optional default label used when no constraint matches.

    Returns:
        A select() expression keyed on the toolchain type, or the fallback.
    """
    conditions = {
        toolchain_type: toolchain_type,
    }
    if fallback != None:
        return select(conditions, no_match_error = "no toolchain for " + str(toolchain_type))
    return select(conditions, default = fallback)
