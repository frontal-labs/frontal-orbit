//! # Orbit Events
//! Event contracts and serialization utilities used by server, orchestrator, and downstream listeners.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fmt;

/// The canonical event envelope emitted over HTTP or WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventEnvelope {
    pub event: HostedEventName,
    pub status: HostedEventStatus,
    #[serde(rename = "emittedAt")]
    pub emitted_at: String,
    pub topic: HostedEventTopic,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lane_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Map<String, Value>>,
}

impl EventEnvelope {
    /// Create a new envelope with the current timestamp.
    pub fn new(
        event: HostedEventName,
        status: HostedEventStatus,
        topic: HostedEventTopic,
        identifiers: EventIdentifiers,
        payload: Option<Value>,
        metadata: Option<Map<String, Value>>,
    ) -> Self {
        Self {
            event,
            status,
            emitted_at: current_timestamp(),
            topic,
            workspace_id: identifiers.workspace_id,
            repo_id: identifiers.repo_id,
            lane_id: identifiers.lane_id,
            task_id: identifiers.task_id,
            payload,
            metadata,
        }
    }
}

/// Optional scoping identifiers carried with every event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct EventIdentifiers {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lane_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
}

/// Connector-scoped interaction request body used by connector callbacks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConnectorInteractionRequest {
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Value>,
}

/// Connector callback response payload, primarily used by interactive connectors like Slack.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConnectorInteractionResponse {
    pub blocks: Vec<Value>,
}

/// Connector event request body accepted by the hosted server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConnectorEventRequest {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    pub data: Value,
}

/// Typed payload emitted for `connector.event.received`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConnectorEventPayload {
    pub connector: String,
    pub source: String,
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    pub data: Value,
}

impl ConnectorEventPayload {
    pub fn new(
        connector: impl Into<String>,
        event_type: impl Into<String>,
        user_id: Option<String>,
        data: Value,
    ) -> Self {
        let connector = connector.into();
        Self {
            source: connector.clone(),
            connector,
            event_type: event_type.into(),
            user_id,
            data,
        }
    }
}

/// Applied orphan-handling policy included in task snapshots and event payloads.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppliedOrphanPolicy {
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_repository: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_priority: Option<String>,
    pub approval_delay_secs: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_retry_after_secs: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_cancel_after_secs: Option<u64>,
}

/// Shared task summary fields carried by most hosted task/lane/approval events.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct HostedTaskEventSummary {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_ts: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_message_ts: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr_number: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr_merged: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr_closed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr_merged_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_commit_sha: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub github_review_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub github_feedback_required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub github_feedback_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_backend: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_item_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worker_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worker_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orphan_policy: Option<AppliedOrphanPolicy>,
}

/// Event-specific payload for `task.routed`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TaskRoutedEventPayload {
    pub lane_count: usize,
}

/// Event-specific payload for lane signal events such as `lane.started` and `lane.blocked`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct LaneSignalEventPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport: Option<Value>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Event-specific payload for `approval.requested`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ApprovalRequestedEventPayload {
    pub approval_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Event-specific payload for `approval.resolved`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ApprovalResolvedEventPayload {
    pub approval_kind: String,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Event-specific payload for terminal and reconcile-style task/lane events.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TerminalEventPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_output: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reconciled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub derived_state: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Human-friendly naming that is still transport-safe.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HostedEventTopic {
    Task,
    Lane,
    Approval,
    Memory,
    Connector,
}

/// Specific event names for typed handling.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HostedEventName {
    #[serde(rename = "task.created")]
    TaskCreated,
    #[serde(rename = "task.routed")]
    TaskRouted,
    #[serde(rename = "task.cancelled")]
    TaskCancelled,
    #[serde(rename = "lane.started")]
    LaneStarted,
    #[serde(rename = "lane.blocked")]
    LaneBlocked,
    #[serde(rename = "lane.green")]
    LaneGreen,
    #[serde(rename = "lane.failed")]
    LaneFailed,
    #[serde(rename = "approval.requested")]
    ApprovalRequested,
    #[serde(rename = "approval.resolved")]
    ApprovalResolved,
    #[serde(rename = "memory.captured")]
    MemoryCaptured,
    #[serde(rename = "connector.event.received")]
    ConnectorEventReceived,
}

