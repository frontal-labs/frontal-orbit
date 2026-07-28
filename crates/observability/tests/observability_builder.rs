use std::sync::Arc;

use orbit_observability::{
    MemoryAgentObservationSink, MemorySentrySink, Observability, ObservabilityBuilder,
    SentryClient, SentryConfig, SentryEvent,
};
use serde_json::Map;

#[test]
fn observability_default() {
    let obs = Observability::default();
    let run = obs.start_agent_run(orbit_observability::AgentRunContext::new("a", "r"));
    assert_eq!(run.context().agent_id, "a");
}

#[test]
fn observability_builder_default() {
    let obs = ObservabilityBuilder::default().build();
    let run = obs.start_agent_run(orbit_observability::AgentRunContext::new("a", "r"));
    assert_eq!(run.context().agent_id, "a");
}

#[test]
fn observability_builder_with_agent_sink() {
    let sink = Arc::new(MemoryAgentObservationSink::default());
    let obs = Observability::builder()
        .with_agent_sink(sink.clone())
        .build();
    let run = obs.start_agent_run(orbit_observability::AgentRunContext::new("a", "r"));
    run.record_run_started();
    assert_eq!(sink.observations().len(), 1);
}

#[test]
fn observability_builder_with_sentry_client() {
    let sentry_sink = Arc::new(MemorySentrySink::default());
    let obs = Observability::builder()
        .with_sentry_client(SentryClient::new(
            SentryConfig::enabled("https://key@o0.ingest.sentry.io/1"),
            sentry_sink.clone(),
        ))
        .build();
    obs.sentry().capture(SentryEvent::error("test"));
    assert_eq!(sentry_sink.events().len(), 1);
}

#[test]
fn observability_builder_with_both_sinks() {
    let sentry_sink = Arc::new(MemorySentrySink::default());
    let agent_sink = Arc::new(MemoryAgentObservationSink::default());
    let obs = Observability::builder()
        .with_sentry_client(SentryClient::new(
            SentryConfig::enabled("https://key@o0.ingest.sentry.io/1"),
            sentry_sink.clone(),
        ))
        .with_agent_sink(agent_sink.clone())
        .build();
    let run = obs.start_agent_run(orbit_observability::AgentRunContext::new("a", "r"));
    run.record_run_started();
    run.capture_error("comp", "err", Map::default());
    assert_eq!(agent_sink.observations().len(), 2);
    assert_eq!(sentry_sink.events().len(), 1);
}

#[test]
fn memory_sentry_sink_empty_initially() {
    let sink = MemorySentrySink::default();
    assert!(sink.events().is_empty());
}

#[test]
fn memory_sentry_sink_captures_multiple_events() {
    let sink = Arc::new(MemorySentrySink::default());
    let client = SentryClient::new(
        SentryConfig::enabled("https://key@o0.ingest.sentry.io/1"),
        sink.clone(),
    );
    client.capture(SentryEvent::error("e1"));
    client.capture(SentryEvent::error("e2"));
    client.capture(SentryEvent::error("e3"));
    assert_eq!(sink.events().len(), 3);
}

#[test]
fn memory_agent_observation_sink_empty_initially() {
    let sink = MemoryAgentObservationSink::default();
    assert!(sink.observations().is_empty());
}

#[test]
fn memory_agent_observation_sink_records_multiple() {
    let sink = Arc::new(MemoryAgentObservationSink::default());
    let obs = Observability::builder()
        .with_agent_sink(sink.clone())
        .build();
    let run = obs.start_agent_run(orbit_observability::AgentRunContext::new("a", "r"));
    run.record_run_started();
    let _ = run.start_span(orbit_observability::AgentSpanKind::Turn, "turn-1");
    run.record_run_completed(Map::default());
    assert_eq!(sink.observations().len(), 3);
}

#[test]
fn observability_sentry_returns_ref() {
    let config = SentryConfig::enabled("https://key@o0.ingest.sentry.io/1");
    let client = SentryClient::new(config.clone(), Arc::new(MemorySentrySink::default()));
    let obs = Observability::builder().with_sentry_client(client).build();
    assert_eq!(obs.sentry().config(), &config);
}

#[test]
fn observability_clone() {
    let obs = Observability::default();
    let cloned = obs;
    let _run = cloned.start_agent_run(orbit_observability::AgentRunContext::new("a", "r"));
}
