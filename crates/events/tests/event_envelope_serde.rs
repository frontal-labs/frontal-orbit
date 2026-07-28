use orbit_events::{
    EventEnvelope, EventIdentifiers, HostedEventName, HostedEventStatus, HostedEventTopic,
};
use serde_json::{json, Map, Value};

#[test]
fn serializes_full_envelope() {
    let identifiers = EventIdentifiers {
        workspace_id: Some("ws-1".into()),
        repo_id: Some("repo-a".into()),
        lane_id: Some("lane-99".into()),
        task_id: Some("task-123".into()),
    };
    let envelope = EventEnvelope::new(
        HostedEventName::TaskCreated,
        HostedEventStatus::Completed,
        HostedEventTopic::Task,
        identifiers,
        Some(json!({"result": "ok"})),
        Some(Map::from_iter([(
            "source".to_string(),
            Value::String("webhook".into()),
        )])),
    );
    let json = serde_json::to_value(&envelope).unwrap();
    assert_eq!(json["event"], "task.created");
    assert_eq!(json["status"], "completed");
    assert_eq!(json["topic"], "task");
    assert_eq!(json["workspace_id"], "ws-1");
    assert_eq!(json["repo_id"], "repo-a");
    assert_eq!(json["lane_id"], "lane-99");
    assert_eq!(json["task_id"], "task-123");
    assert!(json.get("emittedAt").and_then(Value::as_str).is_some());
    assert_eq!(json["payload"]["result"], "ok");
    assert_eq!(json["metadata"]["source"], "webhook");
}

#[test]
fn serializes_minimal_envelope() {
    let envelope = EventEnvelope::new(
        HostedEventName::MemoryCaptured,
        HostedEventStatus::Running,
        HostedEventTopic::Memory,
        EventIdentifiers::default(),
        None,
        None,
    );
    let json = serde_json::to_value(&envelope).unwrap();
    assert_eq!(json["event"], "memory.captured");
    assert_eq!(json["status"], "running");
    assert_eq!(json["topic"], "memory");
    assert!(json.get("payload").is_none());
    assert!(json.get("workspace_id").is_none());
    assert!(json.get("metadata").is_none());
    assert!(json.get("repo_id").is_none());
    assert!(json.get("lane_id").is_none());
    assert!(json.get("task_id").is_none());
}

#[test]
fn roundtrip_event_envelope() {
    let original = EventEnvelope::new(
        HostedEventName::ApprovalResolved,
        HostedEventStatus::Completed,
        HostedEventTopic::Approval,
        EventIdentifiers {
            workspace_id: Some("ws-1".into()),
            ..EventIdentifiers::default()
        },
        Some(json!({"approved": true})),
        None,
    );
    let bytes = serde_json::to_vec(&original).unwrap();
    let deserialized: EventEnvelope = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(original, deserialized);
    assert_eq!(deserialized.event, HostedEventName::ApprovalResolved);
    assert_eq!(deserialized.status, HostedEventStatus::Completed);
}

#[test]
fn roundtrip_with_payload_and_metadata() {
    let original = EventEnvelope::new(
        HostedEventName::ConnectorEventReceived,
        HostedEventStatus::Pending,
        HostedEventTopic::Connector,
        EventIdentifiers {
            lane_id: Some("lane-1".into()),
            ..EventIdentifiers::default()
        },
        Some(json!({"connector": "slack", "type": "message"})),
        Some(Map::from_iter([(
            "version".to_string(),
            Value::Number(1.into()),
        )])),
    );
    let json = serde_json::to_value(&original).unwrap();
    let deserialized: EventEnvelope = serde_json::from_value(json).unwrap();
    assert_eq!(original, deserialized);
}

#[test]
fn all_event_names_roundtrip() {
    let names = [
        HostedEventName::TaskCreated,
        HostedEventName::TaskRouted,
        HostedEventName::TaskCancelled,
        HostedEventName::LaneStarted,
        HostedEventName::LaneBlocked,
        HostedEventName::LaneGreen,
        HostedEventName::LaneFailed,
        HostedEventName::ApprovalRequested,
        HostedEventName::ApprovalResolved,
        HostedEventName::MemoryCaptured,
        HostedEventName::ConnectorEventReceived,
    ];
    for name in names {
        let json = serde_json::to_value(&name).unwrap();
        let deserialized: HostedEventName = serde_json::from_value(json).unwrap();
        assert_eq!(name, deserialized);
    }
}

#[test]
fn all_event_statuses_roundtrip() {
    let statuses = [
        HostedEventStatus::Pending,
        HostedEventStatus::Running,
        HostedEventStatus::Blocked,
        HostedEventStatus::Completed,
        HostedEventStatus::Failed,
        HostedEventStatus::Cancelled,
    ];
    for status in statuses {
        let json = serde_json::to_value(&status).unwrap();
        let deserialized: HostedEventStatus = serde_json::from_value(json).unwrap();
        assert_eq!(status, deserialized);
    }
}

