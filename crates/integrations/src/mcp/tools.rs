//! MCP tool definitions and handlers for Orbit.
//!
//! This module provides the tool definitions and execution logic for
//! MCP-related tools that can be called from within Orbit sessions.

use super::tool_bridge::McpToolRegistry;
use serde::Deserialize;
use serde_json::{from_value, json, Value as JsonValue};
use std::sync::OnceLock;

/// Permission mode for tools
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionMode {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

/// Tool specification for MCP tools
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub input_schema: JsonValue,
    pub required_permission: PermissionMode,
}

/// Global MCP tool registry instance
fn global_mcp_registry() -> &'static McpToolRegistry {
    static REGISTRY: OnceLock<McpToolRegistry> = OnceLock::new();
    REGISTRY.get_or_init(McpToolRegistry::new)
}

/// MCP tool specifications
#[must_use]
pub fn mcp_tool_specs() -> Vec<ToolSpec> {
    vec![
        ToolSpec {
            name: "ListMcpResources".to_string(),
            description: "List available resources from connected MCP servers.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "server": {
                        "type": "string",
                        "description": "Optional server name to list resources from. If not provided, lists from all servers."
                    }
                },
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
        },
        ToolSpec {
            name: "ReadMcpResource".to_string(),
            description: "Read a specific resource from an MCP server by URI.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "server": { "type": "string" },
                    "uri": { "type": "string" }
                },
                "required": ["uri"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
        },
        ToolSpec {
            name: "McpAuth".to_string(),
            description: "Authenticate with an MCP server that requires OAuth or credentials."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "server": { "type": "string" }
                },
                "required": ["server"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
        },
        ToolSpec {
            name: "MCP".to_string(),
            description: "Execute a tool provided by a connected MCP server.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "server": { "type": "string" },
                    "tool": { "type": "string" },
                    "arguments": { "type": "object" }
                },
                "required": ["server", "tool"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
        },
    ]
}

/// Execute an MCP tool by name
pub fn execute_mcp_tool(name: &str, input: &JsonValue) -> Result<String, String> {
    match name {
        "ListMcpResources" => from_value::<McpResourceInput>(input.clone())
            .map_err(|e| format!("Invalid input: {e}"))
            .and_then(run_list_mcp_resources),
        "ReadMcpResource" => from_value::<McpResourceInput>(input.clone())
            .map_err(|e| format!("Invalid input: {e}"))
            .and_then(run_read_mcp_resource),
        "McpAuth" => from_value::<McpAuthInput>(input.clone())
            .map_err(|e| format!("Invalid input: {e}"))
            .and_then(run_mcp_auth),
        "MCP" => from_value::<McpToolInput>(input.clone())
            .map_err(|e| format!("Invalid input: {e}"))
            .and_then(run_mcp_tool),
        _ => Err(format!("Unknown MCP tool: {name}")),
    }
}

#[allow(clippy::needless_pass_by_value)]
fn run_list_mcp_resources(input: McpResourceInput) -> Result<String, String> {
    let registry = global_mcp_registry();
    let server = input.server.as_deref().unwrap_or("default");
    match registry.list_resources(server) {
        Ok(resources) => {
            let json = json!({
                "server": server,
                "resources": resources
            });
            to_pretty_json(json)
        }
        Err(error) => to_pretty_json(json!({
            "server": server,
            "error": error.clone()
        })),
    }
}

#[allow(clippy::needless_pass_by_value)]
fn run_read_mcp_resource(input: McpResourceInput) -> Result<String, String> {
    let registry = global_mcp_registry();
    let uri = input.uri.as_deref().unwrap_or("");
    let server = input.server.as_deref().unwrap_or("default");
    match registry.read_resource(server, uri) {
        Ok(contents) => {
            let json = json!({
                "server": server,
                "uri": uri,
                "contents": contents
            });
            to_pretty_json(json)
        }
        Err(error) => to_pretty_json(json!({
            "server": server,
            "uri": uri,
            "error": error.clone()
        })),
    }
}

#[allow(clippy::needless_pass_by_value)]
fn run_mcp_auth(input: McpAuthInput) -> Result<String, String> {
    let registry = global_mcp_registry();
    match registry.get_server(&input.server) {
        Some(state) => to_pretty_json(json!({
            "server": input.server,
            "status": state.status,
            "tools": state.tools.len(),
            "resources": state.resources.len(),
            "server_info": state.server_info
        })),
        None => to_pretty_json(json!({
            "server": input.server,
            "status": "disconnected",
            "message": "Server not registered. Use MCP tool to connect first."
        })),
    }
}

#[allow(clippy::needless_pass_by_value)]
fn run_mcp_tool(input: McpToolInput) -> Result<String, String> {
    let registry = global_mcp_registry();
    let args = input.arguments.unwrap_or(serde_json::json!({}));
    match registry.call_tool(&input.server, &input.tool, &args) {
        Ok(result) => to_pretty_json(json!({
            "server": input.server,
            "tool": input.tool,
            "result": result
        })),
        Err(error) => to_pretty_json(json!({
            "server": input.server,
            "tool": input.tool,
            "error": error.clone()
        })),
    }
}

// Input types for MCP tools
#[derive(Debug, Deserialize)]
struct McpResourceInput {
    #[serde(default)]
    server: Option<String>,
    #[serde(default)]
    uri: Option<String>,
}

#[derive(Debug, Deserialize)]
struct McpAuthInput {
    server: String,
}

#[derive(Debug, Deserialize)]
struct McpToolInput {
    server: String,
    tool: String,
    #[serde(default)]
    arguments: Option<JsonValue>,
}

// Helper function for pretty JSON output
#[allow(clippy::needless_pass_by_value)]
fn to_pretty_json(value: JsonValue) -> Result<String, String> {
    serde_json::to_string_pretty(&value).map_err(|e| format!("Failed to serialize JSON: {e}"))
}

// Re-export for external use
pub use super::tool_bridge::{McpConnectionStatus, McpServerState};
