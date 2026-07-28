use orbit_events::{
    AppliedOrphanPolicy, ApprovalRequestedEventPayload, ApprovalResolvedEventPayload,
    ConnectorEventPayload, ConnectorInteractionRequest, ConnectorInteractionResponse,
    HostedTaskEventSummary, LaneSignalEventPayload, TaskRoutedEventPayload, TerminalEventPayload,
};
use serde_json::{json, Map};

#[test]
fn connector_interaction_request_roundtrip() {
    let original = ConnectorInteractionRequest {
        action: "button_click".to_string(),
        value: Some("confirm".to_string()),
        user_id: Some("U123".to_string()),
        context: Some(json!({"message_ts": "123456.789"})),
    };
    let json = serde_json::to_value(&original).unwrap();
    assert_eq!(json["action"], "button_click");
    assert_eq!(json["value"], "confirm");
    assert_eq!(json["user_id"], "U123");
    assert_eq!(json["context"]["message_ts"], "123456.789");

    let deserialized: ConnectorInteractionRequest = serde_json::from_value(json).unwrap();
    assert_eq!(original, deserialized);
}

#[test]
fn connector_interaction_request_minimal() {
    let original = ConnectorInteractionRequest {
        action: "submit".to_string(),
        value: None,
        user_id: None,
        context: None,
    };
    let json = serde_json::to_value(&original).unwrap();
    assert_eq!(json["action"], "submit");
    assert!(json.get("value").is_none());
    assert!(json.get("user_id").is_none());
    assert!(json.get("context").is_none());
}

#[test]
fn connector_interaction_response_roundtrip() {
    let original = ConnectorInteractionResponse {
        blocks: vec![
            json!({"type": "section", "text": "Hello"}),
            json!({"type": "divider"}),
        ],
    };
    let json = serde_json::to_value(&original).unwrap();
    assert_eq!(json["blocks"].as_array().unwrap().len(), 2);
    assert_eq!(json["blocks"][0]["type"], "section");

    let deserialized: ConnectorInteractionResponse = serde_json::from_value(json).unwrap();
    assert_eq!(original, deserialized);
}

#[test]
fn connector_event_request_roundtrip() {
    let request = orbit_events::ConnectorEventRequest {
        event_type: "message".to_string(),
        user_id: Some("U456".to_string()),
        data: json!({"text": "hello world", "channel": "C789"}),
    };
    let json = serde_json::to_value(&request).unwrap();
    assert_eq!(json["type"], "message");
    assert_eq!(json["user_id"], "U456");
    assert_eq!(json["data"]["text"], "hello world");

    let deserialized: orbit_events::ConnectorEventRequest = serde_json::from_value(json).unwrap();
    assert_eq!(request, deserialized);
}

#[test]
fn connector_event_payload_constructor() {
    let payload = ConnectorEventPayload::new(
        "slack",
        "reaction_added",
        Some("U789".to_string()),
        json!({"reaction": "eyes", "item": "msg"}),
    );
    assert_eq!(payload.connector, "slack");
    assert_eq!(payload.source, "slack");
    assert_eq!(payload.event_type, "reaction_added");
    assert_eq!(payload.user_id.as_deref(), Some("U789"));
    assert_eq!(payload.data["reaction"], "eyes");
}

#[test]
fn connector_event_payload_roundtrip() {
    let original = ConnectorEventPayload::new(
        "github",
        "push",
        None,
        json!({"ref": "refs/heads/main", "commits": []}),
    );
    let json = serde_json::to_value(&original).unwrap();
    assert_eq!(json["connector"], "github");
    assert!(json.get("user_id").is_none());

    let deserialized: ConnectorEventPayload = serde_json::from_value(json).unwrap();
    assert_eq!(original, deserialized);
}

