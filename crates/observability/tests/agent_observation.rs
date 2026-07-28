use serde_json::{json, Map, Value};
use std::sync::Arc;

use orbit_observability::{
    AgentObservation, AgentObservationKind, AgentRunContext, AgentSpanKind, AgentSpanStatus,
    MemoryAgentObservationSink, Observability,
};

#[test]
fn agent_observation_kind_as_str() {
    assert_eq!(AgentObservationKind::RunStarted.as_str(), "run_started");
    assert_eq!(AgentObservationKind::RunCompleted.as_str(), "run_completed");
    assert_eq!(
        AgentObservationKind::ErrorCaptured.as_str(),
        "error_captured"
    );
    assert_eq!(AgentObservationKind::SpanStarted.as_str(), "span_started");
    assert_eq!(AgentObservationKind::SpanEvent.as_str(), "span_event");
    assert_eq!(AgentObservationKind::SpanFinished.as_str(), "span_finished");
}

#[test]
fn agent_span_kind_as_str() {
    assert_eq!(AgentSpanKind::Workflow.as_str(), "workflow");
    assert_eq!(AgentSpanKind::Turn.as_str(), "turn");
    assert_eq!(AgentSpanKind::Tool.as_str(), "tool");
    assert_eq!(AgentSpanKind::Model.as_str(), "model");
    assert_eq!(AgentSpanKind::Custom.as_str(), "custom");
}

#[test]
fn agent_span_status_as_str() {
    assert_eq!(AgentSpanStatus::Ok.as_str(), "ok");
    assert_eq!(AgentSpanStatus::Error.as_str(), "error");
    assert_eq!(AgentSpanStatus::Cancelled.as_str(), "cancelled");
}

#[test]
fn agent_run_context_new() {
    let ctx = AgentRunContext::new("agent-1", "run-1");
    assert_eq!(ctx.agent_id, "agent-1");
    assert_eq!(ctx.run_id, "run-1");
    assert!(ctx.session_id.is_none());
    assert!(ctx.workflow.is_none());
    assert!(ctx.provider.is_none());
    assert!(ctx.model.is_none());
    assert!(ctx.tags.is_empty());
}

#[test]
fn agent_run_context_with_session_id() {
    let ctx = AgentRunContext::new("a", "r").with_session_id("sess-99");
    assert_eq!(ctx.session_id.as_deref(), Some("sess-99"));
}

#[test]
fn agent_run_context_with_workflow() {
    let ctx = AgentRunContext::new("a", "r").with_workflow("interactive");
    assert_eq!(ctx.workflow.as_deref(), Some("interactive"));
}

#[test]
fn agent_run_context_with_provider() {
    let ctx = AgentRunContext::new("a", "r").with_provider("anthropic");
    assert_eq!(ctx.provider.as_deref(), Some("anthropic"));
}

#[test]
fn agent_run_context_with_model() {
    let ctx = AgentRunContext::new("a", "r").with_model("claude-opus");
    assert_eq!(ctx.model.as_deref(), Some("claude-opus"));
}

#[test]
fn agent_run_context_with_tag() {
    let ctx = AgentRunContext::new("a", "r").with_tag("env", "test");
    assert_eq!(
        ctx.tags.get("env"),
        Some(&Value::String("test".to_string()))
    );
}

#[test]
fn agent_run_context_chained() {
    let ctx = AgentRunContext::new("agent", "run-42")
        .with_session_id("sess-1")
        .with_workflow("batch")
        .with_provider("anthropic")
        .with_model("claude-sonnet")
        .with_tag("env", "prod")
        .with_tag("team", "ml");
    assert_eq!(ctx.agent_id, "agent");
    assert_eq!(ctx.run_id, "run-42");
    assert_eq!(ctx.session_id.as_deref(), Some("sess-1"));
    assert_eq!(ctx.workflow.as_deref(), Some("batch"));
    assert_eq!(ctx.provider.as_deref(), Some("anthropic"));
    assert_eq!(ctx.model.as_deref(), Some("claude-sonnet"));
    assert_eq!(ctx.tags.len(), 2);
}

#[test]
fn agent_observation_struct_creation() {
    let mut attrs = Map::new();
    attrs.insert("key".to_string(), json!("value"));
    let obs = AgentObservation {
        agent_id: "agent".to_string(),
        run_id: "run".to_string(),
        sequence: 0,
        timestamp_ms: 1000,
        kind: AgentObservationKind::RunStarted,
        name: "agent_run_started".to_string(),
        session_id: Some("sess-1".to_string()),
        span_id: None,
        parent_span_id: None,
        attributes: attrs,
    };
    assert_eq!(obs.agent_id, "agent");
    assert_eq!(obs.kind, AgentObservationKind::RunStarted);
    assert_eq!(obs.session_id.as_deref(), Some("sess-1"));
}

#[test]
fn agent_observation_with_span_ids() {
    let obs = AgentObservation {
        agent_id: "a".to_string(),
        run_id: "r".to_string(),
        sequence: 1,
        timestamp_ms: 2000,
        kind: AgentObservationKind::SpanStarted,
        name: "tool_call".to_string(),
        session_id: None,
        span_id: Some("span-0".to_string()),
        parent_span_id: Some("span-parent".to_string()),
        attributes: Map::new(),
    };
    assert_eq!(obs.span_id.as_deref(), Some("span-0"));
    assert_eq!(obs.parent_span_id.as_deref(), Some("span-parent"));
}

#[test]
fn memory_agent_observation_sink_records_and_retrieves() {
    let sink = Arc::new(MemoryAgentObservationSink::default());
    let obs = Observability::builder()
        .with_agent_sink(sink.clone())
        .build();
    let run = obs.start_agent_run(AgentRunContext::new("a", "r"));
    run.record_run_started();
    let observations = sink.observations();
    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].name, "agent_run_started");
}

#[test]
fn agent_observation_kind_serde() {
    let json = serde_json::to_string(&AgentObservationKind::RunStarted).unwrap();
    assert_eq!(json, "\"run_started\"");
    let deserialized: AgentObservationKind = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, AgentObservationKind::RunStarted);
}

#[test]
fn agent_span_kind_serde() {
    let json = serde_json::to_string(&AgentSpanKind::Tool).unwrap();
    assert_eq!(json, "\"tool\"");
    let deserialized: AgentSpanKind = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, AgentSpanKind::Tool);
}

#[test]
fn agent_span_status_serde() {
    let json = serde_json::to_string(&AgentSpanStatus::Cancelled).unwrap();
    assert_eq!(json, "\"cancelled\"");
    let deserialized: AgentSpanStatus = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, AgentSpanStatus::Cancelled);
}
