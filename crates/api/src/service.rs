use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::env;
use std::fmt::Write as _;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

#[derive(Debug, Clone)]
pub struct ApiServiceConfig {
    pub bind_addr: SocketAddr,
    pub cli_bin: Option<PathBuf>,
    pub working_dir: Option<PathBuf>,
    pub api_key: Option<String>,
    pub allowed_commands: Option<BTreeSet<String>>,
    pub command_timeout_ms: u64,
    pub allow_insecure_bind: bool,
    pub allow_dangerous_permissions: bool,
}

impl Default for ApiServiceConfig {
    fn default() -> Self {
        Self {
            bind_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8787),
            cli_bin: None,
            working_dir: None,
            api_key: None,
            allowed_commands: None,
            command_timeout_ms: 120_000,
            allow_insecure_bind: false,
            allow_dangerous_permissions: false,
        }
    }
}

impl ApiServiceConfig {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mut config = Self::default();

        if let Ok(host) = env::var("ORBIT_API_HOST") {
            let ip: IpAddr = host.parse()?;
            config.bind_addr = SocketAddr::new(ip, config.bind_addr.port());
        }

        if let Ok(port) = env::var("ORBIT_API_PORT") {
            let parsed: u16 = port.parse()?;
            config.bind_addr = SocketAddr::new(config.bind_addr.ip(), parsed);
        }

        if let Ok(bin) = env::var("ORBIT_CLI_BIN") {
            if !bin.trim().is_empty() {
                config.cli_bin = Some(PathBuf::from(bin));
            }
        }

        if let Ok(workdir) = env::var("ORBIT_API_WORKDIR") {
            if !workdir.trim().is_empty() {
                config.working_dir = Some(PathBuf::from(workdir));
            }
        }
        if let Ok(api_key) = env::var("ORBIT_API_KEY") {
            if !api_key.trim().is_empty() {
                config.api_key = Some(api_key);
            }
        }
        if let Ok(allowed) = env::var("ORBIT_API_ALLOWED_COMMANDS") {
            let parsed = allowed
                .split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(|item| item.to_ascii_lowercase())
                .collect::<BTreeSet<_>>();
            if !parsed.is_empty() {
                config.allowed_commands = Some(parsed);
            }
        }
        if let Ok(timeout_ms) = env::var("ORBIT_API_COMMAND_TIMEOUT_MS") {
            let parsed: u64 = timeout_ms.parse()?;
            config.command_timeout_ms = parsed;
        }
        if let Ok(value) = env::var("ORBIT_API_ALLOW_INSECURE_BIND") {
            config.allow_insecure_bind = parse_bool_env(&value);
        }
        if let Ok(value) = env::var("ORBIT_API_ALLOW_DANGEROUS_PERMISSIONS") {
            config.allow_dangerous_permissions = parse_bool_env(&value);
        }

