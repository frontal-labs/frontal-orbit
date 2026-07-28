use orbit_repo::normalize_branch_name;

#[test]
fn normalizes_simple_name() {
    assert_eq!(normalize_branch_name("Feature 123"), "feature-123");
}

#[test]
fn normalizes_with_special_characters() {
    let result = normalize_branch_name(" Feature 123 / Fix::Bug ");
    assert_eq!(result, "feature-123-fix-bug");
}

#[test]
fn empty_string_returns_fallback() {
    assert_eq!(normalize_branch_name(""), "orbit-task");
}

#[test]
fn only_separators_returns_fallback() {
    assert_eq!(normalize_branch_name("////"), "orbit-task");
}

#[test]
fn only_special_chars_returns_fallback() {
    assert_eq!(normalize_branch_name("!!!@@@###$$$"), "orbit-task");
}

#[test]
fn all_separators_returns_fallback() {
    assert_eq!(normalize_branch_name("---"), "orbit-task");
}

#[test]
fn collapses_multiple_separators() {
    assert_eq!(normalize_branch_name("feature   branch"), "feature-branch");
    assert_eq!(normalize_branch_name("feature___branch"), "feature-branch");
}

#[test]
fn trims_whitespace() {
    assert_eq!(normalize_branch_name("  my-branch  "), "my-branch");
}

#[test]
fn lowercases_uppercase() {
    assert_eq!(normalize_branch_name("FEATURE"), "feature");
    assert_eq!(normalize_branch_name("MyFeature"), "myfeature");
}

#[test]
fn preserves_hyphens_and_digits() {
    assert_eq!(normalize_branch_name("bugfix-123"), "bugfix-123");
}

#[test]
fn mixed_special_chars() {
    assert_eq!(normalize_branch_name("a!b@c#d$e%f"), "a-b-c-d-e-f");
}

#[test]
fn single_segment() {
    assert_eq!(normalize_branch_name("hello"), "hello");
    assert_eq!(normalize_branch_name("123"), "123");
}

#[test]
fn leading_trailing_slashes() {
    assert_eq!(normalize_branch_name("/feature/"), "feature");
}

#[test]
fn underscores_become_hyphens() {
    assert_eq!(
        normalize_branch_name("snake_case_branch"),
        "snake-case-branch"
    );
}