#[test]
fn all_event_topics_roundtrip() {
    let topics = [
        HostedEventTopic::Task,
        HostedEventTopic::Lane,
        HostedEventTopic::Approval,
        HostedEventTopic::Memory,
        HostedEventTopic::Connector,
    ];
    for topic in topics {
        let json = serde_json::to_value(&topic).unwrap();
        let deserialized: HostedEventTopic = serde_json::from_value(json).unwrap();
        assert_eq!(topic, deserialized);
    }
}

#[test]
fn status_serializes_as_snake_case() {
    assert_eq!(
        serde_json::to_value(HostedEventStatus::Pending).unwrap(),
        "pending"
    );
    assert_eq!(
        serde_json::to_value(HostedEventStatus::Running).unwrap(),
        "running"
    );
    assert_eq!(
        serde_json::to_value(HostedEventStatus::Blocked).unwrap(),
        "blocked"
    );
    assert_eq!(
        serde_json::to_value(HostedEventStatus::Completed).unwrap(),
        "completed"
    );
    assert_eq!(
        serde_json::to_value(HostedEventStatus::Failed).unwrap(),
        "failed"
    );
    assert_eq!(
        serde_json::to_value(HostedEventStatus::Cancelled).unwrap(),
        "cancelled"
    );
}

#[test]
fn topic_serializes_as_snake_case() {
    assert_eq!(
        serde_json::to_value(HostedEventTopic::Task).unwrap(),
        "task"
    );
    assert_eq!(
        serde_json::to_value(HostedEventTopic::Lane).unwrap(),
        "lane"
    );
    assert_eq!(
        serde_json::to_value(HostedEventTopic::Approval).unwrap(),
        "approval"
    );
    assert_eq!(
        serde_json::to_value(HostedEventTopic::Memory).unwrap(),
        "memory"
    );
    assert_eq!(
        serde_json::to_value(HostedEventTopic::Connector).unwrap(),
        "connector"
    );
}

#[test]
fn event_name_serializes_to_correct_string() {
    assert_eq!(
        serde_json::to_value(HostedEventName::TaskCreated).unwrap(),
        "task.created"
    );
    assert_eq!(
        serde_json::to_value(HostedEventName::TaskRouted).unwrap(),
        "task.routed"
    );
    assert_eq!(
        serde_json::to_value(HostedEventName::ApprovalRequested).unwrap(),
        "approval.requested"
    );
    assert_eq!(
        serde_json::to_value(HostedEventName::ConnectorEventReceived).unwrap(),
        "connector.event.received"
    );
}

#[test]
fn display_format_for_status() {
    assert_eq!(HostedEventStatus::Pending.to_string(), "pending");
    assert_eq!(HostedEventStatus::Running.to_string(), "running");
    assert_eq!(HostedEventStatus::Blocked.to_string(), "blocked");
    assert_eq!(HostedEventStatus::Completed.to_string(), "completed");
    assert_eq!(HostedEventStatus::Failed.to_string(), "failed");
    assert_eq!(HostedEventStatus::Cancelled.to_string(), "cancelled");
}

#[test]
fn optional_identifiers_are_skipped_when_none() {
    let envelope = EventEnvelope::new(
        HostedEventName::LaneStarted,
        HostedEventStatus::Running,
        HostedEventTopic::Lane,
        EventIdentifiers::default(),
        None,
        None,
    );
    let json = serde_json::to_value(&envelope).unwrap();
    let obj = json.as_object().unwrap();
    assert!(!obj.contains_key("workspace_id"));
    assert!(!obj.contains_key("repo_id"));
    assert!(!obj.contains_key("lane_id"));
    assert!(!obj.contains_key("task_id"));
    assert!(!obj.contains_key("payload"));
    assert!(!obj.contains_key("metadata"));
}

#[test]
fn event_identifiers_defaults_to_all_none() {
    let ids = EventIdentifiers::default();
    let json = serde_json::to_value(&ids).unwrap();
    let obj = json.as_object().unwrap();
    assert!(
        obj.is_empty(),
        "default identifiers should produce empty JSON map"
    );
}

#[test]
fn partial_identifiers_serializes_only_populated_fields() {
    let ids = EventIdentifiers {
        task_id: Some("task-42".into()),
        ..EventIdentifiers::default()
    };
    let json = serde_json::to_value(&ids).unwrap();
    assert_eq!(json["task_id"], "task-42");
    assert!(json.get("workspace_id").is_none());
    assert!(json.get("repo_id").is_none());
    assert!(json.get("lane_id").is_none());
}

#[test]
fn emitted_at_field_is_iso8601_timestamp() {
    let envelope = EventEnvelope::new(
        HostedEventName::TaskCreated,
        HostedEventStatus::Pending,
        HostedEventTopic::Task,
        EventIdentifiers::default(),
        None,
        None,
    );
    let json = serde_json::to_value(&envelope).unwrap();
    let emitted = json["emittedAt"].as_str().unwrap();
    assert!(
        emitted.contains('T'),
        "expected ISO 8601 timestamp, got {emitted}"
    );
    assert!(
        emitted.ends_with('Z'),
        "expected UTC timestamp, got {emitted}"
    );
}
