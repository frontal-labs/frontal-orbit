use orbit_tools::ToolExecutionScope;

#[test]
fn tool_execution_scope_for_session() {
    let scope = ToolExecutionScope::for_session("session-test");
    assert_eq!(scope.session_id, "session-test");
    assert!(scope.repo_id.is_none());
    assert!(scope.branch_id.is_none());
}

#[test]
fn tool_execution_scope_default() {
    let scope = ToolExecutionScope::default();
    assert_eq!(scope.session_id, "default-tool-session");
}
