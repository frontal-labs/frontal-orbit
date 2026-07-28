#![allow(clippy::default_trait_access)]

use orbit_agents::{
    AgentObservation, AgentObservationKind, AgentObservationSink, AgentRunContext, AgentSpanKind,
    AgentSpanStatus, MemoryAgentObservationSink, MemorySentrySink, NoopAgentObservationSink,
    NoopSentrySink, Observability, SentryClient, SentryConfig, SentryEvent, SentryLevel,
    SentrySink,
};
use std::sync::Arc;

#[test]
fn re_exports_observability_default() {
    let observability = Observability::default();
    let run = observability.start_agent_run(AgentRunContext::new("agent", "run"));
    assert_eq!(run.context().agent_id, "agent");
    assert_eq!(run.context().run_id, "run");
}

#[test]
fn re_exports_observability_builder() {
    let sink = Arc::new(MemoryAgentObservationSink::default());
    let observability = Observability::builder()
        .with_agent_sink(sink.clone())
        .build();
    let run = observability.start_agent_run(AgentRunContext::new("builder", "r1"));
    run.record_run_started();
    assert_eq!(sink.observations().len(), 1);
}

#[test]
fn re_exports_sentry_config() {
    let disabled = SentryConfig::disabled();
    assert!(!disabled.is_enabled());
    let enabled = SentryConfig::enabled("https://key@o0.ingest.sentry.io/1");
    assert!(enabled.is_enabled());
}

#[test]
fn re_exports_sentry_client() {
    let sink = Arc::new(MemorySentrySink::default());
    let client = SentryClient::new(
        SentryConfig::enabled("https://key@o0.ingest.sentry.io/1"),
        sink.clone(),
    );
    client.capture(SentryEvent::error("test error"));
    assert_eq!(sink.events().len(), 1);
}

#[test]
fn re_exports_noop_sinks() {
    let noop_sentry = NoopSentrySink;
    let noop_obs = NoopAgentObservationSink;
    noop_sentry.capture(SentryEvent::error("noop"));
    noop_obs.record(AgentObservation {
        agent_id: "a".to_string(),
        run_id: "r".to_string(),
        sequence: 0,
        timestamp_ms: 0,
        kind: AgentObservationKind::RunStarted,
        name: "test".to_string(),
        session_id: None,
        span_id: None,
        parent_span_id: None,
        attributes: Default::default(),
    });
}

#[test]
fn re_exports_agent_run_context_builder() {
    let ctx = AgentRunContext::new("agent", "run-1")
        .with_session_id("sess-1")
        .with_workflow("test")
        .with_provider("anthropic")
        .with_model("claude")
        .with_tag("env", "test");
    assert_eq!(ctx.agent_id, "agent");
    assert_eq!(ctx.run_id, "run-1");
    assert_eq!(ctx.session_id.as_deref(), Some("sess-1"));
    assert_eq!(ctx.workflow.as_deref(), Some("test"));
    assert_eq!(ctx.provider.as_deref(), Some("anthropic"));
    assert_eq!(ctx.model.as_deref(), Some("claude"));
}

