#![allow(
    dead_code,
    unused_imports,
    unused_variables,
    clippy::unneeded_struct_pattern,
    clippy::unnecessary_wraps,
    clippy::unused_self
)]
mod init;
mod input;
mod render;

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, UNIX_EPOCH};

use futures_util::StreamExt;
use orbit_api::{
    create_provider_client, detect_provider_kind, AnthropicClient, AuthSource, ContentBlockDelta,
    InputContentBlock, InputMessage, JsonlTelemetrySink, MessageRequest, MessageResponse,
    OutputContentBlock, PromptCache, ProviderClient, ProviderKind, SessionTracer,
    StreamEvent as ApiStreamEvent, ToolChoice, ToolDefinition, ToolResultContentBlock,
};

use init::initialize_repo;
use orbit_commands::{
    classify_skills_slash_command, handle_agents_slash_command, handle_agents_slash_command_json,
    handle_mcp_slash_command, handle_mcp_slash_command_json, handle_plugins_slash_command,
    handle_skills_slash_command, handle_skills_slash_command_json, render_slash_command_help,
    resume_supported_slash_commands, slash_command_specs, validate_slash_command_input,
    SkillSlashDispatch, SlashCommand, CONFIG_SECTION_ARGUMENT_HINT, SUPPORTED_CONFIG_SECTIONS,
};
use orbit_harness::{extract_manifest, UpstreamPaths};
use orbit_events::{EventEnvelope, HostedEventName, HostedEventStatus, HostedEventTopic};
use orbit_github::{
    parse_github_repo_url, GitHubCheckRunDraft, GitHubCheckRunOutput, GitHubClient,
    GitHubClientConfig, GitHubIssueCommentDraft, GitHubPullRequestDraft, GitHubRepoRef,
};
use orbit_integrations::ide::{
    collect_status as collect_ide_status, install_extension as install_ide_extension,
    install_packaged_extension as install_packaged_ide_extension,
    launch_target as launch_ide_target, package_extension as package_ide_extension,
    parse_target as parse_ide_target, set_default_target as set_default_ide_target,
    setup_editor_integration as setup_ide_editor_integration, IdeStatus, IdeTarget,
};
use orbit_integrations::mcp::config as integrations_mcp_config;
use orbit_plugins::{PluginHooks, PluginManager, PluginManagerConfig, PluginRegistry};
use orbit_repo::{push_branch, repo_status, stage_and_commit, RepoCommitRequest};
use orbit_runtime::{
    format_usd, load_system_prompt, permission_enforcer::PermissionEnforcer, pricing_for_model,
    resolve_sandbox_status, ApiClient, ApiRequest, AssistantEvent, CompactionConfig, ConfigLoader,
    ConfigSource, ConfigurationManager, ContentBlock, ConversationMessage, ConversationRuntime,
    McpServerManager, McpTool, MessageRole, ModelPricing, PermissionMode, PermissionPolicy,
    ProjectContext, PromptCacheEvent, ResolvedPermissionMode, RuntimeError, Session, TokenUsage,
    ToolError, ToolExecutor, UsageTracker,
};
use orbit_tools::{
    GlobalToolRegistry, RuntimeToolDefinition, ToolExecutionScope, ToolSearchOutput,
};
use render::{MarkdownStreamState, Spinner, TerminalRenderer};
use reqwest::blocking::Client as HttpClient;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use tokio_tungstenite::{connect_async, tungstenite::Message as WebSocketMessage};

const DEFAULT_MODEL: &str = "claude-opus-4-6";
const DEFAULT_HOSTED_SERVER_URL: &str = "http://127.0.0.1:8788";
const HOSTED_SERVER_TIMEOUT_SECS: u64 = 30;
fn max_tokens_for_model(model: &str) -> u32 {
    if model.contains("opus") {
        32_000
    } else {
        64_000
    }
}
const DEFAULT_DATE: &str = "2026-03-31";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const BUILD_TARGET: Option<&str> = option_env!("TARGET");
const GIT_SHA: Option<&str> = option_env!("GIT_SHA");
const ORBIT_TELEMETRY_PATH: &str = "ORBIT_TELEMETRY_PATH";
const INTERNAL_PROGRESS_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(3);
const PRIMARY_SESSION_EXTENSION: &str = "jsonl";
const LEGACY_SESSION_EXTENSION: &str = "json";
const LATEST_SESSION_REFERENCE: &str = "latest";
const SESSION_REFERENCE_ALIASES: &[&str] = &[LATEST_SESSION_REFERENCE, "last", "recent"];
const CLI_OPTION_SUGGESTIONS: &[&str] = &[
    "--help",
    "-h",
    "--version",
    "-V",
    "--model",
    "--output-format",
    "--permission-mode",
    "--dangerously-skip-permissions",
    "--allowedTools",
    "--allowed-tools",
    "--resume",
    "--print",
    "-p",
];

type AllowedToolSet = BTreeSet<String>;
type RuntimePluginStateBuildOutput = (
    Option<Arc<Mutex<RuntimeMcpState>>>,
    Vec<RuntimeToolDefinition>,
);

fn main() {
    if let Err(error) = run() {
        let message = error.to_string();
        if message.contains("`orbit --help`") {
            eprintln!("error: {message}");
        } else {
            eprintln!(
                "error: {message}

Run `orbit --help` for usage."
            );
        }
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    match parse_args(&args)? {
        CliAction::DumpManifests { output_format } => dump_manifests(output_format)?,
        CliAction::BootstrapPlan { output_format } => print_bootstrap_plan(output_format)?,
        CliAction::Agents {
            args,
            output_format,
        } => LiveCli::print_agents(args.as_deref(), output_format)?,
        CliAction::Mcp {
            args,
            output_format,
        } => LiveCli::print_mcp(args.as_deref(), output_format)?,
        CliAction::Skills {
            args,
            output_format,
        } => LiveCli::print_skills(args.as_deref(), output_format)?,
        CliAction::Plugins {
            action,
            target,
            output_format,
        } => LiveCli::print_plugins(action.as_deref(), target.as_deref(), output_format)?,
        CliAction::PrintSystemPrompt {
            cwd,
            date,
            output_format,
        } => print_system_prompt(cwd, date, output_format)?,
        CliAction::Version { output_format } => print_version(output_format)?,
        CliAction::ResumeSession {
            session_path,
            commands,
            output_format,
        } => resume_session(&session_path, &commands, output_format),
        CliAction::Status {
            model,
            provider: _,
            permission_mode,
            output_format,
        } => print_status_snapshot(&model, permission_mode, output_format)?,
        CliAction::Config {
            section,
            output_format,
        } => LiveCli::print_config(section.as_deref(), output_format)?,
        CliAction::Telemetry {
            output_format,
            action,
            target,
        } => print_telemetry_status(output_format, action.as_deref(), target.as_deref())?,
        CliAction::Sandbox { output_format } => print_sandbox_status_snapshot(output_format)?,
        CliAction::Prompt {
            prompt,
            model,
            provider,
            output_format,
            allowed_tools,
            permission_mode,
        } => LiveCli::new_with_provider(model, provider, true, allowed_tools, permission_mode)?
            .run_turn_with_output(&prompt, output_format)?,
        CliAction::Doctor { output_format } => run_doctor(output_format)?,
        CliAction::Init { output_format } => run_init(output_format)?,
        CliAction::Hosted {
            command,
            output_format,
        } => run_hosted_command(command, output_format)?,
        CliAction::Repl {
            model,
            provider,
            allowed_tools,
            permission_mode,
        } => run_repl_with_provider(model, provider, allowed_tools, permission_mode)?,
        CliAction::HelpTopic(topic) => print_help_topic(topic),
        CliAction::Help { output_format } => print_help(output_format)?,
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TelemetryResolution {
    enabled: bool,
    path: Option<String>,
    source: &'static str,
    config_path: Option<PathBuf>,
}

fn resolve_telemetry_config(
    runtime_config: Option<&orbit_runtime::RuntimeConfig>,
) -> TelemetryResolution {
    if let Ok(path) = env::var(ORBIT_TELEMETRY_PATH) {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return TelemetryResolution {
                enabled: true,
                path: Some(trimmed.to_string()),
                source: "env",
                config_path: telemetry_config_entry(runtime_config).map(|(_, path)| path),
            };
        }
    }

    if let Some(telemetry) = runtime_config.map(orbit_runtime::RuntimeConfig::telemetry) {
        if telemetry.enabled() == Some(false) {
            return TelemetryResolution {
                enabled: false,
                path: telemetry.path().map(ToOwned::to_owned),
                source: "config",
                config_path: telemetry_config_entry(runtime_config).map(|(_, path)| path),
            };
        }
        if let Some(path) = telemetry.path().filter(|path| !path.trim().is_empty()) {
            return TelemetryResolution {
                enabled: true,
                path: Some(path.to_string()),
                source: "config",
                config_path: telemetry_config_entry(runtime_config).map(|(_, path)| path),
            };
        }
    }

    TelemetryResolution {
        enabled: false,
        path: None,
        source: "default",
        config_path: None,
    }
}

fn telemetry_config_entry(
    runtime_config: Option<&orbit_runtime::RuntimeConfig>,
) -> Option<(orbit_runtime::ConfigSource, PathBuf)> {
    let runtime_config = runtime_config?;
    runtime_config
        .loaded_entries()
        .iter()
        .rev()
        .find_map(|entry| {
            let contents = fs::read_to_string(&entry.path).ok()?;
            let parsed = serde_json::from_str::<Value>(&contents).ok()?;
            parsed
                .as_object()
                .and_then(|object| object.contains_key("telemetry").then_some(()))?;
            Some((entry.source, entry.path.clone()))
        })
}

fn build_cli_session_tracer(
    session_id: &str,
    runtime_config: Option<&orbit_runtime::RuntimeConfig>,
) -> Option<SessionTracer> {
    let resolution = resolve_telemetry_config(runtime_config);
    if !resolution.enabled {
        return None;
    }
    resolution
        .path
        .and_then(|path| JsonlTelemetrySink::new(path).ok())
        .map(|sink| SessionTracer::new(session_id.to_string(), Arc::new(sink)))
}

fn attach_session_tracer(
    client: ProviderClient,
    session_tracer: Option<SessionTracer>,
) -> ProviderClient {
    match session_tracer {
        Some(tracer) => client.with_session_tracer(tracer),
        None => client,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CliAction {
    DumpManifests {
        output_format: CliOutputFormat,
    },
    BootstrapPlan {
        output_format: CliOutputFormat,
    },
    Agents {
        args: Option<String>,
        output_format: CliOutputFormat,
    },
    Mcp {
        args: Option<String>,
        output_format: CliOutputFormat,
    },
    Skills {
        args: Option<String>,
        output_format: CliOutputFormat,
    },
    Plugins {
        action: Option<String>,
        target: Option<String>,
        output_format: CliOutputFormat,
    },
    PrintSystemPrompt {
        cwd: PathBuf,
        date: String,
        output_format: CliOutputFormat,
    },
    Version {
        output_format: CliOutputFormat,
    },
    ResumeSession {
        session_path: PathBuf,
        commands: Vec<String>,
        output_format: CliOutputFormat,
    },
    Status {
        model: String,
        provider: Option<String>,
        permission_mode: PermissionMode,
        output_format: CliOutputFormat,
    },
    Config {
        section: Option<String>,
        output_format: CliOutputFormat,
    },
    Telemetry {
        output_format: CliOutputFormat,
        action: Option<String>,
        target: Option<String>,
    },
    Sandbox {
        output_format: CliOutputFormat,
    },
    Prompt {
        prompt: String,
        model: String,
        provider: Option<String>,
        output_format: CliOutputFormat,
        allowed_tools: Option<AllowedToolSet>,
        permission_mode: PermissionMode,
    },
    Doctor {
        output_format: CliOutputFormat,
    },
    Init {
        output_format: CliOutputFormat,
    },
    Hosted {
        command: HostedCommand,
        output_format: CliOutputFormat,
    },
    Repl {
        model: String,
        provider: Option<String>,
        allowed_tools: Option<AllowedToolSet>,
        permission_mode: PermissionMode,
    },
    HelpTopic(LocalHelpTopic),
    // prompt-mode formatting is only supported for non-interactive runs
    Help {
        output_format: CliOutputFormat,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum HostedCommand {
    PolicyOrphans {
        repository: Option<String>,
        source: Option<String>,
        priority: Option<String>,
    },
    EventsWatch {
        query: HostedEventWatchQuery,
    },
    TasksList {
        query: HostedTaskListQuery,
    },
    TasksWatch {
        query: HostedTaskListQuery,
    },
    TaskGet {
        task_id: String,
    },
    TaskRuntime {
        task_id: String,
    },
    TaskReconcile {
        task_id: String,
    },
    TaskRun {
        task_id: String,
    },
    TaskCancel {
        task_id: String,
    },
    TaskApproval {
        task_id: String,
        action: HostedApprovalAction,
        resolved_by: Option<String>,
        reason: Option<String>,
        approval_kind: String,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct HostedTaskListQuery {
    status: Option<String>,
    source: Option<String>,
    repository: Option<String>,
    channel_id: Option<String>,
    thread_ts: Option<String>,
    limit: Option<usize>,
    needs_followup: Option<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct HostedEventWatchQuery {
    task_id: Option<String>,
    topic: Option<String>,
    event: Option<String>,
    status: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HostedApprovalAction {
    Retry,
    Cancel,
    Ack,
}

impl HostedApprovalAction {
    fn as_str(self) -> &'static str {
        match self {
            Self::Retry => "retry",
            Self::Cancel => "cancel",
            Self::Ack => "ack",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "retry" => Ok(Self::Retry),
            "cancel" => Ok(Self::Cancel),
            "ack" => Ok(Self::Ack),
            other => Err(format!(
                "unsupported hosted approval action: {other} (expected retry, cancel, or ack)"
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HostedPolicyResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    preview: Option<HostedPolicyPreview>,
    default_policy: HostedAppliedOrphanPolicy,
    effective_policy: HostedAppliedOrphanPolicy,
    configured_rules: Vec<HostedPolicyRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HostedPolicyPreview {
    #[serde(skip_serializing_if = "Option::is_none")]
    repository: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    priority: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HostedAppliedOrphanPolicy {
    source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    match_repository: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    match_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    match_priority: Option<String>,
    approval_delay_secs: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    auto_retry_after_secs: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    auto_cancel_after_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HostedPolicyRule {
    #[serde(skip_serializing_if = "Option::is_none")]
    repository: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    priority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    approval_delay_secs: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    auto_retry_after_secs: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    auto_cancel_after_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HostedTaskSnapshot {
    task_id: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    repository: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    channel_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thread_ts: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    worker_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    worker_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    plan_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    github_review_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    github_feedback_required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    github_feedback_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    linear_issue_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    linear_issue_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    linear_issue_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    linear_issue_identifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    graphite_stack_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    graphite_head_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    graphite_base_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    orphan_policy: Option<HostedAppliedOrphanPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HostedAgentRuntimeSnapshot {
    found: bool,
    #[serde(rename = "liveControl")]
    live_control: bool,
    status: String,
    #[serde(rename = "derivedState")]
    derived_state: String,
    orphaned: bool,
    #[serde(rename = "manifestFile", skip_serializing_if = "Option::is_none")]
    manifest_file: Option<String>,
    #[serde(rename = "outputFile", skip_serializing_if = "Option::is_none")]
    output_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HostedTaskRuntimeResponse {
    task_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    worker_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    worker_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    manifest_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    orphan_policy: Option<HostedAppliedOrphanPolicy>,
    #[serde(rename = "hostedAgent", skip_serializing_if = "Option::is_none")]
    hosted_agent: Option<HostedAgentRuntimeSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct HostedTaskWorkerPayload {
    task_id: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    repository: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repo_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    base_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    permission_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    allowed_tools: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
struct HostedTaskGithubResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    published_remote: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    published_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    published_commit_sha: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pr_number: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pr_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pr_api_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pr_head_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pr_base_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct HostedTaskWatchItem {
    event: EventEnvelope,
    task: HostedTaskSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LocalHelpTopic {
    Status,
    Sandbox,
    Doctor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CliOutputFormat {
    Text,
    Json,
}

impl CliOutputFormat {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "text" => Ok(Self::Text),
            "json" => Ok(Self::Json),
            other => Err(format!(
                "unsupported value for --output-format: {other} (expected text or json)"
            )),
        }
    }
}

#[allow(clippy::too_many_lines)]
fn parse_args(args: &[String]) -> Result<CliAction, String> {
    let mut model = DEFAULT_MODEL.to_string();
    let mut provider = None;
    let mut output_format = CliOutputFormat::Text;
    let mut permission_mode_override = None;
    let mut wants_help = false;
    let mut wants_version = false;
    let mut allowed_tool_values = Vec::new();
    let mut rest = Vec::new();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--help" | "-h" if rest.is_empty() => {
                wants_help = true;
                index += 1;
            }
            "--version" | "-V" => {
                wants_version = true;
                index += 1;
            }
            "--model" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --model".to_string())?;
                model = resolve_model_alias(value).to_string();
                index += 2;
            }
            flag if flag.starts_with("--model=") => {
                model = resolve_model_alias(&flag[8..]).to_string();
                index += 1;
            }
            "--output-format" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --output-format".to_string())?;
                output_format = CliOutputFormat::parse(value)?;
                index += 2;
            }
            "--permission-mode" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --permission-mode".to_string())?;
                permission_mode_override = Some(parse_permission_mode_arg(value)?);
                index += 2;
            }
            "--provider" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --provider".to_string())?;
                provider = Some(value.to_string());
                index += 2;
            }
            flag if flag.starts_with("--output-format=") => {
                output_format = CliOutputFormat::parse(&flag[16..])?;
                index += 1;
            }
            flag if flag.starts_with("--permission-mode=") => {
                permission_mode_override = Some(parse_permission_mode_arg(&flag[18..])?);
                index += 1;
            }
            flag if flag.starts_with("--provider=") => {
                provider = Some(flag[11..].to_string());
                index += 1;
            }
            "--dangerously-skip-permissions" => {
                permission_mode_override = Some(PermissionMode::DangerFullAccess);
                index += 1;
            }
            "-p" => {
                // Orbit compat: -p "prompt" = one-shot prompt
                let prompt = args[index + 1..].join(" ");
                if prompt.trim().is_empty() {
                    return Err("-p requires a prompt string".to_string());
                }
                return Ok(CliAction::Prompt {
                    prompt,
                    model: resolve_model_alias(&model).to_string(),
                    provider: provider.clone(),
                    output_format,
                    allowed_tools: normalize_allowed_tools(&allowed_tool_values)?,
                    permission_mode: permission_mode_override
                        .unwrap_or_else(default_permission_mode),
                });
            }
            "--print" => {
                // Orbit compat: --print makes output non-interactive
                output_format = CliOutputFormat::Text;
                index += 1;
            }
            "--resume" if rest.is_empty() => {
                rest.push("--resume".to_string());
                index += 1;
            }
            flag if rest.is_empty() && flag.starts_with("--resume=") => {
                rest.push("--resume".to_string());
                rest.push(flag[9..].to_string());
                index += 1;
            }
            "--allowedTools" | "--allowed-tools" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --allowedTools".to_string())?;
                allowed_tool_values.push(value.clone());
                index += 2;
            }
            flag if flag.starts_with("--allowedTools=") => {
                allowed_tool_values.push(flag[15..].to_string());
                index += 1;
            }
            flag if flag.starts_with("--allowed-tools=") => {
                allowed_tool_values.push(flag[16..].to_string());
                index += 1;
            }
            other if rest.is_empty() && other.starts_with('-') => {
                return Err(format_unknown_option(other))
            }
            other => {
                rest.push(other.to_string());
                index += 1;
            }
        }
    }

    if wants_help {
        return Ok(CliAction::Help { output_format });
    }

    if wants_version {
        return Ok(CliAction::Version { output_format });
    }

    let allowed_tools = normalize_allowed_tools(&allowed_tool_values)?;

    if rest.is_empty() {
        let permission_mode = permission_mode_override.unwrap_or_else(default_permission_mode);
        return Ok(CliAction::Repl {
            model,
            provider,
            allowed_tools,
            permission_mode,
        });
    }
    if rest.first().map(String::as_str) == Some("--resume") {
        return parse_resume_args(&rest[1..], output_format);
    }
    if let Some(action) = parse_local_help_action(&rest) {
        return action;
    }
    if let Some(action) = parse_single_word_command_alias(
        &rest,
        &model,
        permission_mode_override,
        output_format,
        provider.clone(),
    ) {
        return action;
    }

    let permission_mode = permission_mode_override.unwrap_or_else(default_permission_mode);

    match rest[0].as_str() {
        "dump-manifests" => Ok(CliAction::DumpManifests { output_format }),
        "bootstrap-plan" => Ok(CliAction::BootstrapPlan { output_format }),
        "config" => parse_config_cli_action(&rest[1..], output_format),
        "telemetry" => Ok(CliAction::Telemetry {
            output_format,
            action: rest.get(1).cloned(),
            target: rest.get(2).cloned(),
        }),
        "agents" => Ok(CliAction::Agents {
            args: join_optional_args(&rest[1..]),
            output_format,
        }),
        "mcp" => Ok(CliAction::Mcp {
            args: join_optional_args(&rest[1..]),
            output_format,
        }),
        "skills" => {
            let args = join_optional_args(&rest[1..]);
            match classify_skills_slash_command(args.as_deref()) {
                SkillSlashDispatch::Invoke(prompt) => Ok(CliAction::Prompt {
                    prompt,
                    model,
                    provider: provider.clone(),
                    output_format,
                    allowed_tools,
                    permission_mode,
                }),
                SkillSlashDispatch::Local => Ok(CliAction::Skills {
                    args,
                    output_format,
                }),
            }
        }
        "hosted" => parse_hosted_cli_action(&rest[1..], output_format),
        "system-prompt" => parse_system_prompt_args(&rest[1..], output_format),
        "init" => Ok(CliAction::Init { output_format }),
        "prompt" => {
            let prompt = rest[1..].join(" ");
            if prompt.trim().is_empty() {
                return Err("prompt subcommand requires a prompt string".to_string());
            }
            Ok(CliAction::Prompt {
                prompt,
                model,
                provider: provider.clone(),
                output_format,
                allowed_tools,
                permission_mode,
            })
        }
        other if other.starts_with('/') => parse_direct_slash_cli_action(
            &rest,
            model,
            output_format,
            allowed_tools,
            permission_mode,
            provider,
        ),
        _other => Ok(CliAction::Prompt {
            prompt: rest.join(" "),
            model,
            provider: provider.clone(),
            output_format,
            allowed_tools,
            permission_mode,
        }),
    }
}

fn parse_local_help_action(rest: &[String]) -> Option<Result<CliAction, String>> {
    if rest.len() != 2 || !is_help_flag(&rest[1]) {
        return None;
    }

    let topic = match rest[0].as_str() {
        "status" => LocalHelpTopic::Status,
        "sandbox" => LocalHelpTopic::Sandbox,
        "doctor" => LocalHelpTopic::Doctor,
        _ => return None,
    };
    Some(Ok(CliAction::HelpTopic(topic)))
}

fn parse_hosted_cli_action(
    args: &[String],
    output_format: CliOutputFormat,
) -> Result<CliAction, String> {
    if args.is_empty() {
        return Err(
            "hosted requires a subcommand. Use `orbit hosted policy orphans` or `orbit hosted task ...`."
                .to_string(),
        );
    }

    match args[0].as_str() {
        "policy" => parse_hosted_policy_cli_action(&args[1..], output_format),
        "events" => parse_hosted_events_cli_action(&args[1..], output_format),
        "tasks" => parse_hosted_tasks_cli_action(&args[1..], output_format),
        "task" => parse_hosted_task_cli_action(&args[1..], output_format),
        other => Err(format!(
            "unknown hosted subcommand: {other} (expected policy, events, tasks, or task)"
        )),
    }
}

fn parse_hosted_events_cli_action(
    args: &[String],
    output_format: CliOutputFormat,
) -> Result<CliAction, String> {
    if args.first().map(String::as_str) != Some("watch") {
        return Err(
            "unsupported hosted events command. Use `orbit hosted events watch`.".to_string(),
        );
    }

    let mut query = HostedEventWatchQuery::default();
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--task-id" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("hosted events watch requires a value after --task-id".to_string());
                };
                query.task_id = Some(value.clone());
                index += 2;
            }
            "--topic" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("hosted events watch requires a value after --topic".to_string());
                };
                query.topic = Some(value.clone());
                index += 2;
            }
            "--event" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("hosted events watch requires a value after --event".to_string());
                };
                query.event = Some(value.clone());
                index += 2;
            }
            "--status" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("hosted events watch requires a value after --status".to_string());
                };
                query.status = Some(value.clone());
                index += 2;
            }
            "--limit" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("hosted events watch requires a value after --limit".to_string());
                };
                let parsed = value
                    .parse::<usize>()
                    .map_err(|_| format!("invalid hosted event limit: {value}"))?;
                query.limit = Some(parsed);
                index += 2;
            }
            other => {
                return Err(format!(
                    "unsupported hosted events argument: {other}. Use --task-id, --topic, --event, --status, or --limit."
                ));
            }
        }
    }

    Ok(CliAction::Hosted {
        command: HostedCommand::EventsWatch { query },
        output_format,
    })
}

fn parse_hosted_tasks_cli_action(
    args: &[String],
    output_format: CliOutputFormat,
) -> Result<CliAction, String> {
    let Some(action) = args.first().map(String::as_str) else {
        return Err("hosted tasks requires a subcommand. Use `orbit hosted tasks list` or `orbit hosted tasks watch`.".to_string());
    };

    let mut query = HostedTaskListQuery::default();
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--status" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("hosted tasks list requires a value after --status".to_string());
                };
                query.status = Some(value.clone());
                index += 2;
            }
            "--source" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("hosted tasks list requires a value after --source".to_string());
                };
                query.source = Some(value.clone());
                index += 2;
            }
            "--repository" | "--repo" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("hosted tasks list requires a value after --repository".to_string());
                };
                query.repository = Some(value.clone());
                index += 2;
            }
            "--channel-id" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("hosted tasks list requires a value after --channel-id".to_string());
                };
                query.channel_id = Some(value.clone());
                index += 2;
            }
            "--thread-ts" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("hosted tasks list requires a value after --thread-ts".to_string());
                };
                query.thread_ts = Some(value.clone());
                index += 2;
            }
            "--needs-followup" => {
                query.needs_followup = Some(true);
                index += 1;
            }
            "--limit" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("hosted tasks list requires a value after --limit".to_string());
                };
                let parsed = value
                    .parse::<usize>()
                    .map_err(|_| format!("invalid hosted task limit: {value}"))?;
                query.limit = Some(parsed);
                index += 2;
            }
            other => {
                return Err(format!(
                    "unsupported hosted tasks argument: {other}. Use --status, --source, --repository, --channel-id, --thread-ts, or --limit."
                ));
            }
        }
    }

    Ok(CliAction::Hosted {
        command: match action {
            "list" => HostedCommand::TasksList { query },
            "watch" => HostedCommand::TasksWatch { query },
            other => {
                return Err(format!(
                    "unsupported hosted tasks command: {other}. Use `orbit hosted tasks list` or `orbit hosted tasks watch`."
                ))
            }
        },
        output_format,
    })
}

fn parse_hosted_policy_cli_action(
    args: &[String],
    output_format: CliOutputFormat,
) -> Result<CliAction, String> {
    if args.first().map(String::as_str) != Some("orphans") {
        return Err(
            "unsupported hosted policy command. Use `orbit hosted policy orphans`.".to_string(),
        );
    }

    let mut repository = None;
    let mut source = None;
    let mut priority = None;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--repository" | "--repo" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(
                        "hosted policy orphans requires a value after --repository".to_string()
                    );
                };
                repository = Some(value.clone());
                index += 2;
            }
            "--source" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("hosted policy orphans requires a value after --source".to_string());
                };
                source = Some(value.clone());
                index += 2;
            }
            "--priority" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(
                        "hosted policy orphans requires a value after --priority".to_string()
                    );
                };
                priority = Some(value.clone());
                index += 2;
            }
            other => {
                return Err(format!(
                    "unsupported hosted policy argument: {other}. Use --repository, --source, or --priority."
                ));
            }
        }
    }

    Ok(CliAction::Hosted {
        command: HostedCommand::PolicyOrphans {
            repository,
            source,
            priority,
        },
        output_format,
    })
}

fn parse_hosted_task_cli_action(
    args: &[String],
    output_format: CliOutputFormat,
) -> Result<CliAction, String> {
    let Some(action) = args.first().map(String::as_str) else {
        return Err(
            "hosted task requires a subcommand. Use get, runtime, reconcile, run, cancel, or approval."
                .to_string(),
        );
    };

    match action {
        "get" => {
            let Some(task_id) = args.get(1) else {
                return Err("hosted task get requires a task id".to_string());
            };
            if args.len() != 2 {
                return Err("hosted task get accepts exactly one task id".to_string());
            }
            Ok(CliAction::Hosted {
                command: HostedCommand::TaskGet {
                    task_id: task_id.clone(),
                },
                output_format,
            })
        }
        "runtime" => {
            let Some(task_id) = args.get(1) else {
                return Err("hosted task runtime requires a task id".to_string());
            };
            if args.len() != 2 {
                return Err("hosted task runtime accepts exactly one task id".to_string());
            }
            Ok(CliAction::Hosted {
                command: HostedCommand::TaskRuntime {
                    task_id: task_id.clone(),
                },
                output_format,
            })
        }
        "reconcile" => {
            let Some(task_id) = args.get(1) else {
                return Err("hosted task reconcile requires a task id".to_string());
            };
            if args.len() != 2 {
                return Err("hosted task reconcile accepts exactly one task id".to_string());
            }
            Ok(CliAction::Hosted {
                command: HostedCommand::TaskReconcile {
                    task_id: task_id.clone(),
                },
                output_format,
            })
        }
        "run" => {
            let Some(task_id) = args.get(1) else {
                return Err("hosted task run requires a task id".to_string());
            };
            if args.len() != 2 {
                return Err("hosted task run accepts exactly one task id".to_string());
            }
            Ok(CliAction::Hosted {
                command: HostedCommand::TaskRun {
                    task_id: task_id.clone(),
                },
                output_format,
            })
        }
        "cancel" => {
            let Some(task_id) = args.get(1) else {
                return Err("hosted task cancel requires a task id".to_string());
            };
            if args.len() != 2 {
                return Err("hosted task cancel accepts exactly one task id".to_string());
            }
            Ok(CliAction::Hosted {
                command: HostedCommand::TaskCancel {
                    task_id: task_id.clone(),
                },
                output_format,
            })
        }
        "approval" => {
            let Some(task_id) = args.get(1) else {
                return Err("hosted task approval requires a task id".to_string());
            };
            let Some(action) = args.get(2) else {
                return Err("hosted task approval requires an action: retry, cancel, or ack".to_string());
            };
            let action = HostedApprovalAction::parse(action)?;
            let mut resolved_by = None;
            let mut reason = None;
            let mut approval_kind = "orphaned_hosted_agent".to_string();
            let mut index = 3;
            while index < args.len() {
                match args[index].as_str() {
                    "--resolved-by" => {
                        let Some(value) = args.get(index + 1) else {
                            return Err(
                                "hosted task approval requires a value after --resolved-by"
                                    .to_string(),
                            );
                        };
                        resolved_by = Some(value.clone());
                        index += 2;
                    }
                    "--reason" => {
                        let Some(value) = args.get(index + 1) else {
                            return Err(
                                "hosted task approval requires a value after --reason".to_string()
                            );
                        };
                        reason = Some(value.clone());
                        index += 2;
                    }
                    "--kind" => {
                        let Some(value) = args.get(index + 1) else {
                            return Err(
                                "hosted task approval requires a value after --kind".to_string()
                            );
                        };
                        approval_kind = value.clone();
                        index += 2;
                    }
                    other => {
                        return Err(format!(
                            "unsupported hosted task approval argument: {other}. Use --resolved-by, --reason, or --kind."
                        ));
                    }
                }
            }

            if approval_kind == "orphaned_hosted_agent" {
                if !matches!(action, HostedApprovalAction::Retry | HostedApprovalAction::Cancel) {
                    return Err("orphaned_hosted_agent approval supports actions: retry or cancel".to_string());
                }
            } else if approval_kind == "github_review_followup" {
                if !matches!(action, HostedApprovalAction::Ack | HostedApprovalAction::Retry) {
                    return Err("github_review_followup approval supports actions: ack or retry".to_string());
                }
            } else {
                return Err(format!(
                    "unsupported hosted approval kind: {approval_kind} (expected orphaned_hosted_agent or github_review_followup)"
                ));
            }

            Ok(CliAction::Hosted {
                command: HostedCommand::TaskApproval {
                    task_id: task_id.clone(),
                    action,
                    resolved_by,
                    reason,
                    approval_kind,
                },
                output_format,
            })
        }
        other => Err(format!(
            "unsupported hosted task subcommand: {other} (expected get, runtime, reconcile, run, cancel, or approval)"
        )),
    }
}

fn is_help_flag(value: &str) -> bool {
    matches!(value, "--help" | "-h")
}

fn parse_single_word_command_alias(
    rest: &[String],
    model: &str,
    permission_mode_override: Option<PermissionMode>,
    output_format: CliOutputFormat,
    provider: Option<String>,
) -> Option<Result<CliAction, String>> {
    if rest.len() != 1 {
        return None;
    }

    match rest[0].as_str() {
        "help" => Some(Ok(CliAction::Help { output_format })),
        "version" => Some(Ok(CliAction::Version { output_format })),
        "status" => Some(Ok(CliAction::Status {
            model: model.to_string(),
            provider: provider.clone(),
            permission_mode: permission_mode_override.unwrap_or_else(default_permission_mode),
            output_format,
        })),
        "config" => Some(Ok(CliAction::Config {
            section: None,
            output_format,
        })),
        "telemetry" => Some(Ok(CliAction::Telemetry {
            output_format,
            action: None,
            target: None,
        })),
        "sandbox" => Some(Ok(CliAction::Sandbox { output_format })),
        "doctor" => Some(Ok(CliAction::Doctor { output_format })),
        other => bare_slash_command_guidance(other).map(Err),
    }
}

fn bare_slash_command_guidance(command_name: &str) -> Option<String> {
    if matches!(
        command_name,
        "dump-manifests"
            | "bootstrap-plan"
            | "agents"
            | "mcp"
            | "skills"
            | "system-prompt"
            | "init"
            | "prompt"
    ) {
        return None;
    }
    let slash_command = slash_command_specs()
        .iter()
        .find(|spec| spec.name == command_name)?;
    let guidance = if slash_command.resume_supported {
        format!(
            "`orbit {command_name}` is a slash command. Use `orbit --resume SESSION.jsonl /{command_name}` or start `orbit` and run `/{command_name}`."
        )
    } else {
        format!(
            "`orbit {command_name}` is a slash command. Start `orbit` and run `/{command_name}` inside the REPL."
        )
    };
    Some(guidance)
}

fn join_optional_args(args: &[String]) -> Option<String> {
    let joined = args.join(" ");
    let trimmed = joined.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn supported_config_sections_phrase() -> String {
    match SUPPORTED_CONFIG_SECTIONS {
        [] => String::new(),
        [only] => (*only).to_string(),
        [rest @ .., last] => format!("{}, or {}", rest.join(", "), last),
    }
}

fn unsupported_config_section_message(section: &str) -> String {
    format!(
        "Unsupported config section '{section}'. Use {}.",
        supported_config_sections_phrase()
    )
}

fn config_section_status(section_present: bool) -> &'static str {
    if section_present {
        "set"
    } else {
        "unset"
    }
}

fn report_row(label: &str, value: impl std::fmt::Display) -> String {
    format!("  {label:<16} {value}")
}

enum ConfigSectionResolution {
    Supported { rendered_value: Option<String> },
    Unsupported,
}

struct DiscoveredConfigFileSummary {
    source: &'static str,
    status: &'static str,
    path: String,
}

struct TelemetryTargetStatus {
    target: String,
    settings_path: PathBuf,
    settings_status: &'static str,
    enabled: Option<bool>,
    path: Option<String>,
}

fn resolve_config_section(
    runtime_config: &orbit_runtime::RuntimeConfig,
    section: &str,
) -> ConfigSectionResolution {
    let value = match section {
        "env" => runtime_config.get("env"),
        "hooks" => runtime_config.get("hooks"),
        "model" => runtime_config.get("model"),
        "telemetry" => runtime_config.get("telemetry"),
        "plugins" => runtime_config
            .get("plugins")
            .or_else(|| runtime_config.get("enabledPlugins")),
        _ => return ConfigSectionResolution::Unsupported,
    };

    ConfigSectionResolution::Supported {
        rendered_value: value.map(|value| value.render()),
    }
}

fn summarize_discovered_config_files(
    discovered: &[orbit_runtime::ConfigEntry],
    runtime_config: &orbit_runtime::RuntimeConfig,
) -> Vec<DiscoveredConfigFileSummary> {
    discovered
        .iter()
        .map(|entry| {
            let source = match entry.source {
                ConfigSource::User => "user",
                ConfigSource::Project => "project",
                ConfigSource::Local => "local",
            };
            let status = if runtime_config
                .loaded_entries()
                .iter()
                .any(|loaded_entry| loaded_entry.path == entry.path)
            {
                "loaded"
            } else {
                "missing"
            };
            DiscoveredConfigFileSummary {
                source,
                status,
                path: entry.path.display().to_string(),
            }
        })
        .collect()
}

fn telemetry_target_status(cwd: &Path, target: &str) -> TelemetryTargetStatus {
    let settings_path = telemetry_settings_path(cwd, Some(target));
    let mut status = TelemetryTargetStatus {
        target: target.to_string(),
        settings_path: settings_path.clone(),
        settings_status: "missing",
        enabled: None,
        path: None,
    };

    let Ok(contents) = fs::read_to_string(&settings_path) else {
        return status;
    };

    let trimmed = contents.trim();
    if trimmed.is_empty() {
        status.settings_status = "present";
        return status;
    }

    let Ok(parsed) = serde_json::from_str::<Value>(trimmed) else {
        status.settings_status = "invalid";
        return status;
    };

    let Some(object) = parsed.as_object() else {
        status.settings_status = "invalid";
        return status;
    };

    status.settings_status = "present";
    if let Some(telemetry) = object.get("telemetry").and_then(Value::as_object) {
        status.enabled = telemetry.get("enabled").and_then(Value::as_bool);
        status.path = telemetry
            .get("path")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
    }

    status
}

fn parse_config_cli_action(
    args: &[String],
    output_format: CliOutputFormat,
) -> Result<CliAction, String> {
    match args {
        [] => Ok(CliAction::Config {
            section: None,
            output_format,
        }),
        [section] => Ok(CliAction::Config {
            section: Some(section.to_string()),
            output_format,
        }),
        _ => Err(format!(
            "config accepts at most one section argument: {CONFIG_SECTION_ARGUMENT_HINT}"
        )),
    }
}

fn parse_direct_slash_cli_action(
    rest: &[String],
    model: String,
    output_format: CliOutputFormat,
    allowed_tools: Option<AllowedToolSet>,
    permission_mode: PermissionMode,
    provider: Option<String>,
) -> Result<CliAction, String> {
    let raw = rest.join(" ");
    match SlashCommand::parse(&raw) {
        Ok(Some(SlashCommand::Help)) => Ok(CliAction::Help { output_format }),
        Ok(Some(SlashCommand::Agents { args })) => Ok(CliAction::Agents {
            args,
            output_format,
        }),
        Ok(Some(SlashCommand::Mcp { action, target })) => Ok(CliAction::Mcp {
            args: match (action, target) {
                (None, None) => None,
                (Some(action), None) => Some(action),
                (Some(action), Some(target)) => Some(format!("{action} {target}")),
                (None, Some(target)) => Some(target),
            },
            output_format,
        }),
        Ok(Some(SlashCommand::Skills { args })) => {
            match classify_skills_slash_command(args.as_deref()) {
                SkillSlashDispatch::Invoke(prompt) => Ok(CliAction::Prompt {
                    prompt,
                    model,
                    provider: provider.clone(),
                    output_format,
                    allowed_tools,
                    permission_mode,
                }),
                SkillSlashDispatch::Local => Ok(CliAction::Skills {
                    args,
                    output_format,
                }),
            }
        }
        Ok(Some(SlashCommand::Unknown(name))) => Err(format_unknown_direct_slash_command(&name)),
        Ok(Some(command)) => Err({
            let _ = command;
            format!(
                "`orbit {command_name}` is a slash command. Start `orbit` and run it there, or use `orbit --resume SESSION.jsonl {command_name}` / `orbit --resume {latest} {command_name}` when the command is marked [resume] in /help.",
                command_name = rest[0],
                latest = LATEST_SESSION_REFERENCE,
            )
        }),
        Ok(None) => Err(format!("unknown subcommand: {}", rest[0])),
        Err(error) => Err(error.to_string()),
    }
}

fn format_unknown_option(option: &str) -> String {
    let mut message = format!("unknown option: {option}");
    if let Some(suggestion) = suggest_closest_term(option, CLI_OPTION_SUGGESTIONS) {
        message.push_str("\nDid you mean ");
        message.push_str(suggestion);
        message.push('?');
    }
    message.push_str("\nRun `orbit --help` for usage.");
    message
}

fn format_unknown_direct_slash_command(name: &str) -> String {
    let mut message = format!("unknown slash command outside the REPL: /{name}");
    if let Some(suggestions) = render_suggestion_line("Did you mean", &suggest_slash_commands(name))
    {
        message.push('\n');
        message.push_str(&suggestions);
    }
    if let Some(note) = omc_compatibility_note_for_unknown_slash_command(name) {
        message.push('\n');
        message.push_str(note);
    }
    message.push_str("\nRun `orbit --help` for CLI usage, or start `orbit` and use /help.");
    message
}

fn format_unknown_slash_command(name: &str) -> String {
    let mut message = format!("Unknown slash command: /{name}");
    if let Some(suggestions) = render_suggestion_line("Did you mean", &suggest_slash_commands(name))
    {
        message.push('\n');
        message.push_str(&suggestions);
    }
    if let Some(note) = omc_compatibility_note_for_unknown_slash_command(name) {
        message.push('\n');
        message.push_str(note);
    }
    message.push_str("\n  Help             /help lists available slash commands");
    message
}

fn omc_compatibility_note_for_unknown_slash_command(name: &str) -> Option<&'static str> {
    name.starts_with("oh-my-claudecode:")
        .then_some(
            "Compatibility note: `/oh-my-claudecode:*` is a Claude Code/OMC plugin command. `orbit` does not yet load plugin slash commands, Claude statusline stdin, or OMC session hooks.",
        )
}

fn render_suggestion_line(label: &str, suggestions: &[String]) -> Option<String> {
    (!suggestions.is_empty()).then(|| format!("  {label:<16} {}", suggestions.join(", "),))
}

fn suggest_slash_commands(input: &str) -> Vec<String> {
    let mut candidates = slash_command_specs()
        .iter()
        .flat_map(|spec| {
            std::iter::once(spec.name)
                .chain(spec.aliases.iter().copied())
                .map(|name| format!("/{name}"))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    candidates.sort();
    candidates.dedup();
    let candidate_refs = candidates.iter().map(String::as_str).collect::<Vec<_>>();
    ranked_suggestions(input.trim_start_matches('/'), &candidate_refs)
        .into_iter()
        .map(str::to_string)
        .collect()
}

fn suggest_closest_term<'a>(input: &str, candidates: &'a [&'a str]) -> Option<&'a str> {
    ranked_suggestions(input, candidates).into_iter().next()
}

fn ranked_suggestions<'a>(input: &str, candidates: &'a [&'a str]) -> Vec<&'a str> {
    let normalized_input = input.trim_start_matches('/').to_ascii_lowercase();
    let mut ranked = candidates
        .iter()
        .filter_map(|candidate| {
            let normalized_candidate = candidate.trim_start_matches('/').to_ascii_lowercase();
            let distance = levenshtein_distance(&normalized_input, &normalized_candidate);
            let prefix_bonus = usize::from(
                !(normalized_candidate.starts_with(&normalized_input)
                    || normalized_input.starts_with(&normalized_candidate)),
            );
            let score = distance + prefix_bonus;
            (score <= 4).then_some((score, *candidate))
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| left.cmp(right).then_with(|| left.1.cmp(right.1)));
    ranked
        .into_iter()
        .map(|(_, candidate)| candidate)
        .take(3)
        .collect()
}

fn levenshtein_distance(left: &str, right: &str) -> usize {
    if left.is_empty() {
        return right.chars().count();
    }
    if right.is_empty() {
        return left.chars().count();
    }

    let right_chars = right.chars().collect::<Vec<_>>();
    let mut previous = (0..=right_chars.len()).collect::<Vec<_>>();
    let mut current = vec![0; right_chars.len() + 1];

    for (left_index, left_char) in left.chars().enumerate() {
        current[0] = left_index + 1;
        for (right_index, right_char) in right_chars.iter().enumerate() {
            let substitution_cost = usize::from(left_char != *right_char);
            current[right_index + 1] = (previous[right_index + 1] + 1)
                .min(current[right_index] + 1)
                .min(previous[right_index] + substitution_cost);
        }
        previous.clone_from(&current);
    }

    previous[right_chars.len()]
}

fn resolve_model_alias(model: &str) -> &str {
    match model {
        "opus" => "claude-opus-4-6",
        "sonnet" => "claude-sonnet-4-6",
        "haiku" => "claude-haiku-4-5-20251213",
        _ => model,
    }
}

fn normalize_allowed_tools(values: &[String]) -> Result<Option<AllowedToolSet>, String> {
    if values.is_empty() {
        return Ok(None);
    }
    current_tool_registry()?.normalize_allowed_tools(values)
}

fn current_tool_registry() -> Result<GlobalToolRegistry, String> {
    let cwd = env::current_dir().map_err(|error| error.to_string())?;
    let loader = ConfigLoader::default_for(&cwd);
    let runtime_config = loader.load().map_err(|error| error.to_string())?;
    let state = build_runtime_plugin_state_with_loader(&cwd, &loader, &runtime_config)
        .map_err(|error| error.to_string())?;
    let registry = state.tool_registry.clone();
    if let Some(mcp_state) = state.mcp_state {
        mcp_state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .shutdown()
            .map_err(|error| error.to_string())?;
    }
    Ok(registry)
}

fn parse_permission_mode_arg(value: &str) -> Result<PermissionMode, String> {
    normalize_permission_mode(value)
        .ok_or_else(|| {
            format!(
                "unsupported permission mode '{value}'. Use read-only, workspace-write, or danger-full-access."
            )
        })
        .map(permission_mode_from_label)
}

fn permission_mode_from_label(mode: &str) -> PermissionMode {
    match mode {
        "read-only" => PermissionMode::ReadOnly,
        "workspace-write" => PermissionMode::WorkspaceWrite,
        "danger-full-access" => PermissionMode::DangerFullAccess,
        other => panic!("unsupported permission mode label: {other}"),
    }
}

fn permission_mode_from_resolved(mode: ResolvedPermissionMode) -> PermissionMode {
    match mode {
        ResolvedPermissionMode::ReadOnly => PermissionMode::ReadOnly,
        ResolvedPermissionMode::WorkspaceWrite => PermissionMode::WorkspaceWrite,
        ResolvedPermissionMode::DangerFullAccess => PermissionMode::DangerFullAccess,
    }
}

fn default_permission_mode() -> PermissionMode {
    env::var("RUSTY_CLAUDE_PERMISSION_MODE")
        .ok()
        .as_deref()
        .and_then(normalize_permission_mode)
        .map(permission_mode_from_label)
        .or_else(config_permission_mode_for_current_dir)
        .unwrap_or(PermissionMode::DangerFullAccess)
}

fn config_permission_mode_for_current_dir() -> Option<PermissionMode> {
    let cwd = env::current_dir().ok()?;
    let loader = ConfigLoader::default_for(&cwd);
    loader
        .load()
        .ok()?
        .permission_mode()
        .map(permission_mode_from_resolved)
}

fn filter_tool_specs(
    tool_registry: &GlobalToolRegistry,
    allowed_tools: Option<&AllowedToolSet>,
) -> Vec<ToolDefinition> {
    tool_registry.definitions(allowed_tools)
}

fn parse_system_prompt_args(
    args: &[String],
    output_format: CliOutputFormat,
) -> Result<CliAction, String> {
    let mut cwd = env::current_dir().map_err(|error| error.to_string())?;
    let mut date = DEFAULT_DATE.to_string();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--cwd" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --cwd".to_string())?;
                cwd = PathBuf::from(value);
                index += 2;
            }
            "--date" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --date".to_string())?;
                date.clone_from(value);
                index += 2;
            }
            other => return Err(format!("unknown system-prompt option: {other}")),
        }
    }

    Ok(CliAction::PrintSystemPrompt {
        cwd,
        date,
        output_format,
    })
}

fn parse_resume_args(args: &[String], output_format: CliOutputFormat) -> Result<CliAction, String> {
    let (session_path, command_tokens): (PathBuf, &[String]) = match args.first() {
        None => (PathBuf::from(LATEST_SESSION_REFERENCE), &[]),
        Some(first) if looks_like_slash_command_token(first) => {
            (PathBuf::from(LATEST_SESSION_REFERENCE), args)
        }
        Some(first) => (PathBuf::from(first), &args[1..]),
    };
    let mut commands = Vec::new();
    let mut current_command = String::new();

    for token in command_tokens {
        if token.trim_start().starts_with('/') {
            if resume_command_can_absorb_token(&current_command, token) {
                current_command.push(' ');
                current_command.push_str(token);
                continue;
            }
            if !current_command.is_empty() {
                commands.push(current_command);
            }
            current_command = String::from(token.as_str());
            continue;
        }

        if current_command.is_empty() {
            return Err("--resume trailing arguments must be slash commands".to_string());
        }

        current_command.push(' ');
        current_command.push_str(token);
    }

    if !current_command.is_empty() {
        commands.push(current_command);
    }

    Ok(CliAction::ResumeSession {
        session_path,
        commands,
        output_format,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiagnosticLevel {
    Ok,
    Warn,
    Fail,
}

impl DiagnosticLevel {
    fn label(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Warn => "warn",
            Self::Fail => "fail",
        }
    }

    fn is_failure(self) -> bool {
        matches!(self, Self::Fail)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DiagnosticCheck {
    name: &'static str,
    level: DiagnosticLevel,
    summary: String,
    details: Vec<String>,
    data: Map<String, Value>,
}

impl DiagnosticCheck {
    fn new(name: &'static str, level: DiagnosticLevel, summary: impl Into<String>) -> Self {
        Self {
            name,
            level,
            summary: summary.into(),
            details: Vec::new(),
            data: Map::new(),
        }
    }

    fn with_details(mut self, details: Vec<String>) -> Self {
        self.details = details;
        self
    }

    fn with_data(mut self, data: Map<String, Value>) -> Self {
        self.data = data;
        self
    }

    fn json_value(&self) -> Value {
        let mut value = Map::from_iter([
            (
                "name".to_string(),
                Value::String(self.name.to_ascii_lowercase()),
            ),
            (
                "status".to_string(),
                Value::String(self.level.label().to_string()),
            ),
            ("summary".to_string(), Value::String(self.summary.clone())),
            (
                "details".to_string(),
                Value::Array(
                    self.details
                        .iter()
                        .cloned()
                        .map(Value::String)
                        .collect::<Vec<_>>(),
                ),
            ),
        ]);
        value.extend(self.data.clone());
        Value::Object(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DoctorReport {
    checks: Vec<DiagnosticCheck>,
}

impl DoctorReport {
    fn counts(&self) -> (usize, usize, usize) {
        (
            self.checks
                .iter()
                .filter(|check| check.level == DiagnosticLevel::Ok)
                .count(),
            self.checks
                .iter()
                .filter(|check| check.level == DiagnosticLevel::Warn)
                .count(),
            self.checks
                .iter()
                .filter(|check| check.level == DiagnosticLevel::Fail)
                .count(),
        )
    }

    fn has_failures(&self) -> bool {
        self.checks.iter().any(|check| check.level.is_failure())
    }

    fn render(&self) -> String {
        let (ok_count, warn_count, fail_count) = self.counts();
        let mut lines = vec![
            "Doctor".to_string(),
            format!(
                "Summary\n  OK               {ok_count}\n  Warnings         {warn_count}\n  Failures         {fail_count}"
            ),
        ];
        lines.extend(self.checks.iter().map(render_diagnostic_check));
        lines.join("\n\n")
    }

    fn json_value(&self) -> Value {
        let report = self.render();
        let (ok_count, warn_count, fail_count) = self.counts();
        json!({
            "kind": "doctor",
            "message": report,
            "report": report,
            "has_failures": self.has_failures(),
            "summary": {
                "total": self.checks.len(),
                "ok": ok_count,
                "warnings": warn_count,
                "failures": fail_count,
            },
            "checks": self
                .checks
                .iter()
                .map(DiagnosticCheck::json_value)
                .collect::<Vec<_>>(),
        })
    }
}

fn render_diagnostic_check(check: &DiagnosticCheck) -> String {
    let mut lines = vec![format!(
        "{}\n  Status           {}\n  Summary          {}",
        check.name,
        check.level.label(),
        check.summary
    )];
    if !check.details.is_empty() {
        lines.push("  Details".to_string());
        lines.extend(check.details.iter().map(|detail| format!("    - {detail}")));
    }
    lines.join("\n")
}

fn render_doctor_report() -> Result<DoctorReport, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let config_loader = ConfigLoader::default_for(&cwd);
    let config = config_loader.load();
    let discovered_config = config_loader.discover();

    // Load core configuration
    let config_manager = ConfigurationManager::load_with_cwd(&cwd).unwrap_or_else(|_| {
        // Fallback to just runtime config if core config fails
        let empty_config = orbit_runtime::RuntimeConfig::empty();
        ConfigurationManager {
            core_config: std::sync::Arc::new(orbit_core::config::ProjectConfig::default()),
            runtime_config: std::sync::Arc::new(empty_config),
        }
    });

    let project_context = ProjectContext::discover_with_git(&cwd, DEFAULT_DATE)?;
    let (project_root, git_branch) =
        parse_git_status_metadata(project_context.git_status.as_deref());
    let git_summary = parse_git_workspace_summary(project_context.git_status.as_deref());
    let empty_config = orbit_runtime::RuntimeConfig::empty();
    let sandbox_config = config.as_ref().ok().unwrap_or(&empty_config);
    let context = StatusContext {
        cwd: cwd.clone(),
        session_path: None,
        loaded_config_files: config
            .as_ref()
            .ok()
            .map_or(0, |runtime_config| runtime_config.loaded_entries().len()),
        discovered_config_files: discovered_config.len(),
        memory_file_count: project_context.instruction_files.len(),
        project_root,
        git_branch,
        git_summary,
        sandbox_status: resolve_sandbox_status(sandbox_config.sandbox(), &cwd),
    };

    // Add core configuration information to the report
    let core_info = format!(
        "Core Configuration:\n  Default Provider: {}\n  Max Concurrent Requests: {}\n  Request Timeout: {}s\n  Telemetry Enabled: {}\n  Plugins Enabled: {}\n  Caching Enabled: {}",
        config_manager.default_provider(),
        config_manager.max_concurrent_requests(),
        config_manager.request_timeout_seconds(),
        config_manager.is_telemetry_enabled(),
        config_manager.are_plugins_enabled(),
        config_manager.is_caching_enabled()
    );
    Ok(DoctorReport {
        checks: vec![
            check_auth_health(),
            check_config_health(&config_loader, config.as_ref()),
            check_core_config_health(&config_manager),
            check_workspace_health(&context),
            check_sandbox_health(&context.sandbox_status),
            check_system_health(&cwd, config.as_ref().ok()),
            check_ide_integration_health(&cwd),
        ],
    })
}

fn check_core_config_health(config_manager: &ConfigurationManager) -> DiagnosticCheck {
    let mut details = vec![
        format!("Default provider: {}", config_manager.default_provider()),
        format!(
            "Max concurrent requests: {}",
            config_manager.max_concurrent_requests()
        ),
        format!(
            "Request timeout: {}s",
            config_manager.request_timeout_seconds()
        ),
        format!(
            "Telemetry enabled: {}",
            config_manager.is_telemetry_enabled()
        ),
        format!("Plugins enabled: {}", config_manager.are_plugins_enabled()),
        format!("Caching enabled: {}", config_manager.is_caching_enabled()),
        format!("Metrics enabled: {}", config_manager.are_metrics_enabled()),
        format!("UI theme: {}", config_manager.ui_theme()),
    ];

    // Check provider configurations
    for provider in ["anthropic", "openai", "xai"] {
        if config_manager.is_provider_enabled(provider) {
            if let Some(model) = config_manager.default_model(provider) {
                details.push(format!("{} model: {}", provider, model));
            }
        }
    }

    DiagnosticCheck::new(
        "Core Configuration",
        DiagnosticLevel::Ok,
        "core configuration loaded successfully",
    )
    .with_details(details)
    .with_data(Map::from_iter([
        (
            "default_provider".to_string(),
            json!(config_manager.default_provider()),
        ),
        (
            "max_concurrent_requests".to_string(),
            json!(config_manager.max_concurrent_requests()),
        ),
        (
            "request_timeout_seconds".to_string(),
            json!(config_manager.request_timeout_seconds()),
        ),
        (
            "telemetry_enabled".to_string(),
            json!(config_manager.is_telemetry_enabled()),
        ),
        (
            "plugins_enabled".to_string(),
            json!(config_manager.are_plugins_enabled()),
        ),
        (
            "caching_enabled".to_string(),
            json!(config_manager.is_caching_enabled()),
        ),
        (
            "metrics_enabled".to_string(),
            json!(config_manager.are_metrics_enabled()),
        ),
        ("ui_theme".to_string(), json!(config_manager.ui_theme())),
    ]))
}

fn run_doctor(output_format: CliOutputFormat) -> Result<(), Box<dyn std::error::Error>> {
    let report = render_doctor_report()?;
    let message = report.render();
    match output_format {
        CliOutputFormat::Text => println!("{message}"),
        CliOutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&report.json_value())?);
        }
    }
    if report.has_failures() {
        return Err("doctor found failing checks".into());
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn check_auth_health() -> DiagnosticCheck {
    let providers = [
        ("anthropic_api_key", "ORBIT_API_KEY"),
        ("anthropic_auth_token", "ORBIT_AUTH_TOKEN"),
        ("openai_api_key", "OPENAI_API_KEY"),
        ("xai_api_key", "XAI_API_KEY"),
        ("frontal_api_key", "FRONTAL_API_KEY"),
        ("bedrock_api_key", "BEDROCK_API_KEY"),
        ("azure_openai_api_key", "AZURE_OPENAI_API_KEY"),
    ];
    let provider_presence = providers
        .iter()
        .map(|(name, key)| {
            (
                (*name).to_string(),
                env::var(key)
                    .ok()
                    .is_some_and(|value| !value.trim().is_empty()),
            )
        })
        .collect::<Vec<_>>();
    let ollama_base_url_present = env::var("OLLAMA_BASE_URL")
        .ok()
        .is_some_and(|value| !value.trim().is_empty());
    let any_cloud_key = provider_presence.iter().any(|(_, present)| *present);

    let mut details = provider_presence
        .iter()
        .map(|(name, present)| {
            format!(
                "Environment       {name}={}",
                if *present { "present" } else { "absent" }
            )
        })
        .collect::<Vec<_>>();
    details.push(format!(
        "Environment       ollama_base_url={}",
        if ollama_base_url_present {
            "present"
        } else {
            "absent"
        }
    ));

    let summary = if any_cloud_key || ollama_base_url_present {
        "environment credentials are configured"
    } else {
        "no provider credentials detected in environment variables"
    };
    DiagnosticCheck::new(
        "Auth",
        if any_cloud_key || ollama_base_url_present {
            DiagnosticLevel::Ok
        } else {
            DiagnosticLevel::Warn
        },
        summary,
    )
    .with_details(details)
    .with_data(Map::from_iter(
        provider_presence
            .into_iter()
            .map(|(name, present)| (name, json!(present)))
            .chain(std::iter::once((
                "ollama_base_url_present".to_string(),
                json!(ollama_base_url_present),
            ))),
    ))
}

fn check_config_health(
    config_loader: &ConfigLoader,
    config: Result<&orbit_runtime::RuntimeConfig, &orbit_runtime::ConfigError>,
) -> DiagnosticCheck {
    let discovered = config_loader.discover();
    let discovered_count = discovered.len();
    let discovered_paths = discovered
        .iter()
        .map(|entry| entry.path.display().to_string())
        .collect::<Vec<_>>();
    match config {
        Ok(runtime_config) => {
            let loaded_entries = runtime_config.loaded_entries();
            let mut details = vec![format!(
                "Config files      loaded {}/{}",
                loaded_entries.len(),
                discovered_count
            )];
            if let Some(model) = runtime_config.model() {
                details.push(format!("Resolved model    {model}"));
            }
            details.push(format!(
                "MCP servers       {}",
                runtime_config.mcp().servers().len()
            ));
            if discovered_paths.is_empty() {
                details.push("Discovered files  <none>".to_string());
            } else {
                details.extend(
                    discovered_paths
                        .iter()
                        .map(|path| format!("Discovered file   {path}")),
                );
            }
            DiagnosticCheck::new(
                "Config",
                if discovered_count == 0 {
                    DiagnosticLevel::Warn
                } else {
                    DiagnosticLevel::Ok
                },
                if discovered_count == 0 {
                    "no config files were found; defaults are active"
                } else {
                    "runtime config loaded successfully"
                },
            )
            .with_details(details)
            .with_data(Map::from_iter([
                ("discovered_files".to_string(), json!(discovered_paths)),
                (
                    "discovered_files_count".to_string(),
                    json!(discovered_count),
                ),
                (
                    "loaded_config_files".to_string(),
                    json!(loaded_entries.len()),
                ),
                ("resolved_model".to_string(), json!(runtime_config.model())),
                (
                    "mcp_servers".to_string(),
                    json!(runtime_config.mcp().servers().len()),
                ),
            ]))
        }
        Err(error) => DiagnosticCheck::new(
            "Config",
            DiagnosticLevel::Fail,
            format!("runtime config failed to load: {error}"),
        )
        .with_details(if discovered_paths.is_empty() {
            vec!["Discovered files  <none>".to_string()]
        } else {
            discovered_paths
                .iter()
                .map(|path| format!("Discovered file   {path}"))
                .collect()
        })
        .with_data(Map::from_iter([
            ("discovered_files".to_string(), json!(discovered_paths)),
            (
                "discovered_files_count".to_string(),
                json!(discovered_count),
            ),
            ("loaded_config_files".to_string(), json!(0)),
            ("resolved_model".to_string(), Value::Null),
            ("mcp_servers".to_string(), Value::Null),
            ("load_error".to_string(), json!(error.to_string())),
        ])),
    }
}

fn check_workspace_health(context: &StatusContext) -> DiagnosticCheck {
    let in_repo = context.project_root.is_some();
    DiagnosticCheck::new(
        "Workspace",
        if in_repo {
            DiagnosticLevel::Ok
        } else {
            DiagnosticLevel::Warn
        },
        if in_repo {
            format!(
                "project root detected on branch {}",
                context.git_branch.as_deref().unwrap_or("unknown")
            )
        } else {
            "current directory is not inside a git project".to_string()
        },
    )
    .with_details(vec![
        format!("Cwd              {}", context.cwd.display()),
        format!(
            "Project root     {}",
            context
                .project_root
                .as_ref()
                .map_or_else(|| "<none>".to_string(), |path| path.display().to_string())
        ),
        format!(
            "Git branch       {}",
            context.git_branch.as_deref().unwrap_or("unknown")
        ),
        format!("Git state        {}", context.git_summary.headline()),
        format!("Changed files    {}", context.git_summary.changed_files),
        format!(
            "Memory files     {} · config files loaded {}/{}",
            context.memory_file_count, context.loaded_config_files, context.discovered_config_files
        ),
    ])
    .with_data(Map::from_iter([
        ("cwd".to_string(), json!(context.cwd.display().to_string())),
        (
            "project_root".to_string(),
            json!(context
                .project_root
                .as_ref()
                .map(|path| path.display().to_string())),
        ),
        ("in_git_repo".to_string(), json!(in_repo)),
        ("git_branch".to_string(), json!(context.git_branch)),
        (
            "git_state".to_string(),
            json!(context.git_summary.headline()),
        ),
        (
            "changed_files".to_string(),
            json!(context.git_summary.changed_files),
        ),
        (
            "memory_file_count".to_string(),
            json!(context.memory_file_count),
        ),
        (
            "loaded_config_files".to_string(),
            json!(context.loaded_config_files),
        ),
        (
            "discovered_config_files".to_string(),
            json!(context.discovered_config_files),
        ),
    ]))
}

fn check_sandbox_health(status: &orbit_runtime::SandboxStatus) -> DiagnosticCheck {
    let degraded = status.enabled && !status.active;
    let mut details = vec![
        format!("Enabled          {}", status.enabled),
        format!("Active           {}", status.active),
        format!("Supported        {}", status.supported),
        format!("Filesystem mode  {}", status.filesystem_mode.as_str()),
        format!("Filesystem live  {}", status.filesystem_active),
    ];
    if let Some(reason) = &status.fallback_reason {
        details.push(format!("Fallback reason  {reason}"));
    }
    DiagnosticCheck::new(
        "Sandbox",
        if degraded {
            DiagnosticLevel::Warn
        } else {
            DiagnosticLevel::Ok
        },
        if degraded {
            "sandbox was requested but is not currently active"
        } else if status.active {
            "sandbox protections are active"
        } else {
            "sandbox is not active for this session"
        },
    )
    .with_details(details)
    .with_data(Map::from_iter([
        ("enabled".to_string(), json!(status.enabled)),
        ("active".to_string(), json!(status.active)),
        ("supported".to_string(), json!(status.supported)),
        (
            "namespace_supported".to_string(),
            json!(status.namespace_supported),
        ),
        (
            "namespace_active".to_string(),
            json!(status.namespace_active),
        ),
        (
            "network_supported".to_string(),
            json!(status.network_supported),
        ),
        ("network_active".to_string(), json!(status.network_active)),
        (
            "filesystem_mode".to_string(),
            json!(status.filesystem_mode.as_str()),
        ),
        (
            "filesystem_active".to_string(),
            json!(status.filesystem_active),
        ),
        ("allowed_mounts".to_string(), json!(status.allowed_mounts)),
        ("in_container".to_string(), json!(status.in_container)),
        (
            "container_markers".to_string(),
            json!(status.container_markers),
        ),
        ("fallback_reason".to_string(), json!(status.fallback_reason)),
    ]))
}

fn check_system_health(
    cwd: &Path,
    config: Option<&orbit_runtime::RuntimeConfig>,
) -> DiagnosticCheck {
    let default_model = config.and_then(orbit_runtime::RuntimeConfig::model);
    let mut details = vec![
        format!("OS               {} {}", env::consts::OS, env::consts::ARCH),
        format!("Working dir      {}", cwd.display()),
        format!("Version          {}", VERSION),
        format!("Build target     {}", BUILD_TARGET.unwrap_or("<unknown>")),
        format!("Git SHA          {}", GIT_SHA.unwrap_or("<unknown>")),
    ];
    if let Some(model) = default_model {
        details.push(format!("Default model    {model}"));
    }
    DiagnosticCheck::new(
        "System",
        DiagnosticLevel::Ok,
        "captured local runtime metadata",
    )
    .with_details(details)
    .with_data(Map::from_iter([
        ("os".to_string(), json!(env::consts::OS)),
        ("arch".to_string(), json!(env::consts::ARCH)),
        ("working_dir".to_string(), json!(cwd.display().to_string())),
        ("version".to_string(), json!(VERSION)),
        ("build_target".to_string(), json!(BUILD_TARGET)),
        ("git_sha".to_string(), json!(GIT_SHA)),
        ("default_model".to_string(), json!(default_model)),
    ]))
}

fn check_ide_integration_health(cwd: &Path) -> DiagnosticCheck {
    let status = collect_ide_status(cwd);
    let mut details = vec![
        format!("Config file       {}", status.config_path.display()),
        format!(
            "Configured target {}",
            status
                .configured_target
                .map_or_else(|| "<none>".to_string(), |target| target.to_string())
        ),
        format!(
            "Extension source  {}",
            status.extension_dev_path.as_ref().map_or_else(
                || "<missing>".to_string(),
                |path| path.display().to_string()
            )
        ),
    ];

    let package_result = package_ide_extension(cwd);
    let package_path = package_result.as_ref().ok().cloned();
    match &package_result {
        Ok(path) => details.push(format!("Package           ok ({})", path.display())),
        Err(error) => details.push(format!("Package           failed ({error})")),
    }

    let mut vscode_install = None;
    let mut vscode_launch = None;
    let mut cursor_install = None;
    let mut cursor_launch = None;
    let mut antigravity_install = None;
    let mut antigravity_launch = None;
    let mut windsurf_install = None;
    let mut windsurf_launch = None;
    let mut failures = Vec::new();

    for target in [
        IdeTarget::Vscode,
        IdeTarget::Cursor,
        IdeTarget::Antigravity,
        IdeTarget::Windsurf,
    ] {
        let available = status.available_targets.contains(&target);
        details.push(format!(
            "{target} binary     {}",
            if available { "detected" } else { "missing" }
        ));

        if !available {
            continue;
        }

        let Some(path) = &package_path else {
            details.push(format!("{target} install    skipped (packaging failed)"));
            details.push(format!("{target} launch     skipped (packaging failed)"));
            continue;
        };

        let install_result = install_packaged_ide_extension(target, path);
        let install_ok = install_result.is_ok();
        details.push(format!(
            "{target} install    {}",
            if install_ok { "ok" } else { "failed" }
        ));
        match target {
            IdeTarget::Vscode => {
                vscode_install = Some(install_ok);
            }
            IdeTarget::Cursor => {
                cursor_install = Some(install_ok);
            }
            IdeTarget::Antigravity => {
                antigravity_install = Some(install_ok);
            }
            IdeTarget::Windsurf => {
                windsurf_install = Some(install_ok);
            }
        }
        if let Err(error) = install_result {
            details.push(format!("{target} launch     skipped (install failed)"));
            failures.push(format!("{target} install failed: {error}"));
            continue;
        }

        let launch_result = launch_ide_target(target, cwd);
        let launch_ok = launch_result.is_ok();
        details.push(format!(
            "{target} launch     {}",
            if launch_ok { "ok" } else { "failed" }
        ));
        match target {
            IdeTarget::Vscode => {
                vscode_launch = Some(launch_ok);
            }
            IdeTarget::Cursor => {
                cursor_launch = Some(launch_ok);
            }
            IdeTarget::Antigravity => {
                antigravity_launch = Some(launch_ok);
            }
            IdeTarget::Windsurf => {
                windsurf_launch = Some(launch_ok);
            }
        }
        if let Err(error) = launch_result {
            failures.push(format!("{target} launch failed: {error}"));
        }
    }

    let available_count = status.available_targets.len();
    let package_ok = package_result.is_ok();
    let level = if !failures.is_empty() {
        DiagnosticLevel::Fail
    } else if status.extension_dev_path.is_none() || available_count == 0 || !package_ok {
        DiagnosticLevel::Warn
    } else {
        DiagnosticLevel::Ok
    };

    if !failures.is_empty() {
        details.extend(
            failures
                .iter()
                .map(|failure| format!("Failure          {failure}")),
        );
    }
    if let Some(config_error) = &status.config_error {
        details.push(format!("Config error      {config_error}"));
    }

    let summary = if !failures.is_empty() {
        "one or more IDE integration actions failed".to_string()
    } else if available_count == 0 {
        "no supported IDE binaries detected (VS Code/Cursor/Antigravity/Windsurf)".to_string()
    } else if !package_ok {
        "IDE binaries detected but extension packaging failed".to_string()
    } else if status.extension_dev_path.is_none() {
        "IDE binaries detected but extension source is missing".to_string()
    } else {
        "IDE binaries, extension packaging, install, and launch checks passed".to_string()
    };

    let available_targets = status
        .available_targets
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    DiagnosticCheck::new("IDE Integration", level, summary)
        .with_details(details)
        .with_data(Map::from_iter([
            ("available_targets".to_string(), json!(available_targets)),
            (
                "extension_source_path".to_string(),
                json!(status
                    .extension_dev_path
                    .as_ref()
                    .map(|path| path.display().to_string())),
            ),
            (
                "packaged_extension_path".to_string(),
                json!(package_path.as_ref().map(|path| path.display().to_string())),
            ),
            ("packaging_ok".to_string(), json!(package_ok)),
            (
                "vscode_binary_detected".to_string(),
                json!(status.available_targets.contains(&IdeTarget::Vscode)),
            ),
            (
                "cursor_binary_detected".to_string(),
                json!(status.available_targets.contains(&IdeTarget::Cursor)),
            ),
            (
                "antigravity_binary_detected".to_string(),
                json!(status.available_targets.contains(&IdeTarget::Antigravity)),
            ),
            (
                "windsurf_binary_detected".to_string(),
                json!(status.available_targets.contains(&IdeTarget::Windsurf)),
            ),
            ("vscode_install_ok".to_string(), json!(vscode_install)),
            ("vscode_launch_ok".to_string(), json!(vscode_launch)),
            ("cursor_install_ok".to_string(), json!(cursor_install)),
            ("cursor_launch_ok".to_string(), json!(cursor_launch)),
            (
                "antigravity_install_ok".to_string(),
                json!(antigravity_install),
            ),
            (
                "antigravity_launch_ok".to_string(),
                json!(antigravity_launch),
            ),
            ("windsurf_install_ok".to_string(), json!(windsurf_install)),
            ("windsurf_launch_ok".to_string(), json!(windsurf_launch)),
            ("config_error".to_string(), json!(status.config_error)),
        ]))
}

fn emit_login_browser_open_failure(
    output_format: CliOutputFormat,
    authorize_url: &str,
    error: &io::Error,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> io::Result<()> {
    writeln!(
        stderr,
        "warning: failed to open browser automatically: {error}"
    )?;
    match output_format {
        CliOutputFormat::Text => writeln!(stdout, "Open this URL manually:\n{authorize_url}"),
        CliOutputFormat::Json => writeln!(stderr, "Open this URL manually:\n{authorize_url}"),
    }
}

fn resume_command_can_absorb_token(current_command: &str, token: &str) -> bool {
    matches!(
        SlashCommand::parse(current_command),
        Ok(Some(SlashCommand::Export { path: None }))
    ) && !looks_like_slash_command_token(token)
}

fn looks_like_slash_command_token(token: &str) -> bool {
    let trimmed = token.trim_start();
    let Some(name) = trimmed.strip_prefix('/').and_then(|value| {
        value
            .split_whitespace()
            .next()
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }) else {
        return false;
    };

    slash_command_specs()
        .iter()
        .any(|spec| spec.name == name || spec.aliases.contains(&name))
}

fn dump_manifests(output_format: CliOutputFormat) -> Result<(), Box<dyn std::error::Error>> {
    let workspace_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let paths = UpstreamPaths::from_workspace_dir(&workspace_dir);
    match extract_manifest(&paths) {
        Ok(manifest) => {
            match output_format {
                CliOutputFormat::Text => {
                    println!("commands: {}", manifest.commands.entries().len());
                    println!("tools: {}", manifest.tools.entries().len());
                    println!("bootstrap phases: {}", manifest.bootstrap.phases().len());
                }
                CliOutputFormat::Json => println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "kind": "dump-manifests",
                        "commands": manifest.commands.entries().len(),
                        "tools": manifest.tools.entries().len(),
                        "bootstrap_phases": manifest.bootstrap.phases().len(),
                    }))?
                ),
            }
            Ok(())
        }
        Err(error) => Err(format!("failed to extract manifests: {error}").into()),
    }
}

fn print_bootstrap_plan(output_format: CliOutputFormat) -> Result<(), Box<dyn std::error::Error>> {
    let phases = orbit_runtime::BootstrapPlan::claude_code_default()
        .phases()
        .iter()
        .map(|phase| format!("{phase:?}"))
        .collect::<Vec<_>>();
    match output_format {
        CliOutputFormat::Text => {
            for phase in &phases {
                println!("- {phase}");
            }
        }
        CliOutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "kind": "bootstrap-plan",
                "phases": phases,
            }))?
        ),
    }
    Ok(())
}

fn print_system_prompt(
    cwd: PathBuf,
    date: String,
    output_format: CliOutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let sections = load_system_prompt(cwd, date, env::consts::OS, "unknown")?;
    let message = sections.join(
        "

",
    );
    match output_format {
        CliOutputFormat::Text => println!("{message}"),
        CliOutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "kind": "system-prompt",
                "message": message,
                "sections": sections,
            }))?
        ),
    }
    Ok(())
}

fn print_version(output_format: CliOutputFormat) -> Result<(), Box<dyn std::error::Error>> {
    match output_format {
        CliOutputFormat::Text => println!("{}", render_version_report()),
        CliOutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&version_json_value())?);
        }
    }
    Ok(())
}

fn render_upgrade_guidance() -> String {
    "Upgrade\n  Preferred        Homebrew\n  Install          brew install --HEAD ./homebrew/orbit.rb\n  Update           brew upgrade --fetch-HEAD orbit\n  Fallback         git pull --ff-only\n                   brew reinstall --HEAD ./homebrew/orbit.rb"
        .to_string()
}

fn run_hosted_command(
    command: HostedCommand,
    output_format: CliOutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let server_url = hosted_server_url();
    let client = HttpClient::builder()
        .timeout(Duration::from_secs(HOSTED_SERVER_TIMEOUT_SECS))
        .build()?;

    match command {
        HostedCommand::PolicyOrphans {
            repository,
            source,
            priority,
        } => {
            let response = fetch_hosted_policy(&client, &server_url, repository, source, priority)?;
            match output_format {
                CliOutputFormat::Text => {
                    println!("{}", render_hosted_policy_report(&server_url, &response))
                }
                CliOutputFormat::Json => println!("{}", serde_json::to_string_pretty(&response)?),
            }
        }
        HostedCommand::EventsWatch { query } => {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            runtime.block_on(watch_hosted_events(&server_url, &query, output_format))?;
        }
        HostedCommand::TasksList { query } => {
            let response = list_hosted_tasks(&client, &server_url, &query)?;
            match output_format {
                CliOutputFormat::Text => {
                    println!(
                        "{}",
                        render_hosted_task_list_report(&server_url, &query, &response)
                    )
                }
                CliOutputFormat::Json => println!("{}", serde_json::to_string_pretty(&response)?),
            }
        }
        HostedCommand::TasksWatch { query } => {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            runtime.block_on(watch_hosted_tasks(&server_url, &query, output_format))?;
        }
        HostedCommand::TaskGet { task_id } => {
            let response = get_hosted_task(&client, &server_url, &task_id)?;
            match output_format {
                CliOutputFormat::Text => println!(
                    "{}",
                    render_hosted_task_report("Hosted task", &server_url, &response)
                ),
                CliOutputFormat::Json => println!("{}", serde_json::to_string_pretty(&response)?),
            }
        }
        HostedCommand::TaskRuntime { task_id } => {
            let response = get_hosted_task_runtime(&client, &server_url, &task_id)?;
            match output_format {
                CliOutputFormat::Text => println!(
                    "{}",
                    render_hosted_task_runtime_report(&server_url, &response)
                ),
                CliOutputFormat::Json => println!("{}", serde_json::to_string_pretty(&response)?),
            }
        }
        HostedCommand::TaskReconcile { task_id } => {
            let response = reconcile_hosted_task(&client, &server_url, &task_id)?;
            match output_format {
                CliOutputFormat::Text => println!(
                    "{}",
                    render_hosted_task_report("Hosted task reconcile", &server_url, &response)
                ),
                CliOutputFormat::Json => println!("{}", serde_json::to_string_pretty(&response)?),
            }
        }
        HostedCommand::TaskRun { task_id } => {
            run_hosted_task_worker(&client, &server_url, &task_id, output_format)?;
        }
        HostedCommand::TaskCancel { task_id } => {
            let response = cancel_hosted_task(&client, &server_url, &task_id)?;
            match output_format {
                CliOutputFormat::Text => println!(
                    "{}",
                    render_hosted_task_report("Hosted task cancel", &server_url, &response)
                ),
                CliOutputFormat::Json => println!("{}", serde_json::to_string_pretty(&response)?),
            }
        }
        HostedCommand::TaskApproval {
            task_id,
            action,
            resolved_by,
            reason,
            approval_kind,
        } => {
            let response = resolve_hosted_task_approval(
                &client,
                &server_url,
                &task_id,
                action,
                resolved_by,
                reason,
                approval_kind,
            )?;
            match output_format {
                CliOutputFormat::Text => println!(
                    "{}",
                    render_hosted_task_report(
                        &format!("Hosted task approval ({})", action.as_str()),
                        &server_url,
                        &response,
                    )
                ),
                CliOutputFormat::Json => println!("{}", serde_json::to_string_pretty(&response)?),
            }
        }
    }

    Ok(())
}

fn hosted_server_url() -> String {
    env::var("ORBIT_SERVER_URL")
        .ok()
        .or_else(|| env::var("ORBIT_SERVER_BASE_URL").ok())
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_HOSTED_SERVER_URL.to_string())
}

fn hosted_server_api_key() -> Option<String> {
    env::var("ORBIT_SERVER_API_KEY")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn authorize_hosted_request(
    builder: reqwest::blocking::RequestBuilder,
) -> reqwest::blocking::RequestBuilder {
    if let Some(api_key) = hosted_server_api_key() {
        builder.header("x-api-key", api_key)
    } else {
        builder
    }
}

fn fetch_hosted_policy(
    client: &HttpClient,
    server_url: &str,
    repository: Option<String>,
    source: Option<String>,
    priority: Option<String>,
) -> Result<HostedPolicyResponse, Box<dyn std::error::Error>> {
    let mut url = reqwest::Url::parse(&format!("{server_url}/v1/policies/orphans"))?;
    {
        let mut query = url.query_pairs_mut();
        if let Some(repository) = repository {
            query.append_pair("repository", &repository);
        }
        if let Some(source) = source {
            query.append_pair("source", &source);
        }
        if let Some(priority) = priority {
            query.append_pair("priority", &priority);
        }
    }
    let response = authorize_hosted_request(client.get(url))
        .send()?
        .error_for_status()?;
    Ok(response.json()?)
}

fn list_hosted_tasks(
    client: &HttpClient,
    server_url: &str,
    query: &HostedTaskListQuery,
) -> Result<Vec<HostedTaskSnapshot>, Box<dyn std::error::Error>> {
    let mut url = reqwest::Url::parse(&format!("{server_url}/v1/tasks"))?;
    {
        let mut query_pairs = url.query_pairs_mut();
        if let Some(status) = &query.status {
            query_pairs.append_pair("status", status);
        }
        if let Some(source) = &query.source {
            query_pairs.append_pair("source", source);
        }
        if let Some(repository) = &query.repository {
            query_pairs.append_pair("repository", repository);
        }
        if let Some(channel_id) = &query.channel_id {
            query_pairs.append_pair("channel_id", channel_id);
        }
        if let Some(thread_ts) = &query.thread_ts {
            query_pairs.append_pair("thread_ts", thread_ts);
        }
        if let Some(needs_followup) = query.needs_followup {
            if needs_followup {
                query_pairs.append_pair("needs_followup", "true");
            }
        }
        if let Some(limit) = query.limit {
            query_pairs.append_pair("limit", &limit.to_string());
        }
    }
    let response = authorize_hosted_request(client.get(url))
        .send()?
        .error_for_status()?;
    Ok(response.json()?)
}

fn get_hosted_task(
    client: &HttpClient,
    server_url: &str,
    task_id: &str,
) -> Result<HostedTaskSnapshot, Box<dyn std::error::Error>> {
    let response = authorize_hosted_request(client.get(format!("{server_url}/v1/tasks/{task_id}")))
        .send()?
        .error_for_status()?;
    Ok(response.json()?)
}

fn get_hosted_task_runtime(
    client: &HttpClient,
    server_url: &str,
    task_id: &str,
) -> Result<HostedTaskRuntimeResponse, Box<dyn std::error::Error>> {
    let response =
        authorize_hosted_request(client.get(format!("{server_url}/v1/tasks/{task_id}/runtime")))
            .send()?
            .error_for_status()?;
    Ok(response.json()?)
}

fn get_hosted_task_github(
    client: &HttpClient,
    server_url: &str,
    task_id: &str,
) -> Result<HostedTaskGithubResponse, Box<dyn std::error::Error>> {
    let response =
        authorize_hosted_request(client.get(format!("{server_url}/v1/tasks/{task_id}/github")))
            .send()?
            .error_for_status()?;
    Ok(response.json()?)
}

fn reconcile_hosted_task(
    client: &HttpClient,
    server_url: &str,
    task_id: &str,
) -> Result<HostedTaskSnapshot, Box<dyn std::error::Error>> {
    let response =
        authorize_hosted_request(client.post(format!("{server_url}/v1/tasks/{task_id}/reconcile")))
            .send()?
            .error_for_status()?;
    Ok(response.json()?)
}

fn cancel_hosted_task(
    client: &HttpClient,
    server_url: &str,
    task_id: &str,
) -> Result<HostedTaskSnapshot, Box<dyn std::error::Error>> {
    let response =
        authorize_hosted_request(client.post(format!("{server_url}/v1/tasks/{task_id}/cancel")))
            .send()?
            .error_for_status()?;
    Ok(response.json()?)
}

fn resolve_hosted_task_approval(
    client: &HttpClient,
    server_url: &str,
    task_id: &str,
    action: HostedApprovalAction,
    resolved_by: Option<String>,
    reason: Option<String>,
    approval_kind: String,
) -> Result<HostedTaskSnapshot, Box<dyn std::error::Error>> {
    let response =
        authorize_hosted_request(client.post(format!("{server_url}/v1/tasks/{task_id}/approval")))
            .json(&json!({
                "approval_kind": approval_kind,
                "action": action.as_str(),
                "resolved_by": resolved_by,
                "reason": reason,
            }))
            .send()?
            .error_for_status()?;
    Ok(response.json()?)
}

fn complete_hosted_task(
    client: &HttpClient,
    server_url: &str,
    task_id: &str,
    finish_reason: &str,
    tokens_output: u64,
    result: Option<String>,
    error: Option<String>,
) -> Result<HostedTaskSnapshot, Box<dyn std::error::Error>> {
    let response =
        authorize_hosted_request(client.post(format!("{server_url}/v1/tasks/{task_id}/complete")))
            .json(&json!({
                "finish_reason": finish_reason,
                "tokens_output": tokens_output,
                "result": result,
                "error": error,
            }))
            .send()?
            .error_for_status()?;
    Ok(response.json()?)
}

fn update_hosted_task_github(
    client: &HttpClient,
    server_url: &str,
    task_id: &str,
    github: &HostedTaskGithubResponse,
) -> Result<HostedTaskGithubResponse, Box<dyn std::error::Error>> {
    let response =
        authorize_hosted_request(client.post(format!("{server_url}/v1/tasks/{task_id}/github")))
            .json(github)
            .send()?
            .error_for_status()?;
    Ok(response.json()?)
}

fn hosted_git_author_name() -> String {
    env::var("ORBIT_GIT_AUTHOR_NAME")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Orbit".to_string())
}

fn hosted_git_author_email() -> String {
    env::var("ORBIT_GIT_AUTHOR_EMAIL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "orbit@localhost".to_string())
}

fn hosted_github_api_base() -> String {
    env::var("ORBIT_GITHUB_API_BASE")
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "https://api.github.com".to_string())
}

fn summarize_hosted_prompt(prompt: &str) -> String {
    let first_line = prompt
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("complete hosted task");
    let mut summary = first_line
        .chars()
        .take(72)
        .collect::<String>()
        .trim()
        .to_string();
    if summary.is_empty() {
        summary = "complete hosted task".to_string();
    }
    summary
}

fn default_hosted_commit_message(task_id: &str, prompt: &str) -> String {
    format!("orbit: {} ({task_id})", summarize_hosted_prompt(prompt))
}

fn default_hosted_pr_draft(
    payload: &HostedTaskWorkerPayload,
    published_branch: &str,
    commit_sha: &str,
) -> Option<GitHubPullRequestDraft> {
    let repo_url = payload.repo_url.as_deref()?;
    parse_github_repo_url(repo_url).ok()?;
    let base = payload.base_ref.as_deref()?.trim();
    if base.is_empty() || base == published_branch {
        return None;
    }

    Some(GitHubPullRequestDraft {
        title: summarize_hosted_prompt(&payload.prompt),
        body: format!(
            "Automated by Orbit hosted task `{}`.\n\nRepository: {}\nBranch: {}\nCommit: {}\n\nPrompt:\n{}\n",
            payload.task_id,
            payload
                .repository
                .as_deref()
                .or(payload.repo_url.as_deref())
                .unwrap_or("unknown"),
            published_branch,
            commit_sha,
            payload.prompt.trim()
        ),
        head: published_branch.to_string(),
        base: base.to_string(),
        draft: true,
    })
}

fn publish_hosted_repo_changes(
    checkout_root: &Path,
    payload: &HostedTaskWorkerPayload,
) -> Result<Option<HostedTaskGithubResponse>, Box<dyn std::error::Error>> {
    let status = match repo_status(checkout_root) {
        Ok(status) => status,
        Err(error) => {
            let message = error.to_string();
            if message.contains("not a git repository") {
                return Ok(None);
            }
            return Err(error.into());
        }
    };
    if !status.dirty {
        return Ok(None);
    }

    let published_branch = payload
        .branch
        .clone()
        .or(status.branch.clone())
        .ok_or_else(|| {
            format!(
                "cannot publish hosted task {} from detached HEAD without a target branch",
                payload.task_id
            )
        })?;
    let commit = stage_and_commit(
        checkout_root,
        &RepoCommitRequest {
            message: default_hosted_commit_message(&payload.task_id, &payload.prompt),
            author_name: Some(hosted_git_author_name()),
            author_email: Some(hosted_git_author_email()),
        },
    )?;
    push_branch(checkout_root, "origin", &published_branch)?;

    let mut github = HostedTaskGithubResponse {
        published_remote: Some("origin".to_string()),
        published_branch: Some(published_branch.clone()),
        published_commit_sha: Some(commit.commit_sha.clone()),
        ..HostedTaskGithubResponse::default()
    };

    if let (Some(repo_url), Some(token), Some(draft)) = (
        payload.repo_url.as_deref(),
        env::var("GITHUB_TOKEN")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        default_hosted_pr_draft(payload, &published_branch, &commit.commit_sha),
    ) {
        let repo = parse_github_repo_url(repo_url)?;
        let client = GitHubClient::new(GitHubClientConfig {
            api_base: hosted_github_api_base(),
            token,
        });
        let pr = client.create_pull_request(&repo, &draft)?;
        github.owner = Some(repo.owner);
        github.repo = Some(repo.repo);
        github.pr_number = Some(pr.number);
        github.pr_url = Some(pr.html_url);
        github.pr_api_url = Some(pr.api_url);
        github.pr_head_ref = Some(pr.head_ref);
        github.pr_base_ref = Some(pr.base_ref);
    }

    Ok(Some(github))
}

fn augment_hosted_result_with_publication(
    result: Option<String>,
    github: &HostedTaskGithubResponse,
) -> Option<String> {
    if github.published_branch.is_none() && github.pr_url.is_none() {
        return result;
    }

    let mut value = result.unwrap_or_default();
    if !value.trim().is_empty() {
        value.push_str("\n\n");
    }
    value.push_str("Publication\n");
    if let Some(branch) = github.published_branch.as_deref() {
        value.push_str(&format!("Branch: {branch}\n"));
    }
    if let Some(commit_sha) = github.published_commit_sha.as_deref() {
        value.push_str(&format!("Commit: {commit_sha}\n"));
    }
    if let Some(pr_url) = github.pr_url.as_deref() {
        value.push_str(&format!("PR: {pr_url}\n"));
    }
    Some(value.trim_end().to_string())
}

fn summarize_hosted_result(value: Option<&str>, fallback: &str) -> String {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(240).collect::<String>())
        .unwrap_or_else(|| fallback.to_string())
}

fn report_hosted_task_to_github(
    task_id: &str,
    server_url: &str,
    github: &HostedTaskGithubResponse,
    result: Option<&str>,
    error: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (Some(owner), Some(repo), Some(token)) = (
        github.owner.as_deref(),
        github.repo.as_deref(),
        env::var("GITHUB_TOKEN")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
    ) else {
        return Ok(());
    };

    let repo_ref = GitHubRepoRef {
        owner: owner.to_string(),
        repo: repo.to_string(),
    };
    let client = GitHubClient::new(GitHubClientConfig {
        api_base: hosted_github_api_base(),
        token,
    });
    let details_url = format!("{server_url}/v1/tasks/{task_id}");

    if let Some(head_sha) = github.published_commit_sha.as_deref() {
        let success = error.is_none();
        let summary = if success {
            summarize_hosted_result(result, "Hosted task completed successfully.")
        } else {
            summarize_hosted_result(error, "Hosted task failed.")
        };
        let text = result
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .or(error.map(str::trim).filter(|value| !value.is_empty()))
            .map(str::to_string);
        let _ = client.create_check_run(
            &repo_ref,
            &GitHubCheckRunDraft {
                name: "orbit/hosted-task".to_string(),
                head_sha: head_sha.to_string(),
                status: "completed".to_string(),
                conclusion: Some(if success {
                    "success".to_string()
                } else {
                    "failure".to_string()
                }),
                details_url: Some(details_url.clone()),
                output: Some(GitHubCheckRunOutput {
                    title: if success {
                        format!("Orbit task {task_id} completed")
                    } else {
                        format!("Orbit task {task_id} failed")
                    },
                    summary,
                    text,
                }),
            },
        );
    }

    if let Some(pr_number) = github.pr_number {
        let body = if let Some(error) = error.filter(|value| !value.trim().is_empty()) {
            format!(
                "Orbit hosted task `{task_id}` failed.\n\nTask: {details_url}\n\nError:\n{error}\n"
            )
        } else {
            format!(
                "Orbit hosted task `{task_id}` completed.\n\nTask: {details_url}\n\nSummary:\n{}\n",
                summarize_hosted_result(result, "Hosted task completed successfully.")
            )
        };
        let _ =
            client.create_issue_comment(&repo_ref, pr_number, &GitHubIssueCommentDraft { body });
    }

    Ok(())
}

fn run_hosted_task_worker(
    client: &HttpClient,
    server_url: &str,
    task_id: &str,
    output_format: CliOutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let payload = load_hosted_task_worker_payload(task_id)?;
    let mut github = get_hosted_task_github(client, server_url, task_id).unwrap_or_default();
    let model = payload
        .model
        .clone()
        .unwrap_or_else(|| DEFAULT_MODEL.to_string());
    let permission_mode = payload
        .permission_mode
        .as_deref()
        .map(parse_permission_mode_arg)
        .transpose()?
        .unwrap_or_else(default_permission_mode);
    let allowed_tools = normalize_allowed_tools(&payload.allowed_tools)?;
    let mut cli = LiveCli::new_with_provider(
        model,
        payload.provider.clone(),
        true,
        allowed_tools,
        permission_mode,
    )?;

    match cli.run_prompt_json_value(&payload.prompt) {
        Ok(summary) => {
            let tokens_output = summary
                .get("usage")
                .and_then(|usage| usage.get("output_tokens"))
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let mut result = summary
                .get("message")
                .and_then(Value::as_str)
                .map(str::to_string);
            if let Some(github_update) = publish_hosted_repo_changes(Path::new("."), &payload)? {
                github = update_hosted_task_github(client, server_url, task_id, &github_update)?;
                result = augment_hosted_result_with_publication(result, &github);
            }
            let _ =
                report_hosted_task_to_github(task_id, server_url, &github, result.as_deref(), None);
            let response = complete_hosted_task(
                client,
                server_url,
                task_id,
                "stop",
                tokens_output,
                result,
                None,
            )?;
            match output_format {
                CliOutputFormat::Text => println!(
                    "{}",
                    render_hosted_task_report("Hosted task run", server_url, &response)
                ),
                CliOutputFormat::Json => println!("{}", serde_json::to_string_pretty(&summary)?),
            }
            Ok(())
        }
        Err(error) => {
            let error_message = error.to_string();
            let _ = report_hosted_task_to_github(
                task_id,
                server_url,
                &github,
                None,
                Some(&error_message),
            );
            let _ = complete_hosted_task(
                client,
                server_url,
                task_id,
                "error",
                0,
                None,
                Some(error_message.clone()),
            );
            Err(error)
        }
    }
}

fn load_hosted_task_worker_payload(
    task_id: &str,
) -> Result<HostedTaskWorkerPayload, Box<dyn std::error::Error>> {
    let task_file = env::var("ORBIT_HOSTED_TASK_FILE")
        .map_err(|_| "ORBIT_HOSTED_TASK_FILE must be set for hosted task runs")?;
    let payload: HostedTaskWorkerPayload = serde_json::from_slice(&fs::read(task_file)?)?;
    if payload.task_id != task_id {
        return Err(format!(
            "hosted task payload task id mismatch: expected {task_id}, got {}",
            payload.task_id
        )
        .into());
    }
    Ok(payload)
}

async fn watch_hosted_events(
    server_url: &str,
    query: &HostedEventWatchQuery,
    output_format: CliOutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let ws_url = hosted_events_ws_url(server_url, &hosted_event_query_pairs(query))?;
    if output_format == CliOutputFormat::Text {
        println!("Hosted events watch");
        println!("{}", report_row("Server", server_url));
        println!("{}", report_row("Stream", &ws_url));
        println!(
            "{}",
            report_row("Filters", format_hosted_event_query(query))
        );
        println!("{}", report_row("Stop", "Ctrl-C"));
    }

    let (stream, _) = connect_async(hosted_events_ws_request(&ws_url)?).await?;
    let (_, mut reader) = stream.split();
    let mut matched_events = 0usize;

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                if output_format == CliOutputFormat::Text {
                    println!("{}", report_row("Status", "stopped"));
                }
                break;
            }
            message = reader.next() => {
                let Some(message) = message else {
                    if output_format == CliOutputFormat::Text {
                        println!("{}", report_row("Status", "stream closed"));
                    }
                    break;
                };
                let message = message?;
                let Some(event) = parse_hosted_event_message(message)? else {
                    continue;
                };
                if !hosted_event_matches_query(&event, query) {
                    continue;
                }

                matched_events += 1;
                match output_format {
                    CliOutputFormat::Text => println!("{}", render_hosted_event_line(&event)),
                    CliOutputFormat::Json => println!("{}", serde_json::to_string(&event)?),
                }

                if query.limit.is_some_and(|limit| matched_events >= limit) {
                    if output_format == CliOutputFormat::Text {
                        println!("{}", report_row("Status", format!("matched limit {matched_events}")));
                    }
                    break;
                }
            }
        }
    }

    Ok(())
}

async fn watch_hosted_tasks(
    server_url: &str,
    query: &HostedTaskListQuery,
    output_format: CliOutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let ws_url = hosted_events_ws_url(server_url, &hosted_task_watch_query_pairs(query))?;
    let client = HttpClient::builder()
        .timeout(Duration::from_secs(HOSTED_SERVER_TIMEOUT_SECS))
        .build()?;
    let initial_query = query.clone();
    let initial_server_url = server_url.to_string();
    let initial_client = client.clone();
    let initial_tasks = tokio::task::spawn_blocking(move || {
        let mut startup_query = initial_query;
        startup_query.limit = None;
        list_hosted_tasks(&initial_client, &initial_server_url, &startup_query)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| -> Box<dyn std::error::Error> { error.into() })?;

    let mut tracked_tasks = initial_tasks
        .into_iter()
        .map(|task| (task.task_id.clone(), task))
        .collect::<BTreeMap<_, _>>();

    if output_format == CliOutputFormat::Text {
        println!("Hosted tasks watch");
        println!("{}", report_row("Server", server_url));
        println!("{}", report_row("Stream", &ws_url));
        println!("{}", report_row("Filters", format_hosted_task_query(query)));
        println!("{}", report_row("Tracked", tracked_tasks.len()));
        let pending_followups = tracked_tasks
            .values()
            .filter(|task| task.github_feedback_required.unwrap_or(false))
            .count();
        if pending_followups > 0 {
            println!("{}", report_row("Follow-up pending", pending_followups));
        }
        println!(
            "{}",
            report_row(
                "Limit",
                query
                    .limit
                    .map_or_else(|| "none".to_string(), |value| value.to_string())
            )
        );
        println!("{}", report_row("Stop", "Ctrl-C"));
    }

    let (stream, _) = connect_async(hosted_events_ws_request(&ws_url)?).await?;
    let (_, mut reader) = stream.split();
    let mut matched_updates = 0usize;

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                if output_format == CliOutputFormat::Text {
                    println!("{}", report_row("Status", "stopped"));
                }
                break;
            }
            message = reader.next() => {
                let Some(message) = message else {
                    if output_format == CliOutputFormat::Text {
                        println!("{}", report_row("Status", "stream closed"));
                    }
                    break;
                };
                let message = message?;
                let Some(event) = parse_hosted_event_message(message)? else {
                    continue;
                };
                let Some(task_id) = event.task_id.clone() else {
                    continue;
                };

                let was_tracked = tracked_tasks.contains_key(&task_id);
                let snapshot = if let Some(snapshot) = hosted_task_snapshot_from_event(&event) {
                    snapshot
                } else {
                    let server_url = server_url.to_string();
                    let fetch_client = client.clone();
                    let fetch_task_id = task_id.clone();
                    tokio::task::spawn_blocking(move || {
                        get_hosted_task(&fetch_client, &server_url, &fetch_task_id)
                            .map_err(|error| error.to_string())
                    })
                    .await
                    .map_err(|error| error.to_string())?
                    .map_err(|error| -> Box<dyn std::error::Error> { error.into() })?
                };
                let matches_filter = hosted_task_matches_query(&snapshot, query);
                if !matches_filter && !was_tracked {
                    continue;
                }

                matched_updates += 1;
                let item = HostedTaskWatchItem {
                    event: event.clone(),
                    task: snapshot.clone(),
                };
                match output_format {
                    CliOutputFormat::Text => println!("{}", render_hosted_task_watch_line(&item)),
                    CliOutputFormat::Json => println!("{}", serde_json::to_string(&item)?),
                }

                if matches_filter && !hosted_task_is_terminal(&snapshot) {
                    tracked_tasks.insert(task_id.clone(), snapshot);
                } else {
                    tracked_tasks.remove(&task_id);
                }

                if query.limit.is_some_and(|limit| matched_updates >= limit) {
                    if output_format == CliOutputFormat::Text {
                        println!("{}", report_row("Status", format!("matched limit {matched_updates}")));
                    }
                    break;
                }
            }
        }
    }

    Ok(())
}

fn hosted_events_ws_url(
    server_url: &str,
    query_pairs: &[(String, String)],
) -> Result<String, Box<dyn std::error::Error>> {
    let mut url = reqwest::Url::parse(server_url)?;
    match url.scheme() {
        "http" => {
            url.set_scheme("ws")
                .map_err(|_| "failed to convert hosted server URL from http to ws")?;
        }
        "https" => {
            url.set_scheme("wss")
                .map_err(|_| "failed to convert hosted server URL from https to wss")?;
        }
        "ws" | "wss" => {}
        other => {
            return Err(format!(
                "unsupported hosted server URL scheme: {other} (expected http or https)"
            )
            .into())
        }
    }
    url.set_path("/v1/events/ws");
    url.query_pairs_mut().clear();
    {
        let mut pairs = url.query_pairs_mut();
        for (key, value) in query_pairs {
            pairs.append_pair(key, value);
        }
    }
    url.set_fragment(None);
    Ok(url.to_string())
}

fn hosted_events_ws_request(
    ws_url: &str,
) -> Result<tokio_tungstenite::tungstenite::http::Request<()>, Box<dyn std::error::Error>> {
    let mut builder = tokio_tungstenite::tungstenite::http::Request::builder().uri(ws_url);
    if let Some(api_key) = hosted_server_api_key() {
        builder = builder.header("x-api-key", api_key);
    }
    Ok(builder.body(())?)
}

fn hosted_event_query_pairs(query: &HostedEventWatchQuery) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    if let Some(task_id) = &query.task_id {
        pairs.push(("task_id".to_string(), task_id.clone()));
    }
    if let Some(topic) = &query.topic {
        pairs.push(("topic".to_string(), topic.clone()));
    }
    if let Some(event) = &query.event {
        pairs.push(("event".to_string(), event.clone()));
    }
    if let Some(status) = &query.status {
        pairs.push(("status".to_string(), status.clone()));
    }
    if let Some(limit) = query.limit {
        pairs.push(("limit".to_string(), limit.to_string()));
    }
    pairs
}

fn hosted_task_watch_query_pairs(query: &HostedTaskListQuery) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    if let Some(source) = &query.source {
        pairs.push(("source".to_string(), source.clone()));
    }
    if let Some(repository) = &query.repository {
        pairs.push(("repository".to_string(), repository.clone()));
    }
    if let Some(channel_id) = &query.channel_id {
        pairs.push(("channel_id".to_string(), channel_id.clone()));
    }
    if let Some(thread_ts) = &query.thread_ts {
        pairs.push(("thread_ts".to_string(), thread_ts.clone()));
    }
    pairs
}

fn parse_hosted_event_message(
    message: WebSocketMessage,
) -> Result<Option<EventEnvelope>, Box<dyn std::error::Error>> {
    match message {
        WebSocketMessage::Text(text) => Ok(Some(serde_json::from_str(&text)?)),
        WebSocketMessage::Binary(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
        WebSocketMessage::Close(_) => Ok(None),
        WebSocketMessage::Ping(_) | WebSocketMessage::Pong(_) | WebSocketMessage::Frame(_) => {
            Ok(None)
        }
    }
}

fn hosted_event_matches_query(event: &EventEnvelope, query: &HostedEventWatchQuery) -> bool {
    matches_optional_filter(query.task_id.as_deref(), event.task_id.as_deref())
        && matches_optional_filter(
            query.topic.as_deref(),
            Some(hosted_event_topic_label(&event.topic)),
        )
        && matches_optional_filter(
            query.event.as_deref(),
            Some(hosted_event_name_label(&event.event)),
        )
        && matches_optional_csv_filter(
            query.status.as_deref(),
            Some(hosted_event_status_label(&event.status)),
        )
}

fn hosted_event_topic_label(topic: &HostedEventTopic) -> &'static str {
    match topic {
        HostedEventTopic::Task => "task",
        HostedEventTopic::Lane => "lane",
        HostedEventTopic::Approval => "approval",
        HostedEventTopic::Memory => "memory",
        HostedEventTopic::Connector => "connector",
    }
}

fn hosted_event_name_label(event: &HostedEventName) -> &'static str {
    match event {
        HostedEventName::TaskCreated => "task.created",
        HostedEventName::TaskRouted => "task.routed",
        HostedEventName::TaskCancelled => "task.cancelled",
        HostedEventName::LaneStarted => "lane.started",
        HostedEventName::LaneBlocked => "lane.blocked",
        HostedEventName::LaneGreen => "lane.green",
        HostedEventName::LaneFailed => "lane.failed",
        HostedEventName::ApprovalRequested => "approval.requested",
        HostedEventName::ApprovalResolved => "approval.resolved",
        HostedEventName::MemoryCaptured => "memory.captured",
        HostedEventName::ConnectorEventReceived => "connector.event.received",
    }
}

fn hosted_event_status_label(status: &HostedEventStatus) -> &'static str {
    match status {
        HostedEventStatus::Pending => "pending",
        HostedEventStatus::Running => "running",
        HostedEventStatus::Blocked => "blocked",
        HostedEventStatus::Completed => "completed",
        HostedEventStatus::Failed => "failed",
        HostedEventStatus::Cancelled => "cancelled",
    }
}

fn render_hosted_event_line(event: &EventEnvelope) -> String {
    let mut parts = vec![
        event.emitted_at.clone(),
        hosted_event_name_label(&event.event).to_string(),
        format!("[{}]", hosted_event_status_label(&event.status)),
    ];
    if let Some(task_id) = &event.task_id {
        parts.push(format!("task={task_id}"));
    }
    if let Some(lane_id) = &event.lane_id {
        parts.push(format!("lane={lane_id}"));
    }
    parts.push(format!("topic={}", hosted_event_topic_label(&event.topic)));
    if let Some(payload) = &event.payload {
        parts.push(format!(
            "payload={}",
            truncate_single_line(&payload.to_string(), 120)
        ));
    }
    parts.join(" ")
}

fn hosted_task_matches_query(task: &HostedTaskSnapshot, query: &HostedTaskListQuery) -> bool {
    matches_optional_csv_filter(query.status.as_deref(), Some(task.status.as_str()))
        && matches_optional_filter(query.source.as_deref(), task.source.as_deref())
        && matches_optional_filter(query.repository.as_deref(), task.repository.as_deref())
        && matches_optional_filter(query.channel_id.as_deref(), task.channel_id.as_deref())
        && matches_optional_filter(query.thread_ts.as_deref(), task.thread_ts.as_deref())
}

fn hosted_task_snapshot_from_event(event: &EventEnvelope) -> Option<HostedTaskSnapshot> {
    let task_id = event.task_id.clone()?;
    let payload = event.payload.as_ref();
    let status = payload
        .and_then(|payload| payload.get("task_status"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| infer_hosted_task_status_from_event(event))?;

    Some(HostedTaskSnapshot {
        task_id,
        status,
        repository: payload
            .and_then(|payload| payload.get("repository"))
            .and_then(Value::as_str)
            .map(str::to_string),
        source: payload
            .and_then(|payload| payload.get("source"))
            .and_then(Value::as_str)
            .map(str::to_string),
        user_id: payload
            .and_then(|payload| payload.get("user_id"))
            .and_then(Value::as_str)
            .map(str::to_string),
        channel_id: payload
            .and_then(|payload| payload.get("channel_id"))
            .and_then(Value::as_str)
            .map(str::to_string),
        thread_ts: payload
            .and_then(|payload| payload.get("thread_ts"))
            .and_then(Value::as_str)
            .map(str::to_string),
        worker_id: payload
            .and_then(|payload| payload.get("worker_id"))
            .and_then(Value::as_str)
            .map(str::to_string),
        worker_status: payload
            .and_then(|payload| payload.get("worker_status"))
            .and_then(Value::as_str)
            .map(str::to_string),
        plan_kind: payload
            .and_then(|payload| payload.get("plan_kind"))
            .and_then(Value::as_str)
            .map(str::to_string),
        github_review_state: payload
            .and_then(|payload| payload.get("github_review_state"))
            .and_then(Value::as_str)
            .map(str::to_string),
        github_feedback_required: payload
            .and_then(|payload| payload.get("github_feedback_required"))
            .and_then(Value::as_bool),
        github_feedback_reason: payload
            .and_then(|payload| payload.get("github_feedback_reason"))
            .and_then(Value::as_str)
            .map(str::to_string),
        linear_issue_id: payload
            .and_then(|payload| payload.get("linear_issue_id"))
            .and_then(Value::as_str)
            .map(str::to_string),
        linear_issue_url: payload
            .and_then(|payload| payload.get("linear_issue_url"))
            .and_then(Value::as_str)
            .map(str::to_string),
        linear_issue_state: payload
            .and_then(|payload| payload.get("linear_issue_state"))
            .and_then(Value::as_str)
            .map(str::to_string),
        linear_issue_identifier: payload
            .and_then(|payload| payload.get("linear_issue_identifier"))
            .and_then(Value::as_str)
            .map(str::to_string),
        graphite_stack_id: payload
            .and_then(|payload| payload.get("graphite_stack_id"))
            .and_then(Value::as_str)
            .map(str::to_string),
        graphite_head_branch: payload
            .and_then(|payload| payload.get("graphite_head_branch"))
            .and_then(Value::as_str)
            .map(str::to_string),
        graphite_base_branch: payload
            .and_then(|payload| payload.get("graphite_base_branch"))
            .and_then(Value::as_str)
            .map(str::to_string),
        orphan_policy: None,
        error: payload
            .and_then(|payload| payload.get("error"))
            .and_then(Value::as_str)
            .map(str::to_string),
        result: payload
            .and_then(|payload| payload.get("result"))
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

fn infer_hosted_task_status_from_event(event: &EventEnvelope) -> Option<String> {
    let inferred = match event.event {
        HostedEventName::TaskCreated => "pending",
        HostedEventName::TaskRouted | HostedEventName::LaneStarted => "running",
        HostedEventName::LaneBlocked | HostedEventName::ApprovalRequested => "pending",
        HostedEventName::ApprovalResolved => match event.status {
            HostedEventStatus::Pending
            | HostedEventStatus::Running
            | HostedEventStatus::Blocked => "running",
            HostedEventStatus::Completed => "completed",
            HostedEventStatus::Failed => "failed",
            HostedEventStatus::Cancelled => "cancelled",
        },
        HostedEventName::LaneGreen => "completed",
        HostedEventName::LaneFailed => "failed",
        HostedEventName::TaskCancelled => "cancelled",
        HostedEventName::MemoryCaptured | HostedEventName::ConnectorEventReceived => return None,
    };

    Some(inferred.to_string())
}

fn hosted_task_is_terminal(task: &HostedTaskSnapshot) -> bool {
    matches!(task.status.as_str(), "completed" | "failed" | "cancelled")
}

fn render_hosted_task_watch_line(item: &HostedTaskWatchItem) -> String {
    let task = &item.task;
    let event = &item.event;
    let mut parts = vec![
        event.emitted_at.clone(),
        hosted_event_name_label(&event.event).to_string(),
        format!("[{}]", hosted_event_status_label(&event.status)),
        format!("task={}", task.task_id),
        format!("status={}", task.status),
    ];
    if let Some(source) = &task.source {
        parts.push(format!("source={source}"));
    }
    if let Some(repository) = &task.repository {
        parts.push(format!("repo={repository}"));
    }
    if let Some(worker_status) = &task.worker_status {
        parts.push(format!("worker={worker_status}"));
    }
    if let Some(plan_kind) = &task.plan_kind {
        parts.push(format!("plan={plan_kind}"));
    }
    if task.github_feedback_required.unwrap_or(false) {
        parts.push("followup=github".to_string());
    }
    if let Some(linear) = task
        .linear_issue_identifier
        .as_ref()
        .or_else(|| task.linear_issue_id.as_ref())
    {
        parts.push(format!("linear={linear}"));
    }
    if let Some(stack) = task.graphite_stack_id.as_ref() {
        parts.push(format!("graphite={stack}"));
    }
    if let Some(error) = &task.error {
        parts.push(format!("error={}", truncate_single_line(error, 80)));
    }
    parts.join(" ")
}

fn render_hosted_policy_report(server_url: &str, response: &HostedPolicyResponse) -> String {
    let preview = response
        .preview
        .as_ref()
        .map(format_hosted_preview)
        .unwrap_or_else(|| "global defaults".to_string());
    let default_policy = format_hosted_policy_line(&response.default_policy);
    let effective_policy = format_hosted_policy_line(&response.effective_policy);

    let mut lines = vec![
        "Hosted orphan policy".to_string(),
        report_row("Server", server_url),
        report_row("Preview", preview),
        report_row("Effective", effective_policy),
        report_row("Default", default_policy),
        report_row("Rules", response.configured_rules.len()),
    ];

    if !response.configured_rules.is_empty() {
        lines.push("Configured rules".to_string());
        lines.extend(
            response
                .configured_rules
                .iter()
                .enumerate()
                .map(|(index, rule)| format!("  {}. {}", index + 1, format_hosted_rule(rule))),
        );
    }

    lines.join("\n")
}

fn render_hosted_task_report(title: &str, server_url: &str, task: &HostedTaskSnapshot) -> String {
    let mut lines = vec![
        title.to_string(),
        report_row("Server", server_url),
        report_row("Task", &task.task_id),
        report_row("Status", &task.status),
    ];

    if let Some(repository) = &task.repository {
        lines.push(report_row("Repository", repository));
    }
    if let Some(source) = &task.source {
        lines.push(report_row("Source", source));
    }
    if let Some(plan_kind) = &task.plan_kind {
        lines.push(report_row("Plan kind", plan_kind));
    }
    if let Some(worker_id) = &task.worker_id {
        lines.push(report_row("Worker", worker_id));
    }
    if let Some(worker_status) = &task.worker_status {
        lines.push(report_row("Worker status", worker_status));
    }
    if task.github_feedback_required.unwrap_or(false) {
        let detail = task
            .github_feedback_reason
            .as_deref()
            .unwrap_or("GitHub follow-up required");
        lines.push(report_row("Follow-up", detail));
    } else if let Some(state) = &task.github_review_state {
        lines.push(report_row("Review state", state));
    }
    if let Some(orphan_policy) = &task.orphan_policy {
        lines.push(report_row(
            "Orphan policy",
            format_hosted_policy_line(orphan_policy),
        ));
    }
    if let Some(error) = &task.error {
        lines.push(report_row("Error", error));
    }
    if let Some(result) = &task.result {
        lines.push(report_row("Result", truncate_single_line(result, 120)));
    }

    lines.join("\n")
}

fn render_hosted_task_list_report(
    server_url: &str,
    query: &HostedTaskListQuery,
    tasks: &[HostedTaskSnapshot],
) -> String {
    let mut lines = vec![
        "Hosted tasks".to_string(),
        report_row("Server", server_url),
        report_row("Count", tasks.len()),
        report_row("Filters", format_hosted_task_query(query)),
    ];

    let needs_followup = tasks
        .iter()
        .filter(|task| task.github_feedback_required.unwrap_or(false))
        .count();
    if needs_followup > 0 {
        lines.push(report_row("Follow-up pending", needs_followup));
    }

    if tasks.is_empty() {
        lines.push("  No tasks matched the current filter.".to_string());
        return lines.join("\n");
    }

    lines.push("Results".to_string());
    lines.extend(tasks.iter().enumerate().map(|(index, task)| {
        let mut summary = vec![format!(
            "  {}. {} [{}]",
            index + 1,
            task.task_id,
            task.status
        )];
        if let Some(repository) = &task.repository {
            summary.push(format!("repo={repository}"));
        }
        if let Some(source) = &task.source {
            summary.push(format!("source={source}"));
        }
        if let Some(worker_status) = &task.worker_status {
            summary.push(format!("worker={worker_status}"));
        }
        if let Some(plan_kind) = &task.plan_kind {
            summary.push(format!("plan={plan_kind}"));
        }
        if task.github_feedback_required.unwrap_or(false) {
            summary.push("followup=github".to_string());
        }
        summary.join(" ")
    }));

    lines.join("\n")
}

fn render_hosted_task_runtime_report(
    server_url: &str,
    runtime: &HostedTaskRuntimeResponse,
) -> String {
    let mut lines = vec![
        "Hosted task runtime".to_string(),
        report_row("Server", server_url),
        report_row("Task", &runtime.task_id),
    ];

    if let Some(worker_id) = &runtime.worker_id {
        lines.push(report_row("Worker", worker_id));
    }
    if let Some(worker_status) = &runtime.worker_status {
        lines.push(report_row("Worker status", worker_status));
    }
    if let Some(manifest_file) = &runtime.manifest_file {
        lines.push(report_row("Manifest", manifest_file));
    }
    if let Some(output_file) = &runtime.output_file {
        lines.push(report_row("Output", output_file));
    }
    if let Some(orphan_policy) = &runtime.orphan_policy {
        lines.push(report_row(
            "Orphan policy",
            format_hosted_policy_line(orphan_policy),
        ));
    }

    if let Some(hosted_agent) = &runtime.hosted_agent {
        lines.push("Hosted agent".to_string());
        lines.push(report_row("Found", hosted_agent.found));
        lines.push(report_row("Live control", hosted_agent.live_control));
        lines.push(report_row("Status", &hosted_agent.status));
        lines.push(report_row("Derived state", &hosted_agent.derived_state));
        lines.push(report_row("Orphaned", hosted_agent.orphaned));
        if let Some(detail) = &hosted_agent.detail {
            lines.push(report_row("Detail", detail));
        }
        if let Some(error) = &hosted_agent.error {
            lines.push(report_row("Error", error));
        }
    }

    lines.join("\n")
}

fn format_hosted_preview(preview: &HostedPolicyPreview) -> String {
    let selectors = [
        preview
            .repository
            .as_ref()
            .map(|value| format!("repo={value}")),
        preview
            .source
            .as_ref()
            .map(|value| format!("source={value}")),
        preview
            .priority
            .as_ref()
            .map(|value| format!("priority={value}")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    if selectors.is_empty() {
        "global defaults".to_string()
    } else {
        selectors.join(", ")
    }
}

fn format_hosted_policy_line(policy: &HostedAppliedOrphanPolicy) -> String {
    let selectors = [
        policy
            .match_repository
            .as_ref()
            .map(|value| format!("repo={value}")),
        policy
            .match_source
            .as_ref()
            .map(|value| format!("source={value}")),
        policy
            .match_priority
            .as_ref()
            .map(|value| format!("priority={value}")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    let scope = if selectors.is_empty() {
        policy.source.clone()
    } else {
        format!("{} ({})", policy.source, selectors.join(", "))
    };
    let timings = [
        format!("approval {}s", policy.approval_delay_secs),
        policy
            .auto_retry_after_secs
            .map(|value| format!("retry {}s", value))
            .unwrap_or_else(|| "retry off".to_string()),
        policy
            .auto_cancel_after_secs
            .map(|value| format!("cancel {}s", value))
            .unwrap_or_else(|| "cancel off".to_string()),
    ];
    format!("{scope}; {}", timings.join(", "))
}

fn format_hosted_rule(rule: &HostedPolicyRule) -> String {
    let selectors = [
        rule.repository
            .as_ref()
            .map(|value| format!("repo={value}")),
        rule.source.as_ref().map(|value| format!("source={value}")),
        rule.priority
            .as_ref()
            .map(|value| format!("priority={value}")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    let timings = [
        rule.approval_delay_secs
            .map(|value| format!("approval {}s", value)),
        rule.auto_retry_after_secs
            .map(|value| format!("retry {}s", value)),
        rule.auto_cancel_after_secs
            .map(|value| format!("cancel {}s", value)),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    format!(
        "{} -> {}",
        if selectors.is_empty() {
            "match any".to_string()
        } else {
            selectors.join(", ")
        },
        if timings.is_empty() {
            "inherit defaults".to_string()
        } else {
            timings.join(", ")
        }
    )
}

fn format_hosted_task_query(query: &HostedTaskListQuery) -> String {
    let filters = [
        query.status.as_ref().map(|value| format!("status={value}")),
        query.source.as_ref().map(|value| format!("source={value}")),
        query
            .repository
            .as_ref()
            .map(|value| format!("repo={value}")),
        query
            .channel_id
            .as_ref()
            .map(|value| format!("channel={value}")),
        query
            .thread_ts
            .as_ref()
            .map(|value| format!("thread={value}")),
        query.limit.map(|value| format!("limit={value}")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    if filters.is_empty() {
        "none".to_string()
    } else {
        filters.join(", ")
    }
}

fn format_hosted_event_query(query: &HostedEventWatchQuery) -> String {
    let filters = [
        query.task_id.as_ref().map(|value| format!("task={value}")),
        query.topic.as_ref().map(|value| format!("topic={value}")),
        query.event.as_ref().map(|value| format!("event={value}")),
        query.status.as_ref().map(|value| format!("status={value}")),
        query.limit.map(|value| format!("limit={value}")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    if filters.is_empty() {
        "none".to_string()
    } else {
        filters.join(", ")
    }
}

fn matches_optional_filter(expected: Option<&str>, actual: Option<&str>) -> bool {
    match expected {
        Some(expected) => actual == Some(expected),
        None => true,
    }
}

fn matches_optional_csv_filter(expected: Option<&str>, actual: Option<&str>) -> bool {
    match expected {
        Some(expected) => expected
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .any(|candidate| actual == Some(candidate)),
        None => true,
    }
}

fn truncate_single_line(value: &str, limit: usize) -> String {
    let single_line = value.replace('\n', " ");
    if single_line.chars().count() <= limit {
        return single_line;
    }
    let truncated = single_line
        .chars()
        .take(limit.saturating_sub(3))
        .collect::<String>();
    format!("{truncated}...")
}

fn version_json_value() -> serde_json::Value {
    json!({
        "kind": "version",
        "message": render_version_report(),
        "version": VERSION,
        "git_sha": GIT_SHA,
        "target": BUILD_TARGET,
    })
}

fn resume_session(session_path: &Path, commands: &[String], output_format: CliOutputFormat) {
    let resolved_path = if session_path.exists() {
        session_path.to_path_buf()
    } else {
        match resolve_session_reference(&session_path.display().to_string()) {
            Ok(handle) => handle.path,
            Err(error) => {
                eprintln!("failed to restore session: {error}");
                std::process::exit(1);
            }
        }
    };

    let session = match Session::load_from_path(&resolved_path) {
        Ok(session) => session,
        Err(error) => {
            eprintln!("failed to restore session: {error}");
            std::process::exit(1);
        }
    };

    if commands.is_empty() {
        println!(
            "Restored session from {} ({} messages).",
            resolved_path.display(),
            session.messages.len()
        );
        return;
    }

    let mut session = session;
    for raw_command in commands {
        let command = match SlashCommand::parse(raw_command) {
            Ok(Some(command)) => command,
            Ok(None) => {
                eprintln!("unsupported resumed command: {raw_command}");
                std::process::exit(2);
            }
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(2);
            }
        };
        match run_resume_command(&resolved_path, &session, &command) {
            Ok(ResumeCommandOutcome {
                session: next_session,
                message,
                json,
            }) => {
                session = next_session;
                if output_format == CliOutputFormat::Json {
                    if let Some(value) = json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&value)
                                .expect("resume command json output")
                        );
                    } else if let Some(message) = message {
                        println!("{message}");
                    }
                } else if let Some(message) = message {
                    println!("{message}");
                }
            }
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(2);
            }
        }
    }
}

#[derive(Debug, Clone)]
struct ResumeCommandOutcome {
    session: Session,
    message: Option<String>,
    json: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
struct StatusContext {
    cwd: PathBuf,
    session_path: Option<PathBuf>,
    loaded_config_files: usize,
    discovered_config_files: usize,
    memory_file_count: usize,
    project_root: Option<PathBuf>,
    git_branch: Option<String>,
    git_summary: GitWorkspaceSummary,
    sandbox_status: orbit_runtime::SandboxStatus,
}

#[derive(Debug, Clone, Copy)]
struct StatusUsage {
    message_count: usize,
    turns: u32,
    latest: TokenUsage,
    cumulative: TokenUsage,
    estimated_tokens: usize,
}

#[allow(clippy::struct_field_names)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct GitWorkspaceSummary {
    changed_files: usize,
    staged_files: usize,
    unstaged_files: usize,
    untracked_files: usize,
    conflicted_files: usize,
}

impl GitWorkspaceSummary {
    fn is_clean(self) -> bool {
        self.changed_files == 0
    }

    fn headline(self) -> String {
        if self.is_clean() {
            "clean".to_string()
        } else {
            let mut details = Vec::new();
            if self.staged_files > 0 {
                details.push(format!("{} staged", self.staged_files));
            }
            if self.unstaged_files > 0 {
                details.push(format!("{} unstaged", self.unstaged_files));
            }
            if self.untracked_files > 0 {
                details.push(format!("{} untracked", self.untracked_files));
            }
            if self.conflicted_files > 0 {
                details.push(format!("{} conflicted", self.conflicted_files));
            }
            format!(
                "dirty · {} files · {}",
                self.changed_files,
                details.join(", ")
            )
        }
    }
}

#[cfg(test)]
fn format_unknown_slash_command_message(name: &str) -> String {
    let suggestions = suggest_slash_commands(name);
    let mut message = format!("unknown slash command: /{name}.");
    if !suggestions.is_empty() {
        message.push_str(" Did you mean ");
        message.push_str(&suggestions.join(", "));
        message.push('?');
    }
    if let Some(note) = omc_compatibility_note_for_unknown_slash_command(name) {
        message.push(' ');
        message.push_str(note);
    }
    message.push_str(" Use /help to list available commands.");
    message
}

fn format_model_report(model: &str, message_count: usize, turns: u32) -> String {
    format!(
        "Model
  Current model    {model}
  Session messages {message_count}
  Session turns    {turns}

Usage
  Inspect current model with /model
  Switch models with /model <name>"
    )
}

fn format_model_switch_report(previous: &str, next: &str, message_count: usize) -> String {
    format!(
        "Model updated
  Previous         {previous}
  Current          {next}
  Preserved msgs   {message_count}"
    )
}

fn format_permissions_report(mode: &str) -> String {
    let modes = [
        ("read-only", "Read/search tools only", mode == "read-only"),
        (
            "workspace-write",
            "Edit files inside the workspace",
            mode == "workspace-write",
        ),
        (
            "danger-full-access",
            "Unrestricted tool access",
            mode == "danger-full-access",
        ),
    ]
    .into_iter()
    .map(|(name, description, is_current)| {
        let marker = if is_current {
            "● current"
        } else {
            "○ available"
        };
        format!("  {name:<18} {marker:<11} {description}")
    })
    .collect::<Vec<_>>()
    .join(
        "
",
    );

    format!(
        "Permissions
  Active mode      {mode}
  Mode status      live session default

Modes
{modes}

Usage
  Inspect current mode with /permissions
  Switch modes with /permissions <mode>"
    )
}

fn format_permissions_switch_report(previous: &str, next: &str) -> String {
    format!(
        "Permissions updated
  Result           mode switched
  Previous mode    {previous}
  Active mode      {next}
  Applies to       subsequent tool calls
  Usage            /permissions to inspect current mode"
    )
}

fn format_cost_report(usage: TokenUsage) -> String {
    format!(
        "Cost
  Input tokens     {}
  Output tokens    {}
  Cache create     {}
  Cache read       {}
  Total tokens     {}",
        usage.input_tokens,
        usage.output_tokens,
        usage.cache_creation_input_tokens,
        usage.cache_read_input_tokens,
        usage.total_tokens(),
    )
}

fn format_resume_report(session_path: &str, message_count: usize, turns: u32) -> String {
    format!(
        "Session resumed
  Session file     {session_path}
  Messages         {message_count}
  Turns            {turns}"
    )
}

fn render_resume_usage() -> String {
    format!(
        "Resume
  Usage            /resume <session-path|session-id|{LATEST_SESSION_REFERENCE}>
  Auto-save        .orbit/sessions/<session-id>.{PRIMARY_SESSION_EXTENSION}
  Tip              use /session list to inspect saved sessions"
    )
}

fn format_compact_report(removed: usize, resulting_messages: usize, skipped: bool) -> String {
    if skipped {
        format!(
            "Compact
  Result           skipped
  Reason           session below compaction threshold
  Messages kept    {resulting_messages}"
        )
    } else {
        format!(
            "Compact
  Result           compacted
  Messages removed {removed}
  Messages kept    {resulting_messages}"
        )
    }
}

fn format_auto_compaction_notice(removed: usize) -> String {
    format!("[auto-compacted: removed {removed} messages]")
}

fn parse_git_status_metadata(status: Option<&str>) -> (Option<PathBuf>, Option<String>) {
    parse_git_status_metadata_for(
        &env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        status,
    )
}

fn parse_git_status_branch(status: Option<&str>) -> Option<String> {
    let status = status?;
    let first_line = status.lines().next()?;
    let line = first_line.strip_prefix("## ")?;
    if line.starts_with("HEAD") {
        return Some("detached HEAD".to_string());
    }
    let branch = line.split(['.', ' ']).next().unwrap_or_default().trim();
    if branch.is_empty() {
        None
    } else {
        Some(branch.to_string())
    }
}

fn parse_git_workspace_summary(status: Option<&str>) -> GitWorkspaceSummary {
    let mut summary = GitWorkspaceSummary::default();
    let Some(status) = status else {
        return summary;
    };

    for line in status.lines() {
        if line.starts_with("## ") || line.trim().is_empty() {
            continue;
        }

        summary.changed_files += 1;
        let mut chars = line.chars();
        let index_status = chars.next().unwrap_or(' ');
        let worktree_status = chars.next().unwrap_or(' ');

        if index_status == '?' && worktree_status == '?' {
            summary.untracked_files += 1;
            continue;
        }

        if index_status != ' ' {
            summary.staged_files += 1;
        }
        if worktree_status != ' ' {
            summary.unstaged_files += 1;
        }
        if (matches!(index_status, 'U' | 'A') && matches!(worktree_status, 'U' | 'A'))
            || index_status == 'U'
            || worktree_status == 'U'
        {
            summary.conflicted_files += 1;
        }
    }

    summary
}

fn resolve_git_branch_for(cwd: &Path) -> Option<String> {
    let branch = run_git_capture_in(cwd, &["branch", "--show-current"])?;
    let branch = branch.trim();
    if !branch.is_empty() {
        return Some(branch.to_string());
    }

    let fallback = run_git_capture_in(cwd, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    let fallback = fallback.trim();
    if fallback.is_empty() {
        None
    } else if fallback == "HEAD" {
        Some("detached HEAD".to_string())
    } else {
        Some(fallback.to_string())
    }
}

fn run_git_capture_in(cwd: &Path, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

fn find_git_root_in(cwd: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(cwd)
        .output()?;
    if !output.status.success() {
        return Err("not a git repository".into());
    }
    let path = String::from_utf8(output.stdout)?.trim().to_string();
    if path.is_empty() {
        return Err("empty git root".into());
    }
    Ok(PathBuf::from(path))
}

fn parse_git_status_metadata_for(
    cwd: &Path,
    status: Option<&str>,
) -> (Option<PathBuf>, Option<String>) {
    let branch = resolve_git_branch_for(cwd).or_else(|| parse_git_status_branch(status));
    let project_root = find_git_root_in(cwd).ok();
    (project_root, branch)
}

#[allow(clippy::too_many_lines)]
fn run_resume_command(
    session_path: &Path,
    session: &Session,
    command: &SlashCommand,
) -> Result<ResumeCommandOutcome, Box<dyn std::error::Error>> {
    match command {
        SlashCommand::Help => Ok(ResumeCommandOutcome {
            session: session.clone(),
            message: Some(render_repl_help()),
            json: None,
        }),
        SlashCommand::Compact => {
            let result = orbit_runtime::compact_session(
                session,
                CompactionConfig {
                    max_estimated_tokens: 0,
                    ..CompactionConfig::default()
                },
            );
            let removed = result.removed_message_count;
            let kept = result.compacted_session.messages.len();
            let skipped = removed == 0;
            result.compacted_session.save_to_path(session_path)?;
            Ok(ResumeCommandOutcome {
                session: result.compacted_session,
                message: Some(format_compact_report(removed, kept, skipped)),
                json: None,
            })
        }
        SlashCommand::Clear { confirm } => {
            if !confirm {
                return Ok(ResumeCommandOutcome {
                    session: session.clone(),
                    message: Some(
                        "clear: confirmation required; rerun with /clear --confirm".to_string(),
                    ),
                    json: None,
                });
            }
            let backup_path = write_session_clear_backup(session, session_path)?;
            let previous_session_id = session.session_id.clone();
            let cleared = Session::new();
            let new_session_id = cleared.session_id.clone();
            cleared.save_to_path(session_path)?;
            Ok(ResumeCommandOutcome {
                session: cleared,
                message: Some(format!(
                    "Session cleared\n  Mode             resumed session reset\n  Previous session {previous_session_id}\n  Backup           {}\n  Resume previous  orbit --resume {}\n  New session      {new_session_id}\n  Session file     {}",
                    backup_path.display(),
                    backup_path.display(),
                    session_path.display()
                )),
                json: None,
            })
        }
        SlashCommand::Status => {
            let tracker = UsageTracker::from_session(session);
            let usage = tracker.cumulative_usage();
            let context = status_context(Some(session_path))?;
            Ok(ResumeCommandOutcome {
                session: session.clone(),
                message: Some(format_status_report(
                    "restored-session",
                    StatusUsage {
                        message_count: session.messages.len(),
                        turns: tracker.turns(),
                        latest: tracker.current_turn_usage(),
                        cumulative: usage,
                        estimated_tokens: 0,
                    },
                    default_permission_mode().as_str(),
                    &context,
                )),
                json: Some(status_json_value(
                    "restored-session",
                    StatusUsage {
                        message_count: session.messages.len(),
                        turns: tracker.turns(),
                        latest: tracker.current_turn_usage(),
                        cumulative: usage,
                        estimated_tokens: 0,
                    },
                    default_permission_mode().as_str(),
                    &context,
                )),
            })
        }
        SlashCommand::Sandbox => {
            let cwd = env::current_dir()?;
            let loader = ConfigLoader::default_for(&cwd);
            let runtime_config = loader.load()?;
            let status = resolve_sandbox_status(runtime_config.sandbox(), &cwd);
            Ok(ResumeCommandOutcome {
                session: session.clone(),
                message: Some(format_sandbox_report(&status)),
                json: Some(sandbox_json_value(&status)),
            })
        }
        SlashCommand::Cost => {
            let usage = UsageTracker::from_session(session).cumulative_usage();
            Ok(ResumeCommandOutcome {
                session: session.clone(),
                message: Some(format_cost_report(usage)),
                json: None,
            })
        }
        SlashCommand::Config { section } => Ok(ResumeCommandOutcome {
            session: session.clone(),
            message: Some(render_config_report(section.as_deref())?),
            json: Some(config_json_value(section.as_deref())?),
        }),
        SlashCommand::Telemetry { action, target } => {
            let action = action.as_deref().unwrap_or("status");
            match action {
                "status" => {
                    let cwd = env::current_dir()?;
                    let loader = ConfigLoader::default_for(&cwd);
                    let runtime_config = loader
                        .load()
                        .unwrap_or_else(|_| orbit_runtime::RuntimeConfig::empty());
                    Ok(ResumeCommandOutcome {
                        session: session.clone(),
                        message: Some(render_telemetry_report(target.as_deref())?),
                        json: Some(telemetry_status_json_value(
                            &runtime_config,
                            target.as_deref(),
                        )?),
                    })
                }
                "on" | "off" => {
                    let cwd = env::current_dir()?;
                    let settings_path = update_project_telemetry_settings(
                        &cwd,
                        action == "on",
                        target.as_deref(),
                    )?;
                    let loader = ConfigLoader::default_for(&cwd);
                    let runtime_config = loader.load()?;
                    let resolution = resolve_telemetry_config(Some(&runtime_config));
                    Ok(ResumeCommandOutcome {
                        session: session.clone(),
                        message: Some(telemetry_update_report(
                            action,
                            &settings_path,
                            &runtime_config,
                        )),
                        json: Some(json!({
                            "kind": "telemetry",
                            "status": "updated",
                            "action": action,
                            "target": target,
                            "settings_path": settings_path.display().to_string(),
                            "effective": telemetry_json_value(&resolution, &runtime_config),
                        })),
                    })
                }
                other => Ok(ResumeCommandOutcome {
                    session: session.clone(),
                    message: Some(format!(
                        "Telemetry\n  Result           unsupported\n  Action           {other}\n  Supported        /telemetry [status|on|off] [project|local]"
                    )),
                    json: Some(json!({
                        "kind": "telemetry",
                        "status": "unsupported",
                        "action": other
                    })),
                }),
            }
        }
        SlashCommand::Mcp { action, target } => {
            let cwd = env::current_dir()?;
            let args = match (action.as_deref(), target.as_deref()) {
                (None, None) => None,
                (Some(action), None) => Some(action.to_string()),
                (Some(action), Some(target)) => Some(format!("{action} {target}")),
                (None, Some(target)) => Some(target.to_string()),
            };
            Ok(ResumeCommandOutcome {
                session: session.clone(),
                message: Some(handle_mcp_slash_command(args.as_deref(), &cwd)?),
                json: Some(handle_mcp_slash_command_json(args.as_deref(), &cwd)?),
            })
        }
        SlashCommand::Memory => Ok(ResumeCommandOutcome {
            session: session.clone(),
            message: Some(render_memory_report()?),
            json: None,
        }),
        SlashCommand::Init => {
            let message = init_agents_md()?;
            Ok(ResumeCommandOutcome {
                session: session.clone(),
                message: Some(message.clone()),
                json: Some(init_json_value(&message)),
            })
        }
        SlashCommand::Diff => Ok(ResumeCommandOutcome {
            session: session.clone(),
            message: Some(render_diff_report_for(
                session_path.parent().unwrap_or_else(|| Path::new(".")),
            )?),
            json: None,
        }),
        SlashCommand::Version => Ok(ResumeCommandOutcome {
            session: session.clone(),
            message: Some(render_version_report()),
            json: Some(version_json_value()),
        }),
        SlashCommand::Upgrade => Ok(ResumeCommandOutcome {
            session: session.clone(),
            message: Some(render_upgrade_guidance()),
            json: Some(json!({
                "kind": "upgrade-guidance",
                "install": "brew install --HEAD ./homebrew/orbit.rb",
                "update": "brew upgrade --fetch-HEAD orbit",
                "fallback": [
                    "git pull --ff-only",
                    "brew reinstall --HEAD ./homebrew/orbit.rb"
                ],
            })),
        }),
        SlashCommand::Export { path } => {
            let export_path = resolve_export_path(path.as_deref(), session)?;
            fs::write(&export_path, render_export_text(session))?;
            Ok(ResumeCommandOutcome {
                session: session.clone(),
                message: Some(format!(
                    "Export\n  Result           wrote transcript\n  File             {}\n  Messages         {}",
                    export_path.display(),
                    session.messages.len(),
                )),
                json: None,
            })
        }
        SlashCommand::Agents { args } => {
            let cwd = env::current_dir()?;
            Ok(ResumeCommandOutcome {
                session: session.clone(),
                message: Some(handle_agents_slash_command(args.as_deref(), &cwd)?),
                json: None,
            })
        }
        SlashCommand::Skills { args } => {
            if let SkillSlashDispatch::Invoke(_) = classify_skills_slash_command(args.as_deref()) {
                return Err(
                    "resumed /skills invocations are interactive-only; start `orbit` and run `/skills <skill>` in the REPL".into(),
                );
            }
            let cwd = env::current_dir()?;
            Ok(ResumeCommandOutcome {
                session: session.clone(),
                message: Some(handle_skills_slash_command(args.as_deref(), &cwd)?),
                json: Some(handle_skills_slash_command_json(args.as_deref(), &cwd)?),
            })
        }
        SlashCommand::Doctor => Ok(ResumeCommandOutcome {
            session: session.clone(),
            message: Some(render_doctor_report()?.render()),
            json: None,
        }),
        SlashCommand::Unknown(name) => Err(format_unknown_slash_command(name).into()),
        SlashCommand::Bughunter { .. }
        | SlashCommand::Commit { .. }
        | SlashCommand::Pr { .. }
        | SlashCommand::Issue { .. }
        | SlashCommand::Ultraplan { .. }
        | SlashCommand::Teleport { .. }
        | SlashCommand::DebugToolCall { .. }
        | SlashCommand::Resume { .. }
        | SlashCommand::Model { .. }
        | SlashCommand::Permissions { .. }
        | SlashCommand::Session { .. }
        | SlashCommand::Plugins { .. }
        | SlashCommand::Vim
        | SlashCommand::Stats
        | SlashCommand::Share
        | SlashCommand::Feedback
        | SlashCommand::Files
        | SlashCommand::Fast
        | SlashCommand::Exit
        | SlashCommand::Summary
        | SlashCommand::Desktop
        | SlashCommand::Brief
        | SlashCommand::Advisor
        | SlashCommand::Stickers
        | SlashCommand::Insights
        | SlashCommand::Thinkback
        | SlashCommand::ReleaseNotes
        | SlashCommand::SecurityReview
        | SlashCommand::Keybindings
        | SlashCommand::PrivacySettings
        | SlashCommand::Plan { .. }
        | SlashCommand::Review { .. }
        | SlashCommand::Tasks { .. }
        | SlashCommand::Theme { .. }
        | SlashCommand::Voice { .. }
        | SlashCommand::Usage { .. }
        | SlashCommand::Rename { .. }
        | SlashCommand::Copy { .. }
        | SlashCommand::Hooks { .. }
        | SlashCommand::Context { .. }
        | SlashCommand::Color { .. }
        | SlashCommand::Effort { .. }
        | SlashCommand::Branch { .. }
        | SlashCommand::Rewind { .. }
        | SlashCommand::Ide { .. }
        | SlashCommand::Tag { .. }
        | SlashCommand::OutputStyle { .. }
        | SlashCommand::AddDir { .. } => Err("unsupported resumed slash command".into()),
    }
}

fn run_repl(
    model: String,
    allowed_tools: Option<AllowedToolSet>,
    permission_mode: PermissionMode,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut cli = LiveCli::new(model, true, allowed_tools, permission_mode)?;
    let mut editor =
        input::LineEditor::new("> ", cli.repl_completion_candidates().unwrap_or_default());
    println!("{}", cli.startup_banner());

    loop {
        editor.set_completions(cli.repl_completion_candidates().unwrap_or_default());
        match editor.read_line()? {
            input::ReadOutcome::Submit(input) => {
                let trimmed = input.trim().to_string();
                if trimmed.is_empty() {
                    continue;
                }
                if matches!(trimmed.as_str(), "/exit" | "/quit") {
                    cli.persist_session()?;
                    break;
                }
                match SlashCommand::parse(&trimmed) {
                    Ok(Some(command)) => {
                        if cli.handle_repl_command(command)? {
                            cli.persist_session()?;
                        }
                        continue;
                    }
                    Ok(None) => {}
                    Err(error) => {
                        eprintln!("{error}");
                        continue;
                    }
                }
                editor.push_history(input);
                cli.run_turn(&trimmed)?;
            }
            input::ReadOutcome::Cancel => {}
            input::ReadOutcome::Exit => {
                cli.persist_session()?;
                break;
            }
        }
    }

    Ok(())
}

fn run_repl_with_provider(
    model: String,
    provider: Option<String>,
    allowed_tools: Option<AllowedToolSet>,
    permission_mode: PermissionMode,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut cli =
        LiveCli::new_with_provider(model, provider, true, allowed_tools, permission_mode)?;
    let mut editor =
        input::LineEditor::new("> ", cli.repl_completion_candidates().unwrap_or_default());
    println!("{}", cli.startup_banner());

    loop {
        editor.set_completions(cli.repl_completion_candidates().unwrap_or_default());
        match editor.read_line()? {
            input::ReadOutcome::Submit(input) => {
                let trimmed = input.trim().to_string();
                if trimmed.is_empty() {
                    continue;
                }
                if matches!(trimmed.as_str(), "/exit" | "/quit") {
                    cli.persist_session()?;
                    break;
                }
                match SlashCommand::parse(&trimmed) {
                    Ok(Some(command)) => {
                        if cli.handle_repl_command(command)? {
                            cli.persist_session()?;
                        }
                        continue;
                    }
                    Ok(None) => {}
                    Err(error) => {
                        eprintln!("{error}");
                        continue;
                    }
                }
                editor.push_history(input);
                cli.run_turn(&trimmed)?;
            }
            input::ReadOutcome::Cancel => {}
            input::ReadOutcome::Exit => {
                cli.persist_session()?;
                break;
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct SessionHandle {
    id: String,
    path: PathBuf,
}

#[derive(Debug, Clone)]
struct ManagedSessionSummary {
    id: String,
    path: PathBuf,
    modified_epoch_millis: u128,
    message_count: usize,
    parent_session_id: Option<String>,
    branch_name: Option<String>,
}

struct LiveCli {
    model: String,
    provider: Option<String>,
    allowed_tools: Option<AllowedToolSet>,
    permission_mode: PermissionMode,
    system_prompt: Vec<String>,
    runtime: BuiltRuntime,
    session: SessionHandle,
}

struct RuntimePluginState {
    runtime_config: orbit_runtime::RuntimeConfig,
    feature_config: orbit_runtime::RuntimeFeatureConfig,
    tool_registry: GlobalToolRegistry,
    plugin_registry: PluginRegistry,
    mcp_state: Option<Arc<Mutex<RuntimeMcpState>>>,
}

struct RuntimeMcpState {
    runtime: tokio::runtime::Runtime,
    manager: McpServerManager,
    pending_servers: Vec<String>,
    degraded_report: Option<orbit_runtime::McpDegradedReport>,
}

struct BuiltRuntime {
    runtime: Option<ConversationRuntime<GenericRuntimeClient, CliToolExecutor>>,
    plugin_registry: PluginRegistry,
    mcp_state: Option<Arc<Mutex<RuntimeMcpState>>>,
    mcp_active: bool,
    plugins_active: bool,
}

impl BuiltRuntime {
    fn new(
        runtime: ConversationRuntime<GenericRuntimeClient, CliToolExecutor>,
        plugin_registry: PluginRegistry,
        mcp_state: Option<Arc<Mutex<RuntimeMcpState>>>,
        mcp_active: bool,
        plugins_active: bool,
    ) -> Self {
        Self {
            runtime: Some(runtime),
            plugin_registry,
            plugins_active,
            mcp_state,
            mcp_active,
        }
    }

    fn with_hook_abort_signal(mut self, hook_abort_signal: orbit_runtime::HookAbortSignal) -> Self {
        let runtime = self
            .runtime
            .take()
            .expect("runtime should exist before installing hook abort signal");
        self.runtime = Some(runtime.with_hook_abort_signal(hook_abort_signal));
        self
    }

    fn shutdown_plugins(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.plugins_active {
            self.plugin_registry.shutdown()?;
            self.plugins_active = false;
        }
        Ok(())
    }

    fn shutdown_mcp(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.mcp_active {
            if let Some(mcp_state) = &self.mcp_state {
                mcp_state
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .shutdown()?;
            }
            self.mcp_active = false;
        }
        Ok(())
    }
}

impl Deref for BuiltRuntime {
    type Target = ConversationRuntime<GenericRuntimeClient, CliToolExecutor>;

    fn deref(&self) -> &Self::Target {
        self.runtime
            .as_ref()
            .expect("runtime should exist while built runtime is alive")
    }
}

impl DerefMut for BuiltRuntime {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.runtime
            .as_mut()
            .expect("runtime should exist while built runtime is alive")
    }
}

impl Drop for BuiltRuntime {
    fn drop(&mut self) {
        let _ = self.shutdown_mcp();
        let _ = self.shutdown_plugins();
    }
}

#[derive(Debug, Deserialize)]
struct ToolSearchRequest {
    query: String,
    max_results: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct McpToolRequest {
    #[serde(rename = "qualifiedName")]
    qualified_name: Option<String>,
    tool: Option<String>,
    arguments: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct ListMcpResourcesRequest {
    server: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ReadMcpResourceRequest {
    server: String,
    uri: String,
}

impl RuntimeMcpState {
    fn new(
        runtime_config: &orbit_runtime::RuntimeConfig,
    ) -> Result<Option<(Self, orbit_runtime::McpToolDiscoveryReport)>, Box<dyn std::error::Error>>
    {
        let integration_servers = map_runtime_mcp_servers(runtime_config.mcp().servers());
        let mut manager = McpServerManager::from_servers(&integration_servers);
        if manager.server_names().is_empty() && manager.unsupported_servers().is_empty() {
            return Ok(None);
        }

        let runtime = tokio::runtime::Runtime::new()?;
        let discovery = runtime.block_on(manager.discover_tools_best_effort());
        let pending_servers = discovery
            .failed_servers
            .iter()
            .map(|failure| failure.server_name.clone())
            .chain(
                discovery
                    .unsupported_servers
                    .iter()
                    .map(|server| server.server_name.clone()),
            )
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let available_tools = discovery
            .tools
            .iter()
            .map(|tool| tool.qualified_name.clone())
            .collect::<Vec<_>>();
        let failed_server_names = pending_servers.iter().cloned().collect::<BTreeSet<_>>();
        let working_servers = manager
            .server_names()
            .into_iter()
            .filter(|server_name| !failed_server_names.contains(server_name))
            .collect::<Vec<_>>();
        let failed_servers = discovery
            .failed_servers
            .iter()
            .map(|failure| orbit_runtime::McpFailedServer {
                server_name: failure.server_name.clone(),
                phase: orbit_runtime::McpLifecyclePhase::ToolDiscovery,
                error: orbit_runtime::McpErrorSurface::new(
                    orbit_runtime::McpLifecyclePhase::ToolDiscovery,
                    Some(failure.server_name.clone()),
                    failure.error.clone(),
                    std::collections::BTreeMap::new(),
                    true,
                ),
            })
            .chain(discovery.unsupported_servers.iter().map(|server| {
                orbit_runtime::McpFailedServer {
                    server_name: server.server_name.clone(),
                    phase: orbit_runtime::McpLifecyclePhase::ServerRegistration,
                    error: orbit_runtime::McpErrorSurface::new(
                        orbit_runtime::McpLifecyclePhase::ServerRegistration,
                        Some(server.server_name.clone()),
                        server.reason.clone(),
                        std::collections::BTreeMap::from([(
                            "transport".to_string(),
                            format!("{:?}", server.transport).to_ascii_lowercase(),
                        )]),
                        false,
                    ),
                }
            }))
            .collect::<Vec<_>>();
        let degraded_report = (!failed_servers.is_empty()).then(|| {
            orbit_runtime::McpDegradedReport::new(
                working_servers,
                failed_servers,
                available_tools.clone(),
                available_tools,
            )
        });

        Ok(Some((
            Self {
                runtime,
                manager,
                pending_servers,
                degraded_report,
            },
            discovery,
        )))
    }

    fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.runtime.block_on(self.manager.shutdown())?;
        Ok(())
    }

    fn pending_servers(&self) -> Option<Vec<String>> {
        (!self.pending_servers.is_empty()).then(|| self.pending_servers.clone())
    }

    fn degraded_report(&self) -> Option<orbit_runtime::McpDegradedReport> {
        self.degraded_report.clone()
    }

    fn server_names(&self) -> Vec<String> {
        self.manager.server_names()
    }

    fn call_tool(
        &mut self,
        qualified_tool_name: &str,
        arguments: Option<serde_json::Value>,
    ) -> Result<String, ToolError> {
        let response = self
            .runtime
            .block_on(self.manager.call_tool(qualified_tool_name, arguments))
            .map_err(|error| ToolError::new(error.to_string()))?;
        if let Some(error) = response.error {
            return Err(ToolError::new(format!(
                "MCP tool `{qualified_tool_name}` returned JSON-RPC error: {} ({})",
                error.message, error.code
            )));
        }

        let result = response.result.ok_or_else(|| {
            ToolError::new(format!(
                "MCP tool `{qualified_tool_name}` returned no result payload"
            ))
        })?;
        serde_json::to_string_pretty(&result).map_err(|error| ToolError::new(error.to_string()))
    }

    fn list_resources_for_server(&mut self, server_name: &str) -> Result<String, ToolError> {
        let result = self
            .runtime
            .block_on(self.manager.list_resources(server_name))
            .map_err(|error| ToolError::new(error.to_string()))?;
        serde_json::to_string_pretty(&json!({
            "server": server_name,
            "resources": result.resources,
        }))
        .map_err(|error| ToolError::new(error.to_string()))
    }

    fn list_resources_for_all_servers(&mut self) -> Result<String, ToolError> {
        let mut resources = Vec::new();
        let mut failures = Vec::new();

        for server_name in self.server_names() {
            match self
                .runtime
                .block_on(self.manager.list_resources(&server_name))
            {
                Ok(result) => resources.push(json!({
                    "server": server_name,
                    "resources": result.resources,
                })),
                Err(error) => failures.push(json!({
                    "server": server_name,
                    "error": error.to_string(),
                })),
            }
        }

        if resources.is_empty() && !failures.is_empty() {
            let message = failures
                .iter()
                .filter_map(|failure| failure.get("error").and_then(serde_json::Value::as_str))
                .collect::<Vec<_>>()
                .join("; ");
            return Err(ToolError::new(message));
        }

        serde_json::to_string_pretty(&json!({
            "resources": resources,
            "failures": failures,
        }))
        .map_err(|error| ToolError::new(error.to_string()))
    }

    fn read_resource(&mut self, server_name: &str, uri: &str) -> Result<String, ToolError> {
        let result = self
            .runtime
            .block_on(self.manager.read_resource(server_name, uri))
            .map_err(|error| ToolError::new(error.to_string()))?;
        serde_json::to_string_pretty(&json!({
            "server": server_name,
            "contents": result.contents,
        }))
        .map_err(|error| ToolError::new(error.to_string()))
    }
}

fn map_runtime_mcp_servers(
    servers: &BTreeMap<String, orbit_runtime::ScopedMcpServerConfig>,
) -> BTreeMap<String, integrations_mcp_config::ScopedMcpServerConfig> {
    servers
        .iter()
        .map(|(name, scoped)| {
            (
                name.clone(),
                integrations_mcp_config::ScopedMcpServerConfig {
                    scope: match scoped.scope {
                        orbit_runtime::ConfigSource::User
                        | orbit_runtime::ConfigSource::Project => {
                            integrations_mcp_config::ConfigSource::Remote
                        }
                        orbit_runtime::ConfigSource::Local => {
                            integrations_mcp_config::ConfigSource::Local
                        }
                    },
                    config: map_runtime_mcp_server_config(&scoped.config),
                },
            )
        })
        .collect()
}

fn map_runtime_mcp_server_config(
    config: &orbit_runtime::McpServerConfig,
) -> integrations_mcp_config::McpServerConfig {
    match config {
        orbit_runtime::McpServerConfig::Stdio(stdio) => {
            integrations_mcp_config::McpServerConfig::Stdio(
                integrations_mcp_config::McpStdioServerConfig {
                    command: stdio.command.clone(),
                    args: stdio.args.clone(),
                    env: stdio.env.clone(),
                    tool_call_timeout_ms: stdio.tool_call_timeout_ms,
                },
            )
        }
        orbit_runtime::McpServerConfig::Sse(remote) => {
            integrations_mcp_config::McpServerConfig::Sse(
                integrations_mcp_config::McpRemoteServerConfig {
                    url: remote.url.clone(),
                    headers: remote.headers.clone(),
                    headers_helper: remote.headers_helper.clone(),
                    oauth: remote.oauth.as_ref().map(|oauth| {
                        integrations_mcp_config::McpOAuthConfig {
                            client_id: oauth.client_id.clone(),
                            callback_port: oauth.callback_port,
                            auth_server_metadata_url: oauth.auth_server_metadata_url.clone(),
                            xaa: oauth.xaa,
                        }
                    }),
                },
            )
        }
        orbit_runtime::McpServerConfig::Http(remote) => {
            integrations_mcp_config::McpServerConfig::Http(
                integrations_mcp_config::McpRemoteServerConfig {
                    url: remote.url.clone(),
                    headers: remote.headers.clone(),
                    headers_helper: remote.headers_helper.clone(),
                    oauth: remote.oauth.as_ref().map(|oauth| {
                        integrations_mcp_config::McpOAuthConfig {
                            client_id: oauth.client_id.clone(),
                            callback_port: oauth.callback_port,
                            auth_server_metadata_url: oauth.auth_server_metadata_url.clone(),
                            xaa: oauth.xaa,
                        }
                    }),
                },
            )
        }
        orbit_runtime::McpServerConfig::Ws(ws) => integrations_mcp_config::McpServerConfig::Ws(
            integrations_mcp_config::McpWebSocketServerConfig {
                url: ws.url.clone(),
                headers: ws.headers.clone(),
                headers_helper: ws.headers_helper.clone(),
            },
        ),
        orbit_runtime::McpServerConfig::Sdk(sdk) => integrations_mcp_config::McpServerConfig::Sdk(
            integrations_mcp_config::McpSdkServerConfig {
                name: sdk.name.clone(),
            },
        ),
        orbit_runtime::McpServerConfig::ManagedProxy(proxy) => {
            integrations_mcp_config::McpServerConfig::ManagedProxy(
                integrations_mcp_config::McpManagedProxyServerConfig {
                    url: proxy.url.clone(),
                    id: proxy.id.clone(),
                },
            )
        }
    }
}

fn build_runtime_mcp_state(
    runtime_config: &orbit_runtime::RuntimeConfig,
) -> Result<RuntimePluginStateBuildOutput, Box<dyn std::error::Error>> {
    let Some((mcp_state, discovery)) = RuntimeMcpState::new(runtime_config)? else {
        return Ok((None, Vec::new()));
    };

    let mut runtime_tools = discovery
        .tools
        .iter()
        .map(mcp_runtime_tool_definition)
        .collect::<Vec<_>>();
    if !mcp_state.server_names().is_empty() {
        runtime_tools.extend(mcp_wrapper_tool_definitions());
    }

    Ok((Some(Arc::new(Mutex::new(mcp_state))), runtime_tools))
}

fn mcp_runtime_tool_definition(tool: &orbit_runtime::ManagedMcpTool) -> RuntimeToolDefinition {
    RuntimeToolDefinition {
        name: tool.qualified_name.clone(),
        description: Some(
            tool.tool
                .description
                .clone()
                .unwrap_or_else(|| format!("Invoke MCP tool `{}`.", tool.qualified_name)),
        ),
        input_schema: tool
            .tool
            .input_schema
            .clone()
            .unwrap_or_else(|| json!({ "type": "object", "additionalProperties": true })),
        required_permission: permission_mode_for_mcp_tool(&tool.tool),
    }
}

fn mcp_wrapper_tool_definitions() -> Vec<RuntimeToolDefinition> {
    vec![
        RuntimeToolDefinition {
            name: "MCPTool".to_string(),
            description: Some(
                "Call a configured MCP tool by its qualified name and JSON arguments.".to_string(),
            ),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "qualifiedName": { "type": "string" },
                    "arguments": {}
                },
                "required": ["qualifiedName"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
        },
        RuntimeToolDefinition {
            name: "ListMcpResourcesTool".to_string(),
            description: Some(
                "List MCP resources from one configured server or from every connected server."
                    .to_string(),
            ),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "server": { "type": "string" }
                },
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
        },
        RuntimeToolDefinition {
            name: "ReadMcpResourceTool".to_string(),
            description: Some("Read a specific MCP resource from a configured server.".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "server": { "type": "string" },
                    "uri": { "type": "string" }
                },
                "required": ["server", "uri"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
        },
    ]
}

fn permission_mode_for_mcp_tool(tool: &McpTool) -> PermissionMode {
    let read_only = mcp_annotation_flag(tool, "readOnlyHint");
    let destructive = mcp_annotation_flag(tool, "destructiveHint");
    let open_world = mcp_annotation_flag(tool, "openWorldHint");

    if read_only && !destructive && !open_world {
        PermissionMode::ReadOnly
    } else if destructive || open_world {
        PermissionMode::DangerFullAccess
    } else {
        PermissionMode::WorkspaceWrite
    }
}

fn mcp_annotation_flag(tool: &McpTool, key: &str) -> bool {
    tool.annotations
        .as_ref()
        .and_then(|annotations| annotations.get(key))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

struct HookAbortMonitor {
    stop_tx: Option<Sender<()>>,
    join_handle: Option<JoinHandle<()>>,
}

impl HookAbortMonitor {
    fn spawn(abort_signal: orbit_runtime::HookAbortSignal) -> Self {
        Self::spawn_with_waiter(abort_signal, move |stop_rx, abort_signal| {
            let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            else {
                return;
            };

            runtime.block_on(async move {
                let wait_for_stop = tokio::task::spawn_blocking(move || {
                    let _ = stop_rx.recv();
                });

                tokio::select! {
                    result = tokio::signal::ctrl_c() => {
                        if result.is_ok() {
                            abort_signal.abort();
                        }
                    }
                    _ = wait_for_stop => {}
                }
            });
        })
    }

    fn spawn_with_waiter<F>(
        abort_signal: orbit_runtime::HookAbortSignal,
        wait_for_interrupt: F,
    ) -> Self
    where
        F: FnOnce(Receiver<()>, orbit_runtime::HookAbortSignal) + Send + 'static,
    {
        let (stop_tx, stop_rx) = mpsc::channel();
        let join_handle = thread::spawn(move || wait_for_interrupt(stop_rx, abort_signal));

        Self {
            stop_tx: Some(stop_tx),
            join_handle: Some(join_handle),
        }
    }

    fn stop(mut self) {
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(());
        }
        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}

impl LiveCli {
    fn new(
        model: String,
        enable_tools: bool,
        allowed_tools: Option<AllowedToolSet>,
        permission_mode: PermissionMode,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let system_prompt = build_system_prompt()?;
        let session_state = Session::new();
        let session = create_managed_session_handle(&session_state.session_id)?;
        let runtime = build_runtime(
            session_state.with_persistence_path(session.path.clone()),
            &session.id,
            model.clone(),
            system_prompt.clone(),
            enable_tools,
            true,
            allowed_tools.clone(),
            permission_mode,
            None,
        )?;
        let cli = Self {
            model,
            provider: None,
            allowed_tools,
            permission_mode,
            system_prompt,
            runtime,
            session,
        };
        cli.persist_session()?;
        Ok(cli)
    }

    fn new_with_provider(
        model: String,
        provider: Option<String>,
        enable_tools: bool,
        allowed_tools: Option<AllowedToolSet>,
        permission_mode: PermissionMode,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let system_prompt = build_system_prompt()?;
        let mut effective_model = model;
        if provider
            .as_deref()
            .is_some_and(|name| name.eq_ignore_ascii_case("ollama"))
            && effective_model == DEFAULT_MODEL
        {
            effective_model = env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama2".to_string());
        }
        let session_state = Session::new();
        let session = create_managed_session_handle(&session_state.session_id)?;
        let runtime = build_runtime_with_provider(
            session_state.with_persistence_path(session.path.clone()),
            &session.id,
            effective_model.clone(),
            system_prompt.clone(),
            enable_tools,
            true,
            allowed_tools.clone(),
            permission_mode,
            provider.clone(),
            None,
        )?;
        let cli = Self {
            model: effective_model,
            provider,
            allowed_tools,
            permission_mode,
            system_prompt,
            runtime,
            session,
        };
        cli.persist_session()?;
        Ok(cli)
    }

    fn startup_banner(&self) -> String {
        let cwd = env::current_dir().map_or_else(
            |_| "<unknown>".to_string(),
            |path| path.display().to_string(),
        );
        let status = status_context(None).ok();
        let git_branch = status
            .as_ref()
            .and_then(|context| context.git_branch.as_deref())
            .unwrap_or("unknown");
        let workspace = status.as_ref().map_or_else(
            || "unknown".to_string(),
            |context| context.git_summary.headline(),
        );
        let session_path = self.session.path.strip_prefix(Path::new(&cwd)).map_or_else(
            |_| self.session.path.display().to_string(),
            |path| path.display().to_string(),
        );
        format!(
            "\x1b[38;5;208mCode\x1b[0m \n\n\
  \x1b[2mModel\x1b[0m            {}\n\
  \x1b[2mPermissions\x1b[0m      {}\n\
  \x1b[2mBranch\x1b[0m           {}\n\
  \x1b[2mWorkspace\x1b[0m        {}\n\
  \x1b[2mDirectory\x1b[0m        {}\n\
  \x1b[2mSession\x1b[0m          {}\n\
  \x1b[2mAuto-save\x1b[0m        {}\n\n\
  Type \x1b[1m/help\x1b[0m for commands · \x1b[1m/status\x1b[0m for live context · \x1b[2m/resume latest\x1b[0m jumps back to the newest session · \x1b[1m/diff\x1b[0m then \x1b[1m/commit\x1b[0m to ship · \x1b[2mTab\x1b[0m for workflow completions · \x1b[2mShift+Enter\x1b[0m for newline",
            self.model,
            self.permission_mode.as_str(),
            git_branch,
            workspace,
            cwd,
            self.session.id,
            session_path,
        )
    }

    fn repl_completion_candidates(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        Ok(slash_command_completion_candidates_with_sessions(
            &self.model,
            Some(&self.session.id),
            list_managed_sessions()?
                .into_iter()
                .map(|session| session.id)
                .collect(),
        ))
    }

    fn prepare_turn_runtime(
        &self,
        emit_output: bool,
    ) -> Result<(BuiltRuntime, HookAbortMonitor), Box<dyn std::error::Error>> {
        let hook_abort_signal = orbit_runtime::HookAbortSignal::new();
        let runtime = build_runtime_with_provider(
            self.runtime.session().clone(),
            &self.session.id,
            self.model.clone(),
            self.system_prompt.clone(),
            true,
            emit_output,
            self.allowed_tools.clone(),
            self.permission_mode,
            self.provider.clone(),
            None,
        )?
        .with_hook_abort_signal(hook_abort_signal.clone());
        let hook_abort_monitor = HookAbortMonitor::spawn(hook_abort_signal);

        Ok((runtime, hook_abort_monitor))
    }

    fn replace_runtime(&mut self, runtime: BuiltRuntime) -> Result<(), Box<dyn std::error::Error>> {
        self.runtime.shutdown_plugins()?;
        self.runtime = runtime;
        Ok(())
    }

    fn run_turn(&mut self, input: &str) -> Result<(), Box<dyn std::error::Error>> {
        let (mut runtime, hook_abort_monitor) = self.prepare_turn_runtime(true)?;
        let mut spinner = Spinner::new();
        let mut stdout = io::stdout();
        spinner.tick(
            "Thinking...",
            TerminalRenderer::new().color_theme(),
            &mut stdout,
        )?;
        let mut permission_prompter = CliPermissionPrompter::new(self.permission_mode);
        let result = runtime.run_turn(input, Some(&mut permission_prompter));
        hook_abort_monitor.stop();
        match result {
            Ok(summary) => {
                self.replace_runtime(runtime)?;
                spinner.finish("Done", TerminalRenderer::new().color_theme(), &mut stdout)?;
                println!();
                if let Some(event) = summary.auto_compaction {
                    println!(
                        "{}",
                        format_auto_compaction_notice(event.removed_message_count)
                    );
                }
                self.persist_session()?;
                Ok(())
            }
            Err(error) => {
                runtime.shutdown_plugins()?;
                spinner.fail(
                    "❌ Request failed",
                    TerminalRenderer::new().color_theme(),
                    &mut stdout,
                )?;
                Err(Box::new(error))
            }
        }
    }

    fn run_turn_with_output(
        &mut self,
        input: &str,
        output_format: CliOutputFormat,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match output_format {
            CliOutputFormat::Text => self.run_turn(input),
            CliOutputFormat::Json => self.run_prompt_json(input),
        }
    }

    fn run_prompt_json(&mut self, input: &str) -> Result<(), Box<dyn std::error::Error>> {
        let summary = self.run_prompt_json_value(input)?;
        println!("{}", serde_json::to_string_pretty(&summary)?);
        Ok(())
    }

    fn run_prompt_json_value(&mut self, input: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let (mut runtime, hook_abort_monitor) = self.prepare_turn_runtime(false)?;
        let mut permission_prompter = CliPermissionPrompter::new(self.permission_mode);
        let result = runtime.run_turn(input, Some(&mut permission_prompter));
        hook_abort_monitor.stop();
        let summary = result?;
        self.replace_runtime(runtime)?;
        self.persist_session()?;
        Ok(json!({
            "message": final_assistant_text(&summary),
            "model": self.model,
            "iterations": summary.iterations,
            "auto_compaction": summary.auto_compaction.map(|event| json!({
                "removed_messages": event.removed_message_count,
                "notice": format_auto_compaction_notice(event.removed_message_count),
            })),
            "tool_uses": collect_tool_uses(&summary),
            "tool_results": collect_tool_results(&summary),
            "prompt_cache_events": collect_prompt_cache_events(&summary),
            "usage": {
                "input_tokens": summary.usage.input_tokens,
                "output_tokens": summary.usage.output_tokens,
                "cache_creation_input_tokens": summary.usage.cache_creation_input_tokens,
                "cache_read_input_tokens": summary.usage.cache_read_input_tokens,
            },
            "estimated_cost": format_usd(
                summary.usage.estimate_cost_usd_with_pricing(
                    pricing_for_model(&self.model)
                        .unwrap_or_else(orbit_runtime::ModelPricing::default_sonnet_tier)
                ).total_cost_usd()
            )
        }))
    }

    #[allow(clippy::too_many_lines)]
    fn handle_repl_command(
        &mut self,
        command: SlashCommand,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(match command {
            SlashCommand::Help => {
                println!("{}", render_repl_help());
                false
            }
            SlashCommand::Status => {
                self.print_status();
                false
            }
            SlashCommand::Bughunter { scope } => {
                self.run_bughunter(scope.as_deref())?;
                false
            }
            SlashCommand::Commit => {
                self.run_commit(None)?;
                false
            }
            SlashCommand::Pr { context } => {
                self.run_pr(context.as_deref())?;
                false
            }
            SlashCommand::Issue { context } => {
                self.run_issue(context.as_deref())?;
                false
            }
            SlashCommand::Ultraplan { task } => {
                self.run_ultraplan(task.as_deref())?;
                false
            }
            SlashCommand::Teleport { target } => {
                Self::run_teleport(target.as_deref())?;
                false
            }
            SlashCommand::DebugToolCall => {
                self.run_debug_tool_call(None)?;
                false
            }
            SlashCommand::Sandbox => {
                Self::print_sandbox_status();
                false
            }
            SlashCommand::Compact => {
                self.compact()?;
                false
            }
            SlashCommand::Model { model } => self.set_model(model)?,
            SlashCommand::Permissions { mode } => self.set_permissions(mode)?,
            SlashCommand::Clear { confirm } => self.clear_session(confirm)?,
            SlashCommand::Cost => {
                self.print_cost();
                false
            }
            SlashCommand::Resume { session_path } => self.resume_session(session_path)?,
            SlashCommand::Config { section } => {
                Self::print_config(section.as_deref(), CliOutputFormat::Text)?;
                false
            }
            SlashCommand::Telemetry { action, target } => {
                match action.as_deref().unwrap_or("status") {
                    "status" => println!("{}", render_telemetry_report(target.as_deref())?),
                    "on" | "off" => {
                        let cwd = env::current_dir()?;
                        let settings_path = update_project_telemetry_settings(
                            &cwd,
                            action.as_deref() == Some("on"),
                            target.as_deref(),
                        )?;
                        let loader = ConfigLoader::default_for(&cwd);
                        let runtime_config = loader.load()?;
                        println!(
                            "{}",
                            telemetry_update_report(
                                action.as_deref().unwrap_or("status"),
                                &settings_path,
                                &runtime_config
                            )
                        );
                    }
                    other => println!(
                        "Telemetry\n  Result           unsupported\n  Action           {other}\n  Supported        /telemetry [status|on|off] [project|local]"
                    ),
                }
                false
            }
            SlashCommand::Mcp { action, target } => {
                let args = match (action.as_deref(), target.as_deref()) {
                    (None, None) => None,
                    (Some(action), None) => Some(action.to_string()),
                    (Some(action), Some(target)) => Some(format!("{action} {target}")),
                    (None, Some(target)) => Some(target.to_string()),
                };
                Self::print_mcp(args.as_deref(), CliOutputFormat::Text)?;
                false
            }
            SlashCommand::Memory => {
                Self::print_memory()?;
                false
            }
            SlashCommand::Init => {
                run_init(CliOutputFormat::Text)?;
                false
            }
            SlashCommand::Diff => {
                Self::print_diff()?;
                false
            }
            SlashCommand::Version => {
                Self::print_version(CliOutputFormat::Text);
                false
            }
            SlashCommand::Upgrade => {
                println!("{}", render_upgrade_guidance());
                false
            }
            SlashCommand::Export { path } => {
                self.export_session(path.as_deref())?;
                false
            }
            SlashCommand::Session { action, target } => {
                self.handle_session_command(action.as_deref(), target.as_deref())?
            }
            SlashCommand::Plugins { action, target } => {
                self.handle_plugins_command(action.as_deref(), target.as_deref())?
            }
            SlashCommand::Agents { args } => {
                Self::print_agents(args.as_deref(), CliOutputFormat::Text)?;
                false
            }
            SlashCommand::Skills { args } => {
                match classify_skills_slash_command(args.as_deref()) {
                    SkillSlashDispatch::Invoke(prompt) => self.run_turn(&prompt)?,
                    SkillSlashDispatch::Local => {
                        Self::print_skills(args.as_deref(), CliOutputFormat::Text)?;
                    }
                }
                false
            }
            SlashCommand::Doctor => {
                println!("{}", render_doctor_report()?.render());
                false
            }
            SlashCommand::Ide { target } => {
                self.handle_ide_command(target.as_deref())?;
                false
            }
            SlashCommand::Vim
            | SlashCommand::Stats
            | SlashCommand::Share
            | SlashCommand::Feedback
            | SlashCommand::Files
            | SlashCommand::Fast
            | SlashCommand::Exit
            | SlashCommand::Summary
            | SlashCommand::Desktop
            | SlashCommand::Brief
            | SlashCommand::Advisor
            | SlashCommand::Stickers
            | SlashCommand::Insights
            | SlashCommand::Thinkback
            | SlashCommand::ReleaseNotes
            | SlashCommand::SecurityReview
            | SlashCommand::Keybindings
            | SlashCommand::PrivacySettings
            | SlashCommand::Plan { .. }
            | SlashCommand::Review { .. }
            | SlashCommand::Tasks { .. }
            | SlashCommand::Theme { .. }
            | SlashCommand::Voice { .. }
            | SlashCommand::Usage { .. }
            | SlashCommand::Rename { .. }
            | SlashCommand::Copy { .. }
            | SlashCommand::Hooks { .. }
            | SlashCommand::Context { .. }
            | SlashCommand::Color { .. }
            | SlashCommand::Effort { .. }
            | SlashCommand::Branch { .. }
            | SlashCommand::Rewind { .. }
            | SlashCommand::Tag { .. }
            | SlashCommand::OutputStyle { .. }
            | SlashCommand::AddDir { .. } => {
                eprintln!("Command is currently unavailable in this build.");
                false
            }
            SlashCommand::Unknown(name) => {
                eprintln!("{}", format_unknown_slash_command(&name));
                false
            }
        })
    }

    fn persist_session(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.runtime.session().save_to_path(&self.session.path)?;
        Ok(())
    }

    fn print_status(&self) {
        let cumulative = self.runtime.usage().cumulative_usage();
        let latest = self.runtime.usage().current_turn_usage();
        println!(
            "{}",
            format_status_report(
                &self.model,
                StatusUsage {
                    message_count: self.runtime.session().messages.len(),
                    turns: self.runtime.usage().turns(),
                    latest,
                    cumulative,
                    estimated_tokens: self.runtime.estimated_tokens(),
                },
                self.permission_mode.as_str(),
                &status_context(Some(&self.session.path)).expect("status context should load"),
            )
        );
    }

    fn print_sandbox_status() {
        let cwd = env::current_dir().expect("current dir");
        let loader = ConfigLoader::default_for(&cwd);
        let runtime_config = loader
            .load()
            .unwrap_or_else(|_| orbit_runtime::RuntimeConfig::empty());
        println!(
            "{}",
            format_sandbox_report(&resolve_sandbox_status(runtime_config.sandbox(), &cwd))
        );
    }

    fn handle_ide_command(&self, target: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let cwd = env::current_dir()?;

        match target {
            Some(value) => {
                let parsed_target = parse_ide_target(value)?;
                let config_path = set_default_ide_target(&cwd, parsed_target)?;
                let editor_config_path = setup_ide_editor_integration(&cwd, parsed_target)?;
                let install_result = install_ide_extension(parsed_target, &cwd);
                let launch_result = launch_ide_target(parsed_target, &cwd);
                println!(
                    "{}",
                    format_ide_command_report(
                        parsed_target,
                        &config_path,
                        &editor_config_path,
                        install_result,
                        launch_result,
                    )
                );
            }
            None => {
                let status = collect_ide_status(&cwd);
                println!("{}", format_ide_status_report(&status));
            }
        }

        Ok(())
    }

    fn set_model(&mut self, model: Option<String>) -> Result<bool, Box<dyn std::error::Error>> {
        let Some(model) = model else {
            println!(
                "{}",
                format_model_report(
                    &self.model,
                    self.runtime.session().messages.len(),
                    self.runtime.usage().turns(),
                )
            );
            return Ok(false);
        };

        let model = resolve_model_alias(&model).to_string();

        if model == self.model {
            println!(
                "{}",
                format_model_report(
                    &self.model,
                    self.runtime.session().messages.len(),
                    self.runtime.usage().turns(),
                )
            );
            return Ok(false);
        }

        let previous = self.model.clone();
        let session = self.runtime.session().clone();
        let message_count = session.messages.len();
        let runtime = build_runtime_with_provider(
            session,
            &self.session.id,
            model.clone(),
            self.system_prompt.clone(),
            true,
            true,
            self.allowed_tools.clone(),
            self.permission_mode,
            self.provider.clone(),
            None,
        )?;
        self.replace_runtime(runtime)?;
        self.model.clone_from(&model);
        println!(
            "{}",
            format_model_switch_report(&previous, &model, message_count)
        );
        Ok(true)
    }

    fn set_permissions(
        &mut self,
        mode: Option<String>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let Some(mode) = mode else {
            println!(
                "{}",
                format_permissions_report(self.permission_mode.as_str())
            );
            return Ok(false);
        };

        let normalized = normalize_permission_mode(&mode).ok_or_else(|| {
            format!(
                "unsupported permission mode '{mode}'. Use read-only, workspace-write, or danger-full-access."
            )
        })?;

        if normalized == self.permission_mode.as_str() {
            println!("{}", format_permissions_report(normalized));
            return Ok(false);
        }

        let previous = self.permission_mode.as_str().to_string();
        let session = self.runtime.session().clone();
        self.permission_mode = permission_mode_from_label(normalized);
        let runtime = build_runtime_with_provider(
            session,
            &self.session.id,
            self.model.clone(),
            self.system_prompt.clone(),
            true,
            true,
            self.allowed_tools.clone(),
            self.permission_mode,
            self.provider.clone(),
            None,
        )?;
        self.replace_runtime(runtime)?;
        println!(
            "{}",
            format_permissions_switch_report(&previous, normalized)
        );
        Ok(true)
    }

    fn clear_session(&mut self, confirm: bool) -> Result<bool, Box<dyn std::error::Error>> {
        if !confirm {
            println!(
                "clear: confirmation required; run /clear --confirm to start a fresh session."
            );
            return Ok(false);
        }

        let previous_session = self.session.clone();
        let session_state = Session::new();
        self.session = create_managed_session_handle(&session_state.session_id)?;
        let runtime = build_runtime_with_provider(
            session_state.with_persistence_path(self.session.path.clone()),
            &self.session.id,
            self.model.clone(),
            self.system_prompt.clone(),
            true,
            true,
            self.allowed_tools.clone(),
            self.permission_mode,
            self.provider.clone(),
            None,
        )?;
        self.replace_runtime(runtime)?;
        println!(
            "Session cleared\n  Mode             fresh session\n  Previous session {}\n  Resume previous  /resume {}\n  Preserved model  {}\n  Permission mode  {}\n  New session      {}\n  Session file     {}",
            previous_session.id,
            previous_session.id,
            self.model,
            self.permission_mode.as_str(),
            self.session.id,
            self.session.path.display(),
        );
        Ok(true)
    }

    fn print_cost(&self) {
        let cumulative = self.runtime.usage().cumulative_usage();
        println!("{}", format_cost_report(cumulative));
    }

    fn resume_session(
        &mut self,
        session_path: Option<String>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let Some(session_ref) = session_path else {
            println!("{}", render_resume_usage());
            return Ok(false);
        };

        let handle = resolve_session_reference(&session_ref)?;
        let session = Session::load_from_path(&handle.path)?;
        let message_count = session.messages.len();
        let session_id = session.session_id.clone();
        let runtime = build_runtime_with_provider(
            session,
            &handle.id,
            self.model.clone(),
            self.system_prompt.clone(),
            true,
            true,
            self.allowed_tools.clone(),
            self.permission_mode,
            self.provider.clone(),
            None,
        )?;
        self.replace_runtime(runtime)?;
        self.session = SessionHandle {
            id: session_id,
            path: handle.path,
        };
        println!(
            "{}",
            format_resume_report(
                &self.session.path.display().to_string(),
                message_count,
                self.runtime.usage().turns(),
            )
        );
        Ok(true)
    }

    fn print_config(
        section: Option<&str>,
        output_format: CliOutputFormat,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match output_format {
            CliOutputFormat::Text => println!("{}", render_config_report(section)?),
            CliOutputFormat::Json => println!(
                "{}",
                serde_json::to_string_pretty(&config_json_value(section)?)?
            ),
        }
        Ok(())
    }

    fn print_memory() -> Result<(), Box<dyn std::error::Error>> {
        println!("{}", render_memory_report()?);
        Ok(())
    }

    fn print_agents(
        args: Option<&str>,
        output_format: CliOutputFormat,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let cwd = env::current_dir()?;
        match output_format {
            CliOutputFormat::Text => println!("{}", handle_agents_slash_command(args, &cwd)?),
            CliOutputFormat::Json => println!(
                "{}",
                serde_json::to_string_pretty(&handle_agents_slash_command_json(args, &cwd)?)?
            ),
        }
        Ok(())
    }

    fn print_mcp(
        args: Option<&str>,
        output_format: CliOutputFormat,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let cwd = env::current_dir()?;
        match output_format {
            CliOutputFormat::Text => println!("{}", handle_mcp_slash_command(args, &cwd)?),
            CliOutputFormat::Json => println!(
                "{}",
                serde_json::to_string_pretty(&handle_mcp_slash_command_json(args, &cwd)?)?
            ),
        }
        Ok(())
    }

    fn print_skills(
        args: Option<&str>,
        output_format: CliOutputFormat,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let cwd = env::current_dir()?;
        match output_format {
            CliOutputFormat::Text => println!("{}", handle_skills_slash_command(args, &cwd)?),
            CliOutputFormat::Json => println!(
                "{}",
                serde_json::to_string_pretty(&handle_skills_slash_command_json(args, &cwd)?)?
            ),
        }
        Ok(())
    }

    fn print_plugins(
        action: Option<&str>,
        target: Option<&str>,
        output_format: CliOutputFormat,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let cwd = env::current_dir()?;
        let loader = ConfigLoader::default_for(&cwd);
        let runtime_config = loader.load()?;
        let mut manager = build_plugin_manager(&cwd, &loader, &runtime_config);
        let result = handle_plugins_slash_command(action, target, &mut manager)?;
        match output_format {
            CliOutputFormat::Text => println!("{}", result.message),
            CliOutputFormat::Json => println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "kind": "plugin",
                    "action": action.unwrap_or("list"),
                    "target": target,
                    "message": result.message,
                    "reload_runtime": result.reload_runtime,
                }))?
            ),
        }
        Ok(())
    }

    fn print_diff() -> Result<(), Box<dyn std::error::Error>> {
        println!("{}", render_diff_report()?);
        Ok(())
    }

    fn print_version(output_format: CliOutputFormat) {
        let _ = crate::print_version(output_format);
    }

    fn export_session(
        &self,
        requested_path: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let export_path = resolve_export_path(requested_path, self.runtime.session())?;
        fs::write(&export_path, render_export_text(self.runtime.session()))?;
        println!(
            "Export\n  Result           wrote transcript\n  File             {}\n  Messages         {}",
            export_path.display(),
            self.runtime.session().messages.len(),
        );
        Ok(())
    }

    fn handle_session_command(
        &mut self,
        action: Option<&str>,
        target: Option<&str>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match action {
            None | Some("list") => {
                println!("{}", render_session_list(&self.session.id)?);
                Ok(false)
            }
            Some("switch") => {
                let Some(target) = target else {
                    println!("Usage: /session switch <session-id>");
                    return Ok(false);
                };
                let handle = resolve_session_reference(target)?;
                let session = Session::load_from_path(&handle.path)?;
                let message_count = session.messages.len();
                let session_id = session.session_id.clone();
                let runtime = build_runtime_with_provider(
                    session,
                    &handle.id,
                    self.model.clone(),
                    self.system_prompt.clone(),
                    true,
                    true,
                    self.allowed_tools.clone(),
                    self.permission_mode,
                    self.provider.clone(),
                    None,
                )?;
                self.replace_runtime(runtime)?;
                self.session = SessionHandle {
                    id: session_id,
                    path: handle.path,
                };
                println!(
                    "Session switched\n  Active session   {}\n  File             {}\n  Messages         {}",
                    self.session.id,
                    self.session.path.display(),
                    message_count,
                );
                Ok(true)
            }
            Some("fork") => {
                let forked = self.runtime.fork_session(target.map(ToOwned::to_owned));
                let parent_session_id = self.session.id.clone();
                let handle = create_managed_session_handle(&forked.session_id)?;
                let branch_name = forked
                    .fork
                    .as_ref()
                    .and_then(|fork| fork.branch_name.clone());
                let forked = forked.with_persistence_path(handle.path.clone());
                let message_count = forked.messages.len();
                forked.save_to_path(&handle.path)?;
                let runtime = build_runtime_with_provider(
                    forked,
                    &handle.id,
                    self.model.clone(),
                    self.system_prompt.clone(),
                    true,
                    true,
                    self.allowed_tools.clone(),
                    self.permission_mode,
                    self.provider.clone(),
                    None,
                )?;
                self.replace_runtime(runtime)?;
                self.session = handle;
                println!(
                    "Session forked\n  Parent session   {}\n  Active session   {}\n  Branch           {}\n  File             {}\n  Messages         {}",
                    parent_session_id,
                    self.session.id,
                    branch_name.as_deref().unwrap_or("(unnamed)"),
                    self.session.path.display(),
                    message_count,
                );
                Ok(true)
            }
            Some(other) => {
                println!(
                    "Unknown /session action '{other}'. Use /session list, /session switch <session-id>, or /session fork [branch-name]."
                );
                Ok(false)
            }
        }
    }

    fn handle_plugins_command(
        &mut self,
        action: Option<&str>,
        target: Option<&str>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let cwd = env::current_dir()?;
        let loader = ConfigLoader::default_for(&cwd);
        let runtime_config = loader.load()?;
        let mut manager = build_plugin_manager(&cwd, &loader, &runtime_config);
        let result = handle_plugins_slash_command(action, target, &mut manager)?;
        println!("{}", result.message);
        if result.reload_runtime {
            self.reload_runtime_features()?;
        }
        Ok(false)
    }

    fn reload_runtime_features(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let runtime = build_runtime_with_provider(
            self.runtime.session().clone(),
            &self.session.id,
            self.model.clone(),
            self.system_prompt.clone(),
            true,
            true,
            self.allowed_tools.clone(),
            self.permission_mode,
            self.provider.clone(),
            None,
        )?;
        self.replace_runtime(runtime)?;
        self.persist_session()
    }

    fn compact(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let result = self.runtime.compact(CompactionConfig::default());
        let removed = result.removed_message_count;
        let kept = result.compacted_session.messages.len();
        let skipped = removed == 0;
        let runtime = build_runtime_with_provider(
            result.compacted_session,
            &self.session.id,
            self.model.clone(),
            self.system_prompt.clone(),
            true,
            true,
            self.allowed_tools.clone(),
            self.permission_mode,
            self.provider.clone(),
            None,
        )?;
        self.replace_runtime(runtime)?;
        self.persist_session()?;
        println!("{}", format_compact_report(removed, kept, skipped));
        Ok(())
    }

    fn run_internal_prompt_text_with_progress(
        &self,
        prompt: &str,
        enable_tools: bool,
        progress: Option<InternalPromptProgressReporter>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let session = self.runtime.session().clone();
        let mut runtime = build_runtime_with_provider(
            session,
            &self.session.id,
            self.model.clone(),
            self.system_prompt.clone(),
            enable_tools,
            false,
            self.allowed_tools.clone(),
            self.permission_mode,
            self.provider.clone(),
            progress,
        )?;
        let mut permission_prompter = CliPermissionPrompter::new(self.permission_mode);
        let summary = runtime.run_turn(prompt, Some(&mut permission_prompter))?;
        let text = final_assistant_text(&summary).trim().to_string();
        runtime.shutdown_plugins()?;
        Ok(text)
    }

    fn run_internal_prompt_text(
        &self,
        prompt: &str,
        enable_tools: bool,
    ) -> Result<String, Box<dyn std::error::Error>> {
        self.run_internal_prompt_text_with_progress(prompt, enable_tools, None)
    }

    fn run_bughunter(&self, scope: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        println!("{}", format_bughunter_report(scope));
        Ok(())
    }

    fn run_ultraplan(&self, task: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        println!("{}", format_ultraplan_report(task));
        Ok(())
    }

    fn run_teleport(target: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let Some(target) = target.map(str::trim).filter(|value| !value.is_empty()) else {
            println!("Usage: /teleport <symbol-or-path>");
            return Ok(());
        };

        println!("{}", render_teleport_report(target)?);
        Ok(())
    }

    fn run_debug_tool_call(&self, args: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        validate_no_args("/debug-tool-call", args)?;
        println!("{}", render_last_tool_debug_report(self.runtime.session())?);
        Ok(())
    }

    fn run_commit(&mut self, args: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        validate_no_args("/commit", args)?;
        let status = git_output(&["status", "--short", "--branch"])?;
        let summary = parse_git_workspace_summary(Some(&status));
        let branch = parse_git_status_branch(Some(&status));
        if summary.is_clean() {
            println!("{}", format_commit_skipped_report());
            return Ok(());
        }

        println!(
            "{}",
            format_commit_preflight_report(branch.as_deref(), summary)
        );
        Ok(())
    }

    fn run_pr(&self, context: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let branch =
            resolve_git_branch_for(&env::current_dir()?).unwrap_or_else(|| "unknown".to_string());
        println!("{}", format_pr_report(&branch, context));
        Ok(())
    }

    fn run_issue(&self, context: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        println!("{}", format_issue_report(context));
        Ok(())
    }
}

fn sessions_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let path = cwd.join(".orbit").join("sessions");
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn create_managed_session_handle(
    session_id: &str,
) -> Result<SessionHandle, Box<dyn std::error::Error>> {
    let id = session_id.to_string();
    let path = sessions_dir()?.join(format!("{id}.{PRIMARY_SESSION_EXTENSION}"));
    Ok(SessionHandle { id, path })
}

fn resolve_session_reference(reference: &str) -> Result<SessionHandle, Box<dyn std::error::Error>> {
    if SESSION_REFERENCE_ALIASES
        .iter()
        .any(|alias| reference.eq_ignore_ascii_case(alias))
    {
        let latest = latest_managed_session()?;
        return Ok(SessionHandle {
            id: latest.id,
            path: latest.path,
        });
    }

    let direct = PathBuf::from(reference);
    let looks_like_path = direct.extension().is_some() || direct.components().count() > 1;
    let path = if direct.exists() {
        direct
    } else if looks_like_path {
        return Err(format_missing_session_reference(reference).into());
    } else {
        resolve_managed_session_path(reference)?
    };
    let id = path
        .file_name()
        .and_then(|value| value.to_str())
        .and_then(|name| {
            name.strip_suffix(&format!(".{PRIMARY_SESSION_EXTENSION}"))
                .or_else(|| name.strip_suffix(&format!(".{LEGACY_SESSION_EXTENSION}")))
        })
        .unwrap_or(reference)
        .to_string();
    Ok(SessionHandle { id, path })
}

fn resolve_managed_session_path(session_id: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let directory = sessions_dir()?;
    for extension in [PRIMARY_SESSION_EXTENSION, LEGACY_SESSION_EXTENSION] {
        let path = directory.join(format!("{session_id}.{extension}"));
        if path.exists() {
            return Ok(path);
        }
    }
    Err(format_missing_session_reference(session_id).into())
}

fn is_managed_session_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|extension| {
            extension == PRIMARY_SESSION_EXTENSION || extension == LEGACY_SESSION_EXTENSION
        })
}

fn list_managed_sessions() -> Result<Vec<ManagedSessionSummary>, Box<dyn std::error::Error>> {
    let mut sessions = Vec::new();
    for entry in fs::read_dir(sessions_dir()?)? {
        let entry = entry?;
        let path = entry.path();
        if !is_managed_session_file(&path) {
            continue;
        }
        let metadata = entry.metadata()?;
        let modified_epoch_millis = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis())
            .unwrap_or_default();
        let (id, message_count, parent_session_id, branch_name) =
            match Session::load_from_path(&path) {
                Ok(session) => {
                    let parent_session_id = session
                        .fork
                        .as_ref()
                        .map(|fork| fork.parent_session_id.clone());
                    let branch_name = session
                        .fork
                        .as_ref()
                        .and_then(|fork| fork.branch_name.clone());
                    (
                        session.session_id,
                        session.messages.len(),
                        parent_session_id,
                        branch_name,
                    )
                }
                Err(_) => (
                    path.file_stem()
                        .and_then(|value| value.to_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    0,
                    None,
                    None,
                ),
            };
        sessions.push(ManagedSessionSummary {
            id,
            path,
            modified_epoch_millis,
            message_count,
            parent_session_id,
            branch_name,
        });
    }
    sessions.sort_by(|left, right| {
        right
            .modified_epoch_millis
            .cmp(&left.modified_epoch_millis)
            .then_with(|| right.id.cmp(&left.id))
    });
    Ok(sessions)
}

fn latest_managed_session() -> Result<ManagedSessionSummary, Box<dyn std::error::Error>> {
    list_managed_sessions()?
        .into_iter()
        .next()
        .ok_or_else(|| format_no_managed_sessions().into())
}

fn format_missing_session_reference(reference: &str) -> String {
    format!(
        "session not found: {reference}\nHint: managed sessions live in .orbit/sessions/. Try `{LATEST_SESSION_REFERENCE}` for the most recent session or `/session list` in the REPL."
    )
}

fn format_no_managed_sessions() -> String {
    format!(
        "no managed sessions found in .orbit/sessions/\nStart `orbit` to create a session, then rerun with `--resume {LATEST_SESSION_REFERENCE}`."
    )
}

fn render_session_list(active_session_id: &str) -> Result<String, Box<dyn std::error::Error>> {
    let sessions = list_managed_sessions()?;
    let mut lines = vec![
        "Sessions".to_string(),
        format!("  Directory         {}", sessions_dir()?.display()),
    ];
    if sessions.is_empty() {
        lines.push("  No managed sessions saved yet.".to_string());
        return Ok(lines.join("\n"));
    }
    for session in sessions {
        let marker = if session.id == active_session_id {
            "● current"
        } else {
            "○ saved"
        };
        let lineage = match (
            session.branch_name.as_deref(),
            session.parent_session_id.as_deref(),
        ) {
            (Some(branch_name), Some(parent_session_id)) => {
                format!(" branch={branch_name} from={parent_session_id}")
            }
            (None, Some(parent_session_id)) => format!(" from={parent_session_id}"),
            (Some(branch_name), None) => format!(" branch={branch_name}"),
            (None, None) => String::new(),
        };
        lines.push(format!(
            "  {id:<20} {marker:<10} msgs={msgs:<4} modified={modified}{lineage} path={path}",
            id = session.id,
            msgs = session.message_count,
            modified = format_session_modified_age(session.modified_epoch_millis),
            lineage = lineage,
            path = session.path.display(),
        ));
    }
    Ok(lines.join("\n"))
}

fn format_session_modified_age(modified_epoch_millis: u128) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map_or(modified_epoch_millis, |duration| duration.as_millis());
    let delta_seconds = now
        .saturating_sub(modified_epoch_millis)
        .checked_div(1_000)
        .unwrap_or_default();
    match delta_seconds {
        0..=4 => "just-now".to_string(),
        5..=59 => format!("{delta_seconds}s-ago"),
        60..=3_599 => format!("{}m-ago", delta_seconds / 60),
        3_600..=86_399 => format!("{}h-ago", delta_seconds / 3_600),
        _ => format!("{}d-ago", delta_seconds / 86_400),
    }
}

fn write_session_clear_backup(
    session: &Session,
    session_path: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let backup_path = session_clear_backup_path(session_path);
    session.save_to_path(&backup_path)?;
    Ok(backup_path)
}

fn session_clear_backup_path(session_path: &Path) -> PathBuf {
    let timestamp = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map_or(0, |duration| duration.as_millis());
    let file_name = session_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("session.jsonl");
    session_path.with_file_name(format!("{file_name}.before-clear-{timestamp}.bak"))
}

fn render_repl_help() -> String {
    [
        "REPL".to_string(),
        "  /exit                Quit the REPL".to_string(),
        "  /quit                Quit the REPL".to_string(),
        "  Up/Down              Navigate prompt history".to_string(),
        "  Tab                  Complete commands, modes, and recent sessions".to_string(),
        "  Ctrl-C               Clear input (or exit on empty prompt)".to_string(),
        "  Shift+Enter/Ctrl+J   Insert a newline".to_string(),
        "  Auto-save            .orbit/sessions/<session-id>.jsonl".to_string(),
        "  Resume latest        /resume latest".to_string(),
        "  Browse sessions      /session list".to_string(),
        String::new(),
        render_slash_command_help(),
    ]
    .join(
        "
",
    )
}

fn print_status_snapshot(
    model: &str,
    permission_mode: PermissionMode,
    output_format: CliOutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let usage = StatusUsage {
        message_count: 0,
        turns: 0,
        latest: TokenUsage::default(),
        cumulative: TokenUsage::default(),
        estimated_tokens: 0,
    };
    let context = status_context(None)?;
    match output_format {
        CliOutputFormat::Text => println!(
            "{}",
            format_status_report(model, usage, permission_mode.as_str(), &context)
        ),
        CliOutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&status_json_value(
                model,
                usage,
                permission_mode.as_str(),
                &context,
            ))?
        ),
    }
    Ok(())
}

fn status_json_value(
    model: &str,
    usage: StatusUsage,
    permission_mode: &str,
    context: &StatusContext,
) -> serde_json::Value {
    json!({
        "kind": "status",
        "model": model,
        "permission_mode": permission_mode,
        "usage": {
            "messages": usage.message_count,
            "turns": usage.turns,
            "latest_total": usage.latest.total_tokens(),
            "cumulative_input": usage.cumulative.input_tokens,
            "cumulative_output": usage.cumulative.output_tokens,
            "cumulative_total": usage.cumulative.total_tokens(),
            "estimated_tokens": usage.estimated_tokens,
        },
        "workspace": {
            "cwd": context.cwd,
            "project_root": context.project_root,
            "git_branch": context.git_branch,
            "git_state": context.git_summary.headline(),
            "changed_files": context.git_summary.changed_files,
            "staged_files": context.git_summary.staged_files,
            "unstaged_files": context.git_summary.unstaged_files,
            "untracked_files": context.git_summary.untracked_files,
            "session": context.session_path.as_ref().map_or_else(|| "live-repl".to_string(), |path| path.display().to_string()),
            "loaded_config_files": context.loaded_config_files,
            "discovered_config_files": context.discovered_config_files,
            "memory_file_count": context.memory_file_count,
        },
        "sandbox": {
            "enabled": context.sandbox_status.enabled,
            "active": context.sandbox_status.active,
            "supported": context.sandbox_status.supported,
            "in_container": context.sandbox_status.in_container,
            "requested_namespace": context.sandbox_status.requested.namespace_restrictions,
            "active_namespace": context.sandbox_status.namespace_active,
            "requested_network": context.sandbox_status.requested.network_isolation,
            "active_network": context.sandbox_status.network_active,
            "filesystem_mode": context.sandbox_status.filesystem_mode.as_str(),
            "filesystem_active": context.sandbox_status.filesystem_active,
            "allowed_mounts": context.sandbox_status.allowed_mounts,
            "markers": context.sandbox_status.container_markers,
            "fallback_reason": context.sandbox_status.fallback_reason,
        }
    })
}

fn status_context(
    session_path: Option<&Path>,
) -> Result<StatusContext, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let loader = ConfigLoader::default_for(&cwd);
    let discovered_config_files = loader.discover().len();
    let runtime_config = loader.load()?;
    let project_context = ProjectContext::discover_with_git(&cwd, DEFAULT_DATE)?;
    let (project_root, git_branch) =
        parse_git_status_metadata(project_context.git_status.as_deref());
    let git_summary = parse_git_workspace_summary(project_context.git_status.as_deref());
    let sandbox_status = resolve_sandbox_status(runtime_config.sandbox(), &cwd);
    Ok(StatusContext {
        cwd,
        session_path: session_path.map(Path::to_path_buf),
        loaded_config_files: runtime_config.loaded_entries().len(),
        discovered_config_files,
        memory_file_count: project_context.instruction_files.len(),
        project_root,
        git_branch,
        git_summary,
        sandbox_status,
    })
}

fn format_status_report(
    model: &str,
    usage: StatusUsage,
    permission_mode: &str,
    context: &StatusContext,
) -> String {
    [
        format!(
            "Status
  Model            {model}
  Permission mode  {permission_mode}
  Messages         {}
  Turns            {}
  Estimated tokens {}",
            usage.message_count, usage.turns, usage.estimated_tokens,
        ),
        format!(
            "Usage
  Latest total     {}
  Cumulative input {}
  Cumulative output {}
  Cumulative total {}",
            usage.latest.total_tokens(),
            usage.cumulative.input_tokens,
            usage.cumulative.output_tokens,
            usage.cumulative.total_tokens(),
        ),
        format!(
            "Workspace
  Cwd              {}
  Project root     {}
  Git branch       {}
  Git state        {}
  Changed files    {}
  Staged           {}
  Unstaged         {}
  Untracked        {}
  Session          {}
  Config files     loaded {}/{}
  Memory files     {}
  Suggested flow   /status → /diff → /commit",
            context.cwd.display(),
            context
                .project_root
                .as_ref()
                .map_or_else(|| "unknown".to_string(), |path| path.display().to_string()),
            context.git_branch.as_deref().unwrap_or("unknown"),
            context.git_summary.headline(),
            context.git_summary.changed_files,
            context.git_summary.staged_files,
            context.git_summary.unstaged_files,
            context.git_summary.untracked_files,
            context.session_path.as_ref().map_or_else(
                || "live-repl".to_string(),
                |path| path.display().to_string()
            ),
            context.loaded_config_files,
            context.discovered_config_files,
            context.memory_file_count,
        ),
        format_sandbox_report(&context.sandbox_status),
    ]
    .join(
        "

",
    )
}

fn format_sandbox_report(status: &orbit_runtime::SandboxStatus) -> String {
    format!(
        "Sandbox
  Enabled           {}
  Active            {}
  Supported         {}
  In container      {}
  Requested ns      {}
  Active ns         {}
  Requested net     {}
  Active net        {}
  Filesystem mode   {}
  Filesystem active {}
  Allowed mounts    {}
  Markers           {}
  Fallback reason   {}",
        status.enabled,
        status.active,
        status.supported,
        status.in_container,
        status.requested.namespace_restrictions,
        status.namespace_active,
        status.requested.network_isolation,
        status.network_active,
        status.filesystem_mode.as_str(),
        status.filesystem_active,
        if status.allowed_mounts.is_empty() {
            "<none>".to_string()
        } else {
            status.allowed_mounts.join(", ")
        },
        if status.container_markers.is_empty() {
            "<none>".to_string()
        } else {
            status.container_markers.join(", ")
        },
        status
            .fallback_reason
            .clone()
            .unwrap_or_else(|| "<none>".to_string()),
    )
}

fn format_ide_status_report(status: &IdeStatus) -> String {
    let configured = status
        .configured_target
        .map_or_else(|| "<none>".to_string(), |target| target.to_string());
    let config_error = status
        .config_error
        .as_deref()
        .unwrap_or("<none>")
        .to_string();
    let extension_status = status.extension_dev_path.as_ref().map_or_else(
        || "<not found in repo ancestry>".to_string(),
        |path| path.display().to_string(),
    );
    let packaged_extension = status
        .packaged_extension_path
        .as_ref()
        .map_or_else(|| "<none>".to_string(), |path| path.display().to_string());
    let editor_config = status
        .editor_config_path
        .as_ref()
        .map_or_else(|| "<none>".to_string(), |path| path.display().to_string());
    format!(
        "IDE
  Config file      {}
  Configured       {}
  Available        {}
  Extension path   {}
  Extension pkg    {}
  Editor config    {}
  Config error     {}
  Usage            /ide [vscode|cursor|antigravity|windsurf]",
        status.config_path.display(),
        configured,
        format_available_ide_targets(&status.available_targets),
        extension_status,
        packaged_extension,
        editor_config,
        config_error,
    )
}

fn format_ide_command_report(
    target: IdeTarget,
    config_path: &Path,
    editor_config_path: &Path,
    install_result: Result<PathBuf, orbit_integrations::ide::IdeIntegrationError>,
    launch_result: Result<(), orbit_integrations::ide::IdeIntegrationError>,
) -> String {
    let (install_status, install_detail) = match install_result {
        Ok(path) => ("ok", format!("installed {}", path.display())),
        Err(error) => ("failed", error.to_string()),
    };
    let (launch_status, launch_detail) = match launch_result {
        Ok(()) => ("ok", format!("launched {target}")),
        Err(error) => ("failed", error.to_string()),
    };

    format!(
        "IDE
  Result           configured
  Target           {target}
  Config file      {}
  Editor config    {}
  Install status   {install_status}
  Install detail   {install_detail}
  Launch status    {launch_status}
  Launch detail    {launch_detail}",
        config_path.display(),
        editor_config_path.display(),
    )
}

fn format_available_ide_targets(targets: &[IdeTarget]) -> String {
    if targets.is_empty() {
        return "<none>".to_string();
    }
    targets
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_commit_preflight_report(branch: Option<&str>, summary: GitWorkspaceSummary) -> String {
    format!(
        "Commit
  Result           ready
  Branch           {}
  Workspace        {}
  Changed files    {}
  Action           create a git commit from the current workspace changes",
        branch.unwrap_or("unknown"),
        summary.headline(),
        summary.changed_files,
    )
}

fn format_commit_skipped_report() -> String {
    "Commit
  Result           skipped
  Reason           no workspace changes
  Action           create a git commit from the current workspace changes
  Next             /status to inspect context · /diff to inspect repo changes"
        .to_string()
}

fn print_sandbox_status_snapshot(
    output_format: CliOutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let loader = ConfigLoader::default_for(&cwd);
    let runtime_config = loader
        .load()
        .unwrap_or_else(|_| orbit_runtime::RuntimeConfig::empty());
    let status = resolve_sandbox_status(runtime_config.sandbox(), &cwd);
    match output_format {
        CliOutputFormat::Text => println!("{}", format_sandbox_report(&status)),
        CliOutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&sandbox_json_value(&status))?
        ),
    }
    Ok(())
}

fn sandbox_json_value(status: &orbit_runtime::SandboxStatus) -> serde_json::Value {
    json!({
        "kind": "sandbox",
        "enabled": status.enabled,
        "active": status.active,
        "supported": status.supported,
        "in_container": status.in_container,
        "requested_namespace": status.requested.namespace_restrictions,
        "active_namespace": status.namespace_active,
        "requested_network": status.requested.network_isolation,
        "active_network": status.network_active,
        "filesystem_mode": status.filesystem_mode.as_str(),
        "filesystem_active": status.filesystem_active,
        "allowed_mounts": status.allowed_mounts,
        "markers": status.container_markers,
        "fallback_reason": status.fallback_reason,
    })
}

fn telemetry_json_value(
    resolution: &TelemetryResolution,
    runtime_config: &orbit_runtime::RuntimeConfig,
) -> serde_json::Value {
    let config_source_path = telemetry_config_source_path(resolution);
    let config_shadowed_by_env = telemetry_config_shadowed_by_env(resolution);
    json!({
        "kind": "telemetry",
        "enabled": resolution.enabled,
        "path": resolution.path,
        "source": resolution.source,
        "effective_source": resolution.source,
        "config_source_path": config_source_path,
        "config_shadowed_by_env": config_shadowed_by_env,
        "config_enabled": runtime_config.telemetry().enabled(),
        "config_path": runtime_config.telemetry().path(),
        "env_override": telemetry_env_override_value(),
    })
}

fn telemetry_status_json_value(
    runtime_config: &orbit_runtime::RuntimeConfig,
    target: Option<&str>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let resolution = resolve_telemetry_config(Some(runtime_config));
    let mut payload = telemetry_json_value(&resolution, runtime_config);
    if let Some(target) = target {
        let cwd = env::current_dir()?;
        let status = telemetry_target_status(&cwd, target);
        payload["target"] = json!({
            "scope": status.target,
            "settings_path": status.settings_path.display().to_string(),
            "settings_status": status.settings_status,
            "config_enabled": status.enabled,
            "config_path": status.path,
        });
    }
    Ok(payload)
}

fn telemetry_config_source_path(resolution: &TelemetryResolution) -> Option<String> {
    resolution
        .config_path
        .as_ref()
        .map(|path| path.display().to_string())
}

fn telemetry_config_shadowed_by_env(resolution: &TelemetryResolution) -> bool {
    resolution.source == "env" && resolution.config_path.is_some()
}

fn telemetry_config_file_label(resolution: &TelemetryResolution) -> &'static str {
    if telemetry_config_shadowed_by_env(resolution) {
        "Shadowed config"
    } else {
        "Config file"
    }
}

fn telemetry_env_override_value() -> Option<String> {
    env::var(ORBIT_TELEMETRY_PATH)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn telemetry_env_override_display() -> String {
    telemetry_env_override_value().unwrap_or_else(|| "<unset>".to_string())
}

fn telemetry_config_enabled_display(runtime_config: &orbit_runtime::RuntimeConfig) -> String {
    runtime_config
        .telemetry()
        .enabled()
        .map_or("<unset>".to_string(), |value| {
            if value {
                "true".to_string()
            } else {
                "false".to_string()
            }
        })
}

fn telemetry_text_detail_lines(
    resolution: &TelemetryResolution,
    runtime_config: &orbit_runtime::RuntimeConfig,
) -> Vec<String> {
    vec![
        report_row("Enabled", if resolution.enabled { "yes" } else { "no" }),
        report_row(
            "Effective path",
            resolution.path.as_deref().unwrap_or("<unset>"),
        ),
        report_row("Effective source", resolution.source),
        report_row(
            telemetry_config_file_label(resolution),
            telemetry_config_source_path(resolution).unwrap_or_else(|| "<unset>".to_string()),
        ),
        report_row(
            "Config enabled",
            telemetry_config_enabled_display(runtime_config),
        ),
        report_row(
            "Config path",
            runtime_config.telemetry().path().unwrap_or("<unset>"),
        ),
        report_row("Env override", telemetry_env_override_display()),
    ]
}

fn telemetry_target_detail_lines(status: &TelemetryTargetStatus) -> Vec<String> {
    vec![
        report_row("Target scope", &status.target),
        report_row("Settings file", status.settings_path.display()),
        report_row("Settings status", status.settings_status),
        report_row(
            "Target enabled",
            status
                .enabled
                .map(|value| if value { "true" } else { "false" })
                .unwrap_or("<unset>"),
        ),
        report_row("Target path", status.path.as_deref().unwrap_or("<unset>")),
    ]
}

fn load_runtime_config_for_current_dir(
) -> Result<(PathBuf, orbit_runtime::RuntimeConfig), Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let loader = ConfigLoader::default_for(&cwd);
    Ok((cwd, loader.load()?))
}

fn load_runtime_config_for_current_dir_or_empty(
) -> Result<(PathBuf, orbit_runtime::RuntimeConfig), Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let loader = ConfigLoader::default_for(&cwd);
    Ok((
        cwd,
        loader
            .load()
            .unwrap_or_else(|_| orbit_runtime::RuntimeConfig::empty()),
    ))
}

fn config_json_value(
    section: Option<&str>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let (cwd, runtime_config) = load_runtime_config_for_current_dir()?;
    let discovered = ConfigLoader::default_for(&cwd).discover();
    let discovered_files = summarize_discovered_config_files(&discovered, &runtime_config);

    let discovered_files = discovered_files
        .into_iter()
        .map(|entry| {
            json!({
                "source": entry.source,
                "status": entry.status,
                "path": entry.path,
            })
        })
        .collect::<Vec<_>>();

    let mut payload = json!({
        "kind": "config",
        "working_directory": cwd.display().to_string(),
        "loaded_files": runtime_config.loaded_entries().len(),
        "merged_keys": runtime_config.merged().len(),
        "discovered_files": discovered_files,
    });

    if let Some(section) = section {
        payload["section"] = json!(section);
        match resolve_config_section(&runtime_config, section) {
            ConfigSectionResolution::Unsupported => {
                payload["status"] = json!("unsupported");
                payload["section_supported"] = json!(false);
                payload["section_present"] = json!(false);
                payload["section_status"] = json!("unsupported");
                payload["supported_sections"] = json!(SUPPORTED_CONFIG_SECTIONS);
            }
            ConfigSectionResolution::Supported { rendered_value } => {
                let section_present = rendered_value.is_some();
                payload["section_supported"] = json!(true);
                payload["section_present"] = json!(section_present);
                payload["section_status"] = json!(config_section_status(section_present));
                payload["merged_section"] = rendered_value
                    .map_or(serde_json::Value::Null, |value| {
                        rendered_json_to_serde(&value)
                    });
            }
        }
        if section == "telemetry" {
            let resolution = resolve_telemetry_config(Some(&runtime_config));
            payload["effective"] = telemetry_json_value(&resolution, &runtime_config);
        }
    } else {
        payload["merged_json"] = rendered_json_to_serde(&runtime_config.as_json().render());
    }

    Ok(payload)
}

fn rendered_json_to_serde(rendered: &str) -> serde_json::Value {
    serde_json::from_str(rendered).unwrap_or(serde_json::Value::Null)
}

fn telemetry_settings_path(cwd: &Path, target: Option<&str>) -> PathBuf {
    match target {
        Some("local") => cwd.join(".orbit").join("settings.local.json"),
        _ => cwd.join(".orbit").join("settings.json"),
    }
}

fn update_project_telemetry_settings(
    cwd: &Path,
    enabled: bool,
    target: Option<&str>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let settings_path = telemetry_settings_path(cwd, target);
    if let Some(parent) = settings_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut root = if settings_path.exists() {
        let content = fs::read_to_string(&settings_path)?;
        if content.trim().is_empty() {
            serde_json::Map::new()
        } else {
            match serde_json::from_str::<Value>(&content)? {
                Value::Object(object) => object,
                _ => {
                    return Err(format!(
                        "{} must contain a top-level JSON object",
                        settings_path.display()
                    )
                    .into())
                }
            }
        }
    } else {
        serde_json::Map::new()
    };

    let telemetry_path = cwd.join(".orbit").join("telemetry.jsonl");
    let mut telemetry = root
        .remove("telemetry")
        .and_then(|value| match value {
            Value::Object(object) => Some(object),
            _ => None,
        })
        .unwrap_or_default();
    telemetry.insert("enabled".to_string(), Value::Bool(enabled));
    if !telemetry.contains_key("path") {
        telemetry.insert(
            "path".to_string(),
            Value::String(telemetry_path.display().to_string()),
        );
    }
    root.insert("telemetry".to_string(), Value::Object(telemetry));

    fs::write(
        &settings_path,
        serde_json::to_string_pretty(&Value::Object(root))?,
    )?;
    Ok(settings_path)
}

fn telemetry_update_report(
    action: &str,
    settings_path: &Path,
    runtime_config: &orbit_runtime::RuntimeConfig,
) -> String {
    let resolution = resolve_telemetry_config(Some(runtime_config));
    [
        "Telemetry".to_string(),
        report_row("Result", "updated"),
        report_row("Action", action),
        report_row("Settings file", settings_path.display()),
        report_row("Enabled", if resolution.enabled { "yes" } else { "no" }),
        report_row(
            "Effective path",
            resolution.path.as_deref().unwrap_or("<unset>"),
        ),
        report_row("Source", resolution.source),
    ]
    .join("\n")
}

fn print_telemetry_status(
    output_format: CliOutputFormat,
    action: Option<&str>,
    target: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let action = action.unwrap_or("status");
    match action {
        "status" => {
            let (_cwd, runtime_config) = load_runtime_config_for_current_dir_or_empty()?;
            match output_format {
                CliOutputFormat::Text => println!("{}", render_telemetry_report(target)?),
                CliOutputFormat::Json => println!(
                    "{}",
                    serde_json::to_string_pretty(&telemetry_status_json_value(
                        &runtime_config,
                        target,
                    )?)?
                ),
            }
        }
        "on" | "off" => {
            let cwd = env::current_dir()?;
            let enabled = action == "on";
            let settings_path =
                update_project_telemetry_settings(&cwd, enabled, Some(target.unwrap_or("project")))?;
            let (_cwd, runtime_config) = load_runtime_config_for_current_dir()?;
            let resolution = resolve_telemetry_config(Some(&runtime_config));
            match output_format {
                CliOutputFormat::Text => {
                    println!(
                        "{}",
                        telemetry_update_report(action, &settings_path, &runtime_config)
                    );
                }
                CliOutputFormat::Json => println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                            "kind": "telemetry",
                            "status": "updated",
                            "action": action,
                            "target": target.unwrap_or("project"),
                            "settings_path": settings_path.display().to_string(),
                            "effective": telemetry_json_value(&resolution, &runtime_config),
                        }))?
                ),
            }
        }
        other => match output_format {
            CliOutputFormat::Text => println!(
                "Telemetry\n  Result           unsupported\n  Action           {other}\n  Supported        orbit telemetry [status|on|off] [project|local]"
            ),
            CliOutputFormat::Json => println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "kind": "telemetry",
                    "status": "unsupported",
                    "action": other
                }))?
            ),
        },
    }
    Ok(())
}

fn render_telemetry_report(target: Option<&str>) -> Result<String, Box<dyn std::error::Error>> {
    let (cwd, runtime_config) = load_runtime_config_for_current_dir_or_empty()?;
    let resolution = resolve_telemetry_config(Some(&runtime_config));
    let mut lines = vec!["Telemetry".to_string()];
    if let Some(target) = target {
        let status = telemetry_target_status(&cwd, target);
        lines.extend(telemetry_target_detail_lines(&status));
        lines.push("Effective telemetry".to_string());
    }
    lines.extend(telemetry_text_detail_lines(&resolution, &runtime_config));
    Ok(lines.join("\n"))
}

fn render_help_topic(topic: LocalHelpTopic) -> String {
    match topic {
        LocalHelpTopic::Status => "Status
  Usage            orbit status
  Purpose          show the local workspace snapshot without entering the REPL
  Output           model, permissions, git state, config files, and sandbox status
  Related          /status · orbit --resume latest /status"
            .to_string(),
        LocalHelpTopic::Sandbox => "Sandbox
  Usage            orbit sandbox
  Purpose          inspect the resolved sandbox and isolation state for the current directory
  Output           namespace, network, filesystem, and fallback details
  Related          /sandbox · orbit status"
            .to_string(),
        LocalHelpTopic::Doctor => "Doctor
  Usage            orbit doctor
  Purpose          diagnose local auth, config, workspace, sandbox, and build metadata
  Output           local-only health report; no provider request or session resume required
  Related          /doctor · orbit --resume latest /doctor"
            .to_string(),
    }
}

fn print_help_topic(topic: LocalHelpTopic) {
    println!("{}", render_help_topic(topic));
}

fn render_config_report(section: Option<&str>) -> Result<String, Box<dyn std::error::Error>> {
    let (cwd, runtime_config) = load_runtime_config_for_current_dir()?;
    let discovered = ConfigLoader::default_for(&cwd).discover();
    let discovered_files = summarize_discovered_config_files(&discovered, &runtime_config);

    let mut lines = vec![
        format!(
            "Config
  Working directory {}
  Loaded files      {}
  Merged keys       {}",
            cwd.display(),
            runtime_config.loaded_entries().len(),
            runtime_config.merged().len()
        ),
        "Discovered files".to_string(),
    ];
    for entry in discovered_files {
        lines.push(format!(
            "  {source:<7} {status:<7} {}",
            entry.path,
            source = entry.source,
            status = entry.status,
        ));
    }

    if let Some(section) = section {
        lines.push(format!("Merged section: {section}"));
        match resolve_config_section(&runtime_config, section) {
            ConfigSectionResolution::Unsupported => {
                lines.push(report_row("Section status", "unsupported"));
                lines.push(format!("  {}", unsupported_config_section_message(section)));
            }
            ConfigSectionResolution::Supported { rendered_value } => {
                lines.push(report_row(
                    "Section status",
                    config_section_status(rendered_value.is_some()),
                ));
                lines.push(format!(
                    "  {}",
                    rendered_value.unwrap_or_else(|| "<unset>".to_string())
                ));
            }
        }
        if section == "telemetry" {
            let resolution = resolve_telemetry_config(Some(&runtime_config));
            lines.push("Effective telemetry".to_string());
            lines.extend(telemetry_text_detail_lines(&resolution, &runtime_config));
        }
        return Ok(lines.join(
            "
",
        ));
    }

    lines.push("Merged JSON".to_string());
    lines.push(format!("  {}", runtime_config.as_json().render()));
    Ok(lines.join(
        "
",
    ))
}

fn render_memory_report() -> Result<String, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let project_context = ProjectContext::discover(&cwd, DEFAULT_DATE)?;
    let mut lines = vec![format!(
        "Memory
  Working directory {}
  Instruction files {}",
        cwd.display(),
        project_context.instruction_files.len()
    )];
    if project_context.instruction_files.is_empty() {
        lines.push("Discovered files".to_string());
        lines.push(
            "  No CLAUDE instruction files discovered in the current directory ancestry."
                .to_string(),
        );
    } else {
        lines.push("Discovered files".to_string());
        for (index, file) in project_context.instruction_files.iter().enumerate() {
            let preview = file.content.lines().next().unwrap_or("").trim();
            let preview = if preview.is_empty() {
                "<empty>"
            } else {
                preview
            };
            lines.push(format!("  {}. {}", index + 1, file.path.display(),));
            lines.push(format!(
                "     lines={} preview={}",
                file.content.lines().count(),
                preview
            ));
        }
    }
    Ok(lines.join(
        "
",
    ))
}

fn init_agents_md() -> Result<String, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    Ok(initialize_repo(&cwd)?.render())
}

fn run_init(output_format: CliOutputFormat) -> Result<(), Box<dyn std::error::Error>> {
    let message = init_agents_md()?;
    match output_format {
        CliOutputFormat::Text => println!("{message}"),
        CliOutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&init_json_value(&message))?
        ),
    }
    Ok(())
}

fn init_json_value(message: &str) -> serde_json::Value {
    json!({
        "kind": "init",
        "message": message,
    })
}

fn normalize_permission_mode(mode: &str) -> Option<&'static str> {
    match mode.trim() {
        "read-only" => Some("read-only"),
        "workspace-write" => Some("workspace-write"),
        "danger-full-access" => Some("danger-full-access"),
        _ => None,
    }
}

fn render_diff_report() -> Result<String, Box<dyn std::error::Error>> {
    render_diff_report_for(&env::current_dir()?)
}

fn render_diff_report_for(cwd: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let staged = run_git_diff_command_in(cwd, &["diff", "--cached"])?;
    let unstaged = run_git_diff_command_in(cwd, &["diff"])?;
    if staged.trim().is_empty() && unstaged.trim().is_empty() {
        return Ok(
            "Diff\n  Result           clean working tree\n  Detail           no current changes"
                .to_string(),
        );
    }

    let mut sections = Vec::new();
    if !staged.trim().is_empty() {
        sections.push(format!("Staged changes:\n{}", staged.trim_end()));
    }
    if !unstaged.trim().is_empty() {
        sections.push(format!("Unstaged changes:\n{}", unstaged.trim_end()));
    }

    Ok(format!("Diff\n\n{}", sections.join("\n\n")))
}

fn run_git_diff_command_in(
    cwd: &Path,
    args: &[&str],
) -> Result<String, Box<dyn std::error::Error>> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("git {} failed: {stderr}", args.join(" ")).into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn render_teleport_report(target: &str) -> Result<String, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;

    let file_list = Command::new("rg")
        .args(["--files"])
        .current_dir(&cwd)
        .output()?;
    let file_matches = if file_list.status.success() {
        String::from_utf8(file_list.stdout)?
            .lines()
            .filter(|line| line.contains(target))
            .take(10)
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    let content_output = Command::new("rg")
        .args(["-n", "-S", "--color", "never", target, "."])
        .current_dir(&cwd)
        .output()?;

    let mut lines = vec![
        "Teleport".to_string(),
        format!("  Target           {target}"),
        "  Action           search workspace files and content for the target".to_string(),
    ];
    if !file_matches.is_empty() {
        lines.push(String::new());
        lines.push("File matches".to_string());
        lines.extend(file_matches.into_iter().map(|path| format!("  {path}")));
    }

    if content_output.status.success() {
        let matches = String::from_utf8(content_output.stdout)?;
        if !matches.trim().is_empty() {
            lines.push(String::new());
            lines.push("Content matches".to_string());
            lines.push(truncate_for_prompt(&matches, 4_000));
        }
    }

    if lines.len() == 1 {
        lines.push("  Result           no matches found".to_string());
    }

    Ok(lines.join("\n"))
}

fn render_last_tool_debug_report(session: &Session) -> Result<String, Box<dyn std::error::Error>> {
    let last_tool_use = session
        .messages
        .iter()
        .rev()
        .find_map(|message| {
            message.blocks.iter().rev().find_map(|block| match block {
                ContentBlock::ToolUse { id, name, input } => {
                    Some((id.clone(), name.clone(), input.clone()))
                }
                _ => None,
            })
        })
        .ok_or_else(|| "no prior tool call found in session".to_string())?;

    let tool_result = session.messages.iter().rev().find_map(|message| {
        message.blocks.iter().rev().find_map(|block| match block {
            ContentBlock::ToolResult {
                tool_use_id,
                tool_name,
                output,
                is_error,
            } if tool_use_id == &last_tool_use.0 => {
                Some((tool_name.clone(), output.clone(), *is_error))
            }
            _ => None,
        })
    });

    let mut lines = vec![
        "Debug tool call".to_string(),
        "  Action           inspect the last recorded tool call and its result".to_string(),
        format!("  Tool id          {}", last_tool_use.0),
        format!("  Tool name        {}", last_tool_use.1),
        "  Input".to_string(),
        indent_block(&last_tool_use.2, 4),
    ];

    match tool_result {
        Some((tool_name, output, is_error)) => {
            lines.push("  Result".to_string());
            lines.push(format!("    name           {tool_name}"));
            lines.push(format!(
                "    status         {}",
                if is_error { "error" } else { "ok" }
            ));
            lines.push(indent_block(&output, 4));
        }
        None => lines.push("  Result           missing tool result".to_string()),
    }

    Ok(lines.join("\n"))
}

fn indent_block(value: &str, spaces: usize) -> String {
    let indent = " ".repeat(spaces);
    value
        .lines()
        .map(|line| format!("{indent}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn validate_no_args(
    command_name: &str,
    args: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(args) = args.map(str::trim).filter(|value| !value.is_empty()) {
        return Err(format!(
            "{command_name} does not accept arguments. Received: {args}\nUsage: {command_name}"
        )
        .into());
    }
    Ok(())
}

fn format_bughunter_report(scope: Option<&str>) -> String {
    format!(
        "Bughunter
  Scope            {}
  Action           inspect the selected code for likely bugs and correctness issues
  Output           findings should include file paths, severity, and suggested fixes",
        scope.unwrap_or("the current repository")
    )
}

fn format_ultraplan_report(task: Option<&str>) -> String {
    format!(
        "Ultraplan
  Task             {}
  Action           break work into a multi-step execution plan
  Output           plan should cover goals, risks, sequencing, verification, and rollback",
        task.unwrap_or("the current repo work")
    )
}

fn format_pr_report(branch: &str, context: Option<&str>) -> String {
    format!(
        "PR
  Branch           {branch}
  Context          {}
  Action           draft or create a pull request for the current branch
  Output           title and markdown body suitable for GitHub",
        context.unwrap_or("none")
    )
}

fn format_issue_report(context: Option<&str>) -> String {
    format!(
        "Issue
  Context          {}
  Action           draft or create a GitHub issue from the current context
  Output           title and markdown body suitable for GitHub",
        context.unwrap_or("none")
    )
}

fn git_output(args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .args(args)
        .current_dir(env::current_dir()?)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("git {} failed: {stderr}", args.join(" ")).into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn git_status_ok(args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .args(args)
        .current_dir(env::current_dir()?)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("git {} failed: {stderr}", args.join(" ")).into());
    }
    Ok(())
}

fn command_exists(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn write_temp_text_file(
    filename: &str,
    contents: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = env::temp_dir().join(filename);
    fs::write(&path, contents)?;
    Ok(path)
}

fn recent_user_context(session: &Session, limit: usize) -> String {
    let requests = session
        .messages
        .iter()
        .filter(|message| message.role == MessageRole::User)
        .filter_map(|message| {
            message.blocks.iter().find_map(|block| match block {
                ContentBlock::Text { text } => Some(text.trim().to_string()),
                _ => None,
            })
        })
        .rev()
        .take(limit)
        .collect::<Vec<_>>();

    if requests.is_empty() {
        "<no prior user messages>".to_string()
    } else {
        requests
            .into_iter()
            .rev()
            .enumerate()
            .map(|(index, text)| format!("{}. {}", index + 1, text))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn truncate_for_prompt(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        value.trim().to_string()
    } else {
        let truncated = value.chars().take(limit).collect::<String>();
        format!("{}\n…[truncated]", truncated.trim_end())
    }
}

fn sanitize_generated_message(value: &str) -> String {
    value.trim().trim_matches('`').trim().replace("\r\n", "\n")
}

fn parse_titled_body(value: &str) -> Option<(String, String)> {
    let normalized = sanitize_generated_message(value);
    let title = normalized
        .lines()
        .find_map(|line| line.strip_prefix("TITLE:").map(str::trim))?;
    let body_start = normalized.find("BODY:")?;
    let body = normalized[body_start + "BODY:".len()..].trim();
    Some((title.to_string(), body.to_string()))
}

fn render_version_report() -> String {
    let git_sha = GIT_SHA.unwrap_or("unknown");
    let target = BUILD_TARGET.unwrap_or("unknown");
    format!(
        "Orbit\n  Version          {VERSION}\n  Git SHA          {git_sha}\n  Target           {target}\n  Build date       {DEFAULT_DATE}"
    )
}

fn render_export_text(session: &Session) -> String {
    let mut lines = vec!["# Conversation Export".to_string(), String::new()];
    for (index, message) in session.messages.iter().enumerate() {
        let role = match message.role {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
        };
        lines.push(format!("## {}. {role}", index + 1));
        for block in &message.blocks {
            match block {
                ContentBlock::Text { text } => lines.push(text.clone()),
                ContentBlock::ToolUse { id, name, input } => {
                    lines.push(format!("[tool_use id={id} name={name}] {input}"));
                }
                ContentBlock::ToolResult {
                    tool_use_id,
                    tool_name,
                    output,
                    is_error,
                } => {
                    lines.push(format!(
                        "[tool_result id={tool_use_id} name={tool_name} error={is_error}] {output}"
                    ));
                }
            }
        }
        lines.push(String::new());
    }
    lines.join("\n")
}

fn default_export_filename(session: &Session) -> String {
    let stem = session
        .messages
        .iter()
        .find_map(|message| match message.role {
            MessageRole::User => message.blocks.iter().find_map(|block| match block {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            }),
            _ => None,
        })
        .map_or("conversation", |text| {
            text.lines().next().unwrap_or("conversation")
        })
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .take(8)
        .collect::<Vec<_>>()
        .join("-");
    let fallback = if stem.is_empty() {
        "conversation"
    } else {
        &stem
    };
    format!("{fallback}.txt")
}

fn resolve_export_path(
    requested_path: Option<&str>,
    session: &Session,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let file_name =
        requested_path.map_or_else(|| default_export_filename(session), ToOwned::to_owned);
    let final_name = if Path::new(&file_name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("txt"))
    {
        file_name
    } else {
        format!("{file_name}.txt")
    };
    Ok(cwd.join(final_name))
}

fn build_system_prompt() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(load_system_prompt(
        env::current_dir()?,
        DEFAULT_DATE,
        env::consts::OS,
        "unknown",
    )?)
}

fn build_runtime_plugin_state() -> Result<RuntimePluginState, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let loader = ConfigLoader::default_for(&cwd);
    let runtime_config = loader.load()?;
    build_runtime_plugin_state_with_loader(&cwd, &loader, &runtime_config)
}

fn build_runtime_plugin_state_with_loader(
    cwd: &Path,
    loader: &ConfigLoader,
    runtime_config: &orbit_runtime::RuntimeConfig,
) -> Result<RuntimePluginState, Box<dyn std::error::Error>> {
    let plugin_manager = build_plugin_manager(cwd, loader, runtime_config);
    let plugin_registry = plugin_manager.plugin_registry()?;
    let plugin_hook_config =
        runtime_hook_config_from_plugin_hooks(plugin_registry.aggregated_hooks()?);
    let feature_config = runtime_config
        .feature_config()
        .clone()
        .with_hooks(runtime_config.hooks().merged(&plugin_hook_config));
    let (mcp_state, runtime_tools) = build_runtime_mcp_state(runtime_config)?;
    let tool_registry = GlobalToolRegistry::with_plugin_tools(plugin_registry.aggregated_tools()?)?
        .with_runtime_tools(runtime_tools)?;
    Ok(RuntimePluginState {
        runtime_config: runtime_config.clone(),
        feature_config,
        tool_registry,
        plugin_registry,
        mcp_state,
    })
}

fn build_plugin_manager(
    cwd: &Path,
    loader: &ConfigLoader,
    runtime_config: &orbit_runtime::RuntimeConfig,
) -> PluginManager {
    let plugin_settings = runtime_config.plugins();
    let mut plugin_config = PluginManagerConfig::new(loader.config_home().to_path_buf());
    plugin_config.enabled_plugins = plugin_settings.enabled_plugins().clone();
    plugin_config.external_dirs = plugin_settings
        .external_directories()
        .iter()
        .map(|path| resolve_plugin_path(cwd, loader.config_home(), path))
        .collect();
    plugin_config.install_root = plugin_settings
        .install_root()
        .map(|path| resolve_plugin_path(cwd, loader.config_home(), path));
    plugin_config.registry_path = plugin_settings
        .registry_path()
        .map(|path| resolve_plugin_path(cwd, loader.config_home(), path));
    plugin_config.bundled_root = plugin_settings
        .bundled_root()
        .map(|path| resolve_plugin_path(cwd, loader.config_home(), path));
    PluginManager::new(plugin_config)
}

fn resolve_plugin_path(cwd: &Path, config_home: &Path, value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else if value.starts_with('.') {
        cwd.join(path)
    } else {
        config_home.join(path)
    }
}

fn runtime_hook_config_from_plugin_hooks(hooks: PluginHooks) -> orbit_runtime::RuntimeHookConfig {
    orbit_runtime::RuntimeHookConfig::new(
        hooks.pre_tool_use,
        hooks.post_tool_use,
        hooks.post_tool_use_failure,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InternalPromptProgressState {
    command_label: &'static str,
    task_label: String,
    step: usize,
    phase: String,
    detail: Option<String>,
    saw_final_text: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InternalPromptProgressEvent {
    Started,
    Update,
    Heartbeat,
    Complete,
    Failed,
}

#[derive(Debug)]
struct InternalPromptProgressShared {
    state: Mutex<InternalPromptProgressState>,
    output_lock: Mutex<()>,
    started_at: Instant,
}

#[derive(Debug, Clone)]
struct InternalPromptProgressReporter {
    shared: Arc<InternalPromptProgressShared>,
}

#[derive(Debug)]
struct InternalPromptProgressRun {
    reporter: InternalPromptProgressReporter,
    heartbeat_stop: Option<mpsc::Sender<()>>,
    heartbeat_handle: Option<thread::JoinHandle<()>>,
}

impl InternalPromptProgressReporter {
    fn ultraplan(task: &str) -> Self {
        Self {
            shared: Arc::new(InternalPromptProgressShared {
                state: Mutex::new(InternalPromptProgressState {
                    command_label: "Ultraplan",
                    task_label: task.to_string(),
                    step: 0,
                    phase: "planning started".to_string(),
                    detail: Some(format!("task: {task}")),
                    saw_final_text: false,
                }),
                output_lock: Mutex::new(()),
                started_at: Instant::now(),
            }),
        }
    }

    fn emit(&self, event: InternalPromptProgressEvent, error: Option<&str>) {
        let snapshot = self.snapshot();
        let line = format_internal_prompt_progress_line(event, &snapshot, self.elapsed(), error);
        self.write_line(&line);
    }

    fn mark_model_phase(&self) {
        let snapshot = {
            let mut state = self
                .shared
                .state
                .lock()
                .expect("internal prompt progress state poisoned");
            state.step += 1;
            state.phase = if state.step == 1 {
                "analyzing request".to_string()
            } else {
                "reviewing findings".to_string()
            };
            state.detail = Some(format!("task: {}", state.task_label));
            state.clone()
        };
        self.write_line(&format_internal_prompt_progress_line(
            InternalPromptProgressEvent::Update,
            &snapshot,
            self.elapsed(),
            None,
        ));
    }

    fn mark_tool_phase(&self, name: &str, input: &str) {
        let detail = describe_tool_progress(name, input);
        let snapshot = {
            let mut state = self
                .shared
                .state
                .lock()
                .expect("internal prompt progress state poisoned");
            state.step += 1;
            state.phase = format!("running {name}");
            state.detail = Some(detail);
            state.clone()
        };
        self.write_line(&format_internal_prompt_progress_line(
            InternalPromptProgressEvent::Update,
            &snapshot,
            self.elapsed(),
            None,
        ));
    }

    fn mark_text_phase(&self, text: &str) {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return;
        }
        let detail = truncate_for_summary(first_visible_line(trimmed), 120);
        let snapshot = {
            let mut state = self
                .shared
                .state
                .lock()
                .expect("internal prompt progress state poisoned");
            if state.saw_final_text {
                return;
            }
            state.saw_final_text = true;
            state.step += 1;
            state.phase = "drafting final plan".to_string();
            state.detail = (!detail.is_empty()).then_some(detail);
            state.clone()
        };
        self.write_line(&format_internal_prompt_progress_line(
            InternalPromptProgressEvent::Update,
            &snapshot,
            self.elapsed(),
            None,
        ));
    }

    fn emit_heartbeat(&self) {
        let snapshot = self.snapshot();
        self.write_line(&format_internal_prompt_progress_line(
            InternalPromptProgressEvent::Heartbeat,
            &snapshot,
            self.elapsed(),
            None,
        ));
    }

    fn snapshot(&self) -> InternalPromptProgressState {
        self.shared
            .state
            .lock()
            .expect("internal prompt progress state poisoned")
            .clone()
    }

    fn elapsed(&self) -> Duration {
        self.shared.started_at.elapsed()
    }

    fn write_line(&self, line: &str) {
        let _guard = self
            .shared
            .output_lock
            .lock()
            .expect("internal prompt progress output lock poisoned");
        let mut stdout = io::stdout();
        let _ = writeln!(stdout, "{line}");
        let _ = stdout.flush();
    }
}

impl InternalPromptProgressRun {
    fn start_ultraplan(task: &str) -> Self {
        let reporter = InternalPromptProgressReporter::ultraplan(task);
        reporter.emit(InternalPromptProgressEvent::Started, None);

        let (heartbeat_stop, heartbeat_rx) = mpsc::channel();
        let heartbeat_reporter = reporter.clone();
        let heartbeat_handle = thread::spawn(move || loop {
            match heartbeat_rx.recv_timeout(INTERNAL_PROGRESS_HEARTBEAT_INTERVAL) {
                Ok(()) | Err(RecvTimeoutError::Disconnected) => break,
                Err(RecvTimeoutError::Timeout) => heartbeat_reporter.emit_heartbeat(),
            }
        });

        Self {
            reporter,
            heartbeat_stop: Some(heartbeat_stop),
            heartbeat_handle: Some(heartbeat_handle),
        }
    }

    fn reporter(&self) -> InternalPromptProgressReporter {
        self.reporter.clone()
    }

    fn finish_success(&mut self) {
        self.stop_heartbeat();
        self.reporter
            .emit(InternalPromptProgressEvent::Complete, None);
    }

    fn finish_failure(&mut self, error: &str) {
        self.stop_heartbeat();
        self.reporter
            .emit(InternalPromptProgressEvent::Failed, Some(error));
    }

    fn stop_heartbeat(&mut self) {
        if let Some(sender) = self.heartbeat_stop.take() {
            let _ = sender.send(());
        }
        if let Some(handle) = self.heartbeat_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for InternalPromptProgressRun {
    fn drop(&mut self) {
        self.stop_heartbeat();
    }
}

fn format_internal_prompt_progress_line(
    event: InternalPromptProgressEvent,
    snapshot: &InternalPromptProgressState,
    elapsed: Duration,
    error: Option<&str>,
) -> String {
    let elapsed_seconds = elapsed.as_secs();
    let step_label = if snapshot.step == 0 {
        "current step pending".to_string()
    } else {
        format!("current step {}", snapshot.step)
    };
    let mut status_bits = vec![step_label, format!("phase {}", snapshot.phase)];
    if let Some(detail) = snapshot
        .detail
        .as_deref()
        .filter(|detail| !detail.is_empty())
    {
        status_bits.push(detail.to_string());
    }
    let status = status_bits.join(" · ");
    match event {
        InternalPromptProgressEvent::Started => {
            format!(
                "🧭 {} status · planning started · {status}",
                snapshot.command_label
            )
        }
        InternalPromptProgressEvent::Update => {
            format!("… {} status · {status}", snapshot.command_label)
        }
        InternalPromptProgressEvent::Heartbeat => format!(
            "… {} heartbeat · {elapsed_seconds}s elapsed · {status}",
            snapshot.command_label
        ),
        InternalPromptProgressEvent::Complete => format!(
            "✔ {} status · completed · {elapsed_seconds}s elapsed · {} steps total",
            snapshot.command_label, snapshot.step
        ),
        InternalPromptProgressEvent::Failed => format!(
            "✘ {} status · failed · {elapsed_seconds}s elapsed · {}",
            snapshot.command_label,
            error.unwrap_or("unknown error")
        ),
    }
}

fn describe_tool_progress(name: &str, input: &str) -> String {
    let parsed: serde_json::Value =
        serde_json::from_str(input).unwrap_or(serde_json::Value::String(input.to_string()));
    match name {
        "bash" | "Bash" => {
            let command = parsed
                .get("command")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            if command.is_empty() {
                "running shell command".to_string()
            } else {
                format!("command {}", truncate_for_summary(command.trim(), 100))
            }
        }
        "read_file" | "Read" => format!("reading {}", extract_tool_path(&parsed)),
        "write_file" | "Write" => format!("writing {}", extract_tool_path(&parsed)),
        "edit_file" | "Edit" => format!("editing {}", extract_tool_path(&parsed)),
        "glob_search" | "Glob" => {
            let pattern = parsed
                .get("pattern")
                .and_then(|value| value.as_str())
                .unwrap_or("?");
            let scope = parsed
                .get("path")
                .and_then(|value| value.as_str())
                .unwrap_or(".");
            format!("glob `{pattern}` in {scope}")
        }
        "grep_search" | "Grep" => {
            let pattern = parsed
                .get("pattern")
                .and_then(|value| value.as_str())
                .unwrap_or("?");
            let scope = parsed
                .get("path")
                .and_then(|value| value.as_str())
                .unwrap_or(".");
            format!("grep `{pattern}` in {scope}")
        }
        "web_search" | "WebSearch" => parsed
            .get("query")
            .and_then(|value| value.as_str())
            .map_or_else(
                || "running web search".to_string(),
                |query| format!("query {}", truncate_for_summary(query, 100)),
            ),
        _ => {
            let summary = summarize_tool_payload(input);
            if summary.is_empty() {
                format!("running {name}")
            } else {
                format!("{name}: {summary}")
            }
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
#[allow(clippy::too_many_arguments)]
fn build_runtime(
    session: Session,
    session_id: &str,
    model: String,
    system_prompt: Vec<String>,
    enable_tools: bool,
    emit_output: bool,
    allowed_tools: Option<AllowedToolSet>,
    permission_mode: PermissionMode,
    progress_reporter: Option<InternalPromptProgressReporter>,
) -> Result<BuiltRuntime, Box<dyn std::error::Error>> {
    let runtime_plugin_state = build_runtime_plugin_state()?;
    build_runtime_with_plugin_state(
        session,
        session_id,
        model,
        system_prompt,
        enable_tools,
        emit_output,
        allowed_tools,
        permission_mode,
        progress_reporter,
        runtime_plugin_state,
    )
}

#[allow(clippy::needless_pass_by_value)]
#[allow(clippy::too_many_arguments)]
fn build_runtime_with_provider(
    session: Session,
    session_id: &str,
    model: String,
    system_prompt: Vec<String>,
    enable_tools: bool,
    emit_output: bool,
    allowed_tools: Option<AllowedToolSet>,
    permission_mode: PermissionMode,
    provider: Option<String>,
    progress_reporter: Option<InternalPromptProgressReporter>,
) -> Result<BuiltRuntime, Box<dyn std::error::Error>> {
    let runtime_plugin_state = build_runtime_plugin_state()?;
    build_runtime_with_plugin_state_and_provider(
        session,
        session_id,
        model,
        system_prompt,
        enable_tools,
        emit_output,
        allowed_tools,
        permission_mode,
        provider,
        progress_reporter,
        runtime_plugin_state,
    )
}

#[allow(clippy::needless_pass_by_value)]
#[allow(clippy::too_many_arguments)]
fn build_runtime_with_plugin_state(
    session: Session,
    session_id: &str,
    model: String,
    system_prompt: Vec<String>,
    enable_tools: bool,
    emit_output: bool,
    allowed_tools: Option<AllowedToolSet>,
    permission_mode: PermissionMode,
    progress_reporter: Option<InternalPromptProgressReporter>,
    runtime_plugin_state: RuntimePluginState,
) -> Result<BuiltRuntime, Box<dyn std::error::Error>> {
    let RuntimePluginState {
        runtime_config,
        feature_config,
        tool_registry,
        plugin_registry,
        mcp_state,
    } = runtime_plugin_state;
    plugin_registry.initialize()?;
    let session_tracer = build_cli_session_tracer(session_id, Some(&runtime_config));
    let tool_registry = match session_tracer.clone() {
        Some(tracer) => tool_registry.with_session_tracer(tracer),
        None => tool_registry,
    };
    let policy = permission_policy(permission_mode, &feature_config, &tool_registry)
        .map_err(std::io::Error::other)?;
    let client = if detect_provider_kind(&model) == ProviderKind::Anthropic {
        ProviderClient::Anthropic(
            AnthropicClient::from_auth(resolve_cli_auth_source()?)
                .with_base_url(orbit_api::read_base_url())
                .with_prompt_cache(PromptCache::new(session_id)),
        )
    } else {
        ProviderClient::from_model(&model)?
    };
    let client = attach_session_tracer(client, session_tracer.clone());
    let mcp_active = mcp_state.is_some();
    let plugins_active = true;
    let mut runtime = ConversationRuntime::new_with_features(
        session,
        GenericRuntimeClient::new(
            session_id,
            model,
            enable_tools,
            emit_output,
            allowed_tools.clone(),
            tool_registry.clone(),
            progress_reporter,
            client,
        )?,
        CliToolExecutor::new(
            session_id.to_string(),
            allowed_tools.clone(),
            emit_output,
            tool_registry.clone(),
            mcp_state.clone(),
        ),
        policy,
        system_prompt,
        &feature_config,
    );
    if let Some(session_tracer) = session_tracer {
        runtime = runtime.with_session_tracer(session_tracer);
    }
    if emit_output {
        runtime = runtime.with_hook_progress_reporter(Box::new(CliHookProgressReporter));
    }
    Ok(BuiltRuntime::new(
        runtime,
        plugin_registry,
        mcp_state,
        mcp_active,
        plugins_active,
    ))
}

#[allow(clippy::needless_pass_by_value)]
#[allow(clippy::too_many_arguments)]
fn build_runtime_with_plugin_state_and_provider(
    session: Session,
    session_id: &str,
    model: String,
    system_prompt: Vec<String>,
    enable_tools: bool,
    emit_output: bool,
    allowed_tools: Option<AllowedToolSet>,
    permission_mode: PermissionMode,
    provider: Option<String>,
    progress_reporter: Option<InternalPromptProgressReporter>,
    runtime_plugin_state: RuntimePluginState,
) -> Result<BuiltRuntime, Box<dyn std::error::Error>> {
    let RuntimePluginState {
        runtime_config,
        feature_config,
        tool_registry,
        plugin_registry,
        mcp_state,
    } = runtime_plugin_state;
    plugin_registry.initialize()?;
    let session_tracer = build_cli_session_tracer(session_id, Some(&runtime_config));
    let tool_registry = match session_tracer.clone() {
        Some(tracer) => tool_registry.with_session_tracer(tracer),
        None => tool_registry,
    };
    let policy = permission_policy(permission_mode, &feature_config, &tool_registry)
        .map_err(std::io::Error::other)?;
    let mut effective_model = model;
    let client = if let Some(provider_name) = provider {
        if provider_name.eq_ignore_ascii_case("ollama") && effective_model == DEFAULT_MODEL {
            effective_model = env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama2".to_string());
        }
        if provider_name.eq_ignore_ascii_case("anthropic") {
            ProviderClient::Anthropic(
                AnthropicClient::from_auth(resolve_cli_auth_source()?)
                    .with_base_url(orbit_api::read_base_url())
                    .with_prompt_cache(PromptCache::new(session_id)),
            )
        } else {
            create_provider_client(&provider_name, effective_model.clone())?
        }
    } else {
        ProviderClient::from_model(&effective_model)?
    };
    let client = attach_session_tracer(client, session_tracer.clone());
    let mcp_active = mcp_state.is_some();
    let plugins_active = true;
    let mut runtime = ConversationRuntime::new_with_features(
        session,
        GenericRuntimeClient::new(
            session_id,
            effective_model,
            enable_tools,
            emit_output,
            allowed_tools.clone(),
            tool_registry.clone(),
            progress_reporter,
            client,
        )?,
        CliToolExecutor::new(
            session_id.to_string(),
            allowed_tools.clone(),
            emit_output,
            tool_registry.clone(),
            mcp_state.clone(),
        ),
        policy,
        system_prompt,
        &feature_config,
    );
    if let Some(session_tracer) = session_tracer {
        runtime = runtime.with_session_tracer(session_tracer);
    }
    if emit_output {
        runtime = runtime.with_hook_progress_reporter(Box::new(CliHookProgressReporter));
    }
    Ok(BuiltRuntime::new(
        runtime,
        plugin_registry,
        mcp_state,
        mcp_active,
        plugins_active,
    ))
}

struct CliHookProgressReporter;

impl orbit_runtime::HookProgressReporter for CliHookProgressReporter {
    fn on_event(&mut self, event: &orbit_runtime::HookProgressEvent) {
        match event {
            orbit_runtime::HookProgressEvent::Started {
                event,
                tool_name,
                command,
            } => eprintln!(
                "[hook {event_name}] {tool_name}: {command}",
                event_name = event.as_str()
            ),
            orbit_runtime::HookProgressEvent::Completed {
                event,
                tool_name,
                command,
            } => eprintln!(
                "[hook done {event_name}] {tool_name}: {command}",
                event_name = event.as_str()
            ),
            orbit_runtime::HookProgressEvent::Cancelled {
                event,
                tool_name,
                command,
            } => eprintln!(
                "[hook cancelled {event_name}] {tool_name}: {command}",
                event_name = event.as_str()
            ),
        }
    }
}

struct CliPermissionPrompter {
    current_mode: PermissionMode,
}

impl CliPermissionPrompter {
    fn new(current_mode: PermissionMode) -> Self {
        Self { current_mode }
    }
}

impl orbit_runtime::PermissionPrompter for CliPermissionPrompter {
    fn decide(
        &mut self,
        request: &orbit_runtime::PermissionRequest,
    ) -> orbit_runtime::PermissionPromptDecision {
        println!();
        println!("Permission approval required");
        println!("  Tool             {}", request.tool_name);
        println!("  Current mode     {}", self.current_mode.as_str());
        println!("  Required mode    {}", request.required_mode.as_str());
        if let Some(reason) = &request.reason {
            println!("  Reason           {reason}");
        }
        println!("  Input            {}", request.input);
        print!("Approve this tool call? [y/N]: ");
        let _ = io::stdout().flush();

        let mut response = String::new();
        match io::stdin().read_line(&mut response) {
            Ok(_) => {
                let normalized = response.trim().to_ascii_lowercase();
                if matches!(normalized.as_str(), "y" | "yes") {
                    orbit_runtime::PermissionPromptDecision::Allow
                } else {
                    orbit_runtime::PermissionPromptDecision::Deny {
                        reason: format!(
                            "tool '{}' denied by user approval prompt",
                            request.tool_name
                        ),
                    }
                }
            }
            Err(error) => orbit_runtime::PermissionPromptDecision::Deny {
                reason: format!("permission approval failed: {error}"),
            },
        }
    }
}

struct GenericRuntimeClient {
    runtime: tokio::runtime::Runtime,
    client: orbit_api::ProviderClient,
    session_id: String,
    model: String,
    enable_tools: bool,
    emit_output: bool,
    allowed_tools: Option<AllowedToolSet>,
    tool_registry: GlobalToolRegistry,
    progress_reporter: Option<InternalPromptProgressReporter>,
}

impl GenericRuntimeClient {
    fn new(
        session_id: &str,
        model: String,
        enable_tools: bool,
        emit_output: bool,
        allowed_tools: Option<AllowedToolSet>,
        tool_registry: GlobalToolRegistry,
        progress_reporter: Option<InternalPromptProgressReporter>,
        client: orbit_api::ProviderClient,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            runtime: tokio::runtime::Runtime::new()?,
            client,
            session_id: session_id.to_string(),
            model,
            enable_tools,
            emit_output,
            allowed_tools,
            tool_registry,
            progress_reporter,
        })
    }
}

impl ApiClient for GenericRuntimeClient {
    fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
        if let Some(progress_reporter) = &self.progress_reporter {
            progress_reporter.mark_model_phase();
        }
        let message_request = MessageRequest {
            model: self.model.clone(),
            max_tokens: max_tokens_for_model(&self.model),
            messages: convert_messages(&request.messages),
            system: (!request.system_prompt.is_empty()).then(|| request.system_prompt.join("\n\n")),
            tools: self
                .enable_tools
                .then(|| filter_tool_specs(&self.tool_registry, self.allowed_tools.as_ref())),
            tool_choice: self.enable_tools.then_some(ToolChoice::Auto),
            stream: true,
        };

        self.runtime.block_on(async {
            let mut stream =
                self.client
                    .stream_message(&message_request)
                    .await
                    .map_err(|error| {
                        RuntimeError::new(format_user_visible_api_error(&self.session_id, &error))
                    })?;
            let mut stdout = io::stdout();
            let mut sink = io::sink();
            let out: &mut dyn Write = if self.emit_output {
                &mut stdout
            } else {
                &mut sink
            };
            let renderer = TerminalRenderer::new();
            let mut markdown_stream = MarkdownStreamState::default();
            let mut events = Vec::new();
            let mut pending_tool: Option<(String, String, String)> = None;
            let mut block_has_thinking_summary = false;
            let mut saw_stop = false;

            while let Some(event) = stream.next_event().await.map_err(|error| {
                RuntimeError::new(format_user_visible_api_error(&self.session_id, &error))
            })? {
                match event {
                    ApiStreamEvent::MessageStart(start) => {
                        for block in start.message.content {
                            push_output_block(
                                block,
                                out,
                                &mut events,
                                &mut pending_tool,
                                true,
                                &mut block_has_thinking_summary,
                            )?;
                        }
                    }
                    ApiStreamEvent::ContentBlockStart(start) => {
                        push_output_block(
                            start.content_block,
                            out,
                            &mut events,
                            &mut pending_tool,
                            true,
                            &mut block_has_thinking_summary,
                        )?;
                    }
                    ApiStreamEvent::ContentBlockDelta(delta) => match delta.delta {
                        ContentBlockDelta::TextDelta { text } => {
                            if !text.is_empty() {
                                if let Some(progress_reporter) = &self.progress_reporter {
                                    progress_reporter.mark_text_phase(&text);
                                }
                                if let Some(rendered) = markdown_stream.push(&renderer, &text) {
                                    write!(out, "{rendered}")
                                        .and_then(|()| out.flush())
                                        .map_err(|error| RuntimeError::new(error.to_string()))?;
                                }
                                events.push(AssistantEvent::TextDelta(text));
                            }
                        }
                        ContentBlockDelta::InputJsonDelta { partial_json } => {
                            if let Some((_, _, input)) = &mut pending_tool {
                                input.push_str(&partial_json);
                            }
                        }
                        ContentBlockDelta::ThinkingDelta { .. } => {
                            if !block_has_thinking_summary {
                                render_thinking_block_summary(out, None, false)?;
                                block_has_thinking_summary = true;
                            }
                        }
                        ContentBlockDelta::SignatureDelta { .. } => {}
                    },
                    ApiStreamEvent::ContentBlockStop(_) => {
                        block_has_thinking_summary = false;
                        if let Some(rendered) = markdown_stream.flush(&renderer) {
                            write!(out, "{rendered}")
                                .and_then(|()| out.flush())
                                .map_err(|error| RuntimeError::new(error.to_string()))?;
                        }
                        if let Some((id, name, input)) = pending_tool.take() {
                            if let Some(progress_reporter) = &self.progress_reporter {
                                progress_reporter.mark_tool_phase(&name, &input);
                            }
                            writeln!(out, "\n{}", format_tool_call_start(&name, &input))
                                .and_then(|()| out.flush())
                                .map_err(|error| RuntimeError::new(error.to_string()))?;
                            events.push(AssistantEvent::ToolUse { id, name, input });
                        }
                    }
                    ApiStreamEvent::MessageDelta(delta) => {
                        events.push(AssistantEvent::Usage(delta.usage.token_usage()));
                    }
                    ApiStreamEvent::MessageStop(_) => {
                        saw_stop = true;
                        if let Some(rendered) = markdown_stream.flush(&renderer) {
                            write!(out, "{rendered}")
                                .and_then(|()| out.flush())
                                .map_err(|error| RuntimeError::new(error.to_string()))?;
                        }
                        events.push(AssistantEvent::MessageStop);
                    }
                }
            }

            push_prompt_cache_record_for_provider(&self.client, &mut events);

            let has_meaningful_content = events.iter().any(|event| {
                matches!(event, AssistantEvent::TextDelta(text) if !text.is_empty())
                    || matches!(event, AssistantEvent::ToolUse { .. })
            });
            if !saw_stop && has_meaningful_content {
                events.push(AssistantEvent::MessageStop);
            }

            if has_meaningful_content
                && events
                    .iter()
                    .any(|event| matches!(event, AssistantEvent::MessageStop))
            {
                return Ok(events);
            }

            let response = self
                .client
                .send_message(&MessageRequest {
                    stream: false,
                    ..message_request.clone()
                })
                .await
                .map_err(|error| {
                    RuntimeError::new(format_user_visible_api_error(&self.session_id, &error))
                })?;
            let mut events = response_to_events(response, out)?;
            push_prompt_cache_record_for_provider(&self.client, &mut events);
            Ok(events)
        })
    }
}

struct AnthropicRuntimeClient {
    runtime: tokio::runtime::Runtime,
    client: AnthropicClient,
    session_id: String,
    model: String,
    enable_tools: bool,
    emit_output: bool,
    allowed_tools: Option<AllowedToolSet>,
    tool_registry: GlobalToolRegistry,
    progress_reporter: Option<InternalPromptProgressReporter>,
}

impl AnthropicRuntimeClient {
    fn new(
        session_id: &str,
        model: String,
        enable_tools: bool,
        emit_output: bool,
        allowed_tools: Option<AllowedToolSet>,
        tool_registry: GlobalToolRegistry,
        progress_reporter: Option<InternalPromptProgressReporter>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            runtime: tokio::runtime::Runtime::new()?,
            client: AnthropicClient::from_auth(resolve_cli_auth_source()?)
                .with_base_url(orbit_api::read_base_url())
                .with_prompt_cache(PromptCache::new(session_id)),
            session_id: session_id.to_string(),
            model,
            enable_tools,
            emit_output,
            allowed_tools,
            tool_registry,
            progress_reporter,
        })
    }
}

fn resolve_cli_auth_source() -> Result<AuthSource, Box<dyn std::error::Error>> {
    Ok(AuthSource::from_env()?)
}

impl ApiClient for AnthropicRuntimeClient {
    #[allow(clippy::too_many_lines)]
    fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
        if let Some(progress_reporter) = &self.progress_reporter {
            progress_reporter.mark_model_phase();
        }
        let message_request = MessageRequest {
            model: self.model.clone(),
            max_tokens: max_tokens_for_model(&self.model),
            messages: convert_messages(&request.messages),
            system: (!request.system_prompt.is_empty()).then(|| request.system_prompt.join("\n\n")),
            tools: self
                .enable_tools
                .then(|| filter_tool_specs(&self.tool_registry, self.allowed_tools.as_ref())),
            tool_choice: self.enable_tools.then_some(ToolChoice::Auto),
            stream: true,
        };

        self.runtime.block_on(async {
            let mut stream =
                self.client
                    .stream_message(&message_request)
                    .await
                    .map_err(|error| {
                        RuntimeError::new(format_user_visible_api_error(&self.session_id, &error))
                    })?;
            let mut stdout = io::stdout();
            let mut sink = io::sink();
            let out: &mut dyn Write = if self.emit_output {
                &mut stdout
            } else {
                &mut sink
            };
            let renderer = TerminalRenderer::new();
            let mut markdown_stream = MarkdownStreamState::default();
            let mut events = Vec::new();
            let mut pending_tool: Option<(String, String, String)> = None;
            let mut block_has_thinking_summary = false;
            let mut saw_stop = false;

            while let Some(event) = stream.next_event().await.map_err(|error| {
                RuntimeError::new(format_user_visible_api_error(&self.session_id, &error))
            })? {
                match event {
                    ApiStreamEvent::MessageStart(start) => {
                        for block in start.message.content {
                            push_output_block(
                                block,
                                out,
                                &mut events,
                                &mut pending_tool,
                                true,
                                &mut block_has_thinking_summary,
                            )?;
                        }
                    }
                    ApiStreamEvent::ContentBlockStart(start) => {
                        push_output_block(
                            start.content_block,
                            out,
                            &mut events,
                            &mut pending_tool,
                            true,
                            &mut block_has_thinking_summary,
                        )?;
                    }
                    ApiStreamEvent::ContentBlockDelta(delta) => match delta.delta {
                        ContentBlockDelta::TextDelta { text } => {
                            if !text.is_empty() {
                                if let Some(progress_reporter) = &self.progress_reporter {
                                    progress_reporter.mark_text_phase(&text);
                                }
                                if let Some(rendered) = markdown_stream.push(&renderer, &text) {
                                    write!(out, "{rendered}")
                                        .and_then(|()| out.flush())
                                        .map_err(|error| RuntimeError::new(error.to_string()))?;
                                }
                                events.push(AssistantEvent::TextDelta(text));
                            }
                        }
                        ContentBlockDelta::InputJsonDelta { partial_json } => {
                            if let Some((_, _, input)) = &mut pending_tool {
                                input.push_str(&partial_json);
                            }
                        }
                        ContentBlockDelta::ThinkingDelta { .. } => {
                            if !block_has_thinking_summary {
                                render_thinking_block_summary(out, None, false)?;
                                block_has_thinking_summary = true;
                            }
                        }
                        ContentBlockDelta::SignatureDelta { .. } => {}
                    },
                    ApiStreamEvent::ContentBlockStop(_) => {
                        block_has_thinking_summary = false;
                        if let Some(rendered) = markdown_stream.flush(&renderer) {
                            write!(out, "{rendered}")
                                .and_then(|()| out.flush())
                                .map_err(|error| RuntimeError::new(error.to_string()))?;
                        }
                        if let Some((id, name, input)) = pending_tool.take() {
                            if let Some(progress_reporter) = &self.progress_reporter {
                                progress_reporter.mark_tool_phase(&name, &input);
                            }
                            // Display tool call now that input is fully accumulated
                            writeln!(out, "\n{}", format_tool_call_start(&name, &input))
                                .and_then(|()| out.flush())
                                .map_err(|error| RuntimeError::new(error.to_string()))?;
                            events.push(AssistantEvent::ToolUse { id, name, input });
                        }
                    }
                    ApiStreamEvent::MessageDelta(delta) => {
                        events.push(AssistantEvent::Usage(delta.usage.token_usage()));
                    }
                    ApiStreamEvent::MessageStop(_) => {
                        saw_stop = true;
                        if let Some(rendered) = markdown_stream.flush(&renderer) {
                            write!(out, "{rendered}")
                                .and_then(|()| out.flush())
                                .map_err(|error| RuntimeError::new(error.to_string()))?;
                        }
                        events.push(AssistantEvent::MessageStop);
                    }
                }
            }

            push_prompt_cache_record(&self.client, &mut events);

            if !saw_stop
                && events.iter().any(|event| {
                    matches!(event, AssistantEvent::TextDelta(text) if !text.is_empty())
                        || matches!(event, AssistantEvent::ToolUse { .. })
                })
            {
                events.push(AssistantEvent::MessageStop);
            }

            if events
                .iter()
                .any(|event| matches!(event, AssistantEvent::MessageStop))
            {
                return Ok(events);
            }

            let response = self
                .client
                .send_message(&MessageRequest {
                    stream: false,
                    ..message_request.clone()
                })
                .await
                .map_err(|error| {
                    RuntimeError::new(format_user_visible_api_error(&self.session_id, &error))
                })?;
            let mut events = response_to_events(response, out)?;
            push_prompt_cache_record(&self.client, &mut events);
            Ok(events)
        })
    }
}

fn format_user_visible_api_error(session_id: &str, error: &orbit_api::ApiError) -> String {
    if error.is_context_window_failure() {
        format_context_window_blocked_error(session_id, error)
    } else if error.is_generic_fatal_wrapper() {
        let mut qualifiers = vec![format!("session {session_id}")];
        if let Some(request_id) = error.request_id() {
            qualifiers.push(format!("trace {request_id}"));
        }
        format!(
            "{} ({}): {}",
            error.safe_failure_class(),
            qualifiers.join(", "),
            error
        )
    } else {
        error.to_string()
    }
}

fn format_context_window_blocked_error(session_id: &str, error: &orbit_api::ApiError) -> String {
    let mut lines = vec![
        "Context window blocked".to_string(),
        "  Failure class    context_window_blocked".to_string(),
        format!("  Session          {session_id}"),
    ];

    if let Some(request_id) = error.request_id() {
        lines.push(format!("  Trace            {request_id}"));
    }

    match error {
        orbit_api::ApiError::ContextWindowExceeded {
            model,
            estimated_input_tokens,
            requested_output_tokens,
            estimated_total_tokens,
            context_window_tokens,
        } => {
            lines.push(format!("  Model            {model}"));
            lines.push(format!(
                "  Input estimate   ~{estimated_input_tokens} tokens (heuristic)"
            ));
            lines.push(format!(
                "  Requested output {requested_output_tokens} tokens"
            ));
            lines.push(format!(
                "  Total estimate   ~{estimated_total_tokens} tokens (heuristic)"
            ));
            lines.push(format!("  Context window   {context_window_tokens} tokens"));
        }
        orbit_api::ApiError::Api { message, body, .. } => {
            let detail = message.as_deref().unwrap_or(body).trim();
            if !detail.is_empty() {
                lines.push(format!(
                    "  Detail           {}",
                    truncate_for_summary(detail, 120)
                ));
            }
        }
        orbit_api::ApiError::RetriesExhausted { last_error, .. } => {
            let detail = match last_error.as_ref() {
                orbit_api::ApiError::Api { message, body, .. } => {
                    message.as_deref().unwrap_or(body)
                }
                other => return format_context_window_blocked_error(session_id, other),
            }
            .trim();
            if !detail.is_empty() {
                lines.push(format!(
                    "  Detail           {}",
                    truncate_for_summary(detail, 120)
                ));
            }
        }
        _ => {}
    }

    lines.push(String::new());
    lines.push("Recovery".to_string());
    lines.push("  Compact          /compact".to_string());
    lines.push(format!(
        "  Resume compact   orbit --resume {session_id} /compact"
    ));
    lines.push("  Fresh session    /clear --confirm".to_string());
    lines.push(
        "  Reduce scope     remove large pasted context/files or ask for a smaller slice"
            .to_string(),
    );
    lines.push("  Retry            rerun after compacting or reducing the request".to_string());

    lines.join("\n")
}

fn final_assistant_text(summary: &orbit_runtime::TurnSummary) -> String {
    summary
        .assistant_messages
        .last()
        .map(|message| {
            message
                .blocks
                .iter()
                .filter_map(|block| match block {
                    ContentBlock::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default()
}

fn collect_tool_uses(summary: &orbit_runtime::TurnSummary) -> Vec<serde_json::Value> {
    summary
        .assistant_messages
        .iter()
        .flat_map(|message| message.blocks.iter())
        .filter_map(|block| match block {
            ContentBlock::ToolUse { id, name, input } => Some(json!({
                "id": id,
                "name": name,
                "input": input,
            })),
            _ => None,
        })
        .collect()
}

fn collect_tool_results(summary: &orbit_runtime::TurnSummary) -> Vec<serde_json::Value> {
    summary
        .tool_results
        .iter()
        .flat_map(|message| message.blocks.iter())
        .filter_map(|block| match block {
            ContentBlock::ToolResult {
                tool_use_id,
                tool_name,
                output,
                is_error,
            } => Some(json!({
                "tool_use_id": tool_use_id,
                "tool_name": tool_name,
                "output": output,
                "is_error": is_error,
            })),
            _ => None,
        })
        .collect()
}

fn collect_prompt_cache_events(summary: &orbit_runtime::TurnSummary) -> Vec<serde_json::Value> {
    summary
        .prompt_cache_events
        .iter()
        .map(|event| {
            json!({
                "unexpected": event.unexpected,
                "reason": event.reason,
                "previous_cache_read_input_tokens": event.previous_cache_read_input_tokens,
                "current_cache_read_input_tokens": event.current_cache_read_input_tokens,
                "token_drop": event.token_drop,
            })
        })
        .collect()
}

fn slash_command_completion_candidates_with_sessions(
    model: &str,
    active_session_id: Option<&str>,
    recent_session_ids: Vec<String>,
) -> Vec<String> {
    let mut completions = BTreeSet::new();

    for spec in slash_command_specs() {
        completions.insert(format!("/{}", spec.name));
        for alias in spec.aliases {
            completions.insert(format!("/{alias}"));
        }
    }

    for candidate in [
        "/bughunter ",
        "/clear --confirm",
        "/config ",
        "/config env",
        "/config hooks",
        "/config model",
        "/config telemetry",
        "/config plugins",
        "/mcp ",
        "/mcp list",
        "/mcp show ",
        "/export ",
        "/issue ",
        "/ide",
        "/ide vscode",
        "/ide cursor",
        "/ide antigravity",
        "/ide windsurf",
        "/model ",
        "/model opus",
        "/model sonnet",
        "/model haiku",
        "/permissions ",
        "/permissions read-only",
        "/permissions workspace-write",
        "/permissions danger-full-access",
        "/plugin list",
        "/plugin install ",
        "/plugin enable ",
        "/plugin disable ",
        "/plugin uninstall ",
        "/plugin update ",
        "/plugins list",
        "/pr ",
        "/resume ",
        "/session list",
        "/session switch ",
        "/session fork ",
        "/teleport ",
        "/ultraplan ",
        "/agents help",
        "/mcp help",
        "/skills help",
    ] {
        completions.insert(candidate.to_string());
    }

    if !model.trim().is_empty() {
        completions.insert(format!("/model {}", resolve_model_alias(model)));
        completions.insert(format!("/model {model}"));
    }

    if let Some(active_session_id) = active_session_id.filter(|value| !value.trim().is_empty()) {
        completions.insert(format!("/resume {active_session_id}"));
        completions.insert(format!("/session switch {active_session_id}"));
    }

    for session_id in recent_session_ids
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .take(10)
    {
        completions.insert(format!("/resume {session_id}"));
        completions.insert(format!("/session switch {session_id}"));
    }

    completions.into_iter().collect()
}

fn format_tool_call_start(name: &str, input: &str) -> String {
    let parsed: serde_json::Value =
        serde_json::from_str(input).unwrap_or(serde_json::Value::String(input.to_string()));

    let detail = match name {
        "bash" | "Bash" => format_bash_call(&parsed),
        "read_file" | "Read" => {
            let path = extract_tool_path(&parsed);
            format!("\x1b[2m📄 Reading {path}…\x1b[0m")
        }
        "write_file" | "Write" => {
            let path = extract_tool_path(&parsed);
            let lines = parsed
                .get("content")
                .and_then(|value| value.as_str())
                .map_or(0, |content| content.lines().count());
            format!("\x1b[1;32m✏️ Writing {path}\x1b[0m \x1b[2m({lines} lines)\x1b[0m")
        }
        "edit_file" | "Edit" => {
            let path = extract_tool_path(&parsed);
            let old_value = parsed
                .get("old_string")
                .or_else(|| parsed.get("oldString"))
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            let new_value = parsed
                .get("new_string")
                .or_else(|| parsed.get("newString"))
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            format!(
                "\x1b[1;33m📝 Editing {path}\x1b[0m{}",
                format_patch_preview(old_value, new_value)
                    .map(|preview| format!("\n{preview}"))
                    .unwrap_or_default()
            )
        }
        "glob_search" | "Glob" => format_search_start("🔎 Glob", &parsed),
        "grep_search" | "Grep" => format_search_start("🔎 Grep", &parsed),
        "web_search" | "WebSearch" => parsed
            .get("query")
            .and_then(|value| value.as_str())
            .unwrap_or("?")
            .to_string(),
        _ => summarize_tool_payload(input),
    };

    let border = "─".repeat(name.len() + 8);
    format!(
        "\x1b[38;5;245m╭─ \x1b[1;36m{name}\x1b[0;38;5;245m ─╮\x1b[0m\n\x1b[38;5;245m│\x1b[0m {detail}\n\x1b[38;5;245m╰{border}╯\x1b[0m"
    )
}

fn format_tool_result(name: &str, output: &str, is_error: bool) -> String {
    let icon = if is_error {
        "\x1b[1;31m✗\x1b[0m"
    } else {
        "\x1b[1;32m✓\x1b[0m"
    };
    if is_error {
        let summary = truncate_for_summary(output.trim(), 160);
        return if summary.is_empty() {
            format!("{icon} \x1b[38;5;245m{name}\x1b[0m")
        } else {
            format!("{icon} \x1b[38;5;245m{name}\x1b[0m\n\x1b[38;5;203m{summary}\x1b[0m")
        };
    }

    let parsed: serde_json::Value =
        serde_json::from_str(output).unwrap_or(serde_json::Value::String(output.to_string()));
    match name {
        "bash" | "Bash" => format_bash_result(icon, &parsed),
        "read_file" | "Read" => format_read_result(icon, &parsed),
        "write_file" | "Write" => format_write_result(icon, &parsed),
        "edit_file" | "Edit" => format_edit_result(icon, &parsed),
        "glob_search" | "Glob" => format_glob_result(icon, &parsed),
        "grep_search" | "Grep" => format_grep_result(icon, &parsed),
        _ => format_generic_tool_result(icon, name, &parsed),
    }
}

const DISPLAY_TRUNCATION_NOTICE: &str =
    "\x1b[2m… output truncated for display; full result preserved in session.\x1b[0m";
const READ_DISPLAY_MAX_LINES: usize = 80;
const READ_DISPLAY_MAX_CHARS: usize = 6_000;
const TOOL_OUTPUT_DISPLAY_MAX_LINES: usize = 60;
const TOOL_OUTPUT_DISPLAY_MAX_CHARS: usize = 4_000;

fn extract_tool_path(parsed: &serde_json::Value) -> String {
    parsed
        .get("file_path")
        .or_else(|| parsed.get("filePath"))
        .or_else(|| parsed.get("path"))
        .and_then(|value| value.as_str())
        .unwrap_or("?")
        .to_string()
}

fn format_search_start(label: &str, parsed: &serde_json::Value) -> String {
    let pattern = parsed
        .get("pattern")
        .and_then(|value| value.as_str())
        .unwrap_or("?");
    let scope = parsed
        .get("path")
        .and_then(|value| value.as_str())
        .unwrap_or(".");
    format!("{label} {pattern}\n\x1b[2min {scope}\x1b[0m")
}

fn format_patch_preview(old_value: &str, new_value: &str) -> Option<String> {
    if old_value.is_empty() && new_value.is_empty() {
        return None;
    }
    Some(format!(
        "\x1b[38;5;203m- {}\x1b[0m\n\x1b[38;5;70m+ {}\x1b[0m",
        truncate_for_summary(first_visible_line(old_value), 72),
        truncate_for_summary(first_visible_line(new_value), 72)
    ))
}

fn format_bash_call(parsed: &serde_json::Value) -> String {
    let command = parsed
        .get("command")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    if command.is_empty() {
        String::new()
    } else {
        format!(
            "\x1b[48;5;236;38;5;255m $ {} \x1b[0m",
            truncate_for_summary(command, 160)
        )
    }
}

fn first_visible_line(text: &str) -> &str {
    text.lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or(text)
}

fn format_bash_result(icon: &str, parsed: &serde_json::Value) -> String {
    use std::fmt::Write as _;

    let mut lines = vec![format!("{icon} \x1b[38;5;245mbash\x1b[0m")];
    if let Some(task_id) = parsed
        .get("backgroundTaskId")
        .and_then(|value| value.as_str())
    {
        write!(&mut lines[0], " backgrounded ({task_id})").expect("write to string");
    } else if let Some(status) = parsed
        .get("returnCodeInterpretation")
        .and_then(|value| value.as_str())
        .filter(|status| !status.is_empty())
    {
        write!(&mut lines[0], " {status}").expect("write to string");
    }

    if let Some(stdout) = parsed.get("stdout").and_then(|value| value.as_str()) {
        if !stdout.trim().is_empty() {
            lines.push(truncate_output_for_display(
                stdout,
                TOOL_OUTPUT_DISPLAY_MAX_LINES,
                TOOL_OUTPUT_DISPLAY_MAX_CHARS,
            ));
        }
    }
    if let Some(stderr) = parsed.get("stderr").and_then(|value| value.as_str()) {
        if !stderr.trim().is_empty() {
            lines.push(format!(
                "\x1b[38;5;203m{}\x1b[0m",
                truncate_output_for_display(
                    stderr,
                    TOOL_OUTPUT_DISPLAY_MAX_LINES,
                    TOOL_OUTPUT_DISPLAY_MAX_CHARS,
                )
            ));
        }
    }

    lines.join("\n\n")
}

fn format_read_result(icon: &str, parsed: &serde_json::Value) -> String {
    let file = parsed.get("file").unwrap_or(parsed);
    let path = extract_tool_path(file);
    let start_line = file
        .get("startLine")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(1);
    let num_lines = file
        .get("numLines")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let total_lines = file
        .get("totalLines")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(num_lines);
    let content = file
        .get("content")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let end_line = start_line.saturating_add(num_lines.saturating_sub(1));

    format!(
        "{icon} \x1b[2m📄 Read {path} (lines {}-{} of {})\x1b[0m\n{}",
        start_line,
        end_line.max(start_line),
        total_lines,
        truncate_output_for_display(content, READ_DISPLAY_MAX_LINES, READ_DISPLAY_MAX_CHARS)
    )
}

fn format_write_result(icon: &str, parsed: &serde_json::Value) -> String {
    let path = extract_tool_path(parsed);
    let kind = parsed
        .get("type")
        .and_then(|value| value.as_str())
        .unwrap_or("write");
    let line_count = parsed
        .get("content")
        .and_then(|value| value.as_str())
        .map_or(0, |content| content.lines().count());
    format!(
        "{icon} \x1b[1;32m✏️ {} {path}\x1b[0m \x1b[2m({line_count} lines)\x1b[0m",
        if kind == "create" { "Wrote" } else { "Updated" },
    )
}

fn format_structured_patch_preview(parsed: &serde_json::Value) -> Option<String> {
    let hunks = parsed.get("structuredPatch")?.as_array()?;
    let mut preview = Vec::new();
    for hunk in hunks.iter().take(2) {
        let lines = hunk.get("lines")?.as_array()?;
        for line in lines.iter().filter_map(|value| value.as_str()).take(6) {
            match line.chars().next() {
                Some('+') => preview.push(format!("\x1b[38;5;70m{line}\x1b[0m")),
                Some('-') => preview.push(format!("\x1b[38;5;203m{line}\x1b[0m")),
                _ => preview.push(line.to_string()),
            }
        }
    }
    if preview.is_empty() {
        None
    } else {
        Some(preview.join("\n"))
    }
}

fn format_edit_result(icon: &str, parsed: &serde_json::Value) -> String {
    let path = extract_tool_path(parsed);
    let suffix = if parsed
        .get("replaceAll")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        " (replace all)"
    } else {
        ""
    };
    let preview = format_structured_patch_preview(parsed).or_else(|| {
        let old_value = parsed
            .get("oldString")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        let new_value = parsed
            .get("newString")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        format_patch_preview(old_value, new_value)
    });

    match preview {
        Some(preview) => format!("{icon} \x1b[1;33m📝 Edited {path}{suffix}\x1b[0m\n{preview}"),
        None => format!("{icon} \x1b[1;33m📝 Edited {path}{suffix}\x1b[0m"),
    }
}

fn format_glob_result(icon: &str, parsed: &serde_json::Value) -> String {
    let num_files = parsed
        .get("numFiles")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let filenames = parsed
        .get("filenames")
        .and_then(|value| value.as_array())
        .map(|files| {
            files
                .iter()
                .filter_map(|value| value.as_str())
                .take(8)
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();
    if filenames.is_empty() {
        format!("{icon} \x1b[38;5;245mglob_search\x1b[0m matched {num_files} files")
    } else {
        format!("{icon} \x1b[38;5;245mglob_search\x1b[0m matched {num_files} files\n{filenames}")
    }
}

fn format_grep_result(icon: &str, parsed: &serde_json::Value) -> String {
    let num_matches = parsed
        .get("numMatches")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let num_files = parsed
        .get("numFiles")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let content = parsed
        .get("content")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let filenames = parsed
        .get("filenames")
        .and_then(|value| value.as_array())
        .map(|files| {
            files
                .iter()
                .filter_map(|value| value.as_str())
                .take(8)
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();
    let summary = format!(
        "{icon} \x1b[38;5;245mgrep_search\x1b[0m {num_matches} matches across {num_files} files"
    );
    if !content.trim().is_empty() {
        format!(
            "{summary}\n{}",
            truncate_output_for_display(
                content,
                TOOL_OUTPUT_DISPLAY_MAX_LINES,
                TOOL_OUTPUT_DISPLAY_MAX_CHARS,
            )
        )
    } else if !filenames.is_empty() {
        format!("{summary}\n{filenames}")
    } else {
        summary
    }
}

fn format_generic_tool_result(icon: &str, name: &str, parsed: &serde_json::Value) -> String {
    let rendered_output = match parsed {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Null => String::new(),
        serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
            serde_json::to_string_pretty(parsed).unwrap_or_else(|_| parsed.to_string())
        }
        _ => parsed.to_string(),
    };
    let preview = truncate_output_for_display(
        &rendered_output,
        TOOL_OUTPUT_DISPLAY_MAX_LINES,
        TOOL_OUTPUT_DISPLAY_MAX_CHARS,
    );

    if preview.is_empty() {
        format!("{icon} \x1b[38;5;245m{name}\x1b[0m")
    } else if preview.contains('\n') {
        format!("{icon} \x1b[38;5;245m{name}\x1b[0m\n{preview}")
    } else {
        format!("{icon} \x1b[38;5;245m{name}:\x1b[0m {preview}")
    }
}

fn summarize_tool_payload(payload: &str) -> String {
    let compact = match serde_json::from_str::<serde_json::Value>(payload) {
        Ok(value) => value.to_string(),
        Err(_) => payload.trim().to_string(),
    };
    truncate_for_summary(&compact, 96)
}

fn truncate_for_summary(value: &str, limit: usize) -> String {
    let mut chars = value.chars();
    let truncated = chars.by_ref().take(limit).collect::<String>();
    if chars.next().is_some() {
        format!("{truncated}…")
    } else {
        truncated
    }
}

fn truncate_output_for_display(content: &str, max_lines: usize, max_chars: usize) -> String {
    let original = content.trim_end_matches('\n');
    if original.is_empty() {
        return String::new();
    }

    let mut preview_lines = Vec::new();
    let mut used_chars = 0usize;
    let mut truncated = false;

    for (index, line) in original.lines().enumerate() {
        if index >= max_lines {
            truncated = true;
            break;
        }

        let newline_cost = usize::from(!preview_lines.is_empty());
        let available = max_chars.saturating_sub(used_chars + newline_cost);
        if available == 0 {
            truncated = true;
            break;
        }

        let line_chars = line.chars().count();
        if line_chars > available {
            preview_lines.push(line.chars().take(available).collect::<String>());
            truncated = true;
            break;
        }

        preview_lines.push(line.to_string());
        used_chars += newline_cost + line_chars;
    }

    let mut preview = preview_lines.join("\n");
    if truncated {
        if !preview.is_empty() {
            preview.push('\n');
        }
        preview.push_str(DISPLAY_TRUNCATION_NOTICE);
    }
    preview
}

fn render_thinking_block_summary(
    out: &mut (impl Write + ?Sized),
    char_count: Option<usize>,
    redacted: bool,
) -> Result<(), RuntimeError> {
    let summary = if redacted {
        "\n▶ Thinking block hidden by provider\n".to_string()
    } else if let Some(char_count) = char_count {
        format!("\n▶ Thinking ({char_count} chars hidden)\n")
    } else {
        "\n▶ Thinking hidden\n".to_string()
    };
    write!(out, "{summary}")
        .and_then(|()| out.flush())
        .map_err(|error| RuntimeError::new(error.to_string()))
}

fn push_output_block(
    block: OutputContentBlock,
    out: &mut (impl Write + ?Sized),
    events: &mut Vec<AssistantEvent>,
    pending_tool: &mut Option<(String, String, String)>,
    streaming_tool_input: bool,
    block_has_thinking_summary: &mut bool,
) -> Result<(), RuntimeError> {
    match block {
        OutputContentBlock::Text { text } => {
            if !text.is_empty() {
                let rendered = TerminalRenderer::new().markdown_to_ansi(&text);
                write!(out, "{rendered}")
                    .and_then(|()| out.flush())
                    .map_err(|error| RuntimeError::new(error.to_string()))?;
                events.push(AssistantEvent::TextDelta(text));
            }
        }
        OutputContentBlock::ToolUse { id, name, input } => {
            // During streaming, the initial content_block_start has an empty input ({}).
            // The real input arrives via input_json_delta events. In
            // non-streaming responses, preserve a legitimate empty object.
            let initial_input = if streaming_tool_input
                && input.is_object()
                && input.as_object().is_some_and(serde_json::Map::is_empty)
            {
                String::new()
            } else {
                input.to_string()
            };
            *pending_tool = Some((id, name, initial_input));
        }
        OutputContentBlock::Thinking { thinking, .. } => {
            render_thinking_block_summary(out, Some(thinking.chars().count()), false)?;
            *block_has_thinking_summary = true;
        }
        OutputContentBlock::RedactedThinking { .. } => {
            render_thinking_block_summary(out, None, true)?;
            *block_has_thinking_summary = true;
        }
    }
    Ok(())
}

fn response_to_events(
    response: MessageResponse,
    out: &mut (impl Write + ?Sized),
) -> Result<Vec<AssistantEvent>, RuntimeError> {
    let mut events = Vec::new();
    let mut pending_tool = None;

    for block in response.content {
        let mut block_has_thinking_summary = false;
        push_output_block(
            block,
            out,
            &mut events,
            &mut pending_tool,
            false,
            &mut block_has_thinking_summary,
        )?;
        if let Some((id, name, input)) = pending_tool.take() {
            events.push(AssistantEvent::ToolUse { id, name, input });
        }
    }

    events.push(AssistantEvent::Usage(response.usage.token_usage()));
    events.push(AssistantEvent::MessageStop);
    Ok(events)
}

fn push_prompt_cache_record(client: &AnthropicClient, events: &mut Vec<AssistantEvent>) {
    if let Some(record) = client.take_last_prompt_cache_record() {
        if let Some(event) = prompt_cache_record_to_runtime_event(record) {
            events.push(AssistantEvent::PromptCache(event));
        }
    }
}

fn push_prompt_cache_record_for_provider(
    client: &ProviderClient,
    events: &mut Vec<AssistantEvent>,
) {
    if let Some(record) = client.take_last_prompt_cache_record() {
        if let Some(event) = prompt_cache_record_to_runtime_event(record) {
            events.push(AssistantEvent::PromptCache(event));
        }
    }
}

fn prompt_cache_record_to_runtime_event(
    record: orbit_api::PromptCacheRecord,
) -> Option<PromptCacheEvent> {
    let cache_break = record.cache_break?;
    Some(PromptCacheEvent {
        unexpected: cache_break.unexpected,
        reason: cache_break.reason,
        previous_cache_read_input_tokens: cache_break.previous_cache_read_input_tokens,
        current_cache_read_input_tokens: cache_break.current_cache_read_input_tokens,
        token_drop: cache_break.token_drop,
    })
}

struct CliToolExecutor {
    renderer: TerminalRenderer,
    emit_output: bool,
    allowed_tools: Option<AllowedToolSet>,
    tool_registry: GlobalToolRegistry,
    mcp_state: Option<Arc<Mutex<RuntimeMcpState>>>,
    scope: ToolExecutionScope,
}

impl CliToolExecutor {
    fn new(
        session_id: String,
        allowed_tools: Option<AllowedToolSet>,
        emit_output: bool,
        tool_registry: GlobalToolRegistry,
        mcp_state: Option<Arc<Mutex<RuntimeMcpState>>>,
    ) -> Self {
        Self {
            renderer: TerminalRenderer::new(),
            emit_output,
            allowed_tools,
            tool_registry,
            mcp_state,
            scope: ToolExecutionScope::for_session(session_id),
        }
    }

    fn execute_search_tool(&self, value: serde_json::Value) -> Result<String, ToolError> {
        let input: ToolSearchRequest = serde_json::from_value(value)
            .map_err(|error| ToolError::new(format!("invalid tool input JSON: {error}")))?;
        let (pending_mcp_servers, mcp_degraded) =
            self.mcp_state.as_ref().map_or((None, None), |state| {
                let state = state
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                (state.pending_servers(), state.degraded_report())
            });
        serde_json::to_string_pretty(&self.tool_registry.search(
            &input.query,
            input.max_results.unwrap_or(5),
            pending_mcp_servers,
            mcp_degraded,
        ))
        .map_err(|error| ToolError::new(error.to_string()))
    }

    fn execute_runtime_tool(
        &self,
        tool_name: &str,
        value: serde_json::Value,
    ) -> Result<String, ToolError> {
        let Some(mcp_state) = &self.mcp_state else {
            return Err(ToolError::new(format!(
                "runtime tool `{tool_name}` is unavailable without configured MCP servers"
            )));
        };
        let mut mcp_state = mcp_state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        match tool_name {
            "MCPTool" => {
                let input: McpToolRequest = serde_json::from_value(value)
                    .map_err(|error| ToolError::new(format!("invalid tool input JSON: {error}")))?;
                let qualified_name = input
                    .qualified_name
                    .or(input.tool)
                    .ok_or_else(|| ToolError::new("missing required field `qualifiedName`"))?;
                mcp_state.call_tool(&qualified_name, input.arguments)
            }
            "ListMcpResourcesTool" => {
                let input: ListMcpResourcesRequest = serde_json::from_value(value)
                    .map_err(|error| ToolError::new(format!("invalid tool input JSON: {error}")))?;
                match input.server {
                    Some(server_name) => mcp_state.list_resources_for_server(&server_name),
                    None => mcp_state.list_resources_for_all_servers(),
                }
            }
            "ReadMcpResourceTool" => {
                let input: ReadMcpResourceRequest = serde_json::from_value(value)
                    .map_err(|error| ToolError::new(format!("invalid tool input JSON: {error}")))?;
                mcp_state.read_resource(&input.server, &input.uri)
            }
            _ => mcp_state.call_tool(tool_name, Some(value)),
        }
    }
}

impl ToolExecutor for CliToolExecutor {
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, ToolError> {
        if self
            .allowed_tools
            .as_ref()
            .is_some_and(|allowed| !allowed.contains(tool_name))
        {
            return Err(ToolError::new(format!(
                "tool `{tool_name}` is not enabled by the current --allowedTools setting"
            )));
        }
        let value = serde_json::from_str(input)
            .map_err(|error| ToolError::new(format!("invalid tool input JSON: {error}")))?;
        let result = if tool_name == "ToolSearch" {
            self.execute_search_tool(value)
        } else if self.tool_registry.has_runtime_tool(tool_name) {
            self.execute_runtime_tool(tool_name, value)
        } else {
            self.tool_registry
                .execute_scoped(tool_name, &value, &self.scope)
                .map_err(ToolError::new)
        };
        match result {
            Ok(output) => {
                if self.emit_output {
                    let markdown = format_tool_result(tool_name, &output, false);
                    self.renderer
                        .stream_markdown(&markdown, &mut io::stdout())
                        .map_err(|error| ToolError::new(error.to_string()))?;
                }
                Ok(output)
            }
            Err(error) => {
                if self.emit_output {
                    let markdown = format_tool_result(tool_name, &error.to_string(), true);
                    self.renderer
                        .stream_markdown(&markdown, &mut io::stdout())
                        .map_err(|stream_error| ToolError::new(stream_error.to_string()))?;
                }
                Err(error)
            }
        }
    }
}

fn permission_policy(
    mode: PermissionMode,
    feature_config: &orbit_runtime::RuntimeFeatureConfig,
    tool_registry: &GlobalToolRegistry,
) -> Result<PermissionPolicy, String> {
    Ok(tool_registry.permission_specs(None)?.into_iter().fold(
        PermissionPolicy::new(mode).with_permission_rules(feature_config.permission_rules()),
        |policy, (name, required_permission)| {
            policy.with_tool_requirement(name, required_permission)
        },
    ))
}

fn convert_messages(messages: &[ConversationMessage]) -> Vec<InputMessage> {
    messages
        .iter()
        .filter_map(|message| {
            let role = match message.role {
                MessageRole::System | MessageRole::User | MessageRole::Tool => "user",
                MessageRole::Assistant => "assistant",
            };
            let content = message
                .blocks
                .iter()
                .map(|block| match block {
                    ContentBlock::Text { text } => InputContentBlock::Text { text: text.clone() },
                    ContentBlock::ToolUse { id, name, input } => InputContentBlock::ToolUse {
                        id: id.clone(),
                        name: name.clone(),
                        input: serde_json::from_str(input)
                            .unwrap_or_else(|_| serde_json::json!({ "raw": input })),
                    },
                    ContentBlock::ToolResult {
                        tool_use_id,
                        output,
                        is_error,
                        ..
                    } => InputContentBlock::ToolResult {
                        tool_use_id: tool_use_id.clone(),
                        content: vec![ToolResultContentBlock::Text {
                            text: output.clone(),
                        }],
                        is_error: *is_error,
                    },
                })
                .collect::<Vec<_>>();
            (!content.is_empty()).then(|| InputMessage {
                role: role.to_string(),
                content,
            })
        })
        .collect()
}

#[allow(clippy::too_many_lines)]
fn print_help_to(out: &mut impl Write) -> io::Result<()> {
    writeln!(out, "orbit v{VERSION}")?;
    writeln!(out)?;
    writeln!(out, "Usage:")?;
    writeln!(
        out,
        "  orbit [--model MODEL] [--allowedTools TOOL[,TOOL...]]"
    )?;
    writeln!(out, "      Start the interactive REPL")?;
    writeln!(
        out,
        "  orbit [--model MODEL] [--output-format text|json] prompt TEXT"
    )?;
    writeln!(out, "      Send one prompt and exit")?;
    writeln!(
        out,
        "  orbit [--model MODEL] [--output-format text|json] TEXT"
    )?;
    writeln!(out, "      Shorthand non-interactive prompt mode")?;
    writeln!(
        out,
        "  orbit --resume [SESSION.jsonl|session-id|latest] [/status] [/compact] [...]"
    )?;
    writeln!(
        out,
        "      Inspect or maintain a saved session without entering the REPL"
    )?;
    writeln!(out, "  orbit help")?;
    writeln!(out, "      Alias for --help")?;
    writeln!(out, "  orbit version")?;
    writeln!(out, "      Alias for --version")?;
    writeln!(out, "  orbit status")?;
    writeln!(
        out,
        "      Show the current local workspace status snapshot"
    )?;
    writeln!(out, "  orbit config {CONFIG_SECTION_ARGUMENT_HINT}")?;
    writeln!(
        out,
        "      Inspect merged config sections with text or JSON output"
    )?;
    writeln!(out, "  orbit sandbox")?;
    writeln!(out, "      Show the current sandbox isolation snapshot")?;
    writeln!(out, "  orbit doctor")?;
    writeln!(
        out,
        "      Diagnose local auth, config, workspace, and sandbox health"
    )?;
    writeln!(
        out,
        "  orbit hosted policy orphans [--repository REPO] [--source SOURCE] [--priority PRIORITY]"
    )?;
    writeln!(
        out,
        "      Preview the hosted orphan policy that would apply to a task shape"
    )?;
    writeln!(
        out,
        "  orbit hosted events watch [--task-id TASK_ID] [--topic TOPIC] [--event EVENT] [--status STATUS[,STATUS...]] [--limit N]"
    )?;
    writeln!(
        out,
        "      Stream hosted control-plane events over WebSocket with client-side filters"
    )?;
    writeln!(
        out,
        "  orbit hosted tasks list [--status STATUS[,STATUS...]] [--source SOURCE] [--repository REPO] [--channel-id ID] [--thread-ts TS] [--needs-followup] [--limit N]"
    )?;
    writeln!(out, "      List hosted tasks with server-side filtering")?;
    writeln!(
        out,
        "  orbit hosted tasks watch [--status STATUS[,STATUS...]] [--source SOURCE] [--repository REPO] [--channel-id ID] [--thread-ts TS] [--limit N]"
    )?;
    writeln!(
        out,
        "      Watch live task updates for tasks matching the current filter"
    )?;
    writeln!(out, "  orbit hosted task get TASK_ID")?;
    writeln!(out, "      Inspect the current hosted task snapshot")?;
    writeln!(out, "  orbit hosted task runtime TASK_ID")?;
    writeln!(
        out,
        "      Inspect hosted worker runtime and orphan classification"
    )?;
    writeln!(out, "  orbit hosted task reconcile TASK_ID")?;
    writeln!(
        out,
        "      Reconcile a hosted task against persisted worker artifacts"
    )?;
    writeln!(out, "  orbit hosted task cancel TASK_ID")?;
    writeln!(
        out,
        "      Cancel a hosted task directly through the control plane"
    )?;
    writeln!(
        out,
        "  orbit hosted task approval TASK_ID [retry|cancel|ack] [--kind orphaned_hosted_agent|github_review_followup] [--resolved-by NAME] [--reason TEXT]"
    )?;
    writeln!(
        out,
        "      Resolve a hosted task approval (orphaned agent or GitHub follow-up) through the control plane"
    )?;
    writeln!(out, "  orbit dump-manifests")?;
    writeln!(out, "  orbit bootstrap-plan")?;
    writeln!(out, "  orbit agents")?;
    writeln!(out, "  orbit mcp")?;
    writeln!(out, "  orbit skills")?;
    writeln!(
        out,
        "  orbit system-prompt [--cwd PATH] [--date YYYY-MM-DD]"
    )?;
    writeln!(out, "  orbit init")?;
    writeln!(out)?;
    writeln!(out, "Flags:")?;
    writeln!(
        out,
        "  --model MODEL              Override the active model"
    )?;
    writeln!(
        out,
        "  --provider PROVIDER        Force AI provider (anthropic, openai, xai, frontal, bedrock, azure, ollama)"
    )?;
    writeln!(
        out,
        "  --output-format FORMAT     Non-interactive output format: text or json"
    )?;
    writeln!(
        out,
        "  --permission-mode MODE     Set read-only, workspace-write, or danger-full-access"
    )?;
    writeln!(
        out,
        "  --dangerously-skip-permissions  Skip all permission checks"
    )?;
    writeln!(out, "  --allowedTools TOOLS       Restrict enabled tools (repeatable; comma-separated aliases supported)")?;
    writeln!(
        out,
        "  --version, -V              Print version and build information locally"
    )?;
    writeln!(out)?;
    writeln!(out, "Interactive slash commands:")?;
    writeln!(out, "{}", render_slash_command_help())?;
    writeln!(out)?;
    let resume_commands = resume_supported_slash_commands()
        .into_iter()
        .map(|spec| match spec.argument_hint {
            Some(argument_hint) => format!("/{} {}", spec.name, argument_hint),
            None => format!("/{}", spec.name),
        })
        .collect::<Vec<_>>()
        .join(", ");
    writeln!(out, "Resume-safe commands: {resume_commands}")?;
    writeln!(out)?;
    writeln!(out, "Session shortcuts:")?;
    writeln!(
        out,
        "  REPL turns auto-save to .orbit/sessions/<session-id>.{PRIMARY_SESSION_EXTENSION}"
    )?;
    writeln!(
        out,
        "  Use `{LATEST_SESSION_REFERENCE}` with --resume, /resume, or /session switch to target the newest saved session"
    )?;
    writeln!(
        out,
        "  Use /session list in the REPL to browse managed sessions"
    )?;
    writeln!(out, "Examples:")?;
    writeln!(out, "  orbit --model claude-opus \"summarize this repo\"")?;
    writeln!(
        out,
        "  orbit --output-format json prompt \"explain src/main.rs\""
    )?;
    writeln!(
        out,
        "  orbit --allowedTools read,glob \"summarize Cargo.toml\""
    )?;
    writeln!(out, "  orbit --resume {LATEST_SESSION_REFERENCE}")?;
    writeln!(
        out,
        "  orbit --resume {LATEST_SESSION_REFERENCE} /status /diff /export notes.txt"
    )?;
    writeln!(out, "  orbit --output-format json config telemetry")?;
    writeln!(
        out,
        "  orbit hosted policy orphans --repo myorg/myapp --source slack"
    )?;
    writeln!(
        out,
        "  orbit hosted events watch --task-id task_123 --limit 20"
    )?;
    writeln!(
        out,
        "  orbit hosted tasks watch --status pending,running --source slack --limit 20"
    )?;
    writeln!(
        out,
        "  orbit hosted tasks list --status pending,running --source slack --limit 10"
    )?;
    writeln!(out, "  orbit hosted task get task_123")?;
    writeln!(out, "  orbit hosted task runtime task_123")?;
    writeln!(out, "  orbit hosted task reconcile task_123")?;
    writeln!(out, "  orbit hosted task cancel task_123")?;
    writeln!(
        out,
        "  orbit hosted task approval task_123 retry --resolved-by operator"
    )?;
    writeln!(out, "  orbit agents")?;
    writeln!(out, "  orbit mcp show my-server")?;
    writeln!(out, "  orbit /skills")?;
    writeln!(out, "  orbit doctor")?;
    writeln!(out, "  orbit init")?;
    Ok(())
}

fn print_help(output_format: CliOutputFormat) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = Vec::new();
    print_help_to(&mut buffer)?;
    let message = String::from_utf8(buffer)?;
    match output_format {
        CliOutputFormat::Text => print!("{message}"),
        CliOutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "kind": "help",
                "message": message,
            }))?
        ),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        augment_hosted_result_with_publication, build_runtime_plugin_state_with_loader,
        build_runtime_with_plugin_state, config_json_value, create_managed_session_handle,
        default_hosted_pr_draft, describe_tool_progress, filter_tool_specs,
        format_bughunter_report, format_commit_preflight_report, format_commit_skipped_report,
        format_compact_report, format_cost_report, format_internal_prompt_progress_line,
        format_issue_report, format_model_report, format_model_switch_report,
        format_permissions_report, format_permissions_switch_report, format_pr_report,
        format_resume_report, format_status_report, format_tool_call_start, format_tool_result,
        format_ultraplan_report, format_unknown_slash_command,
        format_unknown_slash_command_message, format_user_visible_api_error, hosted_server_url,
        hosted_task_snapshot_from_event, load_hosted_task_worker_payload,
        normalize_permission_mode, parse_args, parse_git_status_branch,
        parse_git_status_metadata_for, parse_git_workspace_summary, permission_policy,
        print_help_to, publish_hosted_repo_changes, push_output_block, render_config_report,
        render_diff_report, render_diff_report_for, render_memory_report, render_repl_help,
        render_resume_usage, render_telemetry_report, report_row, resolve_model_alias,
        resolve_session_reference, resolve_telemetry_config, response_to_events,
        resume_supported_slash_commands, run_resume_command,
        slash_command_completion_candidates_with_sessions, status_context,
        telemetry_status_json_value, update_project_telemetry_settings, validate_no_args,
        write_mcp_server_fixture, CliAction, CliOutputFormat, CliToolExecutor, EventEnvelope,
        GitWorkspaceSummary, HostedApprovalAction, HostedCommand, HostedEventName,
        HostedEventStatus, HostedEventTopic, HostedEventWatchQuery, HostedTaskGithubResponse,
        HostedTaskListQuery, HostedTaskWorkerPayload, InternalPromptProgressEvent,
        InternalPromptProgressState, LiveCli, LocalHelpTopic, SlashCommand, StatusUsage,
        DEFAULT_MODEL, ORBIT_TELEMETRY_PATH,
    };
    use orbit_api::{ApiError, MessageResponse, OutputContentBlock, Usage};
    use orbit_events::EventIdentifiers;
    use orbit_plugins::{
        PluginManager, PluginManagerConfig, PluginTool, PluginToolDefinition, PluginToolPermission,
    };
    use orbit_runtime::{
        AssistantEvent, ConfigLoader, ContentBlock, ConversationMessage, MessageRole,
        PermissionMode, Session, ToolExecutor,
    };
    use orbit_tools::GlobalToolRegistry;
    use serde_json::json;
    use std::fs;
    use std::io::{Read, Write};
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::{Mutex, MutexGuard, OnceLock};
    use std::thread;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    fn registry_with_plugin_tool() -> GlobalToolRegistry {
        GlobalToolRegistry::with_plugin_tools(vec![PluginTool::new(
            "plugin-demo@external",
            "plugin-demo",
            PluginToolDefinition {
                name: "plugin_echo".to_string(),
                description: Some("Echo plugin payload".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "message": { "type": "string" }
                    },
                    "required": ["message"],
                    "additionalProperties": false
                }),
            },
            "echo".to_string(),
            Vec::new(),
            PluginToolPermission::WorkspaceWrite,
            None,
        )])
        .expect("plugin tool registry should build")
    }

    #[test]
    fn opaque_provider_wrapper_surfaces_failure_class_session_and_trace() {
        let error = ApiError::Api {
            status: "500".parse().expect("status"),
            error_type: Some("api_error".to_string()),
            message: Some(
                "Something went wrong while processing your request. Please try again, or use /new to start a fresh session."
                    .to_string(),
            ),
            request_id: Some("req_jobdori_789".to_string()),
            body: String::new(),
            retryable: true,
        };

        let rendered = format_user_visible_api_error("session-issue-22", &error);
        assert!(rendered.contains("provider_internal"));
        assert!(rendered.contains("session session-issue-22"));
        assert!(rendered.contains("trace req_jobdori_789"));
    }

    #[test]
    fn retry_exhaustion_uses_retry_failure_class_for_generic_provider_wrapper() {
        let error = ApiError::RetriesExhausted {
            attempts: 3,
            last_error: Box::new(ApiError::Api {
                status: "502".parse().expect("status"),
                error_type: Some("api_error".to_string()),
                message: Some(
                    "Something went wrong while processing your request. Please try again, or use /new to start a fresh session."
                        .to_string(),
                ),
                request_id: Some("req_jobdori_790".to_string()),
                body: String::new(),
                retryable: true,
            }),
        };

        let rendered = format_user_visible_api_error("session-issue-22", &error);
        assert!(rendered.contains("provider_retry_exhausted"), "{rendered}");
        assert!(rendered.contains("session session-issue-22"));
        assert!(rendered.contains("trace req_jobdori_790"));
    }

    #[test]
    fn context_window_preflight_errors_render_recovery_steps() {
        let error = ApiError::ContextWindowExceeded {
            model: "claude-sonnet-4-6".to_string(),
            estimated_input_tokens: 182_000,
            requested_output_tokens: 64_000,
            estimated_total_tokens: 246_000,
            context_window_tokens: 200_000,
        };

        let rendered = format_user_visible_api_error("session-issue-32", &error);
        assert!(rendered.contains("Context window blocked"), "{rendered}");
        assert!(rendered.contains("context_window_blocked"), "{rendered}");
        assert!(
            rendered.contains("Session          session-issue-32"),
            "{rendered}"
        );
        assert!(
            rendered.contains("Model            claude-sonnet-4-6"),
            "{rendered}"
        );
        assert!(
            rendered.contains("Input estimate   ~182000 tokens (heuristic)"),
            "{rendered}"
        );
        assert!(
            rendered.contains("Total estimate   ~246000 tokens (heuristic)"),
            "{rendered}"
        );
        assert!(rendered.contains("Compact          /compact"), "{rendered}");
        assert!(
            rendered.contains("Resume compact   orbit --resume session-issue-32 /compact"),
            "{rendered}"
        );
        assert!(
            rendered.contains("Fresh session    /clear --confirm"),
            "{rendered}"
        );
        assert!(rendered.contains("Reduce scope"), "{rendered}");
        assert!(rendered.contains("Retry            rerun"), "{rendered}");
    }

    #[test]
    fn provider_context_window_errors_are_reframed_with_same_guidance() {
        let error = ApiError::Api {
            status: "400".parse().expect("status"),
            error_type: Some("invalid_request_error".to_string()),
            message: Some(
                "This model's maximum context length is 200000 tokens, but your request used 230000 tokens."
                    .to_string(),
            ),
            request_id: Some("req_ctx_456".to_string()),
            body: String::new(),
            retryable: false,
        };

        let rendered = format_user_visible_api_error("session-issue-32", &error);
        assert!(rendered.contains("context_window_blocked"), "{rendered}");
        assert!(
            rendered.contains("Trace            req_ctx_456"),
            "{rendered}"
        );
        assert!(
            rendered
                .contains("Detail           This model's maximum context length is 200000 tokens"),
            "{rendered}"
        );
        assert!(rendered.contains("Compact          /compact"), "{rendered}");
        assert!(
            rendered.contains("Fresh session    /clear --confirm"),
            "{rendered}"
        );
    }

    #[test]
    fn retry_wrapped_context_window_errors_keep_recovery_guidance() {
        let error = ApiError::RetriesExhausted {
            attempts: 2,
            last_error: Box::new(ApiError::Api {
                status: "413".parse().expect("status"),
                error_type: Some("invalid_request_error".to_string()),
                message: Some("Request is too large for this model's context window.".to_string()),
                request_id: Some("req_ctx_retry_789".to_string()),
                body: String::new(),
                retryable: false,
            }),
        };

        let rendered = format_user_visible_api_error("session-issue-32", &error);
        assert!(rendered.contains("Context window blocked"), "{rendered}");
        assert!(rendered.contains("context_window_blocked"), "{rendered}");
        assert!(
            rendered.contains("Trace            req_ctx_retry_789"),
            "{rendered}"
        );
        assert!(
            rendered
                .contains("Detail           Request is too large for this model's context window."),
            "{rendered}"
        );
        assert!(rendered.contains("Compact          /compact"), "{rendered}");
        assert!(
            rendered.contains("Resume compact   orbit --resume session-issue-32 /compact"),
            "{rendered}"
        );
    }

    fn temp_dir() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};

        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be after epoch")
            .as_nanos();
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("orbit-cli-{nanos}-{unique}"))
    }

    fn git(args: &[&str], cwd: &Path) {
        let status = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .status()
            .expect("git command should run");
        assert!(
            status.success(),
            "git command failed: git {}",
            args.join(" ")
        );
    }

    fn env_lock() -> MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn with_current_dir<T>(cwd: &Path, f: impl FnOnce() -> T) -> T {
        let _guard = cwd_lock()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let previous = std::env::current_dir().expect("cwd should load");
        std::env::set_current_dir(cwd).expect("cwd should change");
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        std::env::set_current_dir(previous).expect("cwd should restore");
        match result {
            Ok(value) => value,
            Err(payload) => std::panic::resume_unwind(payload),
        }
    }

    fn write_plugin_fixture(root: &Path, name: &str, include_hooks: bool, include_lifecycle: bool) {
        fs::create_dir_all(root.join(".claude-plugin")).expect("manifest dir");
        if include_hooks {
            fs::create_dir_all(root.join("hooks")).expect("hooks dir");
            fs::write(
                root.join("hooks").join("pre.sh"),
                "#!/bin/sh\nprintf 'plugin pre hook'\n",
            )
            .expect("write hook");
        }
        if include_lifecycle {
            fs::create_dir_all(root.join("lifecycle")).expect("lifecycle dir");
            fs::write(
                root.join("lifecycle").join("init.sh"),
                "#!/bin/sh\nprintf 'init\\n' >> lifecycle.log\n",
            )
            .expect("write init lifecycle");
            fs::write(
                root.join("lifecycle").join("shutdown.sh"),
                "#!/bin/sh\nprintf 'shutdown\\n' >> lifecycle.log\n",
            )
            .expect("write shutdown lifecycle");
        }

        let hooks = if include_hooks {
            ",\n  \"hooks\": {\n    \"PreToolUse\": [\"./hooks/pre.sh\"]\n  }"
        } else {
            ""
        };
        let lifecycle = if include_lifecycle {
            ",\n  \"lifecycle\": {\n    \"Init\": [\"./lifecycle/init.sh\"],\n    \"Shutdown\": [\"./lifecycle/shutdown.sh\"]\n  }"
        } else {
            ""
        };
        fs::write(
            root.join(".claude-plugin").join("plugin.json"),
            format!(
                "{{\n  \"name\": \"{name}\",\n  \"version\": \"1.0.0\",\n  \"description\": \"runtime plugin fixture\"{hooks}{lifecycle}\n}}"
            ),
        )
        .expect("write plugin manifest");
    }
    #[test]
    fn defaults_to_repl_when_no_args() {
        let _guard = env_lock();
        std::env::remove_var("RUSTY_CLAUDE_PERMISSION_MODE");
        assert_eq!(
            parse_args(&[]).expect("args should parse"),
            CliAction::Repl {
                model: DEFAULT_MODEL.to_string(),
                provider: None,
                allowed_tools: None,
                permission_mode: PermissionMode::DangerFullAccess,
            }
        );
    }

    #[test]
    fn default_permission_mode_uses_project_config_when_env_is_unset() {
        let _guard = env_lock();
        let root = temp_dir();
        let cwd = root.join("project");
        let config_home = root.join("config-home");
        std::fs::create_dir_all(cwd.join(".orbit")).expect("project config dir should exist");
        std::fs::create_dir_all(&config_home).expect("config home should exist");
        std::fs::write(
            cwd.join(".orbit").join("settings.json"),
            r#"{"permissionMode":"acceptEdits"}"#,
        )
        .expect("project config should write");

        let original_config_home = std::env::var("ORBIT_CONFIG_HOME").ok();
        let original_permission_mode = std::env::var("RUSTY_CLAUDE_PERMISSION_MODE").ok();
        std::env::set_var("ORBIT_CONFIG_HOME", &config_home);
        std::env::remove_var("RUSTY_CLAUDE_PERMISSION_MODE");

        let resolved = with_current_dir(&cwd, super::default_permission_mode);

        match original_config_home {
            Some(value) => std::env::set_var("ORBIT_CONFIG_HOME", value),
            None => std::env::remove_var("ORBIT_CONFIG_HOME"),
        }
        match original_permission_mode {
            Some(value) => std::env::set_var("RUSTY_CLAUDE_PERMISSION_MODE", value),
            None => std::env::remove_var("RUSTY_CLAUDE_PERMISSION_MODE"),
        }
        std::fs::remove_dir_all(root).expect("temp config root should clean up");

        assert_eq!(resolved, PermissionMode::WorkspaceWrite);
    }

    #[test]
    fn env_permission_mode_overrides_project_config_default() {
        let _guard = env_lock();
        let root = temp_dir();
        let cwd = root.join("project");
        let config_home = root.join("config-home");
        std::fs::create_dir_all(cwd.join(".orbit")).expect("project config dir should exist");
        std::fs::create_dir_all(&config_home).expect("config home should exist");
        std::fs::write(
            cwd.join(".orbit").join("settings.json"),
            r#"{"permissionMode":"acceptEdits"}"#,
        )
        .expect("project config should write");

        let original_config_home = std::env::var("ORBIT_CONFIG_HOME").ok();
        let original_permission_mode = std::env::var("RUSTY_CLAUDE_PERMISSION_MODE").ok();
        std::env::set_var("ORBIT_CONFIG_HOME", &config_home);
        std::env::set_var("RUSTY_CLAUDE_PERMISSION_MODE", "read-only");

        let resolved = with_current_dir(&cwd, super::default_permission_mode);

        match original_config_home {
            Some(value) => std::env::set_var("ORBIT_CONFIG_HOME", value),
            None => std::env::remove_var("ORBIT_CONFIG_HOME"),
        }
        match original_permission_mode {
            Some(value) => std::env::set_var("RUSTY_CLAUDE_PERMISSION_MODE", value),
            None => std::env::remove_var("RUSTY_CLAUDE_PERMISSION_MODE"),
        }
        std::fs::remove_dir_all(root).expect("temp config root should clean up");

        assert_eq!(resolved, PermissionMode::ReadOnly);
    }

    #[test]
    fn parses_prompt_subcommand() {
        let _guard = env_lock();
        std::env::remove_var("RUSTY_CLAUDE_PERMISSION_MODE");
        let args = vec![
            "prompt".to_string(),
            "hello".to_string(),
            "world".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::Prompt {
                prompt: "hello world".to_string(),
                model: DEFAULT_MODEL.to_string(),
                provider: None,
                output_format: CliOutputFormat::Text,
                allowed_tools: None,
                permission_mode: PermissionMode::DangerFullAccess,
            }
        );
    }

    #[test]
    fn parses_bare_prompt_and_json_output_flag() {
        let _guard = env_lock();
        std::env::remove_var("RUSTY_CLAUDE_PERMISSION_MODE");
        let args = vec![
            "--output-format=json".to_string(),
            "--model".to_string(),
            "claude-opus".to_string(),
            "explain".to_string(),
            "this".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::Prompt {
                prompt: "explain this".to_string(),
                model: "claude-opus".to_string(),
                provider: None,
                output_format: CliOutputFormat::Json,
                allowed_tools: None,
                permission_mode: PermissionMode::DangerFullAccess,
            }
        );
    }

    #[test]
    fn resolves_model_aliases_in_args() {
        let _guard = env_lock();
        std::env::remove_var("RUSTY_CLAUDE_PERMISSION_MODE");
        let args = vec![
            "--model".to_string(),
            "opus".to_string(),
            "explain".to_string(),
            "this".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::Prompt {
                prompt: "explain this".to_string(),
                model: "claude-opus-4-6".to_string(),
                provider: None,
                output_format: CliOutputFormat::Text,
                allowed_tools: None,
                permission_mode: PermissionMode::DangerFullAccess,
            }
        );
    }

    #[test]
    fn resolves_known_model_aliases() {
        assert_eq!(resolve_model_alias("opus"), "claude-opus-4-6");
        assert_eq!(resolve_model_alias("sonnet"), "claude-sonnet-4-6");
        assert_eq!(resolve_model_alias("haiku"), "claude-haiku-4-5-20251213");
        assert_eq!(resolve_model_alias("claude-opus"), "claude-opus");
    }

    #[test]
    fn parses_provider_flag() {
        let _guard = env_lock();
        std::env::remove_var("RUSTY_CLAUDE_PERMISSION_MODE");
        let args = vec!["--provider=ollama".to_string(), "hello world".to_string()];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::Prompt {
                prompt: "hello world".to_string(),
                model: DEFAULT_MODEL.to_string(),
                provider: Some("ollama".to_string()),
                output_format: CliOutputFormat::Text,
                allowed_tools: None,
                permission_mode: PermissionMode::DangerFullAccess,
            }
        );
    }

    #[test]
    fn parses_provider_and_model_flags() {
        let _guard = env_lock();
        std::env::remove_var("RUSTY_CLAUDE_PERMISSION_MODE");
        let args = vec![
            "--provider=anthropic".to_string(),
            "--model=opus".to_string(),
            "hello world".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::Prompt {
                prompt: "hello world".to_string(),
                model: "claude-opus-4-6".to_string(),
                provider: Some("anthropic".to_string()),
                output_format: CliOutputFormat::Text,
                allowed_tools: None,
                permission_mode: PermissionMode::DangerFullAccess,
            }
        );
    }

    #[test]
    fn parses_version_flags_without_initializing_prompt_mode() {
        assert_eq!(
            parse_args(&["--version".to_string()]).expect("args should parse"),
            CliAction::Version {
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&["-V".to_string()]).expect("args should parse"),
            CliAction::Version {
                output_format: CliOutputFormat::Text,
            }
        );
    }

    #[test]
    fn parses_permission_mode_flag() {
        let args = vec!["--permission-mode=read-only".to_string()];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::Repl {
                model: DEFAULT_MODEL.to_string(),
                provider: None,
                allowed_tools: None,
                permission_mode: PermissionMode::ReadOnly,
            }
        );
    }

    #[test]
    fn parses_allowed_tools_flags_with_aliases_and_lists() {
        let _guard = env_lock();
        std::env::remove_var("RUSTY_CLAUDE_PERMISSION_MODE");
        let args = vec![
            "--allowedTools".to_string(),
            "read,glob".to_string(),
            "--allowed-tools=write_file".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::Repl {
                model: DEFAULT_MODEL.to_string(),
                provider: None,
                allowed_tools: Some(
                    ["glob_search", "read_file", "write_file"]
                        .into_iter()
                        .map(str::to_string)
                        .collect()
                ),
                permission_mode: PermissionMode::DangerFullAccess,
            }
        );
    }

    #[test]
    fn rejects_unknown_allowed_tools() {
        let error = parse_args(&["--allowedTools".to_string(), "teleport".to_string()])
            .expect_err("tool should be rejected");
        assert!(error.contains("unsupported tool in --allowedTools: teleport"));
    }

    #[test]
    fn parses_system_prompt_options() {
        let args = vec![
            "system-prompt".to_string(),
            "--cwd".to_string(),
            "/tmp/project".to_string(),
            "--date".to_string(),
            "2026-04-01".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::PrintSystemPrompt {
                cwd: PathBuf::from("/tmp/project"),
                date: "2026-04-01".to_string(),
                output_format: CliOutputFormat::Text,
            }
        );
    }

    #[test]
    fn parses_doctor_init_and_supporting_subcommands() {
        assert_eq!(
            parse_args(&["doctor".to_string()]).expect("doctor should parse"),
            CliAction::Doctor {
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&["init".to_string()]).expect("init should parse"),
            CliAction::Init {
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&["agents".to_string()]).expect("agents should parse"),
            CliAction::Agents {
                args: None,
                output_format: CliOutputFormat::Text
            }
        );
        assert_eq!(
            parse_args(&["mcp".to_string()]).expect("mcp should parse"),
            CliAction::Mcp {
                args: None,
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&["config".to_string()]).expect("config should parse"),
            CliAction::Config {
                section: None,
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&["config".to_string(), "telemetry".to_string()])
                .expect("config telemetry should parse"),
            CliAction::Config {
                section: Some("telemetry".to_string()),
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&["telemetry".to_string()]).expect("telemetry should parse"),
            CliAction::Telemetry {
                action: None,
                target: None,
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&["skills".to_string()]).expect("skills should parse"),
            CliAction::Skills {
                args: None,
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&[
                "skills".to_string(),
                "help".to_string(),
                "overview".to_string()
            ])
            .expect("skills help overview should invoke"),
            CliAction::Prompt {
                prompt: "$help overview".to_string(),
                model: DEFAULT_MODEL.to_string(),
                provider: None,
                output_format: CliOutputFormat::Text,
                allowed_tools: None,
                permission_mode: crate::default_permission_mode(),
            }
        );
        assert_eq!(
            parse_args(&["agents".to_string(), "--help".to_string()])
                .expect("agents help should parse"),
            CliAction::Agents {
                args: Some("--help".to_string()),
                output_format: CliOutputFormat::Text,
            }
        );
    }

    #[test]
    fn parses_hosted_policy_preview_command() {
        assert_eq!(
            parse_args(&[
                "hosted".to_string(),
                "policy".to_string(),
                "orphans".to_string(),
                "--repo".to_string(),
                "myorg/myapp".to_string(),
                "--source".to_string(),
                "slack".to_string(),
                "--priority".to_string(),
                "high".to_string(),
            ])
            .expect("hosted policy should parse"),
            CliAction::Hosted {
                command: HostedCommand::PolicyOrphans {
                    repository: Some("myorg/myapp".to_string()),
                    source: Some("slack".to_string()),
                    priority: Some("high".to_string()),
                },
                output_format: CliOutputFormat::Text,
            }
        );
    }

    #[test]
    fn parses_hosted_events_watch_command() {
        assert_eq!(
            parse_args(&[
                "hosted".to_string(),
                "events".to_string(),
                "watch".to_string(),
                "--task-id".to_string(),
                "task_123".to_string(),
                "--topic".to_string(),
                "approval".to_string(),
                "--event".to_string(),
                "approval.requested".to_string(),
                "--status".to_string(),
                "pending".to_string(),
                "--limit".to_string(),
                "5".to_string(),
            ])
            .expect("hosted events watch should parse"),
            CliAction::Hosted {
                command: HostedCommand::EventsWatch {
                    query: HostedEventWatchQuery {
                        task_id: Some("task_123".to_string()),
                        topic: Some("approval".to_string()),
                        event: Some("approval.requested".to_string()),
                        status: Some("pending".to_string()),
                        limit: Some(5),
                    },
                },
                output_format: CliOutputFormat::Text,
            }
        );
    }

    #[test]
    fn parses_hosted_tasks_list_command() {
        assert_eq!(
            parse_args(&[
                "hosted".to_string(),
                "tasks".to_string(),
                "list".to_string(),
                "--status".to_string(),
                "pending,running".to_string(),
                "--source".to_string(),
                "slack".to_string(),
                "--repo".to_string(),
                "myorg/myapp".to_string(),
                "--channel-id".to_string(),
                "C123".to_string(),
                "--thread-ts".to_string(),
                "171234.56".to_string(),
                "--limit".to_string(),
                "10".to_string(),
            ])
            .expect("hosted tasks list should parse"),
            CliAction::Hosted {
                command: HostedCommand::TasksList {
                    query: HostedTaskListQuery {
                        status: Some("pending,running".to_string()),
                        source: Some("slack".to_string()),
                        repository: Some("myorg/myapp".to_string()),
                        channel_id: Some("C123".to_string()),
                        thread_ts: Some("171234.56".to_string()),
                        limit: Some(10),
                        needs_followup: None,
                    },
                },
                output_format: CliOutputFormat::Text,
            }
        );
    }

    #[test]
    fn parses_hosted_tasks_watch_command() {
        assert_eq!(
            parse_args(&[
                "hosted".to_string(),
                "tasks".to_string(),
                "watch".to_string(),
                "--status".to_string(),
                "pending,running".to_string(),
                "--source".to_string(),
                "slack".to_string(),
                "--repo".to_string(),
                "myorg/myapp".to_string(),
                "--channel-id".to_string(),
                "C123".to_string(),
                "--thread-ts".to_string(),
                "171234.56".to_string(),
                "--limit".to_string(),
                "20".to_string(),
            ])
            .expect("hosted tasks watch should parse"),
            CliAction::Hosted {
                command: HostedCommand::TasksWatch {
                    query: HostedTaskListQuery {
                        status: Some("pending,running".to_string()),
                        source: Some("slack".to_string()),
                        repository: Some("myorg/myapp".to_string()),
                        channel_id: Some("C123".to_string()),
                        thread_ts: Some("171234.56".to_string()),
                        limit: Some(20),
                        needs_followup: None,
                    },
                },
                output_format: CliOutputFormat::Text,
            }
        );
    }

    #[test]
    fn parses_hosted_approval_orphan_default_kind() {
        assert_eq!(
            parse_args(&[
                "hosted".to_string(),
                "task".to_string(),
                "approval".to_string(),
                "task_123".to_string(),
                "retry".to_string(),
                "--resolved-by".to_string(),
                "operator".to_string(),
                "--reason".to_string(),
                "retry lane".to_string(),
            ])
            .expect("hosted approval should parse"),
            CliAction::Hosted {
                command: HostedCommand::TaskApproval {
                    task_id: "task_123".to_string(),
                    action: HostedApprovalAction::Retry,
                    resolved_by: Some("operator".to_string()),
                    reason: Some("retry lane".to_string()),
                    approval_kind: "orphaned_hosted_agent".to_string(),
                },
                output_format: CliOutputFormat::Text,
            }
        );
    }

    #[test]
    fn parses_hosted_approval_github_followup_ack() {
        assert_eq!(
            parse_args(&[
                "hosted".to_string(),
                "task".to_string(),
                "approval".to_string(),
                "task_123".to_string(),
                "ack".to_string(),
                "--kind".to_string(),
                "github_review_followup".to_string(),
                "--resolved-by".to_string(),
                "reviewer".to_string(),
            ])
            .expect("hosted approval github followup should parse"),
            CliAction::Hosted {
                command: HostedCommand::TaskApproval {
                    task_id: "task_123".to_string(),
                    action: HostedApprovalAction::Ack,
                    resolved_by: Some("reviewer".to_string()),
                    reason: None,
                    approval_kind: "github_review_followup".to_string(),
                },
                output_format: CliOutputFormat::Text,
            }
        );
    }

    #[test]
    fn hosted_task_snapshot_from_sparse_lane_started_event_infers_running_status() {
        let event = EventEnvelope::new(
            HostedEventName::LaneStarted,
            HostedEventStatus::Running,
            HostedEventTopic::Lane,
            EventIdentifiers {
                task_id: Some("task_sparse".to_string()),
                lane_id: Some("lane_sparse".to_string()),
                ..EventIdentifiers::default()
            },
            Some(json!({
                "channel_id": "C123",
                "thread_ts": "171234.56",
                "worker_status": "running",
            })),
            None,
        );

        let snapshot = hosted_task_snapshot_from_event(&event)
            .expect("sparse lane.started event should infer a task snapshot");
        assert_eq!(snapshot.task_id, "task_sparse");
        assert_eq!(snapshot.status, "running");
        assert_eq!(snapshot.channel_id.as_deref(), Some("C123"));
        assert_eq!(snapshot.thread_ts.as_deref(), Some("171234.56"));
        assert_eq!(snapshot.worker_status.as_deref(), Some("running"));
    }

    #[test]
    fn hosted_task_snapshot_from_sparse_cancel_event_infers_cancelled_status() {
        let event = EventEnvelope::new(
            HostedEventName::TaskCancelled,
            HostedEventStatus::Cancelled,
            HostedEventTopic::Task,
            EventIdentifiers {
                task_id: Some("task_cancelled".to_string()),
                ..EventIdentifiers::default()
            },
            Some(json!({
                "channel_id": "C999",
            })),
            None,
        );

        let snapshot = hosted_task_snapshot_from_event(&event)
            .expect("sparse task.cancelled event should infer a task snapshot");
        assert_eq!(snapshot.task_id, "task_cancelled");
        assert_eq!(snapshot.status, "cancelled");
        assert_eq!(snapshot.channel_id.as_deref(), Some("C999"));
    }

    #[test]
    fn parses_hosted_task_reconcile_and_approval_commands() {
        assert_eq!(
            parse_args(&[
                "hosted".to_string(),
                "task".to_string(),
                "get".to_string(),
                "task_123".to_string(),
            ])
            .expect("hosted get should parse"),
            CliAction::Hosted {
                command: HostedCommand::TaskGet {
                    task_id: "task_123".to_string(),
                },
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&[
                "hosted".to_string(),
                "task".to_string(),
                "runtime".to_string(),
                "task_123".to_string(),
            ])
            .expect("hosted runtime should parse"),
            CliAction::Hosted {
                command: HostedCommand::TaskRuntime {
                    task_id: "task_123".to_string(),
                },
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&[
                "hosted".to_string(),
                "task".to_string(),
                "reconcile".to_string(),
                "task_123".to_string(),
            ])
            .expect("hosted reconcile should parse"),
            CliAction::Hosted {
                command: HostedCommand::TaskReconcile {
                    task_id: "task_123".to_string(),
                },
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&[
                "hosted".to_string(),
                "task".to_string(),
                "run".to_string(),
                "task_123".to_string(),
            ])
            .expect("hosted run should parse"),
            CliAction::Hosted {
                command: HostedCommand::TaskRun {
                    task_id: "task_123".to_string(),
                },
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&[
                "hosted".to_string(),
                "task".to_string(),
                "cancel".to_string(),
                "task_123".to_string(),
            ])
            .expect("hosted cancel should parse"),
            CliAction::Hosted {
                command: HostedCommand::TaskCancel {
                    task_id: "task_123".to_string(),
                },
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&[
                "hosted".to_string(),
                "task".to_string(),
                "approval".to_string(),
                "task_123".to_string(),
                "retry".to_string(),
                "--resolved-by".to_string(),
                "operator".to_string(),
                "--reason".to_string(),
                "manual-recovery".to_string(),
            ])
            .expect("hosted approval should parse"),
            CliAction::Hosted {
                command: HostedCommand::TaskApproval {
                    task_id: "task_123".to_string(),
                    action: HostedApprovalAction::Retry,
                    resolved_by: Some("operator".to_string()),
                    reason: Some("manual-recovery".to_string()),
                    approval_kind: "orphaned_hosted_agent".to_string(),
                },
                output_format: CliOutputFormat::Text,
            }
        );
    }

    #[test]
    fn hosted_server_url_prefers_env_and_trims_trailing_slash() {
        let _guard = env_lock();
        std::env::set_var("ORBIT_SERVER_URL", "http://hosted.orbit.test/");
        std::env::remove_var("ORBIT_SERVER_BASE_URL");
        assert_eq!(hosted_server_url(), "http://hosted.orbit.test");
        std::env::remove_var("ORBIT_SERVER_URL");

        std::env::set_var("ORBIT_SERVER_BASE_URL", "http://fallback.orbit.test/");
        assert_eq!(hosted_server_url(), "http://fallback.orbit.test");
        std::env::remove_var("ORBIT_SERVER_BASE_URL");
    }

    #[test]
    fn load_hosted_task_worker_payload_reads_json_file_from_env() {
        let _guard = env_lock();
        let dir = std::env::temp_dir().join(format!(
            "orbit-cli-hosted-task-payload-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let payload_path = dir.join("task.json");
        fs::write(
            &payload_path,
            serde_json::to_vec(&HostedTaskWorkerPayload {
                task_id: "task_123".to_string(),
                prompt: "Investigate failure".to_string(),
                repository: Some("acme/payments".to_string()),
                repo_url: Some("https://github.com/acme/payments.git".to_string()),
                base_ref: Some("main".to_string()),
                branch: Some("orbit/fix-flake".to_string()),
                model: Some("gpt-5.4".to_string()),
                provider: Some("openai".to_string()),
                permission_mode: Some("workspace-write".to_string()),
                allowed_tools: vec!["git".to_string()],
            })
            .unwrap(),
        )
        .unwrap();
        std::env::set_var("ORBIT_HOSTED_TASK_FILE", &payload_path);

        let payload =
            load_hosted_task_worker_payload("task_123").expect("payload should load successfully");
        assert_eq!(payload.task_id, "task_123");
        assert_eq!(payload.prompt, "Investigate failure");
        assert_eq!(payload.repository.as_deref(), Some("acme/payments"));
        assert_eq!(
            payload.repo_url.as_deref(),
            Some("https://github.com/acme/payments.git")
        );
        assert_eq!(payload.base_ref.as_deref(), Some("main"));
        assert_eq!(payload.branch.as_deref(), Some("orbit/fix-flake"));
        assert_eq!(payload.model.as_deref(), Some("gpt-5.4"));
        assert_eq!(payload.provider.as_deref(), Some("openai"));
        assert_eq!(payload.permission_mode.as_deref(), Some("workspace-write"));
        assert_eq!(payload.allowed_tools, vec!["git".to_string()]);

        std::env::remove_var("ORBIT_HOSTED_TASK_FILE");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn default_hosted_pr_draft_uses_repo_and_prompt_context() {
        let payload = HostedTaskWorkerPayload {
            task_id: "task_123".to_string(),
            prompt: "Fix the flaky release workflow by pinning the Docker image".to_string(),
            repository: Some("acme/payments".to_string()),
            repo_url: Some("https://github.com/acme/payments.git".to_string()),
            base_ref: Some("main".to_string()),
            branch: Some("orbit/fix-release".to_string()),
            model: None,
            provider: None,
            permission_mode: None,
            allowed_tools: Vec::new(),
        };

        let draft = default_hosted_pr_draft(&payload, "orbit/fix-release", "abc123def456")
            .expect("github payload should produce a PR draft");

        assert_eq!(
            draft.title,
            "Fix the flaky release workflow by pinning the Docker image"
        );
        assert_eq!(draft.head, "orbit/fix-release");
        assert_eq!(draft.base, "main");
        assert!(draft.draft);
        assert!(draft.body.contains("task_123"));
        assert!(draft.body.contains("acme/payments"));
        assert!(draft.body.contains("abc123def456"));
    }

    #[test]
    fn publish_hosted_repo_changes_commits_and_pushes_branch() {
        let _guard = env_lock();
        let previous_token = std::env::var_os("GITHUB_TOKEN");
        let previous_api_base = std::env::var_os("ORBIT_GITHUB_API_BASE");
        std::env::remove_var("GITHUB_TOKEN");
        std::env::remove_var("ORBIT_GITHUB_API_BASE");

        let remote = temp_dir();
        let repo = temp_dir();
        fs::create_dir_all(&remote).expect("remote dir should exist");
        fs::create_dir_all(&repo).expect("repo dir should exist");

        git(
            &["init", "--bare", remote.to_str().unwrap()],
            Path::new("."),
        );
        git(&["init", "-b", "main"], &repo);
        git(&["config", "user.name", "Orbit Test"], &repo);
        git(&["config", "user.email", "orbit@test.dev"], &repo);
        fs::write(repo.join("README.md"), "hello\n").expect("seed file");
        git(&["add", "README.md"], &repo);
        git(&["commit", "-m", "initial"], &repo);
        git(
            &["remote", "add", "origin", remote.to_str().unwrap()],
            &repo,
        );
        git(&["push", "-u", "origin", "main"], &repo);
        git(&["checkout", "-b", "orbit/task-publish"], &repo);
        fs::write(repo.join("README.md"), "hello\nworld\n").expect("updated file");

        let payload = HostedTaskWorkerPayload {
            task_id: "task_123".to_string(),
            prompt: "Update the README for the hosted task".to_string(),
            repository: Some("acme/payments".to_string()),
            repo_url: Some(remote.display().to_string()),
            base_ref: Some("main".to_string()),
            branch: Some("orbit/task-publish".to_string()),
            model: None,
            provider: None,
            permission_mode: None,
            allowed_tools: Vec::new(),
        };

        let publication = with_current_dir(&repo, || {
            publish_hosted_repo_changes(Path::new("."), &payload)
                .expect("publish helper should succeed")
                .expect("dirty repo should produce publication metadata")
        });

        assert_eq!(publication.published_remote.as_deref(), Some("origin"));
        assert_eq!(
            publication.published_branch.as_deref(),
            Some("orbit/task-publish")
        );
        assert!(publication
            .published_commit_sha
            .as_deref()
            .is_some_and(|value| !value.is_empty()));
        assert_eq!(publication.pr_url, None);

        let branch_ref = Command::new("git")
            .args([
                "--git-dir",
                remote.to_str().unwrap(),
                "show-ref",
                "--verify",
                "refs/heads/orbit/task-publish",
            ])
            .output()
            .expect("show-ref should run");
        assert!(
            branch_ref.status.success(),
            "expected pushed branch to exist in remote"
        );

        match previous_token {
            Some(value) => std::env::set_var("GITHUB_TOKEN", value),
            None => std::env::remove_var("GITHUB_TOKEN"),
        }
        match previous_api_base {
            Some(value) => std::env::set_var("ORBIT_GITHUB_API_BASE", value),
            None => std::env::remove_var("ORBIT_GITHUB_API_BASE"),
        }
        let _ = fs::remove_dir_all(remote);
        let _ = fs::remove_dir_all(repo);
    }

    #[test]
    fn augment_hosted_result_with_publication_appends_branch_and_pr() {
        let result = augment_hosted_result_with_publication(
            Some("Applied the requested fix.".to_string()),
            &HostedTaskGithubResponse {
                published_branch: Some("orbit/fix-flake".to_string()),
                published_commit_sha: Some("abc123def456".to_string()),
                pr_url: Some("https://github.com/acme/payments/pull/42".to_string()),
                ..HostedTaskGithubResponse::default()
            },
        )
        .expect("publication details should keep a result string");

        assert!(result.contains("Applied the requested fix."));
        assert!(result.contains("Branch: orbit/fix-flake"));
        assert!(result.contains("Commit: abc123def456"));
        assert!(result.contains("PR: https://github.com/acme/payments/pull/42"));
    }

    #[test]
    fn config_subcommand_allows_unknown_sections_and_rejects_extra_args() {
        assert_eq!(
            parse_args(&["config".to_string(), "unknown".to_string()])
                .expect("unknown config section should parse"),
            CliAction::Config {
                section: Some("unknown".to_string()),
                output_format: CliOutputFormat::Text,
            }
        );
        let extra = parse_args(&[
            "config".to_string(),
            "telemetry".to_string(),
            "extra".to_string(),
        ])
        .expect_err("extra config arg should fail");
        assert!(extra.contains("config accepts at most one section argument"));
    }

    #[test]
    fn parses_config_subcommand_with_json_output_format() {
        assert_eq!(
            parse_args(&[
                "--output-format=json".to_string(),
                "config".to_string(),
                "telemetry".to_string(),
            ])
            .expect("config telemetry json should parse"),
            CliAction::Config {
                section: Some("telemetry".to_string()),
                output_format: CliOutputFormat::Json,
            }
        );
    }

    #[test]
    fn local_command_help_flags_stay_on_the_local_parser_path() {
        assert_eq!(
            parse_args(&["status".to_string(), "--help".to_string()])
                .expect("status help should parse"),
            CliAction::HelpTopic(LocalHelpTopic::Status)
        );
        assert_eq!(
            parse_args(&["sandbox".to_string(), "-h".to_string()])
                .expect("sandbox help should parse"),
            CliAction::HelpTopic(LocalHelpTopic::Sandbox)
        );
        assert_eq!(
            parse_args(&["doctor".to_string(), "--help".to_string()])
                .expect("doctor help should parse"),
            CliAction::HelpTopic(LocalHelpTopic::Doctor)
        );
    }

    #[test]
    fn parses_single_word_command_aliases_without_falling_back_to_prompt_mode() {
        let _guard = env_lock();
        std::env::remove_var("RUSTY_CLAUDE_PERMISSION_MODE");
        assert_eq!(
            parse_args(&["help".to_string()]).expect("help should parse"),
            CliAction::Help {
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&["version".to_string()]).expect("version should parse"),
            CliAction::Version {
                output_format: CliOutputFormat::Text,
            }
        );
    }

    #[test]
    fn parses_json_output_for_mcp_and_skills_commands() {
        assert_eq!(
            parse_args(&["--output-format=json".to_string(), "mcp".to_string()])
                .expect("json mcp should parse"),
            CliAction::Mcp {
                args: None,
                output_format: CliOutputFormat::Json,
            }
        );
        assert_eq!(
            parse_args(&[
                "--output-format=json".to_string(),
                "/skills".to_string(),
                "help".to_string(),
            ])
            .expect("json /skills help should parse"),
            CliAction::Skills {
                args: Some("help".to_string()),
                output_format: CliOutputFormat::Json,
            }
        );
    }

    #[test]
    fn single_word_slash_command_names_return_guidance_instead_of_hitting_prompt_mode() {
        let error = parse_args(&["cost".to_string()]).expect_err("cost should return guidance");
        assert!(error.contains("slash command"));
        assert!(error.contains("/cost"));
    }

    #[test]
    fn multi_word_prompt_still_uses_shorthand_prompt_mode() {
        let _guard = env_lock();
        std::env::remove_var("RUSTY_CLAUDE_PERMISSION_MODE");
        assert_eq!(
            parse_args(&["help".to_string(), "me".to_string(), "debug".to_string()])
                .expect("prompt shorthand should still work"),
            CliAction::Prompt {
                prompt: "help me debug".to_string(),
                model: DEFAULT_MODEL.to_string(),
                provider: None,
                output_format: CliOutputFormat::Text,
                allowed_tools: None,
                permission_mode: PermissionMode::DangerFullAccess,
            }
        );
    }

    #[test]
    fn parses_direct_agents_mcp_and_skills_slash_commands() {
        assert_eq!(
            parse_args(&["/agents".to_string()]).expect("/agents should parse"),
            CliAction::Agents {
                args: None,
                output_format: CliOutputFormat::Text
            }
        );
        assert_eq!(
            parse_args(&["/mcp".to_string(), "show".to_string(), "demo".to_string()])
                .expect("/mcp show demo should parse"),
            CliAction::Mcp {
                args: Some("show demo".to_string()),
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&["/skills".to_string()]).expect("/skills should parse"),
            CliAction::Skills {
                args: None,
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&["/skill".to_string()]).expect("/skill should parse"),
            CliAction::Skills {
                args: None,
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&["/skills".to_string(), "help".to_string()])
                .expect("/skills help should parse"),
            CliAction::Skills {
                args: Some("help".to_string()),
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&["/skill".to_string(), "list".to_string()])
                .expect("/skill list should parse"),
            CliAction::Skills {
                args: Some("list".to_string()),
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&[
                "/skills".to_string(),
                "help".to_string(),
                "overview".to_string()
            ])
            .expect("/skills help overview should invoke"),
            CliAction::Prompt {
                prompt: "$help overview".to_string(),
                model: DEFAULT_MODEL.to_string(),
                provider: None,
                output_format: CliOutputFormat::Text,
                allowed_tools: None,
                permission_mode: crate::default_permission_mode(),
            }
        );
        assert_eq!(
            parse_args(&[
                "/skills".to_string(),
                "install".to_string(),
                "./fixtures/help-skill".to_string(),
            ])
            .expect("/skills install should parse"),
            CliAction::Skills {
                args: Some("install ./fixtures/help-skill".to_string()),
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&["/skills".to_string(), "/test".to_string()])
                .expect("/skills /test should normalize to a single skill prompt prefix"),
            CliAction::Prompt {
                prompt: "$test".to_string(),
                model: DEFAULT_MODEL.to_string(),
                provider: None,
                output_format: CliOutputFormat::Text,
                allowed_tools: None,
                permission_mode: crate::default_permission_mode(),
            }
        );
        let error = parse_args(&["/status".to_string()])
            .expect_err("/status should remain REPL-only when invoked directly");
        assert!(error.contains("slash command"));
        assert!(error.contains("orbit --resume SESSION.jsonl /status"));
    }

    #[test]
    fn direct_slash_commands_surface_shared_validation_errors() {
        let compact_error = parse_args(&["/compact".to_string(), "now".to_string()])
            .expect_err("invalid /compact shape should be rejected");
        assert!(compact_error.contains("Unexpected arguments for /compact."));
        assert!(compact_error.contains("Usage            /compact"));

        let plugins_error = parse_args(&[
            "/plugins".to_string(),
            "list".to_string(),
            "extra".to_string(),
        ])
        .expect_err("invalid /plugins list shape should be rejected");
        assert!(plugins_error.contains("Usage: /plugin list"));
        assert!(plugins_error.contains("Aliases          /plugins, /marketplace"));
    }

    #[test]
    fn formats_unknown_slash_command_with_suggestions() {
        let report = format_unknown_slash_command_message("statsu");
        assert!(report.contains("unknown slash command: /statsu"));
        assert!(report.contains("Did you mean"));
        assert!(report.contains("Use /help"));
    }

    #[test]
    fn formats_namespaced_omc_slash_command_with_contract_guidance() {
        let report = format_unknown_slash_command_message("oh-my-claudecode:hud");
        assert!(report.contains("unknown slash command: /oh-my-claudecode:hud"));
        assert!(report.contains("Claude Code/OMC plugin command"));
        assert!(report.contains("plugin slash commands"));
        assert!(report.contains("statusline"));
        assert!(report.contains("session hooks"));
    }

    #[test]
    fn parses_resume_flag_with_slash_command() {
        let args = vec![
            "--resume".to_string(),
            "session.jsonl".to_string(),
            "/compact".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::ResumeSession {
                session_path: PathBuf::from("session.jsonl"),
                commands: vec!["/compact".to_string()],
                output_format: CliOutputFormat::Text,
            }
        );
    }

    #[test]
    fn parses_resume_flag_without_path_as_latest_session() {
        assert_eq!(
            parse_args(&["--resume".to_string()]).expect("args should parse"),
            CliAction::ResumeSession {
                session_path: PathBuf::from("latest"),
                commands: vec![],
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&["--resume".to_string(), "/status".to_string()])
                .expect("resume shortcut should parse"),
            CliAction::ResumeSession {
                session_path: PathBuf::from("latest"),
                commands: vec!["/status".to_string()],
                output_format: CliOutputFormat::Text,
            }
        );
    }

    #[test]
    fn parses_resume_flag_with_multiple_slash_commands() {
        let args = vec![
            "--resume".to_string(),
            "session.jsonl".to_string(),
            "/status".to_string(),
            "/compact".to_string(),
            "/cost".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::ResumeSession {
                session_path: PathBuf::from("session.jsonl"),
                commands: vec![
                    "/status".to_string(),
                    "/compact".to_string(),
                    "/cost".to_string(),
                ],
                output_format: CliOutputFormat::Text,
            }
        );
    }

    #[test]
    fn rejects_unknown_options_with_helpful_guidance() {
        let error = parse_args(&["--resum".to_string()]).expect_err("unknown option should fail");
        assert!(error.contains("unknown option: --resum"));
        assert!(error.contains("Did you mean --resume?"));
        assert!(error.contains("orbit --help"));
    }

    #[test]
    fn parses_resume_flag_with_slash_command_arguments() {
        let args = vec![
            "--resume".to_string(),
            "session.jsonl".to_string(),
            "/export".to_string(),
            "notes.txt".to_string(),
            "/clear".to_string(),
            "--confirm".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::ResumeSession {
                session_path: PathBuf::from("session.jsonl"),
                commands: vec![
                    "/export notes.txt".to_string(),
                    "/clear --confirm".to_string(),
                ],
                output_format: CliOutputFormat::Text,
            }
        );
    }

    #[test]
    fn parses_resume_flag_with_absolute_export_path() {
        let args = vec![
            "--resume".to_string(),
            "session.jsonl".to_string(),
            "/export".to_string(),
            "/tmp/notes.txt".to_string(),
            "/status".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::ResumeSession {
                session_path: PathBuf::from("session.jsonl"),
                commands: vec!["/export /tmp/notes.txt".to_string(), "/status".to_string()],
                output_format: CliOutputFormat::Text,
            }
        );
    }

    #[test]
    fn filtered_tool_specs_respect_allowlist() {
        let allowed = ["read_file", "grep_search"]
            .into_iter()
            .map(str::to_string)
            .collect();
        let filtered = filter_tool_specs(&GlobalToolRegistry::builtin(), Some(&allowed));
        let names = filtered
            .into_iter()
            .map(|spec| spec.name)
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["read_file", "grep_search"]);
    }

    #[test]
    fn filtered_tool_specs_include_plugin_tools() {
        let filtered = filter_tool_specs(&registry_with_plugin_tool(), None);
        let names = filtered
            .into_iter()
            .map(|definition| definition.name)
            .collect::<Vec<_>>();
        assert!(names.contains(&"bash".to_string()));
        assert!(names.contains(&"plugin_echo".to_string()));
    }

    #[test]
    fn permission_policy_uses_plugin_tool_permissions() {
        let feature_config = orbit_runtime::RuntimeFeatureConfig::default();
        let policy = permission_policy(
            PermissionMode::ReadOnly,
            &feature_config,
            &registry_with_plugin_tool(),
        )
        .expect("permission policy should build");
        let required = policy.required_mode_for("plugin_echo");
        assert_eq!(required, PermissionMode::WorkspaceWrite);
    }

    #[test]
    fn shared_help_uses_resume_annotation_copy() {
        let help = orbit_commands::render_slash_command_help();
        assert!(help.contains("Slash commands"));
        assert!(help.contains("works with --resume SESSION.jsonl"));
    }

    #[test]
    fn repl_help_includes_shared_commands_and_exit() {
        let help = render_repl_help();
        assert!(help.contains("REPL"));
        assert!(help.contains("/help"));
        assert!(help.contains("Complete commands, modes, and recent sessions"));
        assert!(help.contains("/status"));
        assert!(help.contains("/sandbox"));
        assert!(help.contains("/model [model]"));
        assert!(help.contains("/permissions [read-only|workspace-write|danger-full-access]"));
        assert!(help.contains("/clear [--confirm]"));
        assert!(help.contains("/cost"));
        assert!(help.contains("/resume <session-path>"));
        assert!(help.contains("/config [env|hooks|model|telemetry|plugins]"));
        assert!(help.contains("/mcp [list|show <server>|help]"));
        assert!(help.contains("/memory"));
        assert!(help.contains("/init"));
        assert!(help.contains("/diff"));
        assert!(help.contains("/version"));
        assert!(help.contains("/export [file]"));
        assert!(help.contains("/session [list|switch <session-id>|fork [branch-name]]"));
        assert!(help.contains(
            "/plugin [list|install <path>|enable <name>|disable <name>|uninstall <id>|update <id>]"
        ));
        assert!(help.contains("aliases: /plugins, /marketplace"));
        assert!(help.contains("/agents"));
        assert!(help.contains("/skills"));
        assert!(help.contains("/exit"));
        assert!(help.contains("Auto-save            .orbit/sessions/<session-id>.jsonl"));
        assert!(help.contains("Resume latest        /resume latest"));
    }

    #[test]
    fn completion_candidates_include_workflow_shortcuts_and_dynamic_sessions() {
        let completions = slash_command_completion_candidates_with_sessions(
            "sonnet",
            Some("session-current"),
            vec!["session-old".to_string()],
        );

        assert!(completions.contains(&"/model claude-sonnet-4-6".to_string()));
        assert!(completions.contains(&"/permissions workspace-write".to_string()));
        assert!(completions.contains(&"/session list".to_string()));
        assert!(completions.contains(&"/session switch session-current".to_string()));
        assert!(completions.contains(&"/resume session-old".to_string()));
        assert!(completions.contains(&"/mcp list".to_string()));
        assert!(completions.contains(&"/config telemetry".to_string()));
        assert!(completions.contains(&"/ultraplan ".to_string()));
    }

    #[test]
    fn startup_banner_mentions_workflow_completions() {
        let _guard = env_lock();
        // Inject dummy credentials so LiveCli can construct without real Anthropic key
        std::env::set_var("ORBIT_API_KEY", "test-dummy-key-for-banner-test");
        let root = temp_dir();
        fs::create_dir_all(&root).expect("root dir");

        let banner = with_current_dir(&root, || {
            LiveCli::new(
                "claude-sonnet-4-6".to_string(),
                true,
                None,
                PermissionMode::DangerFullAccess,
            )
            .expect("cli should initialize")
            .startup_banner()
        });

        assert!(banner.contains("Tab"));
        assert!(banner.contains("workflow completions"));

        fs::remove_dir_all(root).expect("cleanup temp dir");
        std::env::remove_var("ORBIT_API_KEY");
    }

    #[test]
    fn resume_supported_command_list_matches_expected_surface() {
        let names = resume_supported_slash_commands()
            .into_iter()
            .map(|spec| spec.name)
            .collect::<Vec<_>>();
        // Now with 135+ slash commands, verify minimum resume support
        assert!(
            names.len() >= 39,
            "expected at least 39 resume-supported commands, got {}",
            names.len()
        );
        // Verify key resume commands still exist
        assert!(names.contains(&"help"));
        assert!(names.contains(&"status"));
        assert!(names.contains(&"compact"));
    }

    #[test]
    fn resume_report_uses_sectioned_layout() {
        let report = format_resume_report("session.jsonl", 14, 6);
        assert!(report.contains("Session resumed"));
        assert!(report.contains("Session file     session.jsonl"));
        assert!(report.contains("Messages         14"));
        assert!(report.contains("Turns            6"));
    }

    #[test]
    fn compact_report_uses_structured_output() {
        let compacted = format_compact_report(8, 5, false);
        assert!(compacted.contains("Compact"));
        assert!(compacted.contains("Result           compacted"));
        assert!(compacted.contains("Messages removed 8"));
        let skipped = format_compact_report(0, 3, true);
        assert!(skipped.contains("Result           skipped"));
    }

    #[test]
    fn cost_report_uses_sectioned_layout() {
        let report = format_cost_report(orbit_runtime::TokenUsage {
            input_tokens: 20,
            output_tokens: 8,
            cache_creation_input_tokens: 3,
            cache_read_input_tokens: 1,
        });
        assert!(report.contains("Cost"));
        assert!(report.contains("Input tokens     20"));
        assert!(report.contains("Output tokens    8"));
        assert!(report.contains("Cache create     3"));
        assert!(report.contains("Cache read       1"));
        assert!(report.contains("Total tokens     32"));
    }

    #[test]
    fn permissions_report_uses_sectioned_layout() {
        let report = format_permissions_report("workspace-write");
        assert!(report.contains("Permissions"));
        assert!(report.contains("Active mode      workspace-write"));
        assert!(report.contains("Modes"));
        assert!(report.contains("read-only          ○ available Read/search tools only"));
        assert!(report.contains("workspace-write    ● current   Edit files inside the workspace"));
        assert!(report.contains("danger-full-access ○ available Unrestricted tool access"));
    }

    #[test]
    fn permissions_switch_report_is_structured() {
        let report = format_permissions_switch_report("read-only", "workspace-write");
        assert!(report.contains("Permissions updated"));
        assert!(report.contains("Result           mode switched"));
        assert!(report.contains("Previous mode    read-only"));
        assert!(report.contains("Active mode      workspace-write"));
        assert!(report.contains("Applies to       subsequent tool calls"));
    }

    #[test]
    fn init_help_mentions_direct_subcommand() {
        let mut help = Vec::new();
        print_help_to(&mut help).expect("help should render");
        let help = String::from_utf8(help).expect("help should be utf8");
        assert!(help.contains("orbit help"));
        assert!(help.contains("orbit version"));
        assert!(help.contains("orbit status"));
        assert!(help.contains("orbit config [env|hooks|model|telemetry|plugins]"));
        assert!(help.contains("orbit sandbox"));
        assert!(help.contains("orbit init"));
        assert!(help.contains("orbit agents"));
        assert!(help.contains("orbit mcp"));
        assert!(help.contains("orbit skills"));
        assert!(help.contains("orbit /skills"));
    }

    #[test]
    fn model_report_uses_sectioned_layout() {
        let report = format_model_report("claude-sonnet", 12, 4);
        assert!(report.contains("Model"));
        assert!(report.contains("Current model    claude-sonnet"));
        assert!(report.contains("Session messages 12"));
        assert!(report.contains("Switch models with /model <name>"));
    }

    #[test]
    fn model_switch_report_preserves_context_summary() {
        let report = format_model_switch_report("claude-sonnet", "claude-opus", 9);
        assert!(report.contains("Model updated"));
        assert!(report.contains("Previous         claude-sonnet"));
        assert!(report.contains("Current          claude-opus"));
        assert!(report.contains("Preserved msgs   9"));
    }

    #[test]
    fn status_line_reports_model_and_token_totals() {
        let status = format_status_report(
            "claude-sonnet",
            StatusUsage {
                message_count: 7,
                turns: 3,
                latest: orbit_runtime::TokenUsage {
                    input_tokens: 5,
                    output_tokens: 4,
                    cache_creation_input_tokens: 1,
                    cache_read_input_tokens: 0,
                },
                cumulative: orbit_runtime::TokenUsage {
                    input_tokens: 20,
                    output_tokens: 8,
                    cache_creation_input_tokens: 2,
                    cache_read_input_tokens: 1,
                },
                estimated_tokens: 128,
            },
            "workspace-write",
            &super::StatusContext {
                cwd: PathBuf::from("/tmp/project"),
                session_path: Some(PathBuf::from("session.jsonl")),
                loaded_config_files: 2,
                discovered_config_files: 3,
                memory_file_count: 4,
                project_root: Some(PathBuf::from("/tmp")),
                git_branch: Some("main".to_string()),
                git_summary: GitWorkspaceSummary {
                    changed_files: 3,
                    staged_files: 1,
                    unstaged_files: 1,
                    untracked_files: 1,
                    conflicted_files: 0,
                },
                sandbox_status: orbit_runtime::SandboxStatus::default(),
            },
        );
        assert!(status.contains("Status"));
        assert!(status.contains("Model            claude-sonnet"));
        assert!(status.contains("Permission mode  workspace-write"));
        assert!(status.contains("Messages         7"));
        assert!(status.contains("Latest total     10"));
        assert!(status.contains("Cumulative total 31"));
        assert!(status.contains("Cwd              /tmp/project"));
        assert!(status.contains("Project root     /tmp"));
        assert!(status.contains("Git branch       main"));
        assert!(
            status.contains("Git state        dirty · 3 files · 1 staged, 1 unstaged, 1 untracked")
        );
        assert!(status.contains("Changed files    3"));
        assert!(status.contains("Staged           1"));
        assert!(status.contains("Unstaged         1"));
        assert!(status.contains("Untracked        1"));
        assert!(status.contains("Session          session.jsonl"));
        assert!(status.contains("Config files     loaded 2/3"));
        assert!(status.contains("Memory files     4"));
        assert!(status.contains("Suggested flow   /status → /diff → /commit"));
    }

    #[test]
    fn commit_reports_surface_workspace_context() {
        let summary = GitWorkspaceSummary {
            changed_files: 2,
            staged_files: 1,
            unstaged_files: 1,
            untracked_files: 0,
            conflicted_files: 0,
        };

        let preflight = format_commit_preflight_report(Some("feature/ux"), summary);
        assert!(preflight.contains("Result           ready"));
        assert!(preflight.contains("Branch           feature/ux"));
        assert!(preflight.contains("Workspace        dirty · 2 files · 1 staged, 1 unstaged"));
        assert!(preflight
            .contains("Action           create a git commit from the current workspace changes"));
    }

    #[test]
    fn commit_skipped_report_points_to_next_steps() {
        let report = format_commit_skipped_report();
        assert!(report.contains("Reason           no workspace changes"));
        assert!(report
            .contains("Action           create a git commit from the current workspace changes"));
        assert!(report.contains("/status to inspect context"));
        assert!(report.contains("/diff to inspect repo changes"));
    }

    #[test]
    fn runtime_slash_reports_describe_command_behavior() {
        let bughunter = format_bughunter_report(Some("runtime"));
        assert!(bughunter.contains("Scope            runtime"));
        assert!(bughunter.contains("inspect the selected code for likely bugs"));

        let ultraplan = format_ultraplan_report(Some("ship the release"));
        assert!(ultraplan.contains("Task             ship the release"));
        assert!(ultraplan.contains("break work into a multi-step execution plan"));

        let pr = format_pr_report("feature/ux", Some("ready for review"));
        assert!(pr.contains("Branch           feature/ux"));
        assert!(pr.contains("draft or create a pull request"));

        let issue = format_issue_report(Some("flaky test"));
        assert!(issue.contains("Context          flaky test"));
        assert!(issue.contains("draft or create a GitHub issue"));
    }

    #[test]
    fn no_arg_commands_reject_unexpected_arguments() {
        assert!(validate_no_args("/commit", None).is_ok());

        let error = validate_no_args("/commit", Some("now"))
            .expect_err("unexpected arguments should fail")
            .to_string();
        assert!(error.contains("/commit does not accept arguments"));
        assert!(error.contains("Received: now"));
    }

    #[test]
    fn config_report_supports_section_views() {
        let report = render_config_report(Some("env")).expect("config report should render");
        assert!(report.contains("Merged section: env"));
        assert!(report.contains("Section status"));
        let plugins_report =
            render_config_report(Some("plugins")).expect("plugins config report should render");
        assert!(plugins_report.contains("Merged section: plugins"));
        let telemetry_report =
            render_config_report(Some("telemetry")).expect("telemetry config report should render");
        assert!(telemetry_report.contains("Merged section: telemetry"));
        assert!(telemetry_report.contains("Effective telemetry"));
    }

    #[test]
    fn telemetry_resolution_prefers_env_over_config() {
        let _guard = env_lock();
        std::env::set_var(ORBIT_TELEMETRY_PATH, "/tmp/from-env.jsonl");
        let config = orbit_runtime::RuntimeConfig::empty();

        let resolution = resolve_telemetry_config(Some(&config));
        assert_eq!(resolution.source, "env");
        assert_eq!(resolution.path.as_deref(), Some("/tmp/from-env.jsonl"));

        std::env::remove_var(ORBIT_TELEMETRY_PATH);
    }

    #[test]
    fn telemetry_report_uses_sectioned_layout() {
        let _guard = env_lock();
        std::env::remove_var(ORBIT_TELEMETRY_PATH);
        let report = render_telemetry_report(None).expect("telemetry report should render");
        assert!(report.contains("Telemetry"));
        assert!(report.contains("Enabled"));
        assert!(report.contains("Effective path"));
        assert!(report.contains("Effective source"));
        assert!(report.contains("Config file"));
    }

    #[test]
    fn telemetry_report_shows_highest_precedence_config_file() {
        let _guard = env_lock();
        std::env::remove_var(ORBIT_TELEMETRY_PATH);
        let cwd = temp_dir();
        fs::create_dir_all(cwd.join(".orbit")).expect("orbit dir should exist");
        fs::write(
            cwd.join(".orbit").join("settings.json"),
            r#"{"telemetry":{"enabled":true,"path":"project/log.jsonl"}}"#,
        )
        .expect("project settings");
        fs::write(
            cwd.join(".orbit").join("settings.local.json"),
            r#"{"telemetry":{"enabled":true,"path":"local/log.jsonl"}}"#,
        )
        .expect("local settings");

        let report = with_current_dir(&cwd, || {
            render_telemetry_report(None).expect("telemetry report should render")
        });
        assert!(report.contains("Effective path   local/log.jsonl"));
        assert!(report.contains(".orbit/settings.local.json"));

        fs::remove_dir_all(cwd).expect("cleanup temp dir");
    }

    #[test]
    fn telemetry_report_marks_config_as_shadowed_when_env_override_is_set() {
        let _guard = env_lock();
        let cwd = temp_dir();
        fs::create_dir_all(cwd.join(".orbit")).expect("orbit dir should exist");
        fs::write(
            cwd.join(".orbit").join("settings.local.json"),
            r#"{"telemetry":{"enabled":true,"path":"local/log.jsonl"}}"#,
        )
        .expect("local settings");
        std::env::set_var(ORBIT_TELEMETRY_PATH, "/tmp/from-env.jsonl");

        let report = with_current_dir(&cwd, || {
            render_telemetry_report(None).expect("telemetry report should render")
        });
        assert!(report.contains("Effective source env"));
        assert!(report.contains("Shadowed config"));
        assert!(report.contains(".orbit/settings.local.json"));
        assert!(report.contains("Env override     /tmp/from-env.jsonl"));

        std::env::remove_var(ORBIT_TELEMETRY_PATH);
        fs::remove_dir_all(cwd).expect("cleanup temp dir");
    }

    #[test]
    fn telemetry_report_status_target_shows_requested_scope_details() {
        let _guard = env_lock();
        std::env::remove_var(ORBIT_TELEMETRY_PATH);
        let cwd = temp_dir();
        fs::create_dir_all(cwd.join(".orbit")).expect("orbit dir should exist");
        fs::write(
            cwd.join(".orbit").join("settings.json"),
            r#"{"telemetry":{"enabled":true,"path":"project/log.jsonl"}}"#,
        )
        .expect("project settings");
        fs::write(
            cwd.join(".orbit").join("settings.local.json"),
            r#"{"telemetry":{"enabled":false,"path":"local/log.jsonl"}}"#,
        )
        .expect("local settings");

        let report = with_current_dir(&cwd, || {
            render_telemetry_report(Some("project")).expect("telemetry report should render")
        });
        assert!(report.contains("Target scope     project"));
        assert!(report.contains("Settings status  present"));
        assert!(report.contains("Target enabled   true"));
        assert!(report.contains("Target path      project/log.jsonl"));
        assert!(report.contains("Effective telemetry"));
        assert!(report.contains("Effective path   local/log.jsonl"));

        fs::remove_dir_all(cwd).expect("cleanup temp dir");
    }

    #[test]
    fn config_telemetry_report_shows_effective_precedence_details() {
        let _guard = env_lock();
        std::env::remove_var(ORBIT_TELEMETRY_PATH);
        let cwd = temp_dir();
        fs::create_dir_all(cwd.join(".orbit")).expect("orbit dir should exist");
        fs::write(
            cwd.join(".orbit").join("settings.json"),
            r#"{"telemetry":{"enabled":true,"path":"project/log.jsonl"}}"#,
        )
        .expect("project settings");
        fs::write(
            cwd.join(".orbit").join("settings.local.json"),
            r#"{"telemetry":{"enabled":true,"path":"local/log.jsonl"}}"#,
        )
        .expect("local settings");

        let report = with_current_dir(&cwd, || {
            render_config_report(Some("telemetry")).expect("telemetry config report should render")
        });
        assert!(report.contains("Merged section: telemetry"));
        assert!(report.contains("Effective telemetry"));
        assert!(report.contains("Effective path   local/log.jsonl"));
        assert!(report.contains(".orbit/settings.local.json"));

        fs::remove_dir_all(cwd).expect("cleanup temp dir");
    }

    #[test]
    fn config_telemetry_report_marks_shadowed_config_when_env_override_is_set() {
        let _guard = env_lock();
        let cwd = temp_dir();
        fs::create_dir_all(cwd.join(".orbit")).expect("orbit dir should exist");
        fs::write(
            cwd.join(".orbit").join("settings.local.json"),
            r#"{"telemetry":{"enabled":true,"path":"local/log.jsonl"}}"#,
        )
        .expect("local settings");
        std::env::set_var(ORBIT_TELEMETRY_PATH, "/tmp/from-env.jsonl");

        let report = with_current_dir(&cwd, || {
            render_config_report(Some("telemetry")).expect("telemetry config report should render")
        });
        assert!(report.contains("Effective source env"));
        assert!(report.contains("Shadowed config"));
        assert!(report.contains(".orbit/settings.local.json"));
        assert!(report.contains("Env override     /tmp/from-env.jsonl"));

        std::env::remove_var(ORBIT_TELEMETRY_PATH);
        fs::remove_dir_all(cwd).expect("cleanup temp dir");
    }

    #[test]
    fn config_report_marks_supported_sections_as_unset_when_missing() {
        let _guard = env_lock();
        let cwd = temp_dir();
        fs::create_dir_all(cwd.join(".orbit")).expect("orbit dir should exist");
        fs::write(
            cwd.join(".orbit").join("settings.json"),
            r#"{"model":"claude-sonnet-4-6"}"#,
        )
        .expect("project settings");

        let report = with_current_dir(&cwd, || {
            render_config_report(Some("hooks")).expect("hooks config report should render")
        });
        assert!(report.contains("Merged section: hooks"));
        assert!(report.contains(&report_row("Section status", "unset")));
        assert!(report.contains("  <unset>"));

        fs::remove_dir_all(cwd).expect("cleanup temp dir");
    }

    #[test]
    fn config_report_marks_supported_sections_as_set_when_present() {
        let _guard = env_lock();
        let cwd = temp_dir();
        fs::create_dir_all(cwd.join(".orbit")).expect("orbit dir should exist");
        fs::write(
            cwd.join(".orbit").join("settings.json"),
            r#"{"env":{"API_BASE_URL":"https://example.test"}}"#,
        )
        .expect("project settings");

        let report = with_current_dir(&cwd, || {
            render_config_report(Some("env")).expect("env config report should render")
        });
        assert!(report.contains("Merged section: env"));
        assert!(report.contains(&report_row("Section status", "set")));
        assert!(report.contains("\"API_BASE_URL\":\"https://example.test\""));

        fs::remove_dir_all(cwd).expect("cleanup temp dir");
    }

    #[test]
    fn config_report_marks_unsupported_sections_explicitly() {
        let _guard = env_lock();
        let cwd = temp_dir();
        fs::create_dir_all(&cwd).expect("cwd should exist");

        let report = with_current_dir(&cwd, || {
            render_config_report(Some("unknown")).expect("unknown config report should render")
        });
        assert!(report.contains("Merged section: unknown"));
        assert!(report.contains(&report_row("Section status", "unsupported")));
        assert!(report.contains(
            "Unsupported config section 'unknown'. Use env, hooks, model, telemetry, or plugins."
        ));

        fs::remove_dir_all(cwd).expect("cleanup temp dir");
    }

    #[test]
    fn config_telemetry_json_includes_effective_resolution_details() {
        let _guard = env_lock();
        std::env::remove_var(ORBIT_TELEMETRY_PATH);
        let cwd = temp_dir();
        fs::create_dir_all(cwd.join(".orbit")).expect("orbit dir should exist");
        fs::write(
            cwd.join(".orbit").join("settings.json"),
            r#"{"telemetry":{"enabled":true,"path":"project/log.jsonl"}}"#,
        )
        .expect("project settings");
        fs::write(
            cwd.join(".orbit").join("settings.local.json"),
            r#"{"telemetry":{"enabled":true,"path":"local/log.jsonl"}}"#,
        )
        .expect("local settings");

        let value = with_current_dir(&cwd, || {
            config_json_value(Some("telemetry")).expect("telemetry config json should render")
        });
        assert_eq!(value["kind"], "config");
        assert_eq!(value["section"], "telemetry");
        assert_eq!(value["section_supported"], true);
        assert_eq!(value["section_present"], true);
        assert_eq!(value["section_status"], "set");
        assert_eq!(value["merged_section"]["path"], "local/log.jsonl");
        assert_eq!(value["effective"]["path"], "local/log.jsonl");
        assert_eq!(value["effective"]["effective_source"], "config");
        assert!(value["effective"]["config_source_path"]
            .as_str()
            .expect("config path")
            .ends_with(".orbit/settings.local.json"));

        fs::remove_dir_all(cwd).expect("cleanup temp dir");
    }

    #[test]
    fn config_telemetry_json_marks_shadowed_config_when_env_override_is_set() {
        let _guard = env_lock();
        let cwd = temp_dir();
        fs::create_dir_all(cwd.join(".orbit")).expect("orbit dir should exist");
        fs::write(
            cwd.join(".orbit").join("settings.local.json"),
            r#"{"telemetry":{"enabled":true,"path":"local/log.jsonl"}}"#,
        )
        .expect("local settings");
        std::env::set_var(ORBIT_TELEMETRY_PATH, "/tmp/from-env.jsonl");

        let value = with_current_dir(&cwd, || {
            config_json_value(Some("telemetry")).expect("telemetry config json should render")
        });
        assert_eq!(value["effective"]["effective_source"], "env");
        assert_eq!(value["effective"]["env_override"], "/tmp/from-env.jsonl");
        assert_eq!(value["effective"]["config_shadowed_by_env"], true);
        assert!(value["effective"]["config_source_path"]
            .as_str()
            .expect("config path")
            .ends_with(".orbit/settings.local.json"));

        std::env::remove_var(ORBIT_TELEMETRY_PATH);
        fs::remove_dir_all(cwd).expect("cleanup temp dir");
    }

    #[test]
    fn telemetry_status_json_includes_requested_target_details() {
        let _guard = env_lock();
        std::env::remove_var(ORBIT_TELEMETRY_PATH);
        let cwd = temp_dir();
        fs::create_dir_all(cwd.join(".orbit")).expect("orbit dir should exist");
        fs::write(
            cwd.join(".orbit").join("settings.json"),
            r#"{"telemetry":{"enabled":true,"path":"project/log.jsonl"}}"#,
        )
        .expect("project settings");
        fs::write(
            cwd.join(".orbit").join("settings.local.json"),
            r#"{"telemetry":{"enabled":false,"path":"local/log.jsonl"}}"#,
        )
        .expect("local settings");

        let value = with_current_dir(&cwd, || {
            let runtime_config = ConfigLoader::default_for(&cwd)
                .load()
                .expect("runtime config should load");
            telemetry_status_json_value(&runtime_config, Some("project"))
                .expect("telemetry status json should render")
        });

        assert_eq!(value["kind"], "telemetry");
        assert_eq!(value["target"]["scope"], "project");
        assert_eq!(value["target"]["settings_status"], "present");
        assert_eq!(value["target"]["config_enabled"], true);
        assert_eq!(value["target"]["config_path"], "project/log.jsonl");
        assert_eq!(value["path"], "local/log.jsonl");

        fs::remove_dir_all(cwd).expect("cleanup temp dir");
    }

    #[test]
    fn config_json_marks_supported_sections_as_unset_when_missing() {
        let _guard = env_lock();
        std::env::remove_var(ORBIT_TELEMETRY_PATH);
        let cwd = temp_dir();
        fs::create_dir_all(cwd.join(".orbit")).expect("orbit dir should exist");
        fs::write(
            cwd.join(".orbit").join("settings.json"),
            r#"{"model":"claude-sonnet-4-6"}"#,
        )
        .expect("project settings");

        let value = with_current_dir(&cwd, || {
            config_json_value(Some("hooks")).expect("hooks config json should render")
        });
        assert_eq!(value["kind"], "config");
        assert_eq!(value["section"], "hooks");
        assert_eq!(value["section_supported"], true);
        assert_eq!(value["section_present"], false);
        assert_eq!(value["section_status"], "unset");
        assert!(value["merged_section"].is_null());

        fs::remove_dir_all(cwd).expect("cleanup temp dir");
    }

    #[test]
    fn config_json_marks_unsupported_sections_explicitly() {
        let _guard = env_lock();
        let cwd = temp_dir();
        fs::create_dir_all(&cwd).expect("cwd should exist");

        let value = with_current_dir(&cwd, || {
            config_json_value(Some("unknown")).expect("unsupported config json should render")
        });
        assert_eq!(value["kind"], "config");
        assert_eq!(value["section"], "unknown");
        assert_eq!(value["status"], "unsupported");
        assert_eq!(value["section_supported"], false);
        assert_eq!(value["section_present"], false);
        assert_eq!(value["section_status"], "unsupported");
        assert!(value["supported_sections"].is_array());

        fs::remove_dir_all(cwd).expect("cleanup temp dir");
    }

    #[test]
    fn telemetry_update_writes_project_settings_json() {
        let _guard = env_lock();
        std::env::remove_var(ORBIT_TELEMETRY_PATH);
        let cwd = temp_dir();
        fs::create_dir_all(&cwd).expect("cwd should exist");

        let settings_path =
            update_project_telemetry_settings(&cwd, true, None).expect("telemetry should update");
        let written = fs::read_to_string(&settings_path).expect("settings should exist");
        let parsed: serde_json::Value = serde_json::from_str(&written).expect("settings json");

        assert_eq!(settings_path, cwd.join(".orbit").join("settings.json"));
        assert_eq!(parsed["telemetry"]["enabled"], true);
        assert_eq!(
            parsed["telemetry"]["path"],
            cwd.join(".orbit")
                .join("telemetry.jsonl")
                .display()
                .to_string()
        );

        fs::remove_dir_all(cwd).expect("cleanup temp dir");
    }

    #[test]
    fn telemetry_update_preserves_existing_path_when_disabling() {
        let _guard = env_lock();
        std::env::remove_var(ORBIT_TELEMETRY_PATH);
        let cwd = temp_dir();
        fs::create_dir_all(cwd.join(".orbit")).expect("orbit dir should exist");
        fs::write(
            cwd.join(".orbit").join("settings.json"),
            r#"{"telemetry":{"enabled":true,"path":"custom/log.jsonl"},"model":"claude-sonnet"}"#,
        )
        .expect("seed settings");

        update_project_telemetry_settings(&cwd, false, None).expect("telemetry should update");
        let written = fs::read_to_string(cwd.join(".orbit").join("settings.json"))
            .expect("settings should exist");
        let parsed: serde_json::Value = serde_json::from_str(&written).expect("settings json");
        assert_eq!(parsed["telemetry"]["enabled"], false);
        assert_eq!(parsed["telemetry"]["path"], "custom/log.jsonl");
        assert_eq!(parsed["model"], "claude-sonnet");

        fs::remove_dir_all(cwd).expect("cleanup temp dir");
    }

    #[test]
    fn telemetry_update_can_write_local_settings_json() {
        let _guard = env_lock();
        std::env::remove_var(ORBIT_TELEMETRY_PATH);
        let cwd = temp_dir();
        fs::create_dir_all(&cwd).expect("cwd should exist");

        let settings_path = update_project_telemetry_settings(&cwd, true, Some("local"))
            .expect("telemetry should update");
        let written = fs::read_to_string(&settings_path).expect("settings should exist");
        let parsed: serde_json::Value = serde_json::from_str(&written).expect("settings json");

        assert_eq!(
            settings_path,
            cwd.join(".orbit").join("settings.local.json")
        );
        assert_eq!(parsed["telemetry"]["enabled"], true);

        fs::remove_dir_all(cwd).expect("cleanup temp dir");
    }

    #[test]
    fn memory_report_uses_sectioned_layout() {
        let report = render_memory_report().expect("memory report should render");
        assert!(report.contains("Memory"));
        assert!(report.contains("Working directory"));
        assert!(report.contains("Instruction files"));
        assert!(report.contains("Discovered files"));
    }

    #[test]
    fn config_report_uses_sectioned_layout() {
        let report = render_config_report(None).expect("config report should render");
        assert!(report.contains("Config"));
        assert!(report.contains("Discovered files"));
        assert!(report.contains("Merged JSON"));
    }

    #[test]
    fn parses_git_status_metadata() {
        let _guard = env_lock();
        let temp_root = temp_dir();
        fs::create_dir_all(&temp_root).expect("root dir");
        let (project_root, branch) = parse_git_status_metadata_for(
            &temp_root,
            Some(
                "## rcc/cli...origin/rcc/cli
 M src/main.rs",
            ),
        );
        assert_eq!(branch.as_deref(), Some("rcc/cli"));
        assert!(project_root.is_none());
        fs::remove_dir_all(temp_root).expect("cleanup temp dir");
    }

    #[test]
    fn parses_detached_head_from_status_snapshot() {
        let _guard = env_lock();
        assert_eq!(
            parse_git_status_branch(Some(
                "## HEAD (no branch)
 M src/main.rs"
            )),
            Some("detached HEAD".to_string())
        );
    }

    #[test]
    fn parses_git_workspace_summary_counts() {
        let summary = parse_git_workspace_summary(Some(
            "## feature/ux
M  src/main.rs
 M README.md
?? notes.md
UU conflicted.rs",
        ));

        assert_eq!(
            summary,
            GitWorkspaceSummary {
                changed_files: 4,
                staged_files: 2,
                unstaged_files: 2,
                untracked_files: 1,
                conflicted_files: 1,
            }
        );
        assert_eq!(
            summary.headline(),
            "dirty · 4 files · 2 staged, 2 unstaged, 1 untracked, 1 conflicted"
        );
    }

    #[test]
    fn render_diff_report_shows_clean_tree_for_committed_repo() {
        let _guard = env_lock();
        let root = temp_dir();
        fs::create_dir_all(&root).expect("root dir");
        git(&["init", "--quiet"], &root);
        git(&["config", "user.email", "tests@example.com"], &root);
        git(&["config", "user.name", "Rusty Claude Tests"], &root);
        fs::write(root.join("tracked.txt"), "hello\n").expect("write file");
        git(&["add", "tracked.txt"], &root);
        git(&["commit", "-m", "init", "--quiet"], &root);

        let report = render_diff_report_for(&root).expect("diff report should render");
        assert!(report.contains("clean working tree"));

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn render_diff_report_includes_staged_and_unstaged_sections() {
        let _guard = env_lock();
        let root = temp_dir();
        fs::create_dir_all(&root).expect("root dir");
        git(&["init", "--quiet"], &root);
        git(&["config", "user.email", "tests@example.com"], &root);
        git(&["config", "user.name", "Rusty Claude Tests"], &root);
        fs::write(root.join("tracked.txt"), "hello\n").expect("write file");
        git(&["add", "tracked.txt"], &root);
        git(&["commit", "-m", "init", "--quiet"], &root);

        fs::write(root.join("tracked.txt"), "hello\nstaged\n").expect("update file");
        git(&["add", "tracked.txt"], &root);
        fs::write(root.join("tracked.txt"), "hello\nstaged\nunstaged\n")
            .expect("update file twice");

        let report = render_diff_report_for(&root).expect("diff report should render");
        assert!(report.contains("Staged changes:"));
        assert!(report.contains("Unstaged changes:"));
        assert!(report.contains("tracked.txt"));

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn render_diff_report_omits_ignored_files() {
        let _guard = env_lock();
        let root = temp_dir();
        fs::create_dir_all(&root).expect("root dir");
        git(&["init", "--quiet"], &root);
        git(&["config", "user.email", "tests@example.com"], &root);
        git(&["config", "user.name", "Rusty Claude Tests"], &root);
        fs::write(root.join(".gitignore"), ".omx/\nignored.txt\n").expect("write gitignore");
        fs::write(root.join("tracked.txt"), "hello\n").expect("write tracked");
        git(&["add", ".gitignore", "tracked.txt"], &root);
        git(&["commit", "-m", "init", "--quiet"], &root);
        fs::create_dir_all(root.join(".omx")).expect("write omx dir");
        fs::write(root.join(".omx").join("state.json"), "{}").expect("write ignored omx");
        fs::write(root.join("ignored.txt"), "secret\n").expect("write ignored file");
        fs::write(root.join("tracked.txt"), "hello\nworld\n").expect("write tracked change");

        let report = render_diff_report_for(&root).expect("diff report should render");
        assert!(report.contains("tracked.txt"));
        assert!(!report.contains("+++ b/ignored.txt"));
        assert!(!report.contains("+++ b/.omx/state.json"));

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn resume_diff_command_renders_report_for_saved_session() {
        let _guard = env_lock();
        let root = temp_dir();
        fs::create_dir_all(&root).expect("root dir");
        git(&["init", "--quiet"], &root);
        git(&["config", "user.email", "tests@example.com"], &root);
        git(&["config", "user.name", "Rusty Claude Tests"], &root);
        fs::write(root.join("tracked.txt"), "hello\n").expect("write tracked");
        git(&["add", "tracked.txt"], &root);
        git(&["commit", "-m", "init", "--quiet"], &root);
        fs::write(root.join("tracked.txt"), "hello\nworld\n").expect("modify tracked");
        let session_path = root.join("session.json");
        Session::new()
            .save_to_path(&session_path)
            .expect("session should save");

        let session = Session::load_from_path(&session_path).expect("session should load");
        let outcome = with_current_dir(&root, || {
            run_resume_command(&session_path, &session, &SlashCommand::Diff)
                .expect("resume diff should work")
        });
        let message = outcome.message.expect("diff message should exist");
        assert!(message.contains("Unstaged changes:"));
        assert!(message.contains("tracked.txt"));

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn status_context_reads_real_workspace_metadata() {
        let context = status_context(None).expect("status context should load");
        assert!(context.cwd.is_absolute());
        assert!(context.discovered_config_files >= context.loaded_config_files);
        assert!(context.loaded_config_files <= context.discovered_config_files);
    }

    #[test]
    fn normalizes_supported_permission_modes() {
        assert_eq!(normalize_permission_mode("read-only"), Some("read-only"));
        assert_eq!(
            normalize_permission_mode("workspace-write"),
            Some("workspace-write")
        );
        assert_eq!(
            normalize_permission_mode("danger-full-access"),
            Some("danger-full-access")
        );
        assert_eq!(normalize_permission_mode("unknown"), None);
    }

    #[test]
    fn clear_command_requires_explicit_confirmation_flag() {
        assert_eq!(
            SlashCommand::parse("/clear"),
            Ok(Some(SlashCommand::Clear { confirm: false }))
        );
        assert_eq!(
            SlashCommand::parse("/clear --confirm"),
            Ok(Some(SlashCommand::Clear { confirm: true }))
        );
    }

    #[test]
    fn parses_resume_and_config_slash_commands() {
        assert_eq!(
            SlashCommand::parse("/resume saved-session.jsonl"),
            Ok(Some(SlashCommand::Resume {
                session_path: Some("saved-session.jsonl".to_string())
            }))
        );
        assert_eq!(
            SlashCommand::parse("/clear --confirm"),
            Ok(Some(SlashCommand::Clear { confirm: true }))
        );
        assert_eq!(
            SlashCommand::parse("/config"),
            Ok(Some(SlashCommand::Config { section: None }))
        );
        assert_eq!(
            SlashCommand::parse("/config env"),
            Ok(Some(SlashCommand::Config {
                section: Some("env".to_string())
            }))
        );
        assert_eq!(
            SlashCommand::parse("/config telemetry"),
            Ok(Some(SlashCommand::Config {
                section: Some("telemetry".to_string())
            }))
        );
        assert_eq!(
            SlashCommand::parse("/config unknown"),
            Ok(Some(SlashCommand::Config {
                section: Some("unknown".to_string())
            }))
        );
        assert_eq!(
            SlashCommand::parse("/telemetry status"),
            Ok(Some(SlashCommand::Telemetry {
                action: Some("status".to_string()),
                target: None,
            }))
        );
        assert_eq!(
            SlashCommand::parse("/telemetry on"),
            Ok(Some(SlashCommand::Telemetry {
                action: Some("on".to_string()),
                target: None,
            }))
        );
        assert_eq!(
            SlashCommand::parse("/telemetry on local"),
            Ok(Some(SlashCommand::Telemetry {
                action: Some("on".to_string()),
                target: Some("local".to_string()),
            }))
        );
        assert_eq!(
            SlashCommand::parse("/memory"),
            Ok(Some(SlashCommand::Memory))
        );
        assert_eq!(SlashCommand::parse("/init"), Ok(Some(SlashCommand::Init)));
        assert_eq!(
            SlashCommand::parse("/session fork incident-review"),
            Ok(Some(SlashCommand::Session {
                action: Some("fork".to_string()),
                target: Some("incident-review".to_string())
            }))
        );
    }

    #[test]
    fn help_mentions_jsonl_resume_examples() {
        let mut help = Vec::new();
        print_help_to(&mut help).expect("help should render");
        let help = String::from_utf8(help).expect("help should be utf8");
        assert!(help.contains("orbit --resume [SESSION.jsonl|session-id|latest]"));
        assert!(help.contains("Use `latest` with --resume, /resume, or /session switch"));
        assert!(help.contains("orbit --resume latest"));
        assert!(help.contains("orbit --resume latest /status /diff /export notes.txt"));
        assert!(help.contains("orbit --output-format json config telemetry"));
    }

    #[test]
    fn managed_sessions_default_to_jsonl_and_resolve_legacy_json() {
        let _guard = cwd_lock().lock().expect("cwd lock");
        let workspace = temp_workspace("session-resolution");
        std::fs::create_dir_all(&workspace).expect("workspace should create");
        let previous = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(&workspace).expect("switch cwd");

        let handle = create_managed_session_handle("session-alpha").expect("jsonl handle");
        assert!(handle.path.ends_with("session-alpha.jsonl"));

        let legacy_path = workspace.join(".orbit/sessions/legacy.json");
        std::fs::create_dir_all(
            legacy_path
                .parent()
                .expect("legacy path should have parent directory"),
        )
        .expect("session dir should exist");
        Session::new()
            .with_persistence_path(legacy_path.clone())
            .save_to_path(&legacy_path)
            .expect("legacy session should save");

        let resolved = resolve_session_reference("legacy").expect("legacy session should resolve");
        assert_eq!(
            resolved
                .path
                .canonicalize()
                .expect("resolved path should exist"),
            legacy_path
                .canonicalize()
                .expect("legacy path should exist")
        );

        std::env::set_current_dir(previous).expect("restore cwd");
        std::fs::remove_dir_all(workspace).expect("workspace should clean up");
    }

    #[test]
    fn latest_session_alias_resolves_most_recent_managed_session() {
        let _guard = cwd_lock().lock().expect("cwd lock");
        let workspace = temp_workspace("latest-session-alias");
        std::fs::create_dir_all(&workspace).expect("workspace should create");
        let previous = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(&workspace).expect("switch cwd");

        let older = create_managed_session_handle("session-older").expect("older handle");
        Session::new()
            .with_persistence_path(older.path.clone())
            .save_to_path(&older.path)
            .expect("older session should save");
        std::thread::sleep(Duration::from_millis(20));
        let newer = create_managed_session_handle("session-newer").expect("newer handle");
        Session::new()
            .with_persistence_path(newer.path.clone())
            .save_to_path(&newer.path)
            .expect("newer session should save");

        let resolved = resolve_session_reference("latest").expect("latest session should resolve");
        assert_eq!(
            resolved
                .path
                .canonicalize()
                .expect("resolved path should exist"),
            newer.path.canonicalize().expect("newer path should exist")
        );

        std::env::set_current_dir(previous).expect("restore cwd");
        std::fs::remove_dir_all(workspace).expect("workspace should clean up");
    }

    #[test]
    fn unknown_slash_command_guidance_suggests_nearby_commands() {
        let message = format_unknown_slash_command("stats");
        assert!(message.contains("Unknown slash command: /stats"));
        assert!(message.contains("/status"));
        assert!(message.contains("/help"));
    }

    #[test]
    fn unknown_omc_slash_command_guidance_explains_runtime_gap() {
        let message = format_unknown_slash_command("oh-my-claudecode:hud");
        assert!(message.contains("Unknown slash command: /oh-my-claudecode:hud"));
        assert!(message.contains("Claude Code/OMC plugin command"));
        assert!(message.contains("does not yet load plugin slash commands"));
    }

    #[test]
    fn resume_usage_mentions_latest_shortcut() {
        let usage = render_resume_usage();
        assert!(usage.contains("/resume <session-path|session-id|latest>"));
        assert!(usage.contains(".orbit/sessions/<session-id>.jsonl"));
        assert!(usage.contains("/session list"));
    }

    fn cwd_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn temp_workspace(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("orbit-cli-{label}-{nanos}"))
    }

    #[test]
    fn init_template_mentions_detected_rust_workspace() {
        let _guard = cwd_lock()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let rendered = crate::init::render_init_agents_md(&workspace_root);
        assert!(rendered.contains("# AGENTS.md"));
        assert!(rendered.contains("cargo clippy --workspace --all-targets -- -D warnings"));
    }

    #[test]
    fn converts_tool_roundtrip_messages() {
        let messages = vec![
            ConversationMessage::user_text("hello"),
            ConversationMessage::assistant(vec![ContentBlock::ToolUse {
                id: "tool-1".to_string(),
                name: "bash".to_string(),
                input: "{\"command\":\"pwd\"}".to_string(),
            }]),
            ConversationMessage {
                role: MessageRole::Tool,
                blocks: vec![ContentBlock::ToolResult {
                    tool_use_id: "tool-1".to_string(),
                    tool_name: "bash".to_string(),
                    output: "ok".to_string(),
                    is_error: false,
                }],
                usage: None,
            },
        ];

        let converted = super::convert_messages(&messages);
        assert_eq!(converted.len(), 3);
        assert_eq!(converted[1].role, "assistant");
        assert_eq!(converted[2].role, "user");
    }
    #[test]
    fn repl_help_mentions_history_completion_and_multiline() {
        let help = render_repl_help();
        assert!(help.contains("Up/Down"));
        assert!(help.contains("Tab"));
        assert!(help.contains("Shift+Enter/Ctrl+J"));
    }

    #[test]
    fn tool_rendering_helpers_compact_output() {
        let start = format_tool_call_start("read_file", r#"{"path":"src/main.rs"}"#);
        assert!(start.contains("read_file"));
        assert!(start.contains("src/main.rs"));

        let done = format_tool_result(
            "read_file",
            r#"{"file":{"filePath":"src/main.rs","content":"hello","numLines":1,"startLine":1,"totalLines":1}}"#,
            false,
        );
        assert!(done.contains("📄 Read src/main.rs"));
        assert!(done.contains("hello"));
    }

    #[test]
    fn tool_rendering_truncates_large_read_output_for_display_only() {
        let content = (0..200)
            .map(|index| format!("line {index:03}"))
            .collect::<Vec<_>>()
            .join("\n");
        let output = json!({
            "file": {
                "filePath": "src/main.rs",
                "content": content,
                "numLines": 200,
                "startLine": 1,
                "totalLines": 200
            }
        })
        .to_string();

        let rendered = format_tool_result("read_file", &output, false);

        assert!(rendered.contains("line 000"));
        assert!(rendered.contains("line 079"));
        assert!(!rendered.contains("line 199"));
        assert!(rendered.contains("full result preserved in session"));
        assert!(output.contains("line 199"));
    }

    #[test]
    fn tool_rendering_truncates_large_bash_output_for_display_only() {
        let stdout = (0..120)
            .map(|index| format!("stdout {index:03}"))
            .collect::<Vec<_>>()
            .join("\n");
        let output = json!({
            "stdout": stdout,
            "stderr": "",
            "returnCodeInterpretation": "completed successfully"
        })
        .to_string();

        let rendered = format_tool_result("bash", &output, false);

        assert!(rendered.contains("stdout 000"));
        assert!(rendered.contains("stdout 059"));
        assert!(!rendered.contains("stdout 119"));
        assert!(rendered.contains("full result preserved in session"));
        assert!(output.contains("stdout 119"));
    }

    #[test]
    fn tool_rendering_truncates_generic_long_output_for_display_only() {
        let items = (0..120)
            .map(|index| format!("payload {index:03}"))
            .collect::<Vec<_>>();
        let output = json!({
            "summary": "plugin payload",
            "items": items,
        })
        .to_string();

        let rendered = format_tool_result("plugin_echo", &output, false);

        assert!(rendered.contains("plugin_echo"));
        assert!(rendered.contains("payload 000"));
        assert!(rendered.contains("payload 040"));
        assert!(!rendered.contains("payload 080"));
        assert!(!rendered.contains("payload 119"));
        assert!(rendered.contains("full result preserved in session"));
        assert!(output.contains("payload 119"));
    }

    #[test]
    fn tool_rendering_truncates_raw_generic_output_for_display_only() {
        let output = (0..120)
            .map(|index| format!("raw {index:03}"))
            .collect::<Vec<_>>()
            .join("\n");

        let rendered = format_tool_result("plugin_echo", &output, false);

        assert!(rendered.contains("plugin_echo"));
        assert!(rendered.contains("raw 000"));
        assert!(rendered.contains("raw 059"));
        assert!(!rendered.contains("raw 119"));
        assert!(rendered.contains("full result preserved in session"));
        assert!(output.contains("raw 119"));
    }

    #[test]
    fn ultraplan_progress_lines_include_phase_step_and_elapsed_status() {
        let snapshot = InternalPromptProgressState {
            command_label: "Ultraplan",
            task_label: "ship plugin progress".to_string(),
            step: 3,
            phase: "running read_file".to_string(),
            detail: Some("reading crates/cli/src/main.rs".to_string()),
            saw_final_text: false,
        };

        let started = format_internal_prompt_progress_line(
            InternalPromptProgressEvent::Started,
            &snapshot,
            Duration::from_secs(0),
            None,
        );
        let heartbeat = format_internal_prompt_progress_line(
            InternalPromptProgressEvent::Heartbeat,
            &snapshot,
            Duration::from_secs(9),
            None,
        );
        let completed = format_internal_prompt_progress_line(
            InternalPromptProgressEvent::Complete,
            &snapshot,
            Duration::from_secs(12),
            None,
        );
        let failed = format_internal_prompt_progress_line(
            InternalPromptProgressEvent::Failed,
            &snapshot,
            Duration::from_secs(12),
            Some("network timeout"),
        );

        assert!(started.contains("planning started"));
        assert!(started.contains("current step 3"));
        assert!(heartbeat.contains("heartbeat"));
        assert!(heartbeat.contains("9s elapsed"));
        assert!(heartbeat.contains("phase running read_file"));
        assert!(completed.contains("completed"));
        assert!(completed.contains("3 steps total"));
        assert!(failed.contains("failed"));
        assert!(failed.contains("network timeout"));
    }

    #[test]
    fn describe_tool_progress_summarizes_known_tools() {
        assert_eq!(
            describe_tool_progress("read_file", r#"{"path":"src/main.rs"}"#),
            "reading src/main.rs"
        );
        assert!(
            describe_tool_progress("bash", r#"{"command":"cargo test -p orbit-cli"}"#)
                .contains("cargo test -p orbit-cli")
        );
        assert_eq!(
            describe_tool_progress("grep_search", r#"{"pattern":"ultraplan","path":"rust"}"#),
            "grep `ultraplan` in rust"
        );
    }

    #[test]
    fn push_output_block_renders_markdown_text() {
        let mut out = Vec::new();
        let mut events = Vec::new();
        let mut pending_tool = None;
        let mut block_has_thinking_summary = false;

        push_output_block(
            OutputContentBlock::Text {
                text: "# Heading".to_string(),
            },
            &mut out,
            &mut events,
            &mut pending_tool,
            false,
            &mut block_has_thinking_summary,
        )
        .expect("text block should render");

        let rendered = String::from_utf8(out).expect("utf8");
        assert!(rendered.contains("Heading"));
        assert!(rendered.contains('\u{1b}'));
    }

    #[test]
    fn push_output_block_skips_empty_object_prefix_for_tool_streams() {
        let mut out = Vec::new();
        let mut events = Vec::new();
        let mut pending_tool = None;
        let mut block_has_thinking_summary = false;

        push_output_block(
            OutputContentBlock::ToolUse {
                id: "tool-1".to_string(),
                name: "read_file".to_string(),
                input: json!({}),
            },
            &mut out,
            &mut events,
            &mut pending_tool,
            true,
            &mut block_has_thinking_summary,
        )
        .expect("tool block should accumulate");

        assert!(events.is_empty());
        assert_eq!(
            pending_tool,
            Some(("tool-1".to_string(), "read_file".to_string(), String::new(),))
        );
    }

    #[test]
    fn response_to_events_preserves_empty_object_json_input_outside_streaming() {
        let mut out = Vec::new();
        let events = response_to_events(
            MessageResponse {
                id: "msg-1".to_string(),
                kind: "message".to_string(),
                model: "claude-opus-4-6".to_string(),
                role: "assistant".to_string(),
                content: vec![OutputContentBlock::ToolUse {
                    id: "tool-1".to_string(),
                    name: "read_file".to_string(),
                    input: json!({}),
                }],
                stop_reason: Some("tool_use".to_string()),
                stop_sequence: None,
                usage: Usage {
                    input_tokens: 1,
                    output_tokens: 1,
                    cache_creation_input_tokens: 0,
                    cache_read_input_tokens: 0,
                },
                request_id: None,
            },
            &mut out,
        )
        .expect("response conversion should succeed");

        assert!(matches!(
            &events[0],
            AssistantEvent::ToolUse { name, input, .. }
                if name == "read_file" && input == "{}"
        ));
    }

    #[test]
    fn response_to_events_preserves_non_empty_json_input_outside_streaming() {
        let mut out = Vec::new();
        let events = response_to_events(
            MessageResponse {
                id: "msg-2".to_string(),
                kind: "message".to_string(),
                model: "claude-opus-4-6".to_string(),
                role: "assistant".to_string(),
                content: vec![OutputContentBlock::ToolUse {
                    id: "tool-2".to_string(),
                    name: "read_file".to_string(),
                    input: json!({ "path": "Cargo.toml" }),
                }],
                stop_reason: Some("tool_use".to_string()),
                stop_sequence: None,
                usage: Usage {
                    input_tokens: 1,
                    output_tokens: 1,
                    cache_creation_input_tokens: 0,
                    cache_read_input_tokens: 0,
                },
                request_id: None,
            },
            &mut out,
        )
        .expect("response conversion should succeed");

        assert!(matches!(
            &events[0],
            AssistantEvent::ToolUse { name, input, .. }
                if name == "read_file" && input == "{\"path\":\"Cargo.toml\"}"
        ));
    }

    #[test]
    fn response_to_events_renders_collapsed_thinking_summary() {
        let mut out = Vec::new();
        let events = response_to_events(
            MessageResponse {
                id: "msg-3".to_string(),
                kind: "message".to_string(),
                model: "claude-opus-4-6".to_string(),
                role: "assistant".to_string(),
                content: vec![
                    OutputContentBlock::Thinking {
                        thinking: "step 1".to_string(),
                        signature: Some("sig_123".to_string()),
                    },
                    OutputContentBlock::Text {
                        text: "Final answer".to_string(),
                    },
                ],
                stop_reason: Some("end_turn".to_string()),
                stop_sequence: None,
                usage: Usage {
                    input_tokens: 1,
                    output_tokens: 1,
                    cache_creation_input_tokens: 0,
                    cache_read_input_tokens: 0,
                },
                request_id: None,
            },
            &mut out,
        )
        .expect("response conversion should succeed");

        assert!(matches!(
            &events[0],
            AssistantEvent::TextDelta(text) if text == "Final answer"
        ));
        let rendered = String::from_utf8(out).expect("utf8");
        assert!(rendered.contains("▶ Thinking (6 chars hidden)"));
        assert!(!rendered.contains("step 1"));
    }

    #[test]
    fn build_runtime_plugin_state_merges_plugin_hooks_into_runtime_features() {
        let config_home = temp_dir();
        let workspace = temp_dir();
        let source_root = temp_dir();
        fs::create_dir_all(&config_home).expect("config home");
        fs::create_dir_all(&workspace).expect("workspace");
        fs::create_dir_all(&source_root).expect("source root");
        write_plugin_fixture(&source_root, "hook-runtime-demo", true, false);

        let mut manager = PluginManager::new(PluginManagerConfig::new(&config_home));
        manager
            .install(source_root.to_str().expect("utf8 source path"))
            .expect("plugin install should succeed");
        let loader = ConfigLoader::new(&workspace, &config_home);
        let runtime_config = loader.load().expect("runtime config should load");
        let state = build_runtime_plugin_state_with_loader(&workspace, &loader, &runtime_config)
            .expect("plugin state should load");
        let pre_hooks = state.feature_config.hooks().pre_tool_use();
        assert_eq!(pre_hooks.len(), 1);
        assert!(
            pre_hooks[0].ends_with("hooks/pre.sh"),
            "expected installed plugin hook path, got {pre_hooks:?}"
        );

        let _ = fs::remove_dir_all(config_home);
        let _ = fs::remove_dir_all(workspace);
        let _ = fs::remove_dir_all(source_root);
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn build_runtime_plugin_state_discovers_mcp_tools_and_surfaces_pending_servers() {
        let config_home = temp_dir();
        let workspace = temp_dir();
        fs::create_dir_all(&config_home).expect("config home");
        fs::create_dir_all(&workspace).expect("workspace");
        let script_path = workspace.join("fixture-mcp.py");
        write_mcp_server_fixture(&script_path);
        fs::write(
            config_home.join("settings.json"),
            format!(
                r#"{{
                  "mcpServers": {{
                    "alpha": {{
                      "command": "python3",
                      "args": ["{}"]
                    }},
                    "broken": {{
                      "command": "python3",
                      "args": ["-c", "import sys; sys.exit(0)"]
                    }}
                  }}
                }}"#,
                script_path.to_string_lossy()
            ),
        )
        .expect("write mcp settings");

        let loader = ConfigLoader::new(&workspace, &config_home);
        let runtime_config = loader.load().expect("runtime config should load");
        let state = build_runtime_plugin_state_with_loader(&workspace, &loader, &runtime_config)
            .expect("runtime plugin state should load");

        let allowed = state
            .tool_registry
            .normalize_allowed_tools(&["mcp__alpha__echo".to_string(), "MCPTool".to_string()])
            .expect("mcp tools should be allow-listable")
            .expect("allow-list should exist");
        assert!(allowed.contains("mcp__alpha__echo"));
        assert!(allowed.contains("MCPTool"));

        let mut executor = CliToolExecutor::new(
            "test-mcp-session".to_string(),
            None,
            false,
            state.tool_registry.clone(),
            state.mcp_state.clone(),
        );

        let tool_output = executor
            .execute("mcp__alpha__echo", r#"{"text":"hello"}"#)
            .expect("discovered mcp tool should execute");
        let tool_json: serde_json::Value =
            serde_json::from_str(&tool_output).expect("tool output should be json");
        assert_eq!(tool_json["structuredContent"]["echoed"], "hello");

        let wrapped_output = executor
            .execute(
                "MCPTool",
                r#"{"qualifiedName":"mcp__alpha__echo","arguments":{"text":"wrapped"}}"#,
            )
            .expect("generic mcp wrapper should execute");
        let wrapped_json: serde_json::Value =
            serde_json::from_str(&wrapped_output).expect("wrapped output should be json");
        assert_eq!(wrapped_json["structuredContent"]["echoed"], "wrapped");

        let search_output = executor
            .execute("ToolSearch", r#"{"query":"alpha echo","max_results":5}"#)
            .expect("tool search should execute");
        let search_json: serde_json::Value =
            serde_json::from_str(&search_output).expect("search output should be json");
        assert_eq!(search_json["matches"][0], "mcp__alpha__echo");
        assert_eq!(search_json["pending_mcp_servers"][0], "broken");
        assert_eq!(
            search_json["mcp_degraded"]["failed_servers"][0]["server_name"],
            "broken"
        );
        assert_eq!(
            search_json["mcp_degraded"]["failed_servers"][0]["phase"],
            "tool_discovery"
        );
        assert_eq!(
            search_json["mcp_degraded"]["available_tools"][0],
            "mcp__alpha__echo"
        );

        let listed = executor
            .execute("ListMcpResourcesTool", r#"{"server":"alpha"}"#)
            .expect("resources should list");
        let listed_json: serde_json::Value =
            serde_json::from_str(&listed).expect("resource output should be json");
        assert_eq!(listed_json["resources"][0]["uri"], "file://guide.txt");

        let read = executor
            .execute(
                "ReadMcpResourceTool",
                r#"{"server":"alpha","uri":"file://guide.txt"}"#,
            )
            .expect("resource should read");
        let read_json: serde_json::Value =
            serde_json::from_str(&read).expect("resource read output should be json");
        assert_eq!(
            read_json["contents"][0]["text"],
            "contents for file://guide.txt"
        );

        if let Some(mcp_state) = state.mcp_state {
            mcp_state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .shutdown()
                .expect("mcp shutdown should succeed");
        }

        let _ = fs::remove_dir_all(config_home);
        let _ = fs::remove_dir_all(workspace);
    }

    #[test]
    fn build_runtime_plugin_state_surfaces_unsupported_mcp_servers_structurally() {
        let config_home = temp_dir();
        let workspace = temp_dir();
        fs::create_dir_all(&config_home).expect("config home");
        fs::create_dir_all(&workspace).expect("workspace");
        fs::write(
            config_home.join("settings.json"),
            r#"{
              "mcpServers": {
                "remote": {
                  "url": "https://example.test/mcp"
                }
              }
            }"#,
        )
        .expect("write mcp settings");

        let loader = ConfigLoader::new(&workspace, &config_home);
        let runtime_config = loader.load().expect("runtime config should load");
        let state = build_runtime_plugin_state_with_loader(&workspace, &loader, &runtime_config)
            .expect("runtime plugin state should load");
        let mut executor = CliToolExecutor::new(
            "test-mcp-session".to_string(),
            None,
            false,
            state.tool_registry.clone(),
            state.mcp_state.clone(),
        );

        let search_output = executor
            .execute("ToolSearch", r#"{"query":"remote","max_results":5}"#)
            .expect("tool search should execute");
        let search_json: serde_json::Value =
            serde_json::from_str(&search_output).expect("search output should be json");
        assert_eq!(search_json["pending_mcp_servers"][0], "remote");
        assert_eq!(
            search_json["mcp_degraded"]["failed_servers"][0]["server_name"],
            "remote"
        );
        assert_eq!(
            search_json["mcp_degraded"]["failed_servers"][0]["phase"],
            "server_registration"
        );
        assert_eq!(
            search_json["mcp_degraded"]["failed_servers"][0]["error"]["context"]["transport"],
            "http"
        );

        let _ = fs::remove_dir_all(config_home);
        let _ = fs::remove_dir_all(workspace);
    }

    #[test]
    fn build_runtime_runs_plugin_lifecycle_init_and_shutdown() {
        let config_home = temp_dir();
        // Inject a dummy API key so runtime construction succeeds without real credentials.
        // This test only exercises plugin lifecycle (init/shutdown), never calls the API.
        std::env::set_var("ORBIT_API_KEY", "test-dummy-key-for-plugin-lifecycle");
        let workspace = temp_dir();
        let source_root = temp_dir();
        fs::create_dir_all(&config_home).expect("config home");
        fs::create_dir_all(&workspace).expect("workspace");
        fs::create_dir_all(&source_root).expect("source root");
        write_plugin_fixture(&source_root, "lifecycle-runtime-demo", false, true);

        let mut manager = PluginManager::new(PluginManagerConfig::new(&config_home));
        let install = manager
            .install(source_root.to_str().expect("utf8 source path"))
            .expect("plugin install should succeed");
        let log_path = install.install_path.join("lifecycle.log");
        let loader = ConfigLoader::new(&workspace, &config_home);
        let runtime_config = loader.load().expect("runtime config should load");
        let runtime_plugin_state =
            build_runtime_plugin_state_with_loader(&workspace, &loader, &runtime_config)
                .expect("plugin state should load");
        let mut runtime = build_runtime_with_plugin_state(
            Session::new(),
            "runtime-plugin-lifecycle",
            DEFAULT_MODEL.to_string(),
            vec!["test system prompt".to_string()],
            true,
            false,
            None,
            PermissionMode::DangerFullAccess,
            None,
            runtime_plugin_state,
        )
        .expect("runtime should build");

        assert_eq!(
            fs::read_to_string(&log_path).expect("init log should exist"),
            "init\n"
        );

        runtime
            .shutdown_plugins()
            .expect("plugin shutdown should succeed");

        assert_eq!(
            fs::read_to_string(&log_path).expect("shutdown log should exist"),
            "init\nshutdown\n"
        );

        let _ = fs::remove_dir_all(config_home);
        let _ = fs::remove_dir_all(workspace);
        let _ = fs::remove_dir_all(source_root);
        std::env::remove_var("ORBIT_API_KEY");
    }
}

fn write_mcp_server_fixture(script_path: &Path) {
    let script = [
            "#!/usr/bin/env python3",
            "import json, sys",
            "",
            "def read_message():",
            "    header = b''",
            r"    while not header.endswith(b'\r\n\r\n'):",
            "        chunk = sys.stdin.buffer.read(1)",
            "        if not chunk:",
            "            return None",
            "        header += chunk",
            "    length = 0",
            r"    for line in header.decode().split('\r\n'):",
            r"        if line.lower().startswith('content-length:'):",
            "            length = int(line.split(':', 1)[1].strip())",
            "    payload = sys.stdin.buffer.read(length)",
            "    return json.loads(payload.decode())",
            "",
            "def send_message(message):",
            "    payload = json.dumps(message).encode()",
            r"    sys.stdout.buffer.write(f'Content-Length: {len(payload)}\r\n\r\n'.encode() + payload)",
            "    sys.stdout.buffer.flush()",
            "",
            "while True:",
            "    request = read_message()",
            "    if request is None:",
            "        break",
            "    method = request['method']",
            "    if method == 'initialize':",
            "        send_message({",
            "            'jsonrpc': '2.0',",
            "            'id': request['id'],",
            "            'result': {",
            "                'protocolVersion': request['params']['protocolVersion'],",
            "                'capabilities': {'tools': {}, 'resources': {}},",
            "                'serverInfo': {'name': 'fixture', 'version': '1.0.0'}",
            "            }",
            "        })",
            "    elif method == 'tools/list':",
            "        send_message({",
            "            'jsonrpc': '2.0',",
            "            'id': request['id'],",
            "            'result': {",
            "                'tools': [",
            "                    {",
            "                        'name': 'echo',",
            "                        'description': 'Echo from MCP fixture',",
            "                        'inputSchema': {",
            "                            'type': 'object',",
            "                            'properties': {'text': {'type': 'string'}},",
            "                            'required': ['text'],",
            "                            'additionalProperties': False",
            "                        },",
            "                        'annotations': {'readOnlyHint': True}",
            "                    }",
            "                ]",
            "            }",
            "        })",
            "    elif method == 'tools/call':",
            "        args = request['params'].get('arguments') or {}",
            "        send_message({",
            "            'jsonrpc': '2.0',",
            "            'id': request['id'],",
            "            'result': {",
            "                'content': [{'type': 'text', 'text': f\"echo:{args.get('text', '')}\"}],",
            "                'structuredContent': {'echoed': args.get('text', '')},",
            "                'isError': False",
            "            }",
            "        })",
            "    elif method == 'resources/list':",
            "        send_message({",
            "            'jsonrpc': '2.0',",
            "            'id': request['id'],",
            "            'result': {",
            "                'resources': [{'uri': 'file://guide.txt', 'name': 'guide', 'mimeType': 'text/plain'}]",
            "            }",
            "        })",
            "    elif method == 'resources/read':",
            "        uri = request['params']['uri']",
            "        send_message({",
            "            'jsonrpc': '2.0',",
            "            'id': request['id'],",
            "            'result': {",
            "                'contents': [{'uri': uri, 'mimeType': 'text/plain', 'text': f'contents for {uri}'}]",
            "            }",
            "        })",
            "    else:",
            "        send_message({",
            "            'jsonrpc': '2.0',",
            "            'id': request['id'],",
            "            'error': {'code': -32601, 'message': method}",
            "        })",
            "",
        ]
        .join("\n");
    fs::write(script_path, script).expect("mcp fixture script should write");
}

#[cfg(test)]
mod sandbox_report_tests {
    use super::{format_sandbox_report, HookAbortMonitor};
    use orbit_runtime::HookAbortSignal;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn sandbox_report_renders_expected_fields() {
        let report = format_sandbox_report(&orbit_runtime::SandboxStatus::default());
        assert!(report.contains("Sandbox"));
        assert!(report.contains("Enabled"));
        assert!(report.contains("Filesystem mode"));
        assert!(report.contains("Fallback reason"));
    }

    #[test]
    fn hook_abort_monitor_stops_without_aborting() {
        let abort_signal = HookAbortSignal::new();
        let (ready_tx, ready_rx) = mpsc::channel();
        let monitor = HookAbortMonitor::spawn_with_waiter(
            abort_signal.clone(),
            move |stop_rx, abort_signal| {
                ready_tx.send(()).expect("ready signal");
                let _ = stop_rx.recv();
                assert!(!abort_signal.is_aborted());
            },
        );

        ready_rx.recv().expect("waiter should be ready");
        monitor.stop();

        assert!(!abort_signal.is_aborted());
    }

    #[test]
    fn hook_abort_monitor_propagates_interrupt() {
        let abort_signal = HookAbortSignal::new();
        let (done_tx, done_rx) = mpsc::channel();
        let monitor = HookAbortMonitor::spawn_with_waiter(
            abort_signal.clone(),
            move |_stop_rx, abort_signal| {
                abort_signal.abort();
                done_tx.send(()).expect("done signal");
            },
        );

        done_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("interrupt should complete");
        monitor.stop();

        assert!(abort_signal.is_aborted());
    }
}
