use orbit_server::{
    CreateTaskRequest, EventStreamQuery, HealthResponse, HostedTaskContext, HostedTaskSnapshot,
    HostedTaskStatus, ListTasksQuery, OrphanPolicyQuery, OrphanPolicyRuleResponse, TaskCounters,
    TaskRuntimeResponse,
};

#[test]
fn hosted_task_status_variants() {
    assert_ne!(HostedTaskStatus::Pending, HostedTaskStatus::Running);
    assert_ne!(HostedTaskStatus::Pending, HostedTaskStatus::Completed);
    assert_ne!(HostedTaskStatus::Pending, HostedTaskStatus::Failed);
    assert_ne!(HostedTaskStatus::Pending, HostedTaskStatus::Cancelled);
    assert_eq!(HostedTaskStatus::Pending, HostedTaskStatus::Pending);
    assert_eq!(HostedTaskStatus::Running, HostedTaskStatus::Running);
    assert_eq!(HostedTaskStatus::Completed, HostedTaskStatus::Completed);
    assert_eq!(HostedTaskStatus::Failed, HostedTaskStatus::Failed);
    assert_eq!(HostedTaskStatus::Cancelled, HostedTaskStatus::Cancelled);
}

#[test]
fn hosted_task_status_serialize() {
    let pending = serde_json::to_value(HostedTaskStatus::Pending).unwrap();
    assert_eq!(pending, serde_json::json!("pending"));

    let running = serde_json::to_value(HostedTaskStatus::Running).unwrap();
    assert_eq!(running, serde_json::json!("running"));

    let completed = serde_json::to_value(HostedTaskStatus::Completed).unwrap();
    assert_eq!(completed, serde_json::json!("completed"));

    let failed = serde_json::to_value(HostedTaskStatus::Failed).unwrap();
    assert_eq!(failed, serde_json::json!("failed"));

    let cancelled = serde_json::to_value(HostedTaskStatus::Cancelled).unwrap();
    assert_eq!(cancelled, serde_json::json!("cancelled"));
}

#[test]
fn hosted_task_status_deserialize() {
    let pending: HostedTaskStatus = serde_json::from_value(serde_json::json!("pending")).unwrap();
    assert_eq!(pending, HostedTaskStatus::Pending);

    let running: HostedTaskStatus = serde_json::from_value(serde_json::json!("running")).unwrap();
    assert_eq!(running, HostedTaskStatus::Running);

    let completed: HostedTaskStatus =
        serde_json::from_value(serde_json::json!("completed")).unwrap();
    assert_eq!(completed, HostedTaskStatus::Completed);

    let failed: HostedTaskStatus = serde_json::from_value(serde_json::json!("failed")).unwrap();
    assert_eq!(failed, HostedTaskStatus::Failed);

    let cancelled: HostedTaskStatus =
        serde_json::from_value(serde_json::json!("cancelled")).unwrap();
    assert_eq!(cancelled, HostedTaskStatus::Cancelled);
}

#[test]
fn hosted_task_context_default() {
    let context = HostedTaskContext::default();
    assert!(context.source.is_none());
    assert!(context.user_id.is_none());
    assert!(context.channel_id.is_none());
    assert!(context.repository.is_none());
    assert!(context.repo_url.is_none());
    assert!(context.priority.is_none());
    assert!(context.plan_id.is_none());
    assert!(context.model.is_none());
    assert!(context.allowed_tools.is_empty());
}

#[test]
fn hosted_task_context_serialize_roundtrip() {
    let context = HostedTaskContext {
        source: Some("slack".to_string()),
        user_id: Some("U123".to_string()),
        channel_id: Some("C456".to_string()),
        repository: Some("acme/orbit".to_string()),
        priority: Some("high".to_string()),
        ..HostedTaskContext::default()
    };

    let json = serde_json::to_value(&context).unwrap();
    assert_eq!(json["source"], "slack");
    assert_eq!(json["user_id"], "U123");
    assert!(json.get("plan_id").is_none());

    let deserialized: HostedTaskContext = serde_json::from_value(json).unwrap();
    assert_eq!(deserialized.source.as_deref(), Some("slack"));
    assert_eq!(deserialized.user_id.as_deref(), Some("U123"));
    assert_eq!(deserialized.repository.as_deref(), Some("acme/orbit"));
}

