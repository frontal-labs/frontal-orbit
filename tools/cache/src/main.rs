//! `tools-cache` — inspect and prune build caches (Bazel + Cargo).
//!
//! Subcommands:
//!   status  report sizes of Bazel output base and Cargo target dir
//!   clean   run `bazel clean` (optionally --expunge)
//!   prune   remove Cargo `target/` (with --yes)
//!   bazel   Bazel-specific cache/server operations (status/clean/info)

#![allow(clippy::cast_precision_loss)]

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(name = "orbit-cache", about = "Inspect and prune Bazel/Cargo caches")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Report cache sizes.
    Status,
    /// Run `bazel clean`.
    Clean {
        /// Pass --expunge to Bazel.
        #[arg(long)]
        expunge: bool,
    },
    /// Remove Cargo target dir.
    Prune {
        #[arg(long)]
        yes: bool,
    },
    /// Bazel-specific cache / server operations.
    Bazel {
        #[command(subcommand)]
        op: BazelOp,
    },
}

#[derive(Subcommand)]
enum BazelOp {
    /// Show Bazel output base, server pid, and release.
    Status,
    /// Run `bazel clean` (optionally --expunge).
    Clean {
        /// Pass --expunge to Bazel.
        #[arg(long)]
        expunge: bool,
    },
    /// Print selected `bazel info` keys.
    Info,
}

fn dir_size(path: &std::path::Path) -> u64 {
    if !path.exists() {
        return 0;
    }
    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];
    while let Some(p) = stack.pop() {
        let Ok(read) = std::fs::read_dir(&p) else {
            continue;
        };
        for entry in read.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if let Ok(m) = entry.metadata() {
                total += m.len();
            }
        }
    }
    total
}

fn human(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB"];
    let mut v = bytes as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    format!("{:.1}{}", v, UNITS[i])
}

fn bazel_info(key: &str) -> Option<String> {
    Command::new("bazel")
        .args(["info", key])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Status => {
            let base = bazel_info("output_base");
            let bazel_bytes = base
                .as_ref()
                .map_or(0, |b| dir_size(std::path::Path::new(b)));
            let target = PathBuf::from("target");
            let target_bytes = dir_size(&target);
            println!("Bazel output base : {base:?}");
            println!("Bazel cache size  : {}", human(bazel_bytes));
            println!(
                "Cargo target      : {:?}",
                target.to_string_lossy().into_owned()
            );
            println!("Cargo target size : {}", human(target_bytes));
            println!("Total            : {}", human(bazel_bytes + target_bytes));
        }
        Cmd::Clean { expunge } => {
            let mut cmd = Command::new("bazel");
            cmd.arg("clean");
            if expunge {
                cmd.arg("--expunge");
            }
            let status = cmd.status().context("failed to run `bazel clean`")?;
            if !status.success() {
                anyhow::bail!("bazel clean exited with {status}");
            }
            println!("Bazel cache cleared.");
        }
        Cmd::Prune { yes } => {
            if !yes {
                anyhow::bail!("refusing to delete target/ without --yes");
            }
            let target = PathBuf::from("target");
            if target.exists() {
                std::fs::remove_dir_all(&target).context("failed to remove target/")?;
                println!("Removed target/.");
            } else {
                println!("target/ does not exist.");
            }
        }
        Cmd::Bazel { op } => match op {
            BazelOp::Status => {
                let base = bazel_info("output_base");
                let pid = bazel_info("server_pid");
                let release = bazel_info("release");
                println!("output_base : {}", base.unwrap_or_default());
                println!("server_pid  : {}", pid.unwrap_or_default());
                println!("release     : {}", release.unwrap_or_default());
            }
            BazelOp::Clean { expunge } => {
                let mut cmd = Command::new("bazel");
                cmd.arg("clean");
                if expunge {
                    cmd.arg("--expunge");
                }
                let status = cmd.status().context("failed to run `bazel clean`")?;
                if !status.success() {
                    anyhow::bail!("bazel clean exited with {status}");
                }
                println!("Bazel cache cleared.");
            }
            BazelOp::Info => {
                for key in [
                    "output_base",
                    "execution_root",
                    "bazel-bin",
                    "bazel-testlogs",
                    "workspace",
                ] {
                    match bazel_info(key) {
                        Some(v) => println!("{key:<16} {v}"),
                        None => println!("{key:<16} (unavailable)"),
                    }
                }
            }
        },
    }
    Ok(())
}