        Ok(config)
    }

    fn validate(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.bind_addr.ip().is_loopback() && self.api_key.is_none() && !self.allow_insecure_bind
        {
            return Err(
                "refusing to bind orbit-api to a non-loopback address without ORBIT_API_KEY; set ORBIT_API_ALLOW_INSECURE_BIND=true to override"
                    .into(),
            );
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct AppState {
    cli_bin: PathBuf,
    working_dir: Option<PathBuf>,
    api_key: Option<String>,
    allowed_commands: Option<BTreeSet<String>>,
    command_timeout_ms: u64,
    allow_dangerous_permissions: bool,
}

#[derive(Debug, Deserialize)]
pub struct CliRunRequest {
    pub args: Vec<String>,
    #[serde(default = "default_true")]
    pub force_json_output: bool,
}

#[derive(Debug, Deserialize)]
pub struct PromptRequest {
    pub prompt: String,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub permission_mode: Option<String>,
    pub allowed_tools: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct CliRunResponse {
    pub ok: bool,
    pub exit_code: Option<i32>,
    pub args: Vec<String>,
    pub duration_ms: u128,
    pub stdout: String,
    pub stderr: String,
    pub json: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

fn default_true() -> bool {
    true
}

fn parse_bool_env(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn constant_time_equals(expected: &str, provided: &str) -> bool {
    let expected = expected.as_bytes();
    let provided = provided.as_bytes();
    let max_len = expected.len().max(provided.len());
    let mut diff = expected.len() ^ provided.len();

    for idx in 0..max_len {
        let left = expected.get(idx).copied().unwrap_or(0);
        let right = provided.get(idx).copied().unwrap_or(0);
        diff |= usize::from(left ^ right);
    }

    diff == 0
}

fn is_dangerous_permission_mode(mode: &str) -> bool {
    mode.trim().eq_ignore_ascii_case("danger-full-access")
}

fn validate_permission_flags(
    state: &AppState,
    args: &[String],
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if state.allow_dangerous_permissions {
        return Ok(());
    }

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--dangerously-skip-permissions" => {
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(ErrorResponse {
                        error: "dangerous permission overrides are disabled for orbit-api"
                            .to_string(),
                    }),
                ));
            }
            "--permission-mode" if index + 1 < args.len() => {
                if is_dangerous_permission_mode(&args[index + 1]) {
                    return Err((
                        StatusCode::FORBIDDEN,
                        Json(ErrorResponse {
                            error: "danger-full-access is disabled for orbit-api".to_string(),
                        }),
                    ));
                }
                index += 2;
                continue;
            }
            flag if flag.starts_with("--permission-mode=") => {
                if let Some(value) = flag.split_once('=').map(|(_, value)| value) {
                    if is_dangerous_permission_mode(value) {
                        return Err((
                            StatusCode::FORBIDDEN,
                            Json(ErrorResponse {
                                error: "danger-full-access is disabled for orbit-api".to_string(),
                            }),
                        ));
                    }
                }
            }
            _ => {}
        }
        index += 1;
    }

    Ok(())
}

pub async fn serve(
    config: ApiServiceConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    config.validate()?;
    let cli_bin = resolve_cli_bin(config.cli_bin)?;
    let display_cli_bin = cli_bin.display().to_string();
    let state = Arc::new(AppState {
        cli_bin,
        working_dir: config.working_dir,
        api_key: config.api_key,
        allowed_commands: config.allowed_commands,
        command_timeout_ms: config.command_timeout_ms,
        allow_dangerous_permissions: config.allow_dangerous_permissions,
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/cli/run", post(run_cli))
        .route("/v1/prompt", post(run_prompt))
        .route("/v1/status", get(run_status))
        .route("/v1/sandbox", get(run_sandbox))
        .route("/v1/version", get(run_version))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    println!(
        "orbit-api listening on http://{} (cli: {display_cli_bin})",
        config.bind_addr
    );
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn run_cli(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(mut request): Json<CliRunRequest>,
) -> Result<Json<CliRunResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize(&state, &headers)?;
    if request.args.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "args must not be empty".to_string(),
            }),
        ));
    }

    if request.force_json_output {
        request.args = with_json_output(request.args);
    }
    authorize_command(&state, &request.args)?;
    validate_permission_flags(&state, &request.args)?;

    let response = execute_cli(&state, request.args)
        .await
        .map_err(internal_error)?;
    Ok(Json(response))
}

async fn run_prompt(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(request): Json<PromptRequest>,
) -> Result<Json<CliRunResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize(&state, &headers)?;
    if request.prompt.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "prompt must not be empty".to_string(),
            }),
        ));
    }

    let mut args = Vec::new();

    if let Some(model) = request.model {
        args.extend(["--model".to_string(), model]);
    }
    if let Some(provider) = request.provider {
        args.extend(["--provider".to_string(), provider]);
    }
    if let Some(permission_mode) = request.permission_mode {
        if !state.allow_dangerous_permissions && is_dangerous_permission_mode(&permission_mode) {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    error: "danger-full-access is disabled for orbit-api".to_string(),
                }),
            ));
        }
        args.extend(["--permission-mode".to_string(), permission_mode]);
    }
    if let Some(allowed_tools) = request.allowed_tools {
        if !allowed_tools.is_empty() {
            args.extend(["--allowedTools".to_string(), allowed_tools.join(",")]);
        }
    }
    args.push("prompt".to_string());
    args.push(request.prompt);
    validate_permission_flags(&state, &args)?;

    let response = execute_cli(&state, with_json_output(args))
        .await
        .map_err(internal_error)?;
    Ok(Json(response))
}

