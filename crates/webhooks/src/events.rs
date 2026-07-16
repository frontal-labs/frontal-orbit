//! Webhook event processing and management.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Types of webhook events
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEventType {
    /// Custom webhook event
    Custom { name: String },
    /// MCP server event
    McpServer { server_name: String, event: String },
    /// GitHub webhook event
    GitHub { action: String },
    /// Generic webhook event
    Generic,
}

/// Represents a webhook event received from an external source
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebhookEvent {
    /// Unique event ID
    pub id: String,
    /// Event type
    pub event_type: WebhookEventType,
    /// Event source (e.g., "github", "custom-webhook")
    pub source: String,
    /// Event payload
    pub payload: serde_json::Value,
    /// Event timestamp
    pub timestamp: u64,
    /// Event headers
    pub headers: BTreeMap<String, String>,
}

impl WebhookEvent {
    /// Create a new webhook event
    #[must_use] 
    pub fn new(
        event_type: WebhookEventType,
        source: String,
        payload: serde_json::Value,
        headers: BTreeMap<String, String>,
    ) -> Self {
        Self {
            id: generate_event_id(),
            event_type,
            source,
            payload,
            timestamp: current_timestamp(),
            headers,
        }
    }

    /// Get a header value
    #[must_use] 
    pub fn header(&self, name: &str) -> Option<&String> {
        self.headers.get(name)
    }

    /// Check if this is a GitHub event
    #[must_use] 
    pub fn is_github(&self) -> bool {
        matches!(self.event_type, WebhookEventType::GitHub { .. })
    }

    /// Check if this is an MCP server event
    #[must_use] 
    pub fn is_mcp(&self) -> bool {
        matches!(self.event_type, WebhookEventType::McpServer { .. })
    }
}

/// Event processor for handling webhook events
#[derive(Debug)]
pub struct EventProcessor {
    events: Vec<WebhookEvent>,
    max_events: usize,
}

impl EventProcessor {
    /// Create a new event processor
    #[must_use] 
    pub fn new(max_events: usize) -> Self {
        Self {
            events: Vec::with_capacity(max_events),
            max_events,
        }
    }

    /// Add an event to the processor
    pub fn add_event(&mut self, event: WebhookEvent) {
        self.events.push(event);

        // Remove oldest events if we exceed the limit
        if self.events.len() > self.max_events {
            self.events.remove(0);
        }
    }

    /// Get all events
    #[must_use] 
    pub fn events(&self) -> &[WebhookEvent] {
        &self.events
    }

    /// Get events by source
    #[must_use] 
    pub fn events_by_source(&self, source: &str) -> Vec<&WebhookEvent> {
        self.events
            .iter()
            .filter(|event| event.source == source)
            .collect()
    }

    /// Get events by type
    #[must_use] 
    pub fn events_by_type(&self, event_type: &WebhookEventType) -> Vec<&WebhookEvent> {
        self.events
            .iter()
            .filter(|event| &event.event_type == event_type)
            .collect()
    }

    /// Clear all events
    pub fn clear(&mut self) {
        self.events.clear();
    }

    /// Get event count
    #[must_use] 
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Check if empty
    #[must_use] 
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

impl Default for EventProcessor {
    fn default() -> Self {
        Self::new(1000) // Default to 1000 events
    }
}

/// Generate a unique event ID
fn generate_event_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let timestamp = current_timestamp();
    let counter = COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("evt_{timestamp}_{counter}")
}

/// Get current timestamp in seconds
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_event_creation() {
        let headers = BTreeMap::from([
            ("Content-Type".to_string(), "application/json".to_string()),
            ("X-Event".to_string(), "push".to_string()),
        ]);

        let event = WebhookEvent::new(
            WebhookEventType::GitHub {
                action: "push".to_string(),
            },
            "github".to_string(),
            serde_json::json!({"repository": "test"}),
            headers,
        );

        assert_eq!(event.source, "github");
        assert!(event.is_github());
        assert!(!event.is_mcp());
        assert_eq!(
            event.header("Content-Type"),
            Some(&"application/json".to_string())
        );
    }

    #[test]
    fn test_event_processor() {
        let mut processor = EventProcessor::new(2);

        let event1 = WebhookEvent::new(
            WebhookEventType::Custom {
                name: "test".to_string(),
            },
            "test".to_string(),
            serde_json::json!({}),
            BTreeMap::new(),
        );

        let event2 = WebhookEvent::new(
            WebhookEventType::Custom {
                name: "test2".to_string(),
            },
            "test".to_string(),
            serde_json::json!({}),
            BTreeMap::new(),
        );

        processor.add_event(event1.clone());
        assert_eq!(processor.len(), 1);

        processor.add_event(event2);
        assert_eq!(processor.len(), 2);

        // Adding a third should remove the first
        let event3 = WebhookEvent::new(
            WebhookEventType::Custom {
                name: "test3".to_string(),
            },
            "test".to_string(),
            serde_json::json!({}),
            BTreeMap::new(),
        );
        processor.add_event(event3);

        assert_eq!(processor.len(), 2);
        assert!(!processor.events().contains(&event1));
    }
}
