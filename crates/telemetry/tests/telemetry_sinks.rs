use std::sync::Arc;

use orbit_telemetry::{AnalyticsEvent, MemoryTelemetrySink, TelemetryEvent, TelemetrySink};
use serde_json::{Map, Value};

#[test]
fn memory_telemetry_sink_empty_initially() {
    let sink = MemoryTelemetrySink::default();
    assert!(sink.events().is_empty());
}

#[test]
fn memory_telemetry_sink_records_events() {
    let sink = Arc::new(MemoryTelemetrySink::default());
    let event = TelemetryEvent::Analytics(AnalyticsEvent::new("cli", "start"));
    sink.record(event.clone());
    assert_eq!(sink.events(), vec![event]);
}

#[test]
fn memory_telemetry_sink_records_multiple_events() {
    let sink = Arc::new(MemoryTelemetrySink::default());
    sink.record(TelemetryEvent::Analytics(AnalyticsEvent::new("cli", "a")));
    sink.record(TelemetryEvent::Analytics(AnalyticsEvent::new("cli", "b")));
    sink.record(TelemetryEvent::Analytics(AnalyticsEvent::new("cli", "c")));
    assert_eq!(sink.events().len(), 3);
}

#[test]
fn memory_telemetry_sink_thread_safe() {
    let sink = Arc::new(MemoryTelemetrySink::default());
    let sink2 = sink.clone();
    let sink3 = sink.clone();
    let t1 = std::thread::spawn(move || {
        for _ in 0..10 {
            sink2.record(TelemetryEvent::Analytics(AnalyticsEvent::new("t1", "e")));
        }
    });
    let t2 = std::thread::spawn(move || {
        for _ in 0..10 {
            sink3.record(TelemetryEvent::Analytics(AnalyticsEvent::new("t2", "e")));
        }
    });
    t1.join().unwrap();
    t2.join().unwrap();
    assert_eq!(sink.events().len(), 20);
}

#[test]
fn analytics_event_construction() {
    let event = AnalyticsEvent::new("namespace", "action");
    assert_eq!(event.namespace, "namespace");
    assert_eq!(event.action, "action");
    assert!(event.properties.is_empty());
}

#[test]
fn analytics_event_with_properties() {
    let event = AnalyticsEvent::new("cli", "send")
        .with_property("model", Value::String("claude".to_string()))
        .with_property("tokens", Value::from(100));
    assert_eq!(event.properties.len(), 2);
    assert_eq!(
        event.properties.get("model"),
        Some(&Value::String("claude".to_string()))
    );
}

#[test]
fn telemetry_event_http_request_started() {
    let event = TelemetryEvent::HttpRequestStarted {
        session_id: "sess-1".to_string(),
        attempt: 1,
        method: "POST".to_string(),
        path: "/v1/messages".to_string(),
        attributes: Map::default(),
    };
    match event {
        TelemetryEvent::HttpRequestStarted {
            session_id, method, ..
        } => {
            assert_eq!(session_id, "sess-1");
            assert_eq!(method, "POST");
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn telemetry_event_http_request_succeeded() {
    let event = TelemetryEvent::HttpRequestSucceeded {
        session_id: "sess-1".to_string(),
        attempt: 1,
        method: "GET".to_string(),
        path: "/health".to_string(),
        status: 200,
        request_id: Some("req-1".to_string()),
        attributes: Map::default(),
    };
    match event {
        TelemetryEvent::HttpRequestSucceeded {
            status, request_id, ..
        } => {
            assert_eq!(status, 200);
            assert_eq!(request_id.as_deref(), Some("req-1"));
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn telemetry_event_http_request_failed() {
    let event = TelemetryEvent::HttpRequestFailed {
        session_id: "sess-1".to_string(),
        attempt: 2,
        method: "POST".to_string(),
        path: "/v1/messages".to_string(),
        error: "timeout".to_string(),
        retryable: true,
        attributes: Map::default(),
    };
    match event {
        TelemetryEvent::HttpRequestFailed {
            error, retryable, ..
        } => {
            assert_eq!(error, "timeout");
            assert!(retryable);
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn telemetry_event_session_trace() {
    let event = TelemetryEvent::SessionTrace(orbit_telemetry::SessionTraceRecord {
        session_id: "sess-1".to_string(),
        sequence: 0,
        name: "trace".to_string(),
        timestamp_ms: 1000,
        attributes: Map::default(),
    });
    match event {
        TelemetryEvent::SessionTrace(record) => {
            assert_eq!(record.name, "trace");
            assert_eq!(record.sequence, 0);
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn analytics_event_clone() {
    let event = AnalyticsEvent::new("ns", "act").with_property("k", Value::from(1));
    let cloned = event.clone();
    assert_eq!(event, cloned);
}

#[test]
fn telemetry_event_serde_roundtrip() {
    let event = TelemetryEvent::Analytics(
        AnalyticsEvent::new("cli", "ping").with_property("ok", Value::Bool(true)),
    );
    let json = serde_json::to_string(&event).unwrap();
    let deserialized: TelemetryEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(event, deserialized);
}