async fn run_status(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<CliRunResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize(&state, &headers)?;
    execute_simple_command(state, vec!["status".to_string()]).await
}

async fn run_sandbox(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<CliRunResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize(&state, &headers)?;
    execute_simple_command(state, vec!["sandbox".to_string()]).await
}

async fn run_version(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<CliRunResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize(&state, &headers)?;
    execute_simple_command(state, vec!["version".to_string()]).await
}

async fn execute_simple_command(
    state: Arc<AppState>,
    args: Vec<String>,
) -> Result<Json<CliRunResponse>, (StatusCode, Json<ErrorResponse>)> {
    let response = execute_cli(&state, with_json_output(args))
        .await
        .map_err(internal_error)?;
    Ok(Json(response))
}

async fn execute_cli(
    state: &AppState,
    args: Vec<String>,
) -> Result<CliRunResponse, Box<dyn std::error::Error + Send + Sync>> {
    let started = Instant::now();
    let mut command = Command::new(&state.cli_bin);
    command.args(&args);
    command.kill_on_drop(true);

    if let Some(working_dir) = &state.working_dir {
        command.current_dir(working_dir);
    }

    let output = timeout(
        Duration::from_millis(state.command_timeout_ms),
        command.output(),
    )
    .await
    .map_err(|_| {
        format!(
            "orbit command timed out after {}ms",
            state.command_timeout_ms
        )
    })??;

    let duration_ms = started.elapsed().as_millis();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let json = serde_json::from_str::<Value>(&stdout).ok();

    Ok(CliRunResponse {
        ok: output.status.success(),
        exit_code: output.status.code(),
        args,
        duration_ms,
        stdout,
        stderr,
        json,
    })
}

fn with_json_output(mut args: Vec<String>) -> Vec<String> {
    if args
        .iter()
        .any(|arg| arg == "--output-format" || arg.starts_with("--output-format="))
    {
        return args;
    }

    let mut output = vec!["--output-format".to_string(), "json".to_string()];
    output.append(&mut args);
    output
}

fn resolve_cli_bin(
    configured: Option<PathBuf>,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    if let Some(path) = configured {
        return Ok(path);
    }

    if let Ok(path) = env::var("ORBIT_CLI_BIN") {
        if !path.trim().is_empty() {
            return Ok(PathBuf::from(path));
        }
    }

    if let Some(path) = find_workspace_binary(Path::new("target/debug/orbit")) {
        return Ok(path);
    }

    Ok(PathBuf::from("orbit"))
}

fn find_workspace_binary(relative: &Path) -> Option<PathBuf> {
    let mut current = env::current_dir().ok()?;

    loop {
        let candidate = current.join(relative);
        if candidate.exists() {
            return Some(candidate);
        }
        if !current.pop() {
            break;
        }
    }

    None
}

fn internal_error(
    error: Box<dyn std::error::Error + Send + Sync>,
) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: error.to_string(),
        }),
    )
}

fn authorize(
    state: &AppState,
    headers: &axum::http::HeaderMap,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let Some(expected) = &state.api_key else {
        return Ok(());
    };
    let provided_key = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
        .or_else(|| {
            headers
                .get(axum::http::header::AUTHORIZATION)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.strip_prefix("Bearer "))
                .map(ToOwned::to_owned)
        });

    if provided_key
        .as_deref()
        .map(|provided| constant_time_equals(expected, provided))
        .unwrap_or(false)
    {
        Ok(())
    } else {
        Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "missing or invalid API key".to_string(),
            }),
        ))
    }
}

fn authorize_command(
    state: &AppState,
    args: &[String],
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let Some(allowed) = &state.allowed_commands else {
        return Ok(());
    };
    let command = detect_primary_command(args).unwrap_or_default();
    if allowed.contains(command.as_str()) {
        return Ok(());
    }

    let mut allowed_list = String::new();
    for (idx, item) in allowed.iter().enumerate() {
        if idx > 0 {
            let _ = write!(&mut allowed_list, ", ");
        }
        let _ = write!(&mut allowed_list, "{item}");
    }
    Err((
        StatusCode::FORBIDDEN,
        Json(ErrorResponse {
            error: format!("command '{command}' is not allowed; allowed commands: {allowed_list}"),
        }),
    ))
}

