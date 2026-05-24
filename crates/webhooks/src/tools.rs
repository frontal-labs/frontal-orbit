//! Webhook tool definitions and handlers for Orbit.

use std::sync::{OnceLock, RwLock};

use super::events::{EventProcessor, WebhookEvent, WebhookEventType};
use serde::Deserialize;
use serde_json::{from_value, json, Value as JsonValue};

/// Permission mode for tools
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionMode {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

/// Tool specification for webhook tools
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub input_schema: JsonValue,
    pub required_permission: PermissionMode,
}

/// Webhook tool specifications
pub fn webhook_tool_specs() -> Vec<ToolSpec> {
    vec![
        ToolSpec {
            name: "RemoteTrigger".to_string(),
            description: "Trigger a remote action or webhook endpoint.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string" },
                    "method": {
                        "type": "string",
                        "enum": ["GET", "POST", "PUT", "DELETE"],
                        "default": "GET"
                    },
                    "headers": {
                        "type": "object",
                        "additionalProperties": { "type": "string" }
                    },
                    "body": { "type": "string" }
                },
                "required": ["url"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
        },
        ToolSpec {
            name: "ListWebhookEvents".to_string(),
            description: "List received webhook events from the webhook receiver.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "source": {
                        "type": "string",
                        "description": "Optional source filter"
                    },
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 100,
                        "default": 10
                    }
                },
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
        },
        ToolSpec {
            name: "TriggerWebhook".to_string(),
            description: "Trigger a webhook event to be processed by the webhook receiver."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "source": { "type": "string" },
                    "event_type": { "type": "string" },
                    "payload": { "type": "object" },
                    "headers": {
                        "type": "object",
                        "additionalProperties": { "type": "string" }
                    }
                },
                "required": ["source", "event_type"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
        },
    ]
}

/// Execute a webhook tool by name
pub fn execute_webhook_tool(name: &str, input: &JsonValue) -> Result<String, String> {
    match name {
        "RemoteTrigger" => from_value::<RemoteTriggerInput>(input.clone())
            .map_err(|e| format!("Invalid input: {e}"))
            .and_then(run_remote_trigger),
        "ListWebhookEvents" => from_value::<ListWebhookEventsInput>(input.clone())
            .map_err(|e| format!("Invalid input: {e}"))
            .and_then(run_list_webhook_events),
        "TriggerWebhook" => from_value::<TriggerWebhookInput>(input.clone())
            .map_err(|e| format!("Invalid input: {e}"))
            .and_then(run_trigger_webhook),
        _ => Err(format!("Unknown webhook tool: {name}")),
    }
}

/// Execute RemoteTrigger tool (synchronous version)
#[allow(clippy::needless_pass_by_value)]
fn run_remote_trigger(input: RemoteTriggerInput) -> Result<String, String> {
    let method = input.method.unwrap_or_else(|| "GET".to_string());
    let client = reqwest::blocking::Client::new();

    let mut request = match method.as_str() {
        "GET" => client.get(&input.url),
        "POST" => client.post(&input.url),
        "PUT" => client.put(&input.url),
        "DELETE" => client.delete(&input.url),
        _ => return Err(format!("Unsupported HTTP method: {method}")),
    };

    // Add headers
    for (key, value) in input.headers.unwrap_or_default() {
        request = request.header(&key, &value);
    }

    // Add body if provided
    if let Some(body) = input.body {
        request = request.body(body);
    }

    match request.send() {
        Ok(response) => {
            let status = response.status();
            let headers: std::collections::BTreeMap<String, String> = response
                .headers()
                .iter()
                .map(
                    |(k, v): (&reqwest::header::HeaderName, &reqwest::header::HeaderValue)| {
                        (k.to_string(), v.to_str().unwrap_or("").to_string())
                    },
                )
                .collect();

            let response_text = response
                .text()
                .map_err(|e| format!("Failed to read response: {e}"))?;

            let result = json!({
                "url": input.url,
                "method": method,
                "status": status.as_u16(),
                "status_text": status.canonical_reason().unwrap_or("Unknown"),
                "headers": headers,
                "response": response_text
            });

            serde_json::to_string_pretty(&result)
                .map_err(|e| format!("Failed to serialize response: {e}"))
        }
        Err(error) => {
            let result = json!({
                "url": input.url,
                "method": method,
                "error": error.to_string(),
                "success": false
            });

            serde_json::to_string_pretty(&result)
                .map_err(|e| format!("Failed to serialize error: {e}"))
        }
    }
}

/// Execute ListWebhookEvents tool
#[allow(clippy::needless_pass_by_value)]
fn run_list_webhook_events(input: ListWebhookEventsInput) -> Result<String, String> {
    let store = global_event_store();
    let guard = store
        .read()
        .map_err(|_| "failed to read webhook event store".to_string())?;
    let limit = input.limit.unwrap_or(10).min(100);
    let source_filter = input.source.clone();
    let mut events = if let Some(ref source) = source_filter {
        guard
            .events_by_source(&source)
            .into_iter()
            .cloned()
            .collect::<Vec<_>>()
    } else {
        guard.events().to_vec()
    };

    events.sort_by_key(|event| event.timestamp);
    events.reverse();
    events.truncate(limit);

    let result = json!({
        "source_filter": source_filter,
        "limit": limit,
        "total_events": guard.len(),
        "events": events
    });

    serde_json::to_string_pretty(&result)
        .map_err(|e| format!("Failed to serialize response: {e}"))
}

/// Execute TriggerWebhook tool
#[allow(clippy::needless_pass_by_value)]
fn run_trigger_webhook(input: TriggerWebhookInput) -> Result<String, String> {
    let event_type = WebhookEventType::Custom {
        name: input.event_type.clone(),
    };
    let payload = input.payload.unwrap_or_else(|| json!({}));
    let headers = input.headers.unwrap_or_default();
    let event = WebhookEvent::new(event_type, input.source, payload, headers);

    let store = global_event_store();
    {
        let mut guard = store
            .write()
            .map_err(|_| "failed to write webhook event store".to_string())?;
        guard.add_event(event.clone());
    }

    let result = json!({
        "success": true,
        "event": event
    });

    serde_json::to_string_pretty(&result)
        .map_err(|e| format!("Failed to serialize response: {e}"))
}

fn global_event_store() -> &'static RwLock<EventProcessor> {
    static STORE: OnceLock<RwLock<EventProcessor>> = OnceLock::new();
    STORE.get_or_init(|| RwLock::new(EventProcessor::default()))
}

// Input types for webhook tools
#[derive(Debug, Deserialize)]
struct RemoteTriggerInput {
    url: String,
    #[serde(default)]
    method: Option<String>,
    #[serde(default)]
    headers: Option<std::collections::BTreeMap<String, String>>,
    #[serde(default)]
    body: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ListWebhookEventsInput {
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct TriggerWebhookInput {
    source: String,
    event_type: String,
    #[serde(default)]
    payload: Option<JsonValue>,
    #[serde(default)]
    headers: Option<std::collections::BTreeMap<String, String>>,
}
