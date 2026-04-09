//! # Orbit Webhooks
//!
//! This crate provides webhook receiving and processing capabilities for the Orbit system,
//! including custom webhook event handling and integration with external services.

pub mod auth;
pub mod events;
pub mod receiver;
pub mod tools;

// Re-export commonly used webhook types for convenience
pub use auth::{HmacAuthenticator, WebhookAuth};
pub use events::{EventProcessor, WebhookEvent, WebhookEventType};
pub use receiver::{WebhookConfig, WebhookReceiver};
pub use tools::{execute_webhook_tool, webhook_tool_specs};
