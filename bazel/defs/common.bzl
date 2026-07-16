# Common, side-effect-free helpers for resolving labels in the monorepo.

def repo_root_label(rel):
    """Return a label rooted at the workspace root.

    Args:
        rel: A repo-relative path such as "rust/Cargo.toml".

    Returns:
        A label string "@//<rel>".
    """
    return "@//" + rel

def label_name(label):
    """Extract the target name from a label string.

    Args:
        label: A label such as "//rust:rust_app" or "//:all_apps".

    Returns:
        The final name component ("rust_app", "all_apps").
    """
    if ":" in label:
        return label.split(":")[-1]
    if "/" in label:
        return label.split("/")[-1]
    return label

def with_prefix(prefix, name):
    """Join a prefix and a name with an underscore, skipping empty prefixes.

    Args:
        prefix: A package prefix (may be "").
        name: A target name.

    Returns:
        "<prefix>_<name>" or "<name>" when prefix is empty.
    """
    if prefix == "":
        return name
    return prefix + "_" + name
