//! # Orbit Integrations
//!
//! This crate provides integration capabilities for the Orbit system,
//! including Model Context Protocol (MCP) server management and tool bridging.

pub mod ide;
pub mod mcp;

// Re-export commonly used MCP types for convenience
pub use mcp::{
    client::McpClientTransport,
    integration::{
        global_integration_registry, IntegrationConfig, IntegrationRegistry, IntegrationTools,
    },
    stdio::{McpServerManager, McpToolDiscoveryReport},
    tool_bridge::{McpConnectionStatus, McpServerState, McpToolRegistry},
    tools::{execute_mcp_tool, mcp_tool_specs, ToolSpec},
};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
