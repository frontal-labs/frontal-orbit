use std::collections::HashMap;
use std::io;
use std::sync::Arc;

use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{oneshot, Mutex};
use tokio::task::JoinHandle;

/// Prefix used by parity harness scenarios embedded in the user prompt text.
pub const SCENARIO_PREFIX: &str = "PARITY_SCENARIO:";
/// Default model reflected by the mock gateway (`Frontal AI Gateway` default).
pub const DEFAULT_MODEL: &str = "claude-4-8";

/// Provider-agnostic request as seen on the wire by the gateway. The mock mirrors
/// the `OpenAI` Chat Completions contract that `Frontal AI Gateway` exposes, so it works
/// for any provider routed through the gateway (anthropic, openai, xai, frontal, ...).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedRequest {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub scenario: String,
    pub stream: bool,
    pub model: String,
    pub raw_body: String,
}

pub struct MockGateway {
    base_url: String,
    requests: Arc<Mutex<Vec<CapturedRequest>>>,
    shutdown: Option<oneshot::Sender<()>>,
    join_handle: JoinHandle<()>,
}

impl MockGateway {
    pub async fn spawn() -> io::Result<Self> {
        Self::spawn_on("127.0.0.1:0").await
    }

    pub async fn spawn_on(bind_addr: &str) -> io::Result<Self> {
        let listener = TcpListener::bind(bind_addr).await?;
        let address = listener.local_addr()?;
        let requests = Arc::new(Mutex::new(Vec::new()));
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        let request_state = Arc::clone(&requests);

        let join_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => break,
                    accepted = listener.accept() => {
                        let Ok((socket, _)) = accepted else {
                            break;
                        };
                        let request_state = Arc::clone(&request_state);
                        tokio::spawn(async move {
                            let _ = handle_connection(socket, request_state).await;
                        });
                    }
                }
            }
        });

        Ok(Self {
            base_url: format!("http://{address}"),
            requests,
            shutdown: Some(shutdown_tx),
            join_handle,
        })
    }

    #[must_use]
    pub fn base_url(&self) -> String {
        self.base_url.clone()
    }

    pub async fn captured_requests(&self) -> Vec<CapturedRequest> {
        self.requests.lock().await.clone()
    }
}

impl Drop for MockGateway {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        self.join_handle.abort();
    }
}

/// Scenario catalogue shared across providers. Each scenario describes a tool-use or
/// text round-trip exercised by the parity harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scenario {
    StreamingText,
    ReadFileRoundtrip,
    GrepChunkAssembly,
    WriteFileAllowed,
    WriteFileDenied,
    MultiToolTurnRoundtrip,
    BashStdoutRoundtrip,
    BashPermissionPromptApproved,
    BashPermissionPromptDenied,
    PluginToolRoundtrip,
    AutoCompactTriggered,
    TokenCostReporting,
}

impl Scenario {
    fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "streaming_text" => Some(Self::StreamingText),
            "read_file_roundtrip" => Some(Self::ReadFileRoundtrip),
            "grep_chunk_assembly" => Some(Self::GrepChunkAssembly),
            "write_file_allowed" => Some(Self::WriteFileAllowed),
            "write_file_denied" => Some(Self::WriteFileDenied),
            "multi_tool_turn_roundtrip" => Some(Self::MultiToolTurnRoundtrip),
            "bash_stdout_roundtrip" => Some(Self::BashStdoutRoundtrip),
            "bash_permission_prompt_approved" => Some(Self::BashPermissionPromptApproved),
            "bash_permission_prompt_denied" => Some(Self::BashPermissionPromptDenied),
            "plugin_tool_roundtrip" => Some(Self::PluginToolRoundtrip),
            "auto_compact_triggered" => Some(Self::AutoCompactTriggered),
            "token_cost_reporting" => Some(Self::TokenCostReporting),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::StreamingText => "streaming_text",
            Self::ReadFileRoundtrip => "read_file_roundtrip",
            Self::GrepChunkAssembly => "grep_chunk_assembly",
            Self::WriteFileAllowed => "write_file_allowed",
            Self::WriteFileDenied => "write_file_denied",
            Self::MultiToolTurnRoundtrip => "multi_tool_turn_roundtrip",
            Self::BashStdoutRoundtrip => "bash_stdout_roundtrip",
            Self::BashPermissionPromptApproved => "bash_permission_prompt_approved",
            Self::BashPermissionPromptDenied => "bash_permission_prompt_denied",
            Self::PluginToolRoundtrip => "plugin_tool_roundtrip",
            Self::AutoCompactTriggered => "auto_compact_triggered",
            Self::TokenCostReporting => "token_cost_reporting",
        }
    }
}

