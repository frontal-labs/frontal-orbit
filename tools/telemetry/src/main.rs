//! `tools-telemetry` — wrap a build/CI command and emit a telemetry event.
//!
//! Reads the `telemetry` block of `.orbit.json` for the destination path
//! (`telemetry.path`); if telemetry is disabled or the file is absent, events
//! go to stdout. This keeps the tool usable offline while matching the
//! project's existing telemetry configuration contract.

#![allow(clippy::cast_possible_truncation)]

use anyhow::Result;
use clap::{Parser, Subcommand};
use serde::Serialize;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Parser)]
#[command(name = "orbit-telemetry", about = "Wrap a command and emit a telemetry event")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
    /// Path to .orbit.json (for telemetry config).
    #[arg(long, default_value = ".orbit.json")]
    config: String,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run a command, time it, and emit an event.
    Wrap {
        name: String,
        command: String,
        args: Vec<String>,
    },
    /// Emit a standalone event (e.g. from CI).
    Event {
        name: String,
        #[arg(long, default_value = "info")]
        level: String,
        #[arg(long, default_value = "")]
        message: String,
    },
}

#[derive(Serialize)]
struct Event {
    ts: u64,
    name: String,
    level: String,
    message: String,
    duration_ms: Option<u128>,
    exit_ok: Option<bool>,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis()) as u64
}

fn telemetry_path(config: &str) -> Option<String> {
    let text = std::fs::read_to_string(config).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let tel = v.get("telemetry")?;
    let enabled = tel.get("enabled").and_then(serde_json::Value::as_bool).unwrap_or(false);
    if !enabled {
        return None;
    }
    tel.get("path").and_then(serde_json::Value::as_str).map(str::to_string)
}

fn emit(path: Option<&String>, event: &Event) -> Result<()> {
    let line = serde_json::to_string(event)?;
    match path {
        Some(p) => {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new().create(true).append(true).open(p)?;
            writeln!(f, "{line}")?;
            eprintln!("telemetry: wrote event to {p}");
        }
        None => println!("{line}"),
    }
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let path = telemetry_path(&cli.config);

    match &cli.cmd {
        Cmd::Wrap { name, command, args } => {
            let start = std::time::Instant::now();
            let status = Command::new(command).args(args).status();
            let ms = start.elapsed().as_millis();
            let (ok, msg) = match status {
                Ok(s) => (s.success(), format!("exit {s}")),
                Err(e) => (false, format!("spawn error: {e}")),
            };
            emit(
                path.as_ref(),
                &Event {
                    ts: now_ms(),
                    name: name.clone(),
                    level: if ok { "info" } else { "error" }.into(),
                    message: msg,
                    duration_ms: Some(ms),
                    exit_ok: Some(ok),
                },
            )?;
            if !ok {
                anyhow::bail!("wrapped command failed");
            }
        }
        Cmd::Event { name, level, message } => {
            emit(
                path.as_ref(),
                &Event {
                    ts: now_ms(),
                    name: name.clone(),
                    level: level.clone(),
                    message: message.clone(),
                    duration_ms: None,
                    exit_ok: None,
                },
            )?;
        }
    }
    Ok(())
}
