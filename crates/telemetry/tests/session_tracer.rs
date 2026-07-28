use std::sync::Arc;

use orbit_telemetry::{AnalyticsEvent, MemoryTelemetrySink, SessionTracer, TelemetryEvent};
use serde_json::{Map, Value};

fn make_attrs() -> Map<String, Value> {
    Map::new()
}

#[test]
fn session_tracer_new() {
    let sink = Arc::new(MemoryTelemetrySink::default());
    let tracer = SessionTracer::new("session-1", sink.clone());
    assert_eq!(tracer.session_id(), "session-1");
}

#[test]
fn session_tracer_record() {
    let sink = Arc::new(MemoryTelemetrySink::default());
    let tracer = SessionTracer::new("sess-1", sink.clone());
    tracer.record("event_name", make_attrs());
    let events = sink.events();
    assert_eq!(events.len(), 1);
    match &events[0] {
        TelemetryEvent::SessionTrace(record) => {
            assert_eq!(record.name, "event_name");
            assert_eq!(record.sequence, 0);
        }
        _ => panic!("expected SessionTrace"),
    }
}

#[test]
fn session_tracer_sequence_numbering() {
    let sink = Arc::new(MemoryTelemetrySink::default());
    let tracer = SessionTracer::new("sess-1", sink.clone());
    tracer.record("first", make_attrs());
    tracer.record("second", make_attrs());
    tracer.record("third", make_attrs());
    let events = sink.events();
    assert_eq!(events.len(), 3);
    let sequences: Vec<_> = events
        .iter()
        .map(|e| match e {
            TelemetryEvent::SessionTrace(r) => r.sequence,
            _ => panic!("expected SessionTrace"),
        })
        .collect();
    assert_eq!(sequences, vec![0, 1, 2]);
}

#[test]
fn session_tracer_record_http_request_started() {
    let sink = Arc::new(MemoryTelemetrySink::default());
    let tracer = SessionTracer::new("sess-1", sink.clone());
    tracer.record_http_request_started(1, "POST", "/v1/messages", make_attrs());
    let events = sink.events();
    assert_eq!(events.len(), 2);
    assert!(matches!(
        &events[0],
        TelemetryEvent::HttpRequestStarted { .. }
    ));
    assert!(matches!(&events[1], TelemetryEvent::SessionTrace(_)));
}

#[test]
fn session_tracer_record_http_request_succeeded() {
    let sink = Arc::new(MemoryTelemetrySink::default());
    let tracer = SessionTracer::new("sess-1", sink.clone());
    tracer.record_http_request_succeeded(
        1,
        "GET",
        "/health",
        200,
        Some("req-abc".to_string()),
        make_attrs(),
    );
    let events = sink.events();
    assert_eq!(events.len(), 2);
    match &events[0] {
        TelemetryEvent::HttpRequestSucceeded {
            status, request_id, ..
        } => {
            assert_eq!(*status, 200);
            assert_eq!(request_id.as_deref(), Some("req-abc"));
        }
        _ => panic!("expected HttpRequestSucceeded"),
    }
}

#[test]
fn session_tracer_record_http_request_failed() {
    let sink = Arc::new(MemoryTelemetrySink::default());
    let tracer = SessionTracer::new("sess-1", sink.clone());
    tracer.record_http_request_failed(
        2,
        "POST",
        "/v1/messages",
        "timeout error",
        true,
        make_attrs(),
    );
    let events = sink.events();
    assert_eq!(events.len(), 2);
    match &events[0] {
        TelemetryEvent::HttpRequestFailed {
            error, retryable, ..
        } => {
            assert_eq!(error, "timeout error");
            assert!(*retryable);
        }
        _ => panic!("expected HttpRequestFailed"),
    }
}

#[test]
fn session_tracer_record_analytics() {
    let sink = Arc::new(MemoryTelemetrySink::default());
    let tracer = SessionTracer::new("sess-1", sink.clone());
    let event = AnalyticsEvent::new("cli", "prompt_sent")
        .with_property("model", Value::String("claude".to_string()));
    tracer.record_analytics(event);
    let events = sink.events();
    assert_eq!(events.len(), 2);
    assert!(matches!(&events[0], TelemetryEvent::Analytics(_)));
    assert!(matches!(&events[1], TelemetryEvent::SessionTrace(_)));
}

#[test]
fn session_tracer_mixed_events_maintain_sequence() {
    let sink = Arc::new(MemoryTelemetrySink::default());
    let tracer = SessionTracer::new("sess-1", sink.clone());
    tracer.record("a", make_attrs());
    tracer.record_http_request_started(1, "GET", "/", make_attrs());
    tracer.record("b", make_attrs());
    tracer.record_analytics(AnalyticsEvent::new("c", "d"));
    let events = sink.events();
    let trace_events: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            TelemetryEvent::SessionTrace(r) => Some(r.sequence),
            _ => None,
        })
        .collect();
    assert_eq!(trace_events, vec![0, 1, 2, 3]);
}

#[test]
fn session_tracer_debug() {
    let sink = Arc::new(MemoryTelemetrySink::default());
    let tracer = SessionTracer::new("sess-debug", sink.clone());
    let debug = format!("{tracer:?}");
    assert!(debug.contains("sess-debug"));
}