#[test]
fn hosted_task_snapshot_serialize() {
    let snapshot = HostedTaskSnapshot {
        task_id: "task-123".to_string(),
        status: HostedTaskStatus::Running,
        created_at: 1000,
        updated_at: 2000,
        prompt: "Do something".to_string(),
        description: Some("A test task".to_string()),
        result: None,
        error: None,
        lane_id: Some("lane-1".to_string()),
        source: Some("api".to_string()),
        user_id: Some("U123".to_string()),
        channel_id: Some("C456".to_string()),
        thread_ts: None,
        approval_message_ts: None,
        orphan_policy: None,
        repository: Some("acme/orbit".to_string()),
        repo_url: Some("https://github.com/acme/orbit.git".to_string()),
        base_ref: Some("main".to_string()),
        branch: Some("orbit/test".to_string()),
        published_branch: None,
        published_commit_sha: None,
        published_remote: None,
        pr_number: None,
        pr_url: None,
        pr_state: None,
        pr_merged: None,
        pr_closed_at: None,
        pr_merged_at: None,
        github_review_state: None,
        github_feedback_required: None,
        github_feedback_reason: None,
        linear_issue_id: None,
        linear_issue_url: None,
        linear_issue_state: None,
        linear_issue_identifier: None,
        graphite_stack_id: None,
        graphite_head_branch: None,
        graphite_base_branch: None,
        pr_api_url: None,
        pr_head_ref: None,
        pr_base_ref: None,
        metadata: std::collections::HashMap::new(),
        execution_backend: Some("in_memory".to_string()),
        priority: None,
        plan_id: Some("plan-1".to_string()),
        plan_kind: Some("implementation".to_string()),
        work_item_id: Some("wi-1".to_string()),
        worker_id: Some("worker-1".to_string()),
        worker_status: Some("running".to_string()),
    };

    let json = serde_json::to_value(&snapshot).unwrap();
    assert_eq!(json["task_id"], "task-123");
    assert_eq!(json["status"], "running");
    assert_eq!(json["created_at"], 1000);
    assert_eq!(json["prompt"], "Do something");
    assert_eq!(json["source"], "api");
    assert_eq!(json["execution_backend"], "in_memory");

    let deserialized: HostedTaskSnapshot = serde_json::from_value(json).unwrap();
    assert_eq!(deserialized.task_id, "task-123");
    assert_eq!(deserialized.status, HostedTaskStatus::Running);
}

#[test]
fn hosted_task_snapshot_optional_fields_skipped_when_none() {
    let snapshot = HostedTaskSnapshot {
        task_id: "task-empty".to_string(),
        status: HostedTaskStatus::Pending,
        created_at: 0,
        updated_at: 0,
        prompt: String::new(),
        description: None,
        result: None,
        error: None,
        lane_id: None,
        source: None,
        user_id: None,
        channel_id: None,
        thread_ts: None,
        approval_message_ts: None,
        orphan_policy: None,
        repository: None,
        repo_url: None,
        base_ref: None,
        branch: None,
        published_branch: None,
        published_commit_sha: None,
        published_remote: None,
        pr_number: None,
        pr_url: None,
        pr_state: None,
        pr_merged: None,
        pr_closed_at: None,
        pr_merged_at: None,
        github_review_state: None,
        github_feedback_required: None,
        github_feedback_reason: None,
        linear_issue_id: None,
        linear_issue_url: None,
        linear_issue_state: None,
        linear_issue_identifier: None,
        graphite_stack_id: None,
        graphite_head_branch: None,
        graphite_base_branch: None,
        pr_api_url: None,
        pr_head_ref: None,
        pr_base_ref: None,
        metadata: std::collections::HashMap::new(),
        execution_backend: None,
        priority: None,
        plan_id: None,
        plan_kind: None,
        work_item_id: None,
        worker_id: None,
        worker_status: None,
    };

    let json = serde_json::to_value(&snapshot).unwrap();
    assert_eq!(json["task_id"], "task-empty");
    assert!(json.get("source").is_none());
    assert!(json.get("execution_backend").is_none());
    assert!(json.get("worker_id").is_none());

    let deserialized: HostedTaskSnapshot = serde_json::from_value(json).unwrap();
    assert_eq!(deserialized.task_id, "task-empty");
}

#[test]
fn create_task_request_deserialize() {
    let json = serde_json::json!({
        "prompt": "Hello world",
        "repository": "acme/test",
        "source": "slack",
        "user_id": "U123"
    });
    let request: CreateTaskRequest = serde_json::from_value(json).unwrap();
    assert_eq!(request.prompt, "Hello world");
    assert_eq!(request.repository.as_deref(), Some("acme/test"));
    assert_eq!(request.source.as_deref(), Some("slack"));
    assert_eq!(request.user_id.as_deref(), Some("U123"));
    assert!(request.branch.is_none());
    assert!(request.model.is_none());
}

#[test]
fn create_task_request_defaults() {
    let json = serde_json::json!({"prompt": "Minimal request"});
    let request: CreateTaskRequest = serde_json::from_value(json).unwrap();
    assert_eq!(request.prompt, "Minimal request");
    assert!(request.repository.is_none());
    assert!(request.source.is_none());
    assert!(request.priority.is_none());
}

#[test]
fn list_tasks_query_deserialize() {
    let json = serde_json::json!({
        "status": "running",
        "source": "slack",
        "user_id": "U123",
        "limit": 10
    });
    let query: ListTasksQuery = serde_json::from_value(json).unwrap();
    assert_eq!(query.status.as_deref(), Some("running"));
    assert_eq!(query.source.as_deref(), Some("slack"));
    assert_eq!(query.user_id.as_deref(), Some("U123"));
    assert_eq!(query.limit, Some(10));
    assert!(query.needs_followup.is_none());
}

