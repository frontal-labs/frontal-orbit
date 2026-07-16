//! MCP configuration types to avoid circular dependencies.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// Define the config types locally to avoid circular dependency
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpTransport {
    Stdio,
    Sse,
    Http,
    WebSocket,
    Sdk,
    ManagedProxy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigSource {
    Local,
    Remote,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpStdioServerConfig {
    pub command: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub tool_call_timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpOAuthConfig {
    pub client_id: Option<String>,
    pub callback_port: Option<u16>,
    pub auth_server_metadata_url: Option<String>,
    pub xaa: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpRemoteServerConfig {
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub headers_helper: Option<String>,
    pub oauth: Option<McpOAuthConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpWebSocketServerConfig {
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub headers_helper: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpSdkServerConfig {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpManagedProxyServerConfig {
    pub url: String,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpServerConfig {
    Stdio(McpStdioServerConfig),
    Sse(McpRemoteServerConfig),
    Http(McpRemoteServerConfig),
    Ws(McpWebSocketServerConfig),
    Sdk(McpSdkServerConfig),
    ManagedProxy(McpManagedProxyServerConfig),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopedMcpServerConfig {
    pub scope: ConfigSource,
    pub config: McpServerConfig,
}

impl ScopedMcpServerConfig {
    #[must_use]
    pub fn transport(&self) -> McpTransport {
        match &self.config {
            McpServerConfig::Stdio(_) => McpTransport::Stdio,
            McpServerConfig::Sse(_) => McpTransport::Sse,
            McpServerConfig::Http(_) => McpTransport::Http,
            McpServerConfig::Ws(_) => McpTransport::WebSocket,
            McpServerConfig::Sdk(_) => McpTransport::Sdk,
            McpServerConfig::ManagedProxy(_) => McpTransport::ManagedProxy,
        }
    }
}
