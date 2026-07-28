use orbit_webhooks::{execute_webhook_tool, webhook_tool_specs};
use serde_json::json;

#[test]
fn webhook_tool_specs_returns_three_tools() {
    let specs = webhook_tool_specs();
    assert_eq!(specs.len(), 3);
}

#[test]
fn webhook_tool_specs_contains_remote_trigger() {
    let specs = webhook_tool_specs();
    assert!(specs.iter().any(|s| s.name == "RemoteTrigger"));
}

#[test]
fn webhook_tool_specs_contains_list_webhook_events() {
    let specs = webhook_tool_specs();
    assert!(specs.iter().any(|s| s.name == "ListWebhookEvents"));
}

#[test]
fn webhook_tool_specs_contains_trigger_webhook() {
    let specs = webhook_tool_specs();
    assert!(specs.iter().any(|s| s.name == "TriggerWebhook"));
}

#[test]
fn execute_webhook_tool_unknown_name() {
    let result = execute_webhook_tool("NonExistentTool", &json!({}));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Unknown webhook tool"));
}

#[test]
fn execute_webhook_tool_trigger_webhook() {
    let result = execute_webhook_tool(
        "TriggerWebhook",
        &json!({
            "source": "test-source",
            "event_type": "test-event",
        }),
    );
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("success"));
    assert!(output.contains("test-source"));
}

#[test]
fn execute_webhook_tool_trigger_with_payload() {
    let result = execute_webhook_tool(
        "TriggerWebhook",
        &json!({
            "source": "test",
            "event_type": "custom",
            "payload": {"key": "value"},
        }),
    );
    assert!(result.is_ok());
}

#[test]
fn execute_webhook_tool_trigger_missing_required() {
    let result = execute_webhook_tool(
        "TriggerWebhook",
        &json!({
            "source": "test",
        }),
    );
    assert!(result.is_err());
}

#[test]
fn execute_webhook_tool_list_events() {
    let result = execute_webhook_tool(
        "ListWebhookEvents",
        &json!({
            "source": "test",
            "limit": 5,
        }),
    );
    assert!(result.is_ok());
}

#[test]
fn execute_webhook_tool_list_events_default_limit() {
    let result = execute_webhook_tool("ListWebhookEvents", &json!({}));
    assert!(result.is_ok());
}

#[test]
fn execute_webhook_tool_list_events_invalid_input() {
    let result = execute_webhook_tool("ListWebhookEvents", &json!("not an object"));
    assert!(result.is_err());
}

#[test]
fn remote_trigger_invalid_input() {
    let result = execute_webhook_tool("RemoteTrigger", &json!({}));
    assert!(result.is_err());
}
