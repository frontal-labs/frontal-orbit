//! Webhook HTTP receiver server.

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::post,
    Router,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing::{error, info};

use super::auth::{HmacAuthenticator, WebhookAuth};
use super::events::{EventProcessor, WebhookEvent, WebhookEventType};

/// Configuration for webhook receiver
#[derive(Debug, Clone)]
pub struct WebhookConfig {
    /// Server port
    pub port: u16,
    /// Authentication method
    pub auth: WebhookAuth,
    /// Maximum events to store
    pub max_events: usize,
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            port: 8555,
            auth: WebhookAuth::None,
            max_events: 1000,
        }
    }
}

/// Shared state for the webhook server
#[derive(Debug)]
pub struct WebhookServerState {
    pub event_processor: Arc<RwLock<EventProcessor>>,
    pub auth: WebhookAuth,
}

/// Webhook receiver server
#[derive(Debug)]
pub struct WebhookReceiver {
    config: WebhookConfig,
    state: Arc<WebhookServerState>,
}

impl WebhookReceiver {
    /// Create a new webhook receiver
    pub fn new(config: WebhookConfig) -> Self {
        let event_processor = Arc::new(RwLock::new(EventProcessor::new(config.max_events)));

        Self {
            state: Arc::new(WebhookServerState {
                event_processor,
                auth: config.auth.clone(),
            }),
            config,
        }
    }

    /// Start the webhook server
    pub async fn serve(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let addr = SocketAddr::from(([0, 0, 0, 0], self.config.port));
        info!("Starting webhook server on {}", addr);

        let app = self.create_app();
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }

    /// Create the Axum application
    fn create_app(self) -> Router {
        Router::new()
            .route("/webhook", post(handle_webhook))
            .route("/webhook/:source", post(handle_webhook_with_source))
            .layer(ServiceBuilder::new().layer(CorsLayer::permissive()))
            .with_state(self.state)
    }

    /// Get event processor reference
    pub fn event_processor(&self) -> Arc<RwLock<EventProcessor>> {
        self.state.event_processor.clone()
    }
}

/// Handle webhook requests
async fn handle_webhook(
    State(state): State<Arc<WebhookServerState>>,
    headers: HeaderMap,
    body: String,
) -> Result<Json<Value>, StatusCode> {
    // Convert headers to BTreeMap
    let mut header_map = BTreeMap::new();
    for (name, value) in headers.iter() {
        if let Some(value_str) = value.to_str().ok() {
            header_map.insert(name.as_str().to_string(), value_str.to_string());
        }
    }

    // Determine event source and type
    let source = header_map
        .get("x-source")
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());

    let event_type = determine_event_type(&header_map, &body);

    // Verify authentication if required
    if !verify_auth(&state.auth, &body, &header_map)? {
        error!("Webhook authentication failed for source: {}", source);
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Parse payload
    let payload =
        serde_json::from_str::<Value>(&body).unwrap_or_else(|_| serde_json::json!({"raw": body}));

    // Create and store event
    let event = WebhookEvent::new(event_type, source.clone(), payload, header_map);

    {
        let mut processor = state.event_processor.write().await;
        processor.add_event(event);
    }

    info!("Webhook processed successfully from source: {}", source);

    Ok(Json(serde_json::json!({
        "status": "ok",
        "message": "Webhook received"
    })))
}

/// Handle webhook requests with explicit source
async fn handle_webhook_with_source(
    State(state): State<Arc<WebhookServerState>>,
    axum::extract::Path(source): axum::extract::Path<String>,
    headers: HeaderMap,
    body: String,
) -> Result<Json<Value>, StatusCode> {
    // Convert headers to BTreeMap
    let mut header_map = BTreeMap::new();
    for (name, value) in headers.iter() {
        if let Some(value_str) = value.to_str().ok() {
            header_map.insert(name.as_str().to_string(), value_str.to_string());
        }
    }

    // Use the source from the path
    let event_type = determine_event_type(&header_map, &body);

    // Verify authentication if required
    if !verify_auth(&state.auth, &body, &header_map)? {
        error!("Webhook authentication failed for source: {}", source);
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Parse payload
    let payload =
        serde_json::from_str::<Value>(&body).unwrap_or_else(|_| serde_json::json!({"raw": body}));

    // Create and store event
    let event = WebhookEvent::new(event_type, source.clone(), payload, header_map);

    {
        let mut processor = state.event_processor.write().await;
        processor.add_event(event);
    }

    info!("Webhook processed successfully from source: {}", source);

    Ok(Json(serde_json::json!({
        "status": "ok",
        "message": "Webhook received"
    })))
}

/// Determine event type from headers and payload
fn determine_event_type(headers: &BTreeMap<String, String>, payload: &str) -> WebhookEventType {
    // Check for GitHub events
    if let Some(event) = headers.get("x-github-event") {
        return WebhookEventType::GitHub {
            action: event.clone(),
        };
    }

    // Check for MCP events
    if let Some(mcp_server) = headers.get("x-mcp-server") {
        if let Some(mcp_event) = headers.get("x-mcp-event") {
            return WebhookEventType::McpServer {
                server_name: mcp_server.clone(),
                event: mcp_event.clone(),
            };
        }
    }

    // Try to detect from payload
    if let Ok(json) = serde_json::from_str::<Value>(payload) {
        if let Some(event_name) = json.get("event_type").and_then(|v| v.as_str()) {
            return WebhookEventType::Custom {
                name: event_name.to_string(),
            };
        }
    }

    WebhookEventType::Generic
}

/// Verify webhook authentication
fn verify_auth(
    auth: &WebhookAuth,
    body: &str,
    headers: &BTreeMap<String, String>,
) -> Result<bool, StatusCode> {
    match auth {
        WebhookAuth::Hmac { secret, header } => {
            let signature = headers.get(header).ok_or(StatusCode::UNAUTHORIZED)?;
            let authenticator = HmacAuthenticator::new(secret.clone(), header.clone());
            authenticator
                .verify(body.as_bytes(), signature)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        }
        WebhookAuth::None => Ok(true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_config_default() {
        let config = WebhookConfig::default();
        assert_eq!(config.port, 8555);
        assert!(matches!(config.auth, WebhookAuth::None));
        assert_eq!(config.max_events, 1000);
    }

    #[test]
    fn test_determine_event_type_github() {
        let mut headers = BTreeMap::new();
        headers.insert("x-github-event".to_string(), "push".to_string());

        let event_type = determine_event_type(&headers, "{}");
        assert!(matches!(event_type, WebhookEventType::GitHub { action } if action == "push"));
    }

    #[test]
    fn test_determine_event_type_mcp() {
        let mut headers = BTreeMap::new();
        headers.insert("x-mcp-server".to_string(), "test-server".to_string());
        headers.insert("x-mcp-event".to_string(), "server-started".to_string());

        let event_type = determine_event_type(&headers, "{}");
        assert!(
            matches!(event_type, WebhookEventType::McpServer { server_name, event }
            if server_name == "test-server" && event == "server-started")
        );
    }

    #[test]
    fn test_determine_event_type_custom() {
        let headers = BTreeMap::new();
        let payload = r#"{"event_type": "custom_event", "data": "test"}"#;

        let event_type = determine_event_type(&headers, payload);
        assert!(matches!(event_type, WebhookEventType::Custom { name } if name == "custom_event"));
    }
}
