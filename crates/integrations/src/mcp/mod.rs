//! Model Context Protocol (MCP) integration for Orbit.
//!
//! This module provides comprehensive MCP server management, tool bridging,
//! and lifecycle handling for connecting external MCP servers to Orbit.

pub mod client;
pub mod config;
pub mod integration;
pub mod lifecycle;
pub mod stdio;
pub mod tool_bridge;
pub mod tools;
pub mod utils;

// Re-export core types for easier access
pub use client::McpClientTransport;
pub use integration::{
    global_integration_registry, CheckRunOutput, IntegrationConfig, IntegrationRegistry,
    IntegrationTools,
};
pub use lifecycle::{McpDegradedReport, McpLifecyclePhase};
pub use stdio::{McpServerManager, McpToolDiscoveryReport};
pub use tool_bridge::{McpConnectionStatus, McpServerState, McpToolRegistry};
pub use tools::mcp_tool_specs;
pub use utils::{mcp_tool_name, mcp_tool_prefix, normalize_name_for_mcp};
