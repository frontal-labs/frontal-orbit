#![allow(clippy::must_use_candidate)]

use crate::mcp::tool_bridge::McpToolRegistry;
use crate::mcp::tools::global_mcp_registry;
use serde_json::Value;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IntegrationConfig {
    pub server: String,
    pub tools: IntegrationTools,
    pub oauth: Option<IntegrationOAuth>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IntegrationTools {
    pub create_pr: Option<String>,
    pub create_issue_comment: Option<String>,
    pub create_check_run: Option<String>,
    pub create_stack_comment: Option<String>,
    pub post_message: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IntegrationOAuth {
    pub auth_url: String,
    pub token_url: String,
    pub scopes: Vec<String>,
    pub client_id_env: String,
    pub client_secret_env: String,
}

#[derive(Debug, Clone)]
pub struct IntegrationRegistry {
    config: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, IntegrationConfig>>>,
}

impl IntegrationRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub fn register(&self, name: &str, config: IntegrationConfig) {
        let mut cfg = self
            .config
            .lock()
            .expect("integration registry lock poisoned");
        cfg.insert(name.to_string(), config);
    }

    #[must_use]
    pub fn get_config(&self, name: &str) -> Option<IntegrationConfig> {
        let cfg = self
            .config
            .lock()
            .expect("integration registry lock poisoned");
        cfg.get(name).cloned()
    }

    #[must_use]
    pub fn list_integrations(&self) -> Vec<String> {
        let cfg = self
            .config
            .lock()
            .expect("integration registry lock poisoned");
        cfg.keys().cloned().collect()
    }

    #[allow(clippy::unused_self)]
    fn registry(&self) -> &McpToolRegistry {
        global_mcp_registry()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn call_github_create_pr(
        &self,
        owner: &str,
        repo: &str,
        title: &str,
        body: &str,
        head: &str,
        base: &str,
        draft: bool,
    ) -> Result<Value, String> {
        let config = self
            .get_config("github")
            .ok_or_else(|| "GitHub integration not configured".to_string())?;
        let tool = config
            .tools
            .create_pr
            .as_deref()
            .unwrap_or("create_pull_request");
        self.registry().call_tool(
            &config.server,
            tool,
            &serde_json::json!({
                "owner": owner,
                "repo": repo,
                "title": title,
                "body": body,
                "head": head,
                "base": base,
                "draft": draft,
            }),
        )
    }

    pub fn call_github_create_issue_comment(
        &self,
        owner: &str,
        repo: &str,
        issue_number: u64,
        body: &str,
    ) -> Result<Value, String> {
        let config = self
            .get_config("github")
            .ok_or_else(|| "GitHub integration not configured".to_string())?;
        let tool = config
            .tools
            .create_issue_comment
            .as_deref()
            .unwrap_or("create_issue_comment");
        self.registry().call_tool(
            &config.server,
            tool,
            &serde_json::json!({
                "owner": owner,
                "repo": repo,
                "issue_number": issue_number,
                "body": body,
            }),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn call_github_create_check_run(
        &self,
        owner: &str,
        repo: &str,
        name: &str,
        head_sha: &str,
        status: &str,
        conclusion: Option<&str>,
        details_url: Option<&str>,
        output: Option<&CheckRunOutput>,
    ) -> Result<Value, String> {
        let config = self
            .get_config("github")
            .ok_or_else(|| "GitHub integration not configured".to_string())?;
        let tool = config
            .tools
            .create_check_run
            .as_deref()
            .unwrap_or("create_check_run");
        let mut args = serde_json::json!({
            "owner": owner,
            "repo": repo,
            "name": name,
            "head_sha": head_sha,
            "status": status,
        });
        if let Some(conclusion) = conclusion {
            args["conclusion"] = serde_json::json!(conclusion);
        }
        if let Some(details_url) = details_url {
            args["details_url"] = serde_json::json!(details_url);
        }
        if let Some(output) = output {
            args["output"] = serde_json::json!({
                "title": output.title,
                "summary": output.summary,
                "text": output.text,
            });
        }
        self.registry().call_tool(&config.server, tool, &args)
    }

    pub fn call_graphite_create_stack_comment(
        &self,
        stack_id: &str,
        body: &str,
    ) -> Result<Value, String> {
        let config = self
            .get_config("graphite")
            .ok_or_else(|| "Graphite integration not configured".to_string())?;
        let tool = config
            .tools
            .create_stack_comment
            .as_deref()
            .unwrap_or("create_stack_comment");
        self.registry().call_tool(
            &config.server,
            tool,
            &serde_json::json!({
                "stack_id": stack_id,
                "body": body,
            }),
        )
    }

    pub fn call_linear_create_issue_comment(
        &self,
        issue_id: &str,
        body: &str,
    ) -> Result<Value, String> {
        let config = self
            .get_config("linear")
            .ok_or_else(|| "Linear integration not configured".to_string())?;
        let tool = config
            .tools
            .create_issue_comment
            .as_deref()
            .unwrap_or("create_issue_comment");
        self.registry().call_tool(
            &config.server,
            tool,
            &serde_json::json!({
                "issue_id": issue_id,
                "body": body,
            }),
        )
    }

    pub fn call_slack_post_message(
        &self,
        channel: &str,
        text: &str,
        thread_ts: Option<&str>,
    ) -> Result<Value, String> {
        let config = self
            .get_config("slack")
            .ok_or_else(|| "Slack integration not configured".to_string())?;
        let tool = config
            .tools
            .post_message
            .as_deref()
            .unwrap_or("post_message");
        let mut args = serde_json::json!({
            "channel": channel,
            "text": text,
        });
        if let Some(thread_ts) = thread_ts {
            args["thread_ts"] = serde_json::json!(thread_ts);
        }
        self.registry().call_tool(&config.server, tool, &args)
    }

    #[must_use]
    pub fn get_oauth_config(&self, name: &str) -> Option<IntegrationOAuth> {
        self.get_config(name).and_then(|c| c.oauth)
    }
}

impl Default for IntegrationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Register integrations declared in a JSON object shaped like the
/// `mcp.integrations` block of `.orbit.json`:
///
/// ```json
/// {
///   "github": {
///     "server": "github",
///     "tools": { "create_pr": "create_pull_request" },
///     "oauth": { "auth_url": "...", "token_url": "...", "scopes": [], "client_id_env": "...", "client_secret_env": "..." }
///   }
/// }
/// ```
pub fn register_integrations_from_json(value: &serde_json::Value) -> Result<(), String> {
    let registry = global_integration_registry();
    let map = value
        .as_object()
        .ok_or_else(|| "integrations config must be a JSON object".to_string())?;
    for (name, config) in map {
        let server = config
            .get("server")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("integration '{name}' missing 'server'"))?
            .to_string();
        let tools_obj = config.get("tools").and_then(Value::as_object);
        let tools = IntegrationTools {
            create_pr: tools_obj
                .and_then(|t| t.get("create_pr"))
                .and_then(Value::as_str)
                .map(str::to_string),
            create_issue_comment: tools_obj
                .and_then(|t| t.get("create_issue_comment"))
                .and_then(Value::as_str)
                .map(str::to_string),
            create_check_run: tools_obj
                .and_then(|t| t.get("create_check_run"))
                .and_then(Value::as_str)
                .map(str::to_string),
            create_stack_comment: tools_obj
                .and_then(|t| t.get("create_stack_comment"))
                .and_then(Value::as_str)
                .map(str::to_string),
            post_message: tools_obj
                .and_then(|t| t.get("post_message"))
                .and_then(Value::as_str)
                .map(str::to_string),
        };
        let oauth = config
            .get("oauth")
            .and_then(|o| o.as_object())
            .map(|o| IntegrationOAuth {
                auth_url: o
                    .get("auth_url")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                token_url: o
                    .get("token_url")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                scopes: o
                    .get("scopes")
                    .and_then(Value::as_array)
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|s| s.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default(),
                client_id_env: o
                    .get("client_id_env")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                client_secret_env: o
                    .get("client_secret_env")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            });
        registry.register(
            name,
            IntegrationConfig {
                server,
                tools,
                oauth,
            },
        );
    }
    Ok(())
}

static INTEGRATION_REGISTRY: std::sync::OnceLock<IntegrationRegistry> = std::sync::OnceLock::new();

pub fn global_integration_registry() -> &'static IntegrationRegistry {
    INTEGRATION_REGISTRY.get_or_init(IntegrationRegistry::new)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CheckRunOutput {
    pub title: String,
    pub summary: String,
    pub text: Option<String>,
}
