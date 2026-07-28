use orbit_plugins::{HookRunResult, HookRunner, PluginHooks};

#[test]
fn empty_hooks_returns_allow() {
    let runner = HookRunner::new(PluginHooks::default());
    let result = runner.run_pre_tool_use("Read", r#"{"path":"test.md"}"#);
    assert!(!result.is_denied());
    assert!(!result.is_failed());
    assert!(result.messages().is_empty());
}

#[test]
fn empty_hooks_all_events_return_allow() {
    let runner = HookRunner::new(PluginHooks::default());
    let pre = runner.run_pre_tool_use("Read", "input");
    let post = runner.run_post_tool_use("Read", "input", "output", false);
    let failure = runner.run_post_tool_use_failure("Read", "input", "error");
    assert!(!pre.is_denied());
    assert!(!post.is_denied());
    assert!(!failure.is_denied());
}

#[test]
fn pre_tool_use_allow_when_hook_exits_zero() {
    let runner = HookRunner::new(PluginHooks {
        pre_tool_use: vec!["printf 'allowed'".to_string()],
        ..PluginHooks::default()
    });
    let result = runner.run_pre_tool_use("Bash", r#"{"command":"ls"}"#);
    assert!(!result.is_denied());
    assert!(!result.is_failed());
    assert_eq!(result.messages(), &["allowed".to_string()]);
}

#[test]
fn pre_tool_use_denies_when_hook_exits_two() {
    let runner = HookRunner::new(PluginHooks {
        pre_tool_use: vec!["printf 'blocked'; exit 2".to_string()],
        ..PluginHooks::default()
    });
    let result = runner.run_pre_tool_use("Bash", r#"{"command":"pwd"}"#);
    assert!(result.is_denied());
    assert!(!result.is_failed());
    assert_eq!(result.messages(), &["blocked".to_string()]);
}

#[test]
fn pre_tool_use_fails_when_hook_exits_nonzero() {
    let runner = HookRunner::new(PluginHooks {
        pre_tool_use: vec!["printf 'error occurred'; exit 1".to_string()],
        ..PluginHooks::default()
    });
    let result = runner.run_pre_tool_use("Bash", r#"{"command":"pwd"}"#);
    assert!(!result.is_denied());
    assert!(result.is_failed());
    assert!(result.messages()[0].contains("error occurred"));
}

#[test]
fn deny_prevents_subsequent_hooks() {
    let runner = HookRunner::new(PluginHooks {
        pre_tool_use: vec![
            "printf 'first'".to_string(),
            "printf 'blocked'; exit 2".to_string(),
            "printf 'should_not_run'".to_string(),
        ],
        ..PluginHooks::default()
    });
    let result = runner.run_pre_tool_use("Bash", r#"{"command":"ls"}"#);
    assert!(result.is_denied());
    assert_eq!(result.messages().len(), 2);
    assert_eq!(result.messages()[0], "first");
    assert_eq!(result.messages()[1], "blocked");
}

#[test]
fn fail_prevents_subsequent_hooks() {
    let runner = HookRunner::new(PluginHooks {
        pre_tool_use: vec![
            "printf 'first'".to_string(),
            "printf 'broken'; exit 1".to_string(),
            "printf 'should_not_run'".to_string(),
        ],
        ..PluginHooks::default()
    });
    let result = runner.run_pre_tool_use("Bash", r#"{"command":"ls"}"#);
    assert!(result.is_failed());
    assert_eq!(result.messages().len(), 2);
}

#[test]
fn multiple_allowing_hooks_collect_messages() {
    let runner = HookRunner::new(PluginHooks {
        pre_tool_use: vec![
            "printf 'first message'".to_string(),
            "printf 'second message'".to_string(),
        ],
        ..PluginHooks::default()
    });
    let result = runner.run_pre_tool_use("Read", r#"{"path":"file"}"#);
    assert!(!result.is_denied());
    assert!(!result.is_failed());
    assert_eq!(result.messages().len(), 2);
    assert_eq!(result.messages()[0], "first message");
    assert_eq!(result.messages()[1], "second message");
}

#[test]
fn post_tool_use_runs_with_output() {
    let runner = HookRunner::new(PluginHooks {
        post_tool_use: vec!["printf 'processed'".to_string()],
        ..PluginHooks::default()
    });
    let result = runner.run_post_tool_use("Write", r#"{"path":"file"}"#, "wrote 42 bytes", false);
    assert!(!result.is_denied());
    assert_eq!(result.messages(), &["processed".to_string()]);
}

#[test]
fn post_tool_use_failure_runs_with_error() {
    let runner = HookRunner::new(PluginHooks {
        post_tool_use_failure: vec!["printf 'handled failure'".to_string()],
        ..PluginHooks::default()
    });
    let result =
        runner.run_post_tool_use_failure("Write", r#"{"path":"file"}"#, "permission denied");
    assert!(!result.is_denied());
    assert_eq!(result.messages(), &["handled failure".to_string()]);
}

#[test]
fn different_hook_events_have_independent_lists() {
    let runner = HookRunner::new(PluginHooks {
        pre_tool_use: vec!["printf 'pre'".to_string()],
        post_tool_use: vec!["printf 'post'".to_string()],
        post_tool_use_failure: vec!["printf 'failure'".to_string()],
    });
    let pre = runner.run_pre_tool_use("Tool", "input");
    let post = runner.run_post_tool_use("Tool", "input", "output", false);
    let failure = runner.run_post_tool_use_failure("Tool", "input", "error");
    assert_eq!(pre.messages(), &["pre"]);
    assert_eq!(post.messages(), &["post"]);
    assert_eq!(failure.messages(), &["failure"]);
}

#[test]
fn hook_runner_default_is_empty() {
    let runner = HookRunner::default();
    assert!(runner
        .run_pre_tool_use("Any", "input")
        .messages()
        .is_empty());
}

#[test]
fn hook_run_result_allow_constructor() {
    let result = HookRunResult::allow(vec!["msg".to_string()]);
    assert!(!result.is_denied());
    assert!(!result.is_failed());
    assert_eq!(result.messages(), &["msg".to_string()]);
}

#[test]
fn hook_run_result_accessors() {
    let allow = HookRunResult::allow(vec![]);
    assert!(!allow.is_denied());
    assert!(!allow.is_failed());
    assert!(allow.messages().is_empty());
}

#[test]
fn hook_events_produce_correct_results() {
    let pre_runner = HookRunner::new(PluginHooks {
        pre_tool_use: vec!["printf 'pre'".to_string()],
        ..PluginHooks::default()
    });
    let pre_result = pre_runner.run_pre_tool_use("Tool", "input");
    assert_eq!(pre_result.messages(), &["pre"]);

    let post_runner = HookRunner::new(PluginHooks {
        post_tool_use: vec!["printf 'post'".to_string()],
        ..PluginHooks::default()
    });
    let post_result = post_runner.run_post_tool_use("Tool", "input", "output", false);
    assert_eq!(post_result.messages(), &["post"]);

    let fail_runner = HookRunner::new(PluginHooks {
        post_tool_use_failure: vec!["printf 'failure'".to_string()],
        ..PluginHooks::default()
    });
    let fail_result = post_runner.run_post_tool_use_failure("Tool", "input", "error");
    // fail_result uses the post runner's hooks (empty post_tool_use_failure), not fail_runner's
    assert_eq!(fail_result.messages().len(), 0);

    let fail_result2 = fail_runner.run_post_tool_use_failure("Tool", "input", "error");
    assert_eq!(fail_result2.messages(), &["failure"]);
}
