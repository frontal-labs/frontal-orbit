# Sample Starlark file used by //bazel/aspects/example to demonstrate the
# lint aspect. Kept buildifier-clean on purpose.

def greet(name):
    """Return a greeting for the given name."""
    return "hello, %s" % name