/// Status levels expressing progress inside a topic.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HostedEventStatus {
    Pending,
    Running,
    Blocked,
    Completed,
    Failed,
    Cancelled,
}

impl fmt::Display for HostedEventStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                HostedEventStatus::Pending => "pending",
                HostedEventStatus::Running => "running",
                HostedEventStatus::Blocked => "blocked",
                HostedEventStatus::Completed => "completed",
                HostedEventStatus::Failed => "failed",
                HostedEventStatus::Cancelled => "cancelled",
            }
        )
    }
}

fn current_timestamp() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// Render the shared TypeScript bindings for downstream connector clients.
pub fn render_typescript_bindings() -> String {
    [
        "// This file is generated by `cargo run -p orbit-events --bin export-typescript -- <output>`.",
        "// Do not edit by hand.",
        "",
        "export interface OrbitGeneratedAppliedOrphanPolicy {",
        "  source: string;",
        "  match_repository?: string;",
        "  match_source?: string;",
        "  match_priority?: string;",
        "  approval_delay_secs: number;",
        "  auto_retry_after_secs?: number;",
        "  auto_cancel_after_secs?: number;",
        "}",
        "",
        "export type OrbitGeneratedEventTopic =",
        "  | 'task'",
        "  | 'lane'",
        "  | 'approval'",
        "  | 'memory'",
        "  | 'connector';",
        "",
        "export type OrbitGeneratedEventStatus =",
        "  | 'pending'",
        "  | 'running'",
        "  | 'blocked'",
        "  | 'completed'",
        "  | 'failed'",
        "  | 'cancelled';",
        "",
        "export type OrbitGeneratedEventName =",
        "  | 'task.created'",
        "  | 'task.routed'",
        "  | 'task.cancelled'",
        "  | 'lane.started'",
        "  | 'lane.blocked'",
        "  | 'lane.green'",
        "  | 'lane.failed'",
        "  | 'approval.requested'",
        "  | 'approval.resolved'",
        "  | 'memory.captured'",
        "  | 'connector.event.received';",
        "",
        "export interface OrbitGeneratedHostedTaskEventSummary {",
        "  task_status?: string;",
        "  source?: string;",
        "  user_id?: string;",
        "  channel_id?: string;",
        "  thread_ts?: string;",
        "  approval_message_ts?: string;",
        "  repository?: string;",
        "  repo_url?: string;",
        "  base_ref?: string;",
        "  branch?: string;",
        "  published_branch?: string;",
        "  pr_url?: string;",
        "  pr_number?: number;",
        "  execution_backend?: string;",
        "  priority?: string;",
        "  plan_id?: string;",
        "  plan_kind?: string;",
        "  work_item_id?: string;",
        "  worker_id?: string;",
        "  worker_status?: string;",
        "  orphan_policy?: OrbitGeneratedAppliedOrphanPolicy;",
        "}",
        "",
        "export interface OrbitGeneratedTaskRoutedEventPayload {",
        "  lane_count?: number;",
        "}",
        "",
        "export interface OrbitGeneratedLaneSignalEventPayload {",
        "  role?: string;",
        "  description?: string;",
        "  detail?: string;",
        "  transport?: Record<string, unknown>;",
        "}",
        "",
        "export interface OrbitGeneratedApprovalRequestedEventPayload {",
        "  approval_kind?: string;",
        "  reason?: string;",
        "  detail?: string;",
        "}",
        "",
        "export interface OrbitGeneratedApprovalResolvedEventPayload {",
        "  approval_kind?: string;",
        "  action?: string;",
        "  resolved_by?: string;",
        "  reason?: string;",
        "}",
        "",
        "export interface OrbitGeneratedTerminalEventPayload {",
        "  finish_reason?: string;",
        "  tokens_output?: number;",
        "  result?: string;",
        "  error?: string;",
        "  detail?: string;",
        "  reconciled?: boolean;",
        "  derived_state?: string;",
        "}",
        "",
        "export interface OrbitGeneratedConnectorInteractionRequest {",
        "  action: string;",
        "  value?: string;",
        "  user_id?: string;",
        "  context?: unknown;",
        "}",
        "",
        "export interface OrbitGeneratedConnectorInteractionResponse {",
        "  blocks: Array<Record<string, unknown>>;",
        "}",
        "",
        "export interface OrbitGeneratedConnectorEventRequest {",
        "  type: string;",
        "  user_id?: string;",
        "  data: unknown;",
        "}",
        "",
        "export interface OrbitGeneratedConnectorEventPayload {",
        "  connector?: string;",
        "  source?: string;",
        "  type?: string;",
        "  user_id?: string;",
        "  data?: unknown;",
        "}",
        "",
        "export interface OrbitGeneratedEventPayload",
        "  extends OrbitGeneratedHostedTaskEventSummary,",
        "    OrbitGeneratedTaskRoutedEventPayload,",
        "    OrbitGeneratedLaneSignalEventPayload,",
        "    OrbitGeneratedApprovalRequestedEventPayload,",
        "    OrbitGeneratedApprovalResolvedEventPayload,",
        "    OrbitGeneratedTerminalEventPayload,",
        "    OrbitGeneratedConnectorEventPayload {",
        "  orphaned?: boolean;",
        "  orphaned_at?: number;",
        "  orphaned_for_secs?: number;",
        "  status?: string;",
        "  prompt?: string;",
        "}",
        "",
        "export interface OrbitGeneratedEventEnvelope {",
        "  event: OrbitGeneratedEventName;",
        "  status: OrbitGeneratedEventStatus;",
        "  emittedAt: string;",
        "  topic: OrbitGeneratedEventTopic;",
        "  workspace_id?: string;",
        "  repo_id?: string;",
        "  lane_id?: string;",
        "  task_id?: string;",
        "  payload?: OrbitGeneratedEventPayload;",
        "  metadata?: Record<string, unknown>;",
        "}",
        "",
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serializes_event_envelope() {
        let identifiers = EventIdentifiers {
            workspace_id: Some("ws-1".into()),
            repo_id: Some("repo-a".into()),
            lane_id: Some("lane-99".into()),
            task_id: Some("task-123".into()),
        };
        let envelope = EventEnvelope::new(
            HostedEventName::LaneStarted,
            HostedEventStatus::Running,
            HostedEventTopic::Lane,
            identifiers,
            Some(json!({"phase": "checkout"})),
            Some(Map::from_iter([
                ("source".to_string(), Value::String("cron".into())),
                ("priority".to_string(), Value::String("high".into())),
            ])),
        );

        let serialized = serde_json::to_value(&envelope).expect("should serialize");
        assert_eq!(serialized["event"], "lane.started");
        assert_eq!(serialized["status"], "running");
        assert_eq!(serialized["topic"], "lane");
        assert_eq!(serialized["task_id"], "task-123");
        assert_eq!(serialized["payload"], json!({"phase": "checkout"}));
        assert_eq!(serialized["metadata"]["source"], json!("cron"));
        assert!(serialized["emittedAt"].as_str().is_some());
    }

    #[test]
    fn roundtrip_event_name() {
        let serialized = serde_json::to_string(&HostedEventName::ApprovalRequested).unwrap();
        assert_eq!(serialized, "\"approval.requested\"");
        let deserialized: HostedEventName = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, HostedEventName::ApprovalRequested);
    }

    #[test]
    fn serializes_connector_event_payload() {
        let payload = ConnectorEventPayload::new(
            "slack",
            "reaction_added",
            Some("U123".into()),
            json!({ "reaction": "eyes" }),
        );

        let serialized = serde_json::to_value(&payload).expect("should serialize");
        assert_eq!(serialized["connector"], "slack");
        assert_eq!(serialized["source"], "slack");
        assert_eq!(serialized["type"], "reaction_added");
        assert_eq!(serialized["user_id"], "U123");
        assert_eq!(serialized["data"], json!({ "reaction": "eyes" }));
    }

    #[test]
    fn roundtrip_connector_event_request() {
        let serialized = json!({
            "type": "message",
            "user_id": "U123",
            "data": {
                "text": "hello"
            }
        });
        let request: ConnectorEventRequest = serde_json::from_value(serialized).unwrap();
        assert_eq!(request.event_type, "message");
        assert_eq!(request.user_id.as_deref(), Some("U123"));
        assert_eq!(request.data, json!({ "text": "hello" }));
    }

    #[test]
    fn serializes_hosted_task_event_summary() {
        let summary = HostedTaskEventSummary {
            task_status: Some("running".into()),
            source: Some("slack".into()),
            channel_id: Some("C123".into()),
            thread_ts: Some("1712345678.000100".into()),
            repository: Some("acme/payments".into()),
            repo_url: Some("https://github.com/acme/payments.git".into()),
            base_ref: Some("main".into()),
            branch: Some("orbit/fix-flake".into()),
            published_branch: Some("orbit/fix-flake".into()),
            pr_url: Some("https://github.com/acme/payments/pull/42".into()),
            pr_number: Some(42),
            execution_backend: Some("local_docker".into()),
            worker_id: Some("worker-1".into()),
            worker_status: Some("running".into()),
            orphan_policy: Some(AppliedOrphanPolicy {
                source: "default".into(),
                match_repository: None,
                match_source: None,
                match_priority: None,
                approval_delay_secs: 0,
                auto_retry_after_secs: Some(30),
                auto_cancel_after_secs: None,
            }),
            ..HostedTaskEventSummary::default()
        };

        let serialized = serde_json::to_value(summary).expect("should serialize");
        assert_eq!(serialized["task_status"], "running");
        assert_eq!(serialized["source"], "slack");
        assert_eq!(serialized["channel_id"], "C123");
        assert_eq!(serialized["thread_ts"], "1712345678.000100");
        assert_eq!(serialized["repository"], "acme/payments");
        assert_eq!(
            serialized["repo_url"],
            "https://github.com/acme/payments.git"
        );
        assert_eq!(serialized["base_ref"], "main");
        assert_eq!(serialized["branch"], "orbit/fix-flake");
        assert_eq!(serialized["published_branch"], "orbit/fix-flake");
        assert_eq!(
            serialized["pr_url"],
            "https://github.com/acme/payments/pull/42"
        );
        assert_eq!(serialized["pr_number"], 42);
        assert_eq!(serialized["execution_backend"], "local_docker");
        assert_eq!(serialized["worker_id"], "worker-1");
        assert_eq!(serialized["worker_status"], "running");
        assert_eq!(serialized["orphan_policy"]["source"], "default");
        assert_eq!(serialized["orphan_policy"]["auto_retry_after_secs"], 30);
    }

    #[test]
    fn serializes_approval_resolved_event_payload() {
        let payload = ApprovalResolvedEventPayload {
            approval_kind: "orphaned_hosted_agent".into(),
            action: "retry".into(),
            resolved_by: Some("U-ops".into()),
            reason: Some("operator chose retry".into()),
            extra: Map::from_iter([("worker_id".into(), json!("worker-2"))]),
        };

        let serialized = serde_json::to_value(payload).expect("should serialize");
        assert_eq!(serialized["approval_kind"], "orphaned_hosted_agent");
        assert_eq!(serialized["action"], "retry");
        assert_eq!(serialized["resolved_by"], "U-ops");
        assert_eq!(serialized["reason"], "operator chose retry");
        assert_eq!(serialized["worker_id"], "worker-2");
    }

    #[test]
    fn serializes_terminal_event_payload() {
        let payload = TerminalEventPayload {
            finish_reason: Some("manifest_reconcile".into()),
            tokens_output: Some(0),
            error: Some("provider failed".into()),
            detail: Some("task cancellation restored from hosted agent manifest".into()),
            reconciled: Some(true),
            derived_state: Some("failed".into()),
            extra: Map::from_iter([("worker_id".into(), json!("worker-1"))]),
            ..TerminalEventPayload::default()
        };

        let serialized = serde_json::to_value(payload).expect("should serialize");
        assert_eq!(serialized["finish_reason"], "manifest_reconcile");
        assert_eq!(serialized["tokens_output"], 0);
        assert_eq!(serialized["error"], "provider failed");
        assert_eq!(
            serialized["detail"],
            "task cancellation restored from hosted agent manifest"
        );
        assert_eq!(serialized["reconciled"], true);
        assert_eq!(serialized["derived_state"], "failed");
        assert_eq!(serialized["worker_id"], "worker-1");
    }

    #[test]
    fn renders_typescript_bindings_with_event_payloads() {
        let bindings = render_typescript_bindings();
        assert!(bindings.contains("OrbitGeneratedEventPayload"));
        assert!(bindings.contains("OrbitGeneratedHostedTaskEventSummary"));
        assert!(bindings.contains("OrbitGeneratedTerminalEventPayload"));
        assert!(bindings.contains("OrbitGeneratedEventEnvelope"));
        assert!(bindings.contains("OrbitGeneratedEventName"));
    }
}