#[test]
fn hosted_task_event_summary_full() {
    let summary = HostedTaskEventSummary {
        task_status: Some("running".into()),
        source: Some("slack".into()),
        user_id: Some("U123".into()),
        channel_id: Some("C456".into()),
        thread_ts: Some("1712345678.000100".into()),
        approval_message_ts: Some("1712345678.000200".into()),
        repository: Some("acme/payments".into()),
        repo_url: Some("https://github.com/acme/payments.git".into()),
        base_ref: Some("main".into()),
        branch: Some("feature/fix".into()),
        published_branch: Some("feature/fix".into()),
        pr_url: Some("https://github.com/acme/payments/pull/42".into()),
        pr_number: Some(42),
        pr_state: Some("open".into()),
        pr_merged: Some(false),
        pr_closed_at: None,
        pr_merged_at: None,
        published_commit_sha: Some("abc123".into()),
        github_review_state: Some("approved".into()),
        github_feedback_required: Some(false),
        github_feedback_reason: None,
        linear_issue_id: Some("LIN-123".into()),
        linear_issue_url: Some("https://linear.app/issue/LIN-123".into()),
        linear_issue_state: Some("in_progress".into()),
        linear_issue_identifier: Some("LIN-123".into()),
        graphite_stack_id: Some("stack-1".into()),
        graphite_head_branch: Some("feature/head".into()),
        graphite_base_branch: Some("main".into()),
        execution_backend: Some("local_docker".into()),
        priority: Some("high".into()),
        plan_id: Some("plan-1".into()),
        plan_kind: Some("feature".into()),
        work_item_id: Some("wi-1".into()),
        worker_id: Some("worker-1".into()),
        worker_status: Some("running".into()),
        orphan_policy: Some(AppliedOrphanPolicy {
            source: "default".into(),
            match_repository: None,
            match_source: None,
            match_priority: None,
            approval_delay_secs: 30,
            auto_retry_after_secs: Some(60),
            auto_cancel_after_secs: Some(300),
        }),
    };
    let json = serde_json::to_value(&summary).unwrap();
    assert_eq!(json["task_status"], "running");
    assert_eq!(json["source"], "slack");
    assert_eq!(json["repository"], "acme/payments");
    assert_eq!(json["pr_number"], 42);
    assert_eq!(json["linear_issue_id"], "LIN-123");
    assert_eq!(json["orphan_policy"]["approval_delay_secs"], 30);
    assert_eq!(json["orphan_policy"]["auto_retry_after_secs"], 60);
    assert!(json.get("pr_closed_at").is_none());

    let deserialized: HostedTaskEventSummary = serde_json::from_value(json).unwrap();
    assert_eq!(summary, deserialized);
}

#[test]
fn hosted_task_event_summary_default() {
    let summary = HostedTaskEventSummary::default();
    let json = serde_json::to_value(&summary).unwrap();
    let obj = json.as_object().unwrap();
    assert!(
        obj.is_empty(),
        "default summary should serialize to empty object"
    );
}

#[test]
fn task_routed_event_payload_roundtrip() {
    let original = TaskRoutedEventPayload { lane_count: 3 };
    let json = serde_json::to_value(&original).unwrap();
    assert_eq!(json["lane_count"], 3);

    let deserialized: TaskRoutedEventPayload = serde_json::from_value(json).unwrap();
    assert_eq!(original, deserialized);
}

#[test]
fn task_routed_event_payload_default() {
    let payload = TaskRoutedEventPayload::default();
    assert_eq!(payload.lane_count, 0);
}

#[test]
fn lane_signal_event_payload_roundtrip() {
    let original = LaneSignalEventPayload {
        role: Some("implementer".into()),
        description: Some("Implementation lane".into()),
        detail: Some("Working on feature X".into()),
        transport: Some(json!({"type": "websocket", "url": "wss://example.com"})),
        extra: Map::from_iter([("custom_field".into(), json!("value"))]),
    };
    let json = serde_json::to_value(&original).unwrap();
    assert_eq!(json["role"], "implementer");
    assert_eq!(json["description"], "Implementation lane");
    assert_eq!(json["transport"]["type"], "websocket");
    assert_eq!(json["custom_field"], "value");

    let deserialized: LaneSignalEventPayload = serde_json::from_value(json).unwrap();
    assert_eq!(original, deserialized);
}

#[test]
fn lane_signal_event_payload_minimal() {
    let original = LaneSignalEventPayload::default();
    let json = serde_json::to_value(&original).unwrap();
    assert!(json.as_object().unwrap().is_empty());
}

