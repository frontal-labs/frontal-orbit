//! Spawning the `orbit` CLI and parsing its JSONL event stream.

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, Mutex};

use crate::protocol::{OrbitEvent, ThreadItem, TurnResult, Usage};
use serde_json::Value;

/// Errors surfaced while talking to the `orbit` CLI.
#[derive(Debug, thiserror::Error)]
pub enum OrbitError {
    #[error("failed to spawn orbit cli: {0}")]
    Spawn(#[from] std::io::Error),
    #[error("orbit cli exited with code {code:?}: {stderr}")]
    Exit { code: Option<i32>, stderr: String },
}

type EnvMap = std::collections::HashMap<String, String>;

/// The buffered outcome of a turn.
pub struct BufferedTurn {
    pub result: TurnResult,
    pub session_id: Option<String>,
}

fn spawn_and_stream(
    command: &str,
    args: &[String],
    env: EnvMap,
    cwd: Option<&PathBuf>,
) -> Result<(mpsc::Receiver<OrbitEvent>, Child, Arc<Mutex<String>>), OrbitError> {
    let mut cmd = Command::new(command);
    cmd.args(args)
        .envs(&env)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }

    let mut child = cmd.spawn()?;
    let stdout = child.stdout.take().expect("stdout was configured as piped");
    let stderr = child.stderr.take().expect("stderr was configured as piped");

    let stderr_buffer: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    let stderr_buffer_task = Arc::clone(&stderr_buffer);
    tokio::spawn(async move {
        let mut reader = BufReader::new(stderr);
        let mut line = String::new();
        while reader.read_line(&mut line).await.unwrap_or(0) > 0 {
            stderr_buffer_task.lock().await.push_str(&line);
            line.clear();
        }
    });

    let (tx, rx) = mpsc::channel(64);
    tokio::spawn(async move {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        while reader.read_line(&mut line).await.unwrap_or(0) > 0 {
            let trimmed = line.trim();
            if let Some(event) = parse_event(trimmed) {
                if tx.send(event).await.is_err() {
                    break;
                }
            }
            line.clear();
        }
    });

    Ok((rx, child, stderr_buffer))
}

/// Parse a single JSONL event line from the CLI into an [`OrbitEvent`].
///
/// Parsing is done manually (rather than via serde's internally-tagged enum)
/// because the event payloads nest an internally-tagged `ThreadItem`, which
/// serde cannot deserialize through an internally-tagged parent enum.
fn parse_event(line: &str) -> Option<OrbitEvent> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let value: Value = serde_json::from_str(trimmed).ok()?;
    let obj = value.as_object()?;
    let type_str = obj.get("type")?.as_str()?;
    match type_str {
        "turn.started" => Some(OrbitEvent::TurnStarted),
        "item.completed" => {
            let item = parse_item(obj.get("item")?)?;
            Some(OrbitEvent::ItemCompleted { item })
        }
        "turn.completed" => {
            let final_response = obj.get("finalResponse")?.as_str()?.to_string();
            let usage = obj.get("usage").and_then(parse_usage);
            let session_id = obj
                .get("sessionId")
                .and_then(Value::as_str)
                .map(str::to_string);
            Some(OrbitEvent::TurnCompleted {
                final_response,
                usage,
                session_id,
            })
        }
        "turn.failed" => {
            let error = obj.get("error")?.as_str()?.to_string();
            Some(OrbitEvent::TurnFailed { error })
        }
        _ => None,
    }
}

fn parse_item(value: &Value) -> Option<ThreadItem> {
    let obj = value.as_object()?;
    let type_str = obj.get("type")?.as_str()?;
    match type_str {
        "text" => Some(ThreadItem::Text {
            text: obj.get("text")?.as_str()?.to_string(),
        }),
        "tool_use" => Some(ThreadItem::ToolUse {
            name: obj.get("name")?.as_str()?.to_string(),
            input: obj.get("input")?.as_str()?.to_string(),
        }),
        "tool_result" => Some(ThreadItem::ToolResult {
            content: obj.get("content")?.as_str()?.to_string(),
        }),
        "image" => Some(ThreadItem::Image {
            path: obj.get("path").and_then(Value::as_str).map(str::to_string),
            url: obj.get("url").and_then(Value::as_str).map(str::to_string),
        }),
        _ => None,
    }
}