/// `OpenAI` Chat Completions request as received on the wire.
#[derive(Debug, Clone)]
struct ChatRequest {
    model: String,
    messages: Vec<Value>,
    stream: bool,
}

async fn handle_connection(
    mut socket: tokio::net::TcpStream,
    requests: Arc<Mutex<Vec<CapturedRequest>>>,
) -> io::Result<()> {
    let (method, path, headers, raw_body) = read_http_request(&mut socket).await?;
    let request: ChatRequest = parse_chat_request(&raw_body)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
    let scenario = detect_scenario(&request)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing parity scenario"))?;

    requests.lock().await.push(CapturedRequest {
        method,
        path,
        headers,
        scenario: scenario.name().to_string(),
        stream: request.stream,
        model: request.model.clone(),
        raw_body,
    });

    let response = build_http_response(&request, scenario);
    socket.write_all(response.as_bytes()).await?;
    Ok(())
}

async fn read_http_request(
    socket: &mut tokio::net::TcpStream,
) -> io::Result<(String, String, HashMap<String, String>, String)> {
    let mut buffer = Vec::new();
    let mut header_end = None;

    loop {
        let mut chunk = [0_u8; 1024];
        let read = socket.read(&mut chunk).await?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if let Some(position) = find_header_end(&buffer) {
            header_end = Some(position);
            break;
        }
    }

    let header_end = header_end
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "missing http headers"))?;
    let (header_bytes, remaining) = buffer.split_at(header_end);
    let header_text = String::from_utf8(header_bytes.to_vec())
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
    let mut lines = header_text.split("\r\n");
    let request_line = lines
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing request line"))?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing method"))?
        .to_string();
    let path = request_parts
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing path"))?
        .to_string();

    let mut headers = HashMap::new();
    let mut content_length = 0_usize;
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let (name, value) = line.split_once(':').ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "malformed http header line")
        })?;
        let value = value.trim().to_string();
        if name.eq_ignore_ascii_case("content-length") {
            content_length = value.parse().map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid content-length: {error}"),
                )
            })?;
        }
        headers.insert(name.to_ascii_lowercase(), value);
    }

    let mut body = remaining[4..].to_vec();
    while body.len() < content_length {
        let mut chunk = vec![0_u8; content_length - body.len()];
        let read = socket.read(&mut chunk).await?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..read]);
    }

    let body = String::from_utf8(body)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
    Ok((method, path, headers, body))
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

fn parse_chat_request(raw_body: &str) -> Result<ChatRequest, serde_json::Error> {
    let value: Value = serde_json::from_str(raw_body)?;
    let model = value
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_MODEL)
        .to_string();
    let messages = value
        .get("messages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let stream = value
        .get("stream")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    Ok(ChatRequest {
        model,
        messages,
        stream,
    })
}

/// Find the scenario token embedded in the latest user/tool message text.
fn detect_scenario(request: &ChatRequest) -> Option<Scenario> {
    for message in request.messages.iter().rev() {
        let content = match message.get("content") {
            Some(Value::String(text)) => Some(text.as_str()),
            Some(Value::Array(blocks)) => blocks.iter().find_map(|block| {
                block
                    .get("text")
                    .and_then(Value::as_str)
                    .or_else(|| block.get("content").and_then(Value::as_str))
            }),
            _ => None,
        };
        if let Some(text) = content {
            if let Some(scenario) = text
                .split_whitespace()
                .find_map(|token| token.strip_prefix(SCENARIO_PREFIX))
                .and_then(Scenario::parse)
            {
                return Some(scenario);
            }
        }
    }
    None
}