fn detect_primary_command(args: &[String]) -> Option<String> {
    let mut index = 0;
    while index < args.len() {
        let current = args[index].as_str();
        match current {
            "--model" | "--provider" | "--permission-mode" | "--output-format"
            | "--allowedTools" | "--allowed-tools" | "--resume" => {
                index += 2;
            }
            flag if flag.starts_with("--model=")
                || flag.starts_with("--provider=")
                || flag.starts_with("--permission-mode=")
                || flag.starts_with("--output-format=")
                || flag.starts_with("--allowedTools=")
                || flag.starts_with("--allowed-tools=")
                || flag.starts_with("--resume=") =>
            {
                index += 1;
            }
            "--print" | "--dangerously-skip-permissions" | "--help" | "-h" | "--version" | "-V" => {
                index += 1;
            }
            other if other.starts_with('-') => {
                index += 1;
            }
            _ => return Some(current.to_ascii_lowercase()),
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        detect_primary_command, validate_permission_flags, with_json_output, ApiServiceConfig,
        AppState,
    };
    use axum::{http::StatusCode, Json};
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::path::PathBuf;

    #[test]
    fn json_output_flag_is_prepended_when_missing() {
        let args = vec!["status".to_string()];
        let out = with_json_output(args);
        assert_eq!(
            out,
            vec![
                "--output-format".to_string(),
                "json".to_string(),
                "status".to_string()
            ]
        );
    }

    #[test]
    fn json_output_flag_is_not_duplicated() {
        let args = vec![
            "--output-format".to_string(),
            "json".to_string(),
            "status".to_string(),
        ];
        let out = with_json_output(args.clone());
        assert_eq!(out, args);
    }

    #[test]
    fn detect_primary_command_skips_global_flags() {
        let args = vec![
            "--model".to_string(),
            "claude-sonnet-4-6".to_string(),
            "--output-format".to_string(),
            "json".to_string(),
            "status".to_string(),
        ];
        assert_eq!(detect_primary_command(&args), Some("status".to_string()));
    }

    #[test]
    fn validate_rejects_non_loopback_bind_without_api_key() {
        let config = ApiServiceConfig {
            bind_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8787),
            ..ApiServiceConfig::default()
        };

        let error = config
            .validate()
            .expect_err("config should reject insecure bind");
        assert!(error
            .to_string()
            .contains("refusing to bind orbit-api to a non-loopback address"));
    }

    #[test]
    fn validate_allows_non_loopback_bind_with_api_key() {
        let config = ApiServiceConfig {
            bind_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8787),
            api_key: Some("top-secret".to_string()),
            ..ApiServiceConfig::default()
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn dangerous_permission_overrides_are_blocked_by_default() {
        let state = AppState {
            cli_bin: PathBuf::from("orbit"),
            working_dir: None,
            api_key: None,
            allowed_commands: None,
            command_timeout_ms: 1_000,
            allow_dangerous_permissions: false,
        };

        let result = validate_permission_flags(
            &state,
            &[
                "--dangerously-skip-permissions".to_string(),
                "status".to_string(),
            ],
        );

        let (status, Json(body)) = result.expect_err("override should be rejected");
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(
            body.error,
            "dangerous permission overrides are disabled for orbit-api"
        );
    }

    #[test]
    fn danger_full_access_mode_is_blocked_by_default() {
        let state = AppState {
            cli_bin: PathBuf::from("orbit"),
            working_dir: None,
            api_key: None,
            allowed_commands: None,
            command_timeout_ms: 1_000,
            allow_dangerous_permissions: false,
        };

        let result = validate_permission_flags(
            &state,
            &[
                "--permission-mode".to_string(),
                "danger-full-access".to_string(),
                "status".to_string(),
            ],
        );

        let (status, Json(body)) = result.expect_err("danger mode should be rejected");
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body.error, "danger-full-access is disabled for orbit-api");
    }
}
