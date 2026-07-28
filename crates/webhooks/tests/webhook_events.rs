use orbit_webhooks::{EventProcessor, WebhookEvent, WebhookEventType};
use std::collections::BTreeMap;

#[test]
fn webhook_event_github() {
    let event = WebhookEvent::new(
        WebhookEventType::GitHub {
            action: "push".to_string(),
        },
        "github".to_string(),
        serde_json::json!({"ref": "main"}),
        BTreeMap::new(),
    );
    assert!(event.is_github());
    assert!(!event.is_mcp());
    assert_eq!(event.source, "github");
}

#[test]
fn webhook_event_mcp() {
    let event = WebhookEvent::new(
        WebhookEventType::McpServer {
            server_name: "server-1".to_string(),
            event: "started".to_string(),
        },
        "mcp".to_string(),
        serde_json::json!({}),
        BTreeMap::new(),
    );
    assert!(event.is_mcp());
    assert!(!event.is_github());
}

#[test]
fn webhook_event_custom() {
    let event = WebhookEvent::new(
        WebhookEventType::Custom {
            name: "test_event".to_string(),
        },
        "custom".to_string(),
        serde_json::json!({"data": 42}),
        BTreeMap::new(),
    );
    assert!(!event.is_github());
    assert!(!event.is_mcp());
}

#[test]
fn webhook_event_generic() {
    let event = WebhookEvent::new(
        WebhookEventType::Generic,
        "generic".to_string(),
        serde_json::json!("raw"),
        BTreeMap::new(),
    );
    assert!(!event.is_github());
    assert!(!event.is_mcp());
}

#[test]
fn webhook_event_header_access() {
    let mut headers = BTreeMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    let event = WebhookEvent::new(
        WebhookEventType::Generic,
        "test".to_string(),
        serde_json::json!({}),
        headers,
    );
    assert_eq!(
        event.header("Content-Type"),
        Some(&"application/json".to_string())
    );
    assert_eq!(event.header("Non-Existent"), None);
}

#[test]
fn event_processor_empty() {
    let processor = EventProcessor::new(10);
    assert!(processor.is_empty());
    assert_eq!(processor.len(), 0);
}

#[test]
fn event_processor_add_and_retrieve() {
    let mut processor = EventProcessor::new(10);
    let event = WebhookEvent::new(
        WebhookEventType::Generic,
        "source-a".to_string(),
        serde_json::json!({}),
        BTreeMap::new(),
    );
    processor.add_event(event);
    assert_eq!(processor.len(), 1);
    assert!(!processor.is_empty());
}

#[test]
fn event_processor_events_by_source() {
    let mut processor = EventProcessor::new(10);
    processor.add_event(WebhookEvent::new(
        WebhookEventType::Generic,
        "source-a".to_string(),
        serde_json::json!({}),
        BTreeMap::new(),
    ));
    processor.add_event(WebhookEvent::new(
        WebhookEventType::Generic,
        "source-b".to_string(),
        serde_json::json!({}),
        BTreeMap::new(),
    ));
    processor.add_event(WebhookEvent::new(
        WebhookEventType::Generic,
        "source-a".to_string(),
        serde_json::json!({}),
        BTreeMap::new(),
    ));

    let from_a = processor.events_by_source("source-a");
    assert_eq!(from_a.len(), 2);
}

#[test]
fn event_processor_events_by_type() {
    let mut processor = EventProcessor::new(10);
    let github = WebhookEvent::new(
        WebhookEventType::GitHub {
            action: "push".to_string(),
        },
        "github".to_string(),
        serde_json::json!({}),
        BTreeMap::new(),
    );
    let generic = WebhookEvent::new(
        WebhookEventType::Generic,
        "other".to_string(),
        serde_json::json!({}),
        BTreeMap::new(),
    );
    processor.add_event(github);
    processor.add_event(generic);

    let gh_events = processor.events_by_type(&WebhookEventType::GitHub {
        action: "push".to_string(),
    });
    assert_eq!(gh_events.len(), 1);
}

#[test]
fn event_processor_clear() {
    let mut processor = EventProcessor::new(10);
    processor.add_event(WebhookEvent::new(
        WebhookEventType::Generic,
        "test".to_string(),
        serde_json::json!({}),
        BTreeMap::new(),
    ));
    assert_eq!(processor.len(), 1);
    processor.clear();
    assert!(processor.is_empty());
}

#[test]
fn event_processor_max_events_eviction() {
    let mut processor = EventProcessor::new(2);
    let event1 = WebhookEvent::new(
        WebhookEventType::Custom {
            name: "e1".to_string(),
        },
        "t".to_string(),
        serde_json::json!({}),
        BTreeMap::new(),
    );
    let event2 = WebhookEvent::new(
        WebhookEventType::Custom {
            name: "e2".to_string(),
        },
        "t".to_string(),
        serde_json::json!({}),
        BTreeMap::new(),
    );
    let event3 = WebhookEvent::new(
        WebhookEventType::Custom {
            name: "e3".to_string(),
        },
        "t".to_string(),
        serde_json::json!({}),
        BTreeMap::new(),
    );

    processor.add_event(event1);
    processor.add_event(event2);
    assert_eq!(processor.len(), 2);

    processor.add_event(event3);
    assert_eq!(processor.len(), 2);
}

#[test]
fn event_processor_default() {
    let processor: EventProcessor = EventProcessor::default();
    assert_eq!(processor.len(), 0);
}

#[test]
fn webhook_event_id_format() {
    let event = WebhookEvent::new(
        WebhookEventType::Generic,
        "test".to_string(),
        serde_json::json!({}),
        BTreeMap::new(),
    );
    assert!(event.id.starts_with("evt_"));
}