fn latest_tool_result(request: &ChatRequest) -> Option<(String, bool)> {
    for message in request.messages.iter().rev() {
        if message.get("role").and_then(Value::as_str) != Some("tool") {
            continue;
        }
        let content = message
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let is_error = message
            .get("is_error")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        return Some((content, is_error));
    }
    None
}

fn tool_results_by_name(request: &ChatRequest) -> HashMap<String, (String, bool)> {
    let mut tool_names_by_id = HashMap::new();
    for message in &request.messages {
        if message.get("role").and_then(Value::as_str) == Some("assistant") {
            if let Some(tool_calls) = message.get("tool_calls").and_then(Value::as_array) {
                for tool_call in tool_calls {
                    let id = tool_call
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    let name = tool_call
                        .get("function")
                        .and_then(|function| function.get("name"))
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    tool_names_by_id.insert(id.to_string(), name.to_string());
                }
            }
        }
    }

    let mut results = HashMap::new();
    for message in request.messages.iter().rev() {
        if message.get("role").and_then(Value::as_str) != Some("tool") {
            continue;
        }
        let tool_call_id = message
            .get("tool_call_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let content = message
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let is_error = message
            .get("is_error")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let tool_name = tool_names_by_id
            .get(&tool_call_id)
            .cloned()
            .unwrap_or_else(|| tool_call_id.clone());
        results
            .entry(tool_name)
            .or_insert_with(|| (content, is_error));
    }
    results
}

fn build_http_response(request: &ChatRequest, scenario: Scenario) -> String {
    let model = request.model.clone();
    if request.stream {
        let body = build_stream_body(&model, scenario);
        return http_response(
            "200 OK",
            "text/event-stream",
            &body,
            &[("x-request-id", request_id_for(scenario))],
        );
    }

    let response = build_chat_response(&model, request, scenario);
    http_response(
        "200 OK",
        "application/json",
        &serde_json::to_string(&response).expect("chat completion response should serialize"),
        &[("request-id", request_id_for(scenario))],
    )
}

/// `OpenAI` Chat Completions wire shape (non-streaming).
fn build_chat_response(model: &str, request: &ChatRequest, scenario: Scenario) -> Value {
    let (message, finish_reason) = scenario_message(scenario, request);
    let (prompt_tokens, completion_tokens) = match scenario {
        Scenario::AutoCompactTriggered => (50_000, 200),
        Scenario::TokenCostReporting => (1_000, 500),
        _ => (10, 6),
    };

    json!({
        "id": message_id_for(scenario),
        "object": "chat.completion",
        "model": model,
        "choices": [{
            "index": 0,
            "message": message,
            "finish_reason": finish_reason
        }],
        "usage": {
            "prompt_tokens": prompt_tokens,
            "completion_tokens": completion_tokens,
            "total_tokens": prompt_tokens + completion_tokens
        }
    })
}

/// Resolve the assistant message + finish reason for a scenario, mirroring the
/// tool round-trip: emit a tool call first, then a final answer once the tool
/// result round-trips back in the request.
fn scenario_message(scenario: Scenario, request: &ChatRequest) -> (Value, &'static str) {
    match scenario {
        Scenario::StreamingText => (
            json!({ "role": "assistant", "content": "Mock streaming says hello from the parity harness." }),
            "stop",
        ),
        Scenario::ReadFileRoundtrip => roundtrip(
            request,
            |output| {
                format!(
                    "read_file roundtrip complete: {}",
                    extract_read_content(output)
                )
            },
            "toolu_read_fixture",
            "read_file",
            json!({"path":"fixture.txt"}),
        ),
        Scenario::GrepChunkAssembly => roundtrip(
            request,
            |output| {
                format!(
                    "grep_search matched {} occurrences",
                    extract_num_matches(output)
                )
            },
            "toolu_grep_fixture",
            "grep_search",
            json!({"pattern": "parity", "path": "fixture.txt", "output_mode": "count"}),
        ),
        Scenario::WriteFileAllowed => roundtrip(
            request,
            |output| format!("write_file succeeded: {}", extract_file_path(output)),
            "toolu_write_allowed",
            "write_file",
            json!({"path":"generated/output.txt","content":"created by mock service\n"}),
        ),
        Scenario::WriteFileDenied => roundtrip(
            request,
            |output| format!("write_file denied as expected: {output}"),
            "toolu_write_denied",
            "write_file",
            json!({"path":"generated/denied.txt","content":"should not exist\n"}),
        ),
        Scenario::MultiToolTurnRoundtrip => multi_tool_roundtrip(request),
        Scenario::BashStdoutRoundtrip => roundtrip(
            request,
            |output| format!("bash completed: {}", extract_bash_stdout(output)),
            "toolu_bash_stdout",
            "bash",
            json!({"command":"printf 'alpha from bash'","timeout":1000}),
        ),
        Scenario::BashPermissionPromptApproved => bash_prompt_roundtrip(request),
        Scenario::BashPermissionPromptDenied => roundtrip(
            request,
            |output| format!("bash denied as expected: {output}"),
            "toolu_bash_prompt_deny",
            "bash",
            json!({"command":"printf 'should not run'","timeout":1000}),
        ),
        Scenario::PluginToolRoundtrip => roundtrip(
            request,
            |output| format!("plugin tool completed: {}", extract_plugin_message(output)),
            "toolu_plugin_echo",
            "plugin_echo",
            json!({"message":"hello from plugin parity"}),
        ),
        Scenario::AutoCompactTriggered => (
            json!({ "role": "assistant", "content": "auto compact parity complete." }),
            "stop",
        ),
        Scenario::TokenCostReporting => (
            json!({ "role": "assistant", "content": "token cost reporting parity complete." }),
            "stop",
        ),
    }
}

#[allow(clippy::needless_pass_by_value)]
fn roundtrip(
    request: &ChatRequest,
    final_text: impl FnOnce(&str) -> String,
    tool_id: &str,
    tool_name: &str,
    tool_input: Value,
) -> (Value, &'static str) {
    match latest_tool_result(request) {
        Some((output, _)) => (
            json!({ "role": "assistant", "content": final_text(&output) }),
            "stop",
        ),
        None => tool_message(tool_id, tool_name, tool_input),
    }
}

fn bash_prompt_roundtrip(request: &ChatRequest) -> (Value, &'static str) {
    match latest_tool_result(request) {
        Some((output, is_error)) => {
            let text = if is_error {
                format!("bash approval unexpectedly failed: {output}")
            } else {
                format!(
                    "bash approved and executed: {}",
                    extract_bash_stdout(&output)
                )
            };
            (json!({ "role": "assistant", "content": text }), "stop")
        }
        None => tool_message(
            "toolu_bash_prompt_allow",
            "bash",
            json!({"command":"printf 'approved via prompt'","timeout":1000}),
        ),
    }
}

fn multi_tool_roundtrip(request: &ChatRequest) -> (Value, &'static str) {
    let tool_results = tool_results_by_name(request);
    match (
        tool_results.get("read_file"),
        tool_results.get("grep_search"),
    ) {
        (Some((read_output, _)), Some((grep_output, _))) => (
            json!({ "role": "assistant", "content": format!("multi-tool roundtrip complete: {} / {} occurrences", extract_read_content(read_output), extract_num_matches(grep_output)) }),
            "stop",
        ),
        _ => (
            json!({
                "role": "assistant",
                "content": null,
                "tool_calls": [
                    tool_call("toolu_multi_read", "read_file", json!({"path":"fixture.txt"})),
                    tool_call("toolu_multi_grep", "grep_search", json!({"pattern":"parity","path":"fixture.txt","output_mode":"count"}))
                ]
            }),
            "tool_calls",
        ),
    }
}

#[allow(clippy::too_many_lines)]
fn build_stream_body(model: &str, scenario: Scenario) -> String {
    let mut body = String::new();
    match scenario {
        Scenario::AutoCompactTriggered => {
            append_sse(
                &mut body,
                chat_chunk(
                    model,
                    "msg_auto_compact",
                    "auto compact parity complete.",
                    None,
                    Some((50_000, 200)),
                ),
            );
        }
        Scenario::TokenCostReporting => {
            append_sse(
                &mut body,
                chat_chunk(
                    model,
                    "msg_token_cost",
                    "token cost reporting parity complete.",
                    None,
                    Some((1_000, 500)),
                ),
            );
        }
        Scenario::StreamingText => {
            append_sse(
                &mut body,
                chat_chunk(model, "msg_streaming_text", "Mock streaming ", None, None),
            );
            append_sse(
                &mut body,
                chat_chunk(
                    model,
                    "msg_streaming_text",
                    "says hello from the parity harness.",
                    None,
                    Some((11, 8)),
                ),
            );
        }
        Scenario::MultiToolTurnRoundtrip => {
            append_sse(
                &mut body,
                chat_tool_call(
                    model,
                    "msg_multi_tool",
                    "toolu_multi_read",
                    "read_file",
                    r#"{"path":"fixture.txt"}"#,
                ),
            );
            append_sse(
                &mut body,
                chat_tool_call(
                    model,
                    "msg_multi_tool",
                    "toolu_multi_grep",
                    "grep_search",
                    r#"{"pattern":"parity","path":"fixture.txt","output_mode":"count"}"#,
                ),
            );
        }
        scenario => {
            let tool_call = match scenario {
                Scenario::ReadFileRoundtrip => Some((
                    "toolu_read_fixture",
                    "read_file",
                    r#"{"path":"fixture.txt"}"#,
                )),
                Scenario::GrepChunkAssembly => Some((
                    "toolu_grep_fixture",
                    "grep_search",
                    r#"{"pattern":"parity","path":"fixture.txt","output_mode":"count"}"#,
                )),
                Scenario::WriteFileAllowed => Some((
                    "toolu_write_allowed",
                    "write_file",
                    r#"{"path":"generated/output.txt","content":"created by mock service\n"}"#,
                )),
                Scenario::WriteFileDenied => Some((
                    "toolu_write_denied",
                    "write_file",
                    r#"{"path":"generated/denied.txt","content":"should not exist\n"}"#,
                )),
                Scenario::BashStdoutRoundtrip => Some((
                    "toolu_bash_stdout",
                    "bash",
                    r#"{"command":"printf 'alpha from bash'","timeout":1000}"#,
                )),
                Scenario::BashPermissionPromptApproved => Some((
                    "toolu_bash_prompt_allow",
                    "bash",
                    r#"{"command":"printf 'approved via prompt'","timeout":1000}"#,
                )),
                Scenario::BashPermissionPromptDenied => Some((
                    "toolu_bash_prompt_deny",
                    "bash",
                    r#"{"command":"printf 'should not run'","timeout":1000}"#,
                )),
                Scenario::PluginToolRoundtrip => Some((
                    "toolu_plugin_echo",
                    "plugin_echo",
                    r#"{"message":"hello from plugin parity"}"#,
                )),
                Scenario::StreamingText
                | Scenario::AutoCompactTriggered
                | Scenario::TokenCostReporting
                | Scenario::MultiToolTurnRoundtrip => None,
            };
            if let Some((id, name, arguments)) = tool_call {
                append_sse(
                    &mut body,
                    chat_tool_call(model, &format!("msg_{id}"), id, name, arguments),
                );
            }
        }
    }
    body.push_str("data: [DONE]\n\n");
    body
}

/// Build an `OpenAI` SSE `data:` chunk with assistant text content.
fn chat_chunk(
    model: &str,
    id: &str,
    text: &str,
    finish_reason: Option<&str>,
    usage: Option<(u32, u32)>,
) -> String {
    let mut choice = json!({ "index": 0, "delta": { "content": text } });
    if let Some(reason) = finish_reason {
        choice["finish_reason"] = json!(reason);
    } else {
        choice["finish_reason"] = Value::Null;
    }
    let mut payload = json!({
        "id": id,
        "object": "chat.completion.chunk",
        "model": model,
        "choices": [choice]
    });
    if let Some((prompt, completion)) = usage {
        payload["usage"] = json!({
            "prompt_tokens": prompt,
            "completion_tokens": completion,
            "total_tokens": prompt + completion
        });
    }
    format!("data: {payload}\n\n")
}

/// Build an `OpenAI` SSE `data:` chunk requesting a tool call.
fn chat_tool_call(model: &str, id: &str, tool_id: &str, name: &str, arguments: &str) -> String {
    let payload = json!({
        "id": id,
        "object": "chat.completion.chunk",
        "model": model,
        "choices": [{
            "index": 0,
            "delta": {
                "tool_calls": [{
                    "index": 0,
                    "id": tool_id,
                    "type": "function",
                    "function": { "name": name, "arguments": arguments }
                }]
            },
            "finish_reason": "tool_calls"
        }]
    });
    format!("data: {payload}\n\n")
}

#[allow(clippy::needless_pass_by_value)]
fn append_sse(buffer: &mut String, frame: String) {
    buffer.push_str(&frame);
}

#[allow(clippy::needless_pass_by_value)]
fn tool_message(tool_id: &str, name: &str, input: Value) -> (Value, &'static str) {
    (
        json!({
            "role": "assistant",
            "content": null,
            "tool_calls": [tool_call(tool_id, name, input)]
        }),
        "tool_calls",
    )
}

#[allow(clippy::needless_pass_by_value)]
fn tool_call(tool_id: &str, name: &str, input: Value) -> Value {
    json!({
        "id": tool_id,
        "type": "function",
        "function": { "name": name, "arguments": input.to_string() }
    })
}

fn http_response(status: &str, content_type: &str, body: &str, headers: &[(&str, &str)]) -> String {
    let mut extra_headers = String::new();
    for (name, value) in headers {
        use std::fmt::Write as _;
        write!(&mut extra_headers, "{name}: {value}\r\n").expect("header write should succeed");
    }
    format!(
        "HTTP/1.1 {status}\r\ncontent-type: {content_type}\r\n{extra_headers}content-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    )
}

fn request_id_for(scenario: Scenario) -> &'static str {
    match scenario {
        Scenario::StreamingText => "req_streaming_text",
        Scenario::ReadFileRoundtrip => "req_read_file_roundtrip",
        Scenario::GrepChunkAssembly => "req_grep_chunk_assembly",
        Scenario::WriteFileAllowed => "req_write_file_allowed",
        Scenario::WriteFileDenied => "req_write_file_denied",
        Scenario::MultiToolTurnRoundtrip => "req_multi_tool_turn_roundtrip",
        Scenario::BashStdoutRoundtrip => "req_bash_stdout_roundtrip",
        Scenario::BashPermissionPromptApproved => "req_bash_permission_prompt_approved",
        Scenario::BashPermissionPromptDenied => "req_bash_permission_prompt_denied",
        Scenario::PluginToolRoundtrip => "req_plugin_tool_roundtrip",
        Scenario::AutoCompactTriggered => "req_auto_compact_triggered",
        Scenario::TokenCostReporting => "req_token_cost_reporting",
    }
}

fn message_id_for(scenario: Scenario) -> String {
    format!("msg_{}", request_id_for(scenario))
}

fn extract_read_content(tool_output: &str) -> String {
    serde_json::from_str::<Value>(tool_output)
        .ok()
        .and_then(|value| {
            value
                .get("file")
                .and_then(|file| file.get("content"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| tool_output.trim().to_string())
}

#[allow(clippy::cast_possible_truncation)]
fn extract_num_matches(tool_output: &str) -> usize {
    serde_json::from_str::<Value>(tool_output)
        .ok()
        .and_then(|value| value.get("numMatches").and_then(Value::as_u64))
        .unwrap_or(0) as usize
}

fn extract_file_path(tool_output: &str) -> String {
    serde_json::from_str::<Value>(tool_output)
        .ok()
        .and_then(|value| {
            value
                .get("filePath")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| tool_output.trim().to_string())
}

fn extract_bash_stdout(tool_output: &str) -> String {
    serde_json::from_str::<Value>(tool_output)
        .ok()
        .and_then(|value| {
            value
                .get("stdout")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| tool_output.trim().to_string())
}

fn extract_plugin_message(tool_output: &str) -> String {
    serde_json::from_str::<Value>(tool_output)
        .ok()
        .and_then(|value| {
            value
                .get("input")
                .and_then(|input| input.get("message"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| tool_output.trim().to_string())
}