fn parse_usage(value: &Value) -> Option<Usage> {
    let obj = value.as_object()?;
    Some(Usage {
        input_tokens: obj.get("input_tokens")?.as_u64()?,
        output_tokens: obj.get("output_tokens")?.as_u64()?,
        cache_creation_input_tokens: obj
            .get("cache_creation_input_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        cache_read_input_tokens: obj
            .get("cache_read_input_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0),
    })
}

/// Spawn the CLI and buffer the whole turn into a [`BufferedTurn`].
pub async fn run_turn_buffered(
    command: &str,
    args: &[String],
    env: EnvMap,
    cwd: Option<&PathBuf>,
) -> Result<BufferedTurn, OrbitError> {
    let (mut rx, mut child, stderr_buffer) = spawn_and_stream(command, args, env, cwd)?;

    let mut items: Vec<ThreadItem> = Vec::new();
    let mut final_response = String::new();
    let mut usage: Option<Usage> = None;
    let mut session_id: Option<String> = None;

    while let Some(event) = rx.recv().await {
        match &event {
            OrbitEvent::ItemCompleted { item } => items.push(item.clone()),
            OrbitEvent::TurnCompleted {
                final_response: fr,
                usage: u,
                session_id: sid,
            } => {
                final_response = fr.clone();
                usage = u.clone();
                session_id = sid.clone();
            }
            _ => {}
        }
    }

    let status = child.wait().await?;
    if !status.success() {
        let stderr = stderr_buffer.lock().await.clone();
        return Err(OrbitError::Exit {
            code: status.code(),
            stderr,
        });
    }

    Ok(BufferedTurn {
        result: TurnResult {
            final_response,
            items,
            usage,
        },
        session_id,
    })
}

/// A streaming turn. Drain `events` to receive the CLI's structured events;
/// call [`StreamedTurn::finish`] afterwards to await process exit and surface
/// any error.
pub struct StreamedTurn {
    pub events: mpsc::Receiver<OrbitEvent>,
    session_id: Arc<Mutex<Option<String>>>,
    child: Option<Child>,
    stderr: Arc<Mutex<String>>,
}

impl StreamedTurn {
    /// The session id captured from the streamed `turn.completed` event, once
    /// available.
    pub async fn session_id(&self) -> Option<String> {
        self.session_id.lock().await.clone()
    }

    /// Await process exit. Returns `Ok(())` on a zero exit code.
    pub async fn finish(mut self) -> Result<(), OrbitError> {
        if let Some(mut child) = self.child.take() {
            let status = child.wait().await?;
            if !status.success() {
                let stderr = self.stderr.lock().await.clone();
                return Err(OrbitError::Exit {
                    code: status.code(),
                    stderr,
                });
            }
        }
        Ok(())
    }
}

/// Spawn the CLI and stream events. The provided `session_id` handle is updated
/// as `turn.completed` events arrive.
pub async fn run_turn_streamed(
    command: &str,
    args: &[String],
    env: EnvMap,
    cwd: Option<&PathBuf>,
    session_id: Arc<Mutex<Option<String>>>,
) -> Result<StreamedTurn, OrbitError> {
    let (mut rx, child, stderr_buffer) = spawn_and_stream(command, args, env, cwd)?;

    let (tx, events) = mpsc::channel(64);
    let captured = Arc::clone(&session_id);
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            if let OrbitEvent::TurnCompleted {
                session_id: sid, ..
            } = &event
            {
                *captured.lock().await = sid.clone();
            }
            if tx.send(event).await.is_err() {
                break;
            }
        }
    });

    Ok(StreamedTurn {
        events,
        session_id,
        child: Some(child),
        stderr: stderr_buffer,
    })
}