#[test]
fn re_exports_agent_observation_kind_as_str() {
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
fn re_exports_agent_span_kind_as_str() {
    assert_eq!(AgentSpanKind::Workflow.as_str(), "workflow");
    assert_eq!(AgentSpanKind::Turn.as_str(), "turn");
    assert_eq!(AgentSpanKind::Tool.as_str(), "tool");
    assert_eq!(AgentSpanKind::Model.as_str(), "model");
    assert_eq!(AgentSpanKind::Custom.as_str(), "custom");
}

#[test]
fn re_exports_agent_span_status_as_str() {
    assert_eq!(AgentSpanStatus::Ok.as_str(), "ok");
    assert_eq!(AgentSpanStatus::Error.as_str(), "error");
    assert_eq!(AgentSpanStatus::Cancelled.as_str(), "cancelled");
}

#[test]
fn re_exports_sentry_event_construction() {
    let event = SentryEvent::new("msg", SentryLevel::Warning)
        .with_tag("key", "value")
        .with_extra("num", 42)
        .with_fingerprint("fp1");
    assert_eq!(event.message, "msg");
    assert_eq!(event.level, SentryLevel::Warning);
}

#[test]
fn re_exports_sentry_event_error() {
    let event = SentryEvent::error("oops");
    assert_eq!(event.level, SentryLevel::Error);
}

#[test]
fn re_exports_memory_sentry_sink() {
    let sink = MemorySentrySink::default();
    let sink_arc = Arc::new(sink);
    let client = SentryClient::new(
        SentryConfig::enabled("https://key@o0.ingest.sentry.io/1"),
        sink_arc.clone(),
    );
    client.capture(SentryEvent::error("e1"));
    client.capture(SentryEvent::error("e2"));
    assert_eq!(sink_arc.events().len(), 2);
}

#[test]
fn re_exports_memory_agent_observation_sink() {
    let sink = Arc::new(MemoryAgentObservationSink::default());
    let agent_run = AgentRunContext::new("test-agent", "test-run");
    let obs = Observability::builder()
        .with_agent_sink(sink.clone())
        .build();
    let run = obs.start_agent_run(agent_run);
    run.record_run_started();
    assert_eq!(sink.observations().len(), 1);
}

#[test]
fn re_exports_observability_builder_with_sentry() {
    let sentry_sink = Arc::new(MemorySentrySink::default());
    let agent_sink = Arc::new(MemoryAgentObservationSink::default());
    let obs = Observability::builder()
        .with_sentry_client(SentryClient::new(
            SentryConfig::enabled("https://key@o0.ingest.sentry.io/1"),
            sentry_sink.clone(),
        ))
        .with_agent_sink(agent_sink.clone())
        .build();
    let run = obs.start_agent_run(AgentRunContext::new("a", "r"));
    run.capture_error("comp", "fail", Default::default());
    assert_eq!(sentry_sink.events().len(), 1);
    assert_eq!(agent_sink.observations().len(), 1);
}

#[test]
fn re_exports_agent_span_operations() {
    let sink = Arc::new(MemoryAgentObservationSink::default());
    let obs = Observability::builder()
        .with_agent_sink(sink.clone())
        .build();
    let run = obs.start_agent_run(AgentRunContext::new("a", "r"));
    let span = run.start_span(AgentSpanKind::Tool, "bash");
    assert!(span.span_id().starts_with("span-"));
    assert_eq!(span.name(), "bash");
    let child = span.start_child_span(AgentSpanKind::Custom, "sub");
    assert!(child.span_id().starts_with("span-"));
    child.add_event("event", Default::default());
    child.finish(AgentSpanStatus::Ok, Default::default());
    assert!(!sink.observations().is_empty());
}

#[test]
fn re_exports_sentry_config_with_environment() {
    let config = SentryConfig::enabled("https://key@o0.ingest.sentry.io/1")
        .with_environment("production")
        .with_release("1.0.0")
        .with_server_name("box1")
        .with_traces_sample_rate(0.5)
        .with_profiles_sample_rate(0.1);
    assert_eq!(config.environment.as_deref(), Some("production"));
    assert_eq!(config.release.as_deref(), Some("1.0.0"));
    assert_eq!(config.server_name.as_deref(), Some("box1"));
    assert_eq!(config.traces_sample_rate, Some(0.5));
    assert_eq!(config.profiles_sample_rate, Some(0.1));
}

#[test]
fn re_exports_session_tracer_agent_observation_sink() {
    let sink = Arc::new(MemoryAgentObservationSink::default());
    let obs = Observability::builder()
        .with_agent_sink(sink.clone())
        .build();
    let run = obs.start_agent_run(AgentRunContext::new("a", "r"));
    run.record_run_started();
    assert!(!sink.observations().is_empty());
}
