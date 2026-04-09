#![cfg(unix)]

use reqwest::StatusCode;
use serde_json::Value;
use std::fs;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::process::{Child, Command};

struct TestServer {
    base_url: String,
    child: Child,
    temp_dir: PathBuf,
}

impl TestServer {
    async fn start() -> Self {
        Self::start_with_env(&[]).await
    }

    async fn start_with_env(extra_env: &[(&str, &str)]) -> Self {
        let temp_dir = make_temp_dir();
        let cli_bin = write_mock_cli(&temp_dir);
        let port = reserve_port();
        let api_bin = env!("CARGO_BIN_EXE_orbit-api");

        let mut command = Command::new(api_bin);
        command
            .env("ORBIT_CLI_BIN", &cli_bin)
            .env("ORBIT_API_HOST", "127.0.0.1")
            .env("ORBIT_API_PORT", port.to_string());
        for (key, value) in extra_env {
            command.env(key, value);
        }
        let child = command.spawn().expect("failed to spawn orbit-api");

        let base_url = format!("http://127.0.0.1:{port}");
        wait_for_health(&base_url).await;

        Self {
            base_url,
            child,
            temp_dir,
        }
    }

    async fn shutdown(mut self) {
        let _ = self.child.kill().await;
        let _ = self.child.wait().await;
        let _ = fs::remove_dir_all(&self.temp_dir);
    }
}

#[tokio::test]
async fn health_endpoint_returns_ok() {
    let server = TestServer::start().await;

    let response = reqwest::get(format!("{}/health", server.base_url))
        .await
        .expect("health request should succeed");
    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response
        .json()
        .await
        .expect("health response should be JSON");
    assert_eq!(body["status"], "ok");

    server.shutdown().await;
}

#[tokio::test]
async fn status_endpoint_executes_cli_json_command() {
    let server = TestServer::start().await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/v1/status", server.base_url))
        .send()
        .await
        .expect("status request should succeed");

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await.expect("body should be JSON");
    assert_eq!(body["ok"], true);
    assert_eq!(body["json"]["command"], "status");

    server.shutdown().await;
}

#[tokio::test]
async fn cli_run_endpoint_returns_failure_payload_without_http_error() {
    let server = TestServer::start().await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/v1/cli/run", server.base_url))
        .json(&serde_json::json!({
            "args": ["fail"],
            "force_json_output": true
        }))
        .send()
        .await
        .expect("cli run request should succeed");

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await.expect("body should be JSON");
    assert_eq!(body["ok"], false);
    assert_eq!(body["exit_code"], 3);
    assert!(body["stderr"]
        .as_str()
        .unwrap_or_default()
        .contains("mock failure"));

    server.shutdown().await;
}

#[tokio::test]
async fn cli_run_endpoint_rejects_empty_args() {
    let server = TestServer::start().await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/v1/cli/run", server.base_url))
        .json(&serde_json::json!({
            "args": []
        }))
        .send()
        .await
        .expect("cli run request should return bad request");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body: Value = response.json().await.expect("body should be JSON");
    assert_eq!(body["error"], "args must not be empty");

    server.shutdown().await;
}

#[tokio::test]
async fn prompt_endpoint_forwards_prompt_and_options() {
    let server = TestServer::start().await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/v1/prompt", server.base_url))
        .json(&serde_json::json!({
            "prompt": "hello world",
            "model": "claude-sonnet-4-6",
            "provider": "anthropic",
            "permission_mode": "workspace-write",
            "allowed_tools": ["read", "write"]
        }))
        .send()
        .await
        .expect("prompt request should succeed");

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await.expect("body should be JSON");
    assert_eq!(body["ok"], true);
    assert_eq!(body["json"]["command"], "prompt");
    assert_eq!(body["json"]["prompt"], "hello world");
    assert_eq!(body["json"]["model"], "claude-sonnet-4-6");
    assert_eq!(body["json"]["provider"], "anthropic");

    server.shutdown().await;
}

