//! `tools-benchmark` — lightweight build/CI timing harness.
//!
//! Measures `cargo build` wall time, reports binary sizes, and can time an
//! arbitrary command (e.g. a one-shot `orbit` invocation) to track CLI latency
//! regressions. Emits JSON so results can be diffed against a baseline.

#![allow(clippy::cast_precision_loss)]

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

#[derive(Parser)]
#[command(name = "orbit-benchmark", about = "Build/CI timing harness")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
    #[arg(long, value_enum, default_value_t = Out::Text)]
    format: Out,
}

#[derive(Subcommand)]
enum Cmd {
    /// Time `cargo build --workspace`.
    Build,
    /// List built binaries and their sizes under target/.
    Sizes,
    /// Time an arbitrary command.
    Run {
        /// Command and args to time.
        command: String,
        args: Vec<String>,
    },
}

#[derive(Copy, Clone, ValueEnum)]
enum Out {
    Text,
    Json,
}

#[derive(Serialize)]
struct BenchResult {
    kind: String,
    elapsed_ms: u128,
    detail: String,
}

fn human(bytes: u64) -> String {
    const U: &[&str] = &["B", "KiB", "MiB", "GiB"];
    let mut v = bytes as f64;
    let mut i = 0;
    while v >= 1024.0 && i < U.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    format!("{:.1}{}", v, U[i])
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let result = match &cli.cmd {
        Cmd::Build => {
            let start = Instant::now();
            let status = Command::new("cargo")
                .args(["build", "--workspace"])
                .status()
                .map_err(|e| anyhow::anyhow!("failed to run cargo: {e}"))?;
            let ms = start.elapsed().as_millis();
            if !status.success() {
                anyhow::bail!("cargo build exited with {status}");
            }
            BenchResult { kind: "build".into(), elapsed_ms: ms, detail: "cargo build --workspace".into() }
        }
        Cmd::Sizes => {
            let start = Instant::now();
            let mut entries: Vec<(String, u64)> = Vec::new();
            let dir = PathBuf::from("target/debug");
            if dir.exists() {
                for e in std::fs::read_dir(&dir)?.flatten() {
                    let p = e.path();
                    if p.is_file() {
                        if let Ok(m) = e.metadata() {
                            let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                            entries.push((name, m.len()));
                        }
                    }
                }
            }
            entries.sort_by_key(|b| std::cmp::Reverse(b.1));
            for (name, size) in &entries {
                println!("{:>10}  {}", human(*size), name);
            }
            BenchResult {
                kind: "sizes".into(),
                elapsed_ms: start.elapsed().as_millis(),
                detail: format!("{} binaries", entries.len()),
            }
        }
        Cmd::Run { command, args } => {
            let start = Instant::now();
            let status = Command::new(command).args(args).status();
            let ms = start.elapsed().as_millis();
            match status {
                Ok(s) if s.success() => BenchResult {
                    kind: "run".into(),
                    elapsed_ms: ms,
                    detail: format!("{command} (exit 0)"),
                },
                Ok(s) => anyhow::bail!("{command} exited with {s}"),
                Err(e) => anyhow::bail!("failed to run {command}: {e}"),
            }
        }
    };

    match cli.format {
        Out::Json => println!("{}", serde_json::to_string_pretty(&result)?),
        Out::Text => println!("{:<8} {}  ({})", result.kind, result.elapsed_ms, result.detail),
    }
    Ok(())
}