#[test]
fn list_tasks_query_empty() {
    let json = serde_json::json!({});
    let query: ListTasksQuery = serde_json::from_value(json).unwrap();
    assert!(query.status.is_none());
    assert!(query.source.is_none());
    assert!(query.limit.is_none());
}

#[test]
fn event_stream_query_deserialize() {
    let json = serde_json::json!({
        "task_id": "task-123",
        "topic": "lane",
        "event": "lane.started",
        "limit": 50
    });
    let query: EventStreamQuery = serde_json::from_value(json).unwrap();
    assert_eq!(query.task_id.as_deref(), Some("task-123"));
    assert_eq!(query.topic.as_deref(), Some("lane"));
    assert_eq!(query.event.as_deref(), Some("lane.started"));
    assert_eq!(query.limit, Some(50));
}

#[test]
fn health_response_serialize() {
    let response = HealthResponse { ok: true };
    let json = serde_json::to_value(&response).unwrap();
    assert_eq!(json["ok"], true);
}

#[test]
fn status_response_fields() {
    let counters = TaskCounters {
        total_tasks: 5,
        active_tasks: 2,
        completed_tasks: 2,
        failed_tasks: 1,
    };
    assert_eq!(counters.total_tasks, 5);
    assert_eq!(counters.active_tasks, 2);
    assert_eq!(counters.completed_tasks, 2);
    assert_eq!(counters.failed_tasks, 1);
}

#[test]
fn orphan_policy_query_deserialize() {
    let json = serde_json::json!({
        "repository": "acme/test",
        "source": "slack",
        "priority": "high"
    });
    let query: OrphanPolicyQuery = serde_json::from_value(json).unwrap();
    assert_eq!(query.repository.as_deref(), Some("acme/test"));
    assert_eq!(query.source.as_deref(), Some("slack"));
    assert_eq!(query.priority.as_deref(), Some("high"));
}

#[test]
fn orphan_policy_query_empty() {
    let json = serde_json::json!({});
    let query: OrphanPolicyQuery = serde_json::from_value(json).unwrap();
    assert!(query.repository.is_none());
    assert!(query.source.is_none());
    assert!(query.priority.is_none());
}

#[test]
fn orphan_policy_rule_response_serialize() {
    let rule = OrphanPolicyRuleResponse {
        repository: Some("acme/orbit".to_string()),
        source: Some("slack".to_string()),
        priority: Some("high".to_string()),
        approval_delay_secs: Some(30),
        auto_retry_after_secs: Some(60),
        auto_cancel_after_secs: Some(300),
    };
    let json = serde_json::to_value(&rule).unwrap();
    assert_eq!(json["repository"], "acme/orbit");
    assert_eq!(json["approval_delay_secs"], 30);
    assert!(json.get("auto_cancel_after_secs").is_some());
}

#[test]
fn orphan_policy_rule_response_empty_fields() {
    let rule = OrphanPolicyRuleResponse {
        repository: None,
        source: None,
        priority: None,
        approval_delay_secs: None,
        auto_retry_after_secs: None,
        auto_cancel_after_secs: None,
    };
    let json = serde_json::to_value(&rule).unwrap();
    assert!(json.get("repository").is_none());
    assert!(json.get("source").is_none());
    assert!(json.get("approval_delay_secs").is_none());
}

#[test]
fn task_runtime_response_serialize() {
    let response = TaskRuntimeResponse {
        task_id: "task-1".to_string(),
        worker_id: Some("worker-1".to_string()),
        worker_status: Some("running".to_string()),
        manifest_file: Some("/path/to/manifest.json".to_string()),
        output_file: Some("/path/to/output.md".to_string()),
        orphan_policy: None,
        hosted_agent: None,
    };
    let json = serde_json::to_value(&response).unwrap();
    assert_eq!(json["task_id"], "task-1");
    assert_eq!(json["worker_id"], "worker-1");
    assert_eq!(json["worker_status"], "running");
    assert_eq!(json["manifest_file"], "/path/to/manifest.json");
    assert!(json.get("hostedAgent").is_none());
}

#[test]
fn hosted_task_context_repo_checkout_request_with_remote_url() {
    let context = HostedTaskContext {
        repo_url: Some("https://github.com/acme/orbit.git".to_string()),
        repository: Some("acme/orbit".to_string()),
        base_ref: Some("main".to_string()),
        branch: Some("feature/test".to_string()),
        ..HostedTaskContext::default()
    };
    let request = context.repo_checkout_request("/tmp/workspace", "checkout-1");
    assert!(request.is_some());
    let req = request.unwrap();
    assert_eq!(req.checkout_id, "checkout-1");
    assert_eq!(req.repository.as_deref(), Some("acme/orbit"));
    assert_eq!(req.base_ref.as_deref(), Some("main"));
    assert_eq!(req.branch.as_deref(), Some("feature/test"));
}

#[test]
fn hosted_task_context_repo_checkout_request_without_repo_url() {
    let context = HostedTaskContext {
        repo_url: None,
        repository: Some("acme/orbit".to_string()),
        ..HostedTaskContext::default()
    };
    let request = context.repo_checkout_request("/tmp/workspace", "checkout-1");
    assert!(request.is_none());
}