#[test]
fn approval_requested_event_payload_roundtrip() {
    let original = ApprovalRequestedEventPayload {
        approval_kind: "deploy".to_string(),
        reason: Some("Production deployment requires approval".into()),
        detail: Some("Deploying v2.1.0 to production".into()),
        extra: Map::from_iter([("environment".into(), json!("production"))]),
    };
    let json = serde_json::to_value(&original).unwrap();
    assert_eq!(json["approval_kind"], "deploy");
    assert_eq!(json["reason"], "Production deployment requires approval");
    assert_eq!(json["environment"], "production");

    let deserialized: ApprovalRequestedEventPayload = serde_json::from_value(json).unwrap();
    assert_eq!(original, deserialized);
}

#[test]
fn approval_resolved_event_payload_roundtrip() {
    let original = ApprovalResolvedEventPayload {
        approval_kind: "deploy".to_string(),
        action: "approved".to_string(),
        resolved_by: Some("U-ops".into()),
        reason: Some("Looks good".into()),
        extra: Map::from_iter([("worker_id".into(), json!("worker-2"))]),
    };
    let json = serde_json::to_value(&original).unwrap();
    assert_eq!(json["approval_kind"], "deploy");
    assert_eq!(json["action"], "approved");
    assert_eq!(json["resolved_by"], "U-ops");
    assert_eq!(json["worker_id"], "worker-2");

    let deserialized: ApprovalResolvedEventPayload = serde_json::from_value(json).unwrap();
    assert_eq!(original, deserialized);
}

#[test]
fn approval_resolved_minimal() {
    let original = ApprovalResolvedEventPayload {
        approval_kind: "test".to_string(),
        action: "denied".to_string(),
        resolved_by: None,
        reason: None,
        extra: Map::new(),
    };
    let json = serde_json::to_value(&original).unwrap();
    assert_eq!(json["approval_kind"], "test");
    assert_eq!(json["action"], "denied");
    assert!(json.get("resolved_by").is_none());
    assert!(json.get("reason").is_none());
}

#[test]
fn terminal_event_payload_roundtrip() {
    let original = TerminalEventPayload {
        finish_reason: Some("completed".into()),
        tokens_output: Some(1500),
        result: Some("Successfully implemented feature".into()),
        error: None,
        detail: Some("All tests passed".into()),
        reconciled: Some(true),
        derived_state: Some("completed".into()),
        extra: Map::from_iter([("duration_ms".into(), json!(4200))]),
    };
    let json = serde_json::to_value(&original).unwrap();
    assert_eq!(json["finish_reason"], "completed");
    assert_eq!(json["tokens_output"], 1500);
    assert_eq!(json["result"], "Successfully implemented feature");
    assert!(json.get("error").is_none());
    assert_eq!(json["reconciled"], true);
    assert_eq!(json["duration_ms"], 4200);

    let deserialized: TerminalEventPayload = serde_json::from_value(json).unwrap();
    assert_eq!(original, deserialized);
}

#[test]
fn terminal_event_payload_with_error() {
    let original = TerminalEventPayload {
        finish_reason: Some("error".into()),
        tokens_output: Some(200),
        result: None,
        error: Some("Provider timeout".into()),
        detail: Some("Task failed due to timeout".into()),
        reconciled: Some(false),
        derived_state: Some("failed".into()),
        extra: Map::new(),
    };
    let json = serde_json::to_value(&original).unwrap();
    assert_eq!(json["error"], "Provider timeout");
    assert!(json.get("result").is_none());
    assert_eq!(json["reconciled"], false);
}

#[test]
fn applied_orphan_policy_roundtrip() {
    let original = AppliedOrphanPolicy {
        source: "default".to_string(),
        match_repository: Some("acme/*".to_string()),
        match_source: None,
        match_priority: Some("high".to_string()),
        approval_delay_secs: 120,
        auto_retry_after_secs: None,
        auto_cancel_after_secs: Some(3600),
    };
    let json = serde_json::to_value(&original).unwrap();
    assert_eq!(json["source"], "default");
    assert_eq!(json["approval_delay_secs"], 120);
    assert_eq!(json["auto_cancel_after_secs"], 3600);
    assert!(json.get("auto_retry_after_secs").is_none());

    let deserialized: AppliedOrphanPolicy = serde_json::from_value(json).unwrap();
    assert_eq!(original, deserialized);
}
