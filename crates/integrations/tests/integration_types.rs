use orbit_integrations::{
    IntegrationConfig, IntegrationRegistry, IntegrationTools, McpClientTransport, McpServerManager,
    McpToolRegistry, ToolSpec,
};

#[test]
fn mcp_client_transport_sdk_variant() {
    let sdk = orbit_integrations::mcp::client::McpSdkTransport {
        name: "test".to_string(),
    };
    drop(McpClientTransport::Sdk(sdk));
}

#[test]
fn mcp_client_transport_variant_debug() {
    let sdk = orbit_integrations::mcp::client::McpSdkTransport {
        name: "test".to_string(),
    };
    let transport = McpClientTransport::Sdk(sdk);
    let debug = format!("{transport:?}");
    assert!(debug.contains("Sdk"));
}

#[test]
fn integration_config_construction() {
    let config = IntegrationConfig {
        server: "my-server".to_string(),
        tools: IntegrationTools {
            create_pr: None,
            create_issue_comment: None,
            create_check_run: None,
            create_stack_comment: None,
            post_message: None,
        },
        oauth: None,
    };
    assert_eq!(config.server, "my-server");
    assert!(config.oauth.is_none());
}

#[test]
fn integration_registry_new() {
    let registry = IntegrationRegistry::new();
    assert!(registry.list_integrations().is_empty());
}

#[test]
fn integration_registry_default_is_empty() {
    let registry = IntegrationRegistry::default();
    assert!(registry.list_integrations().is_empty());
}

#[test]
fn integration_registry_register_and_list() {
    let registry = IntegrationRegistry::new();
    let config = IntegrationConfig {
        server: "srv".to_string(),
        tools: IntegrationTools {
            create_pr: None,
            create_issue_comment: None,
            create_check_run: None,
            create_stack_comment: None,
            post_message: None,
        },
        oauth: None,
    };
    registry.register("test", config);
    let names = registry.list_integrations();
    assert_eq!(names, vec!["test"]);
}

#[test]
fn integration_registry_get_config() {
    let registry = IntegrationRegistry::new();
    let config = IntegrationConfig {
        server: "srv".to_string(),
        tools: IntegrationTools {
            create_pr: None,
            create_issue_comment: None,
            create_check_run: None,
            create_stack_comment: None,
            post_message: None,
        },
        oauth: None,
    };
    registry.register("my-app", config);
    let retrieved = registry.get_config("my-app");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().server, "srv");
}

#[test]
fn mcp_server_manager_from_empty_servers() {
    use std::collections::BTreeMap;
    let servers = BTreeMap::new();
    let manager = McpServerManager::from_servers(&servers);
    assert!(manager.unsupported_servers().is_empty());
}

#[test]
fn mcp_server_manager_empty_server_names() {
    use std::collections::BTreeMap;
    let manager = McpServerManager::from_servers(&BTreeMap::new());
    assert!(manager.server_names().is_empty());
}

#[test]
fn mcp_tool_registry_new() {
    let registry = McpToolRegistry::new();
    assert!(registry.is_empty());
}

#[test]
fn mcp_tool_registry_default_is_empty() {
    let registry = McpToolRegistry::default();
    assert!(registry.is_empty());
}

#[test]
fn mcp_tool_registry_len_methods() {
    let registry = McpToolRegistry::new();
    assert_eq!(registry.len(), 0);
}

#[test]
fn tool_spec_construction() {
    use orbit_integrations::mcp::tools::PermissionMode;

    let spec = ToolSpec {
        name: "test_tool".to_string(),
        description: "A test tool".to_string(),
        input_schema: serde_json::json!({"type": "object"}),
        required_permission: PermissionMode::ReadOnly,
    };
    assert_eq!(spec.name, "test_tool");
    assert_eq!(spec.description, "A test tool");
}

#[test]
fn tool_spec_via_mcp_tool_specs() {
    let specs = orbit_integrations::mcp_tool_specs();
    assert!(specs.len() >= 3);
    assert!(specs.iter().any(|s| s.name == "MCP"));
}

#[test]
fn mcp_sdk_transport_construct() {
    drop(orbit_integrations::mcp::client::McpSdkTransport {
        name: "test-sdk".to_string(),
    });
}