#[tokio::test]
async fn auth_rejects_missing_api_key_and_accepts_valid_key() {
    let server = TestServer::start_with_env(&[("ORBIT_API_KEY", "top-secret")]).await;

    let client = reqwest::Client::new();
    let unauthorized = client
        .get(format!("{}/v1/status", server.base_url))
        .send()
        .await
        .expect("status request should return unauthorized");
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let authorized = client
        .get(format!("{}/v1/status", server.base_url))
        .header("x-api-key", "top-secret")
        .send()
        .await
        .expect("status request with key should succeed");
    assert_eq!(authorized.status(), StatusCode::OK);

    server.shutdown().await;
}

#[tokio::test]
async fn cli_run_respects_allowed_commands() {
    let server =
        TestServer::start_with_env(&[("ORBIT_API_ALLOWED_COMMANDS", "status,version")]).await;

    let client = reqwest::Client::new();
    let forbidden = client
        .post(format!("{}/v1/cli/run", server.base_url))
        .json(&serde_json::json!({
            "args": ["fail"]
        }))
        .send()
        .await
        .expect("cli request should return forbidden");
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);

    let allowed = client
        .post(format!("{}/v1/cli/run", server.base_url))
        .json(&serde_json::json!({
            "args": ["status"]
        }))
        .send()
        .await
        .expect("allowed command should succeed");
    assert_eq!(allowed.status(), StatusCode::OK);
    let body: Value = allowed.json().await.expect("body should be JSON");
    assert_eq!(body["ok"], true);

    server.shutdown().await;
}

fn reserve_port() -> u16 {
    let listener = TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
        .expect("should reserve port");
    let port = listener
        .local_addr()
        .expect("should read local addr")
        .port();
    drop(listener);
    port
}

fn make_temp_dir() -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should be after epoch")
        .as_nanos();
    let pid = std::process::id();
    let serial = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("orbit-api-test-{pid}-{now}-{serial}"));
    fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

fn write_mock_cli(dir: &Path) -> PathBuf {
    let script_path = dir.join("mock-orbit.sh");
    let script = r#"#!/bin/sh
set -eu

model=""
provider=""
prompt=""
command=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    --output-format)
      shift 2
      ;;
    --model)
      model="$2"
      shift 2
      ;;
    --provider)
      provider="$2"
      shift 2
      ;;
    --permission-mode)
      shift 2
      ;;
    --allowedTools|--allowed-tools)
      shift 2
      ;;
    prompt|status|sandbox|version|fail)
      command="$1"
      shift
      break
      ;;
    *)
      command="$1"
      shift
      break
      ;;
  esac
done

case "$command" in
  status)
    echo '{"command":"status","ok":true}'
    ;;
  sandbox)
    echo '{"command":"sandbox","ok":true}'
    ;;
  version)
    echo '{"command":"version","ok":true}'
    ;;
  prompt)
    prompt="${1:-}"
    printf '{"command":"prompt","ok":true,"prompt":"%s","model":"%s","provider":"%s"}\n' "$prompt" "$model" "$provider"
    ;;
  fail)
    echo 'mock failure' >&2
    exit 3
    ;;
  *)
    echo '{"command":"unknown","ok":true}'
    ;;
esac
"#;

    fs::write(&script_path, script).expect("should write mock cli");

    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(&script_path)
        .expect("should stat mock cli")
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script_path, perms).expect("should chmod mock cli");

    script_path
}

async fn wait_for_health(base_url: &str) {
    let client = reqwest::Client::new();

    for _ in 0..60 {
        match client.get(format!("{base_url}/health")).send().await {
            Ok(response) if response.status() == StatusCode::OK => return,
            _ => tokio::time::sleep(Duration::from_millis(100)).await,
        }
    }

    panic!("orbit-api did not become healthy in time");
}
