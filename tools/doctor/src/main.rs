//! `tools-doctor` — environment and toolchain health checker for frontal-orbit.
//!
//! Checks the toolchains the monorepo depends on (Bazel, Rust, Node, Docker,
//! Git, pre-commit), validates `.orbit.json`, and optionally probes network
//! egress. Exits non-zero if any required check fails so it can gate CI.

use anyhow::Result;
use clap::{Parser, ValueEnum};
use serde::Serialize;
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(
    name = "orbit-doctor",
    about = "Check the frontal-orbit dev environment"
)]
struct Cli {
    /// Output format.
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,
    /// Also probe outbound network connectivity.
    #[arg(long)]
    check_network: bool,
    /// Directory to inspect (defaults to repo root via git).
    #[arg(long, default_value = ".")]
    root: PathBuf,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum, Serialize)]
#[value(rename_all = "lowercase")]
enum Format {
    Text,
    Json,
}

#[derive(Serialize)]
struct Check {
    name: String,
    ok: bool,
    detail: String,
}

fn run_version(bin: &str, args: &[&str]) -> Option<String> {
    Command::new(bin)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().lines().next().unwrap_or("").to_string())
}

fn check(name: &str, detail: Option<String>) -> Check {
    match detail {
        Some(d) => Check {
            name: name.into(),
            ok: true,
            detail: d,
        },
        None => Check {
            name: name.into(),
            ok: false,
            detail: "not found".into(),
        },
    }
}

fn first_line_of(path: &std::path::Path) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
}

#[allow(clippy::too_many_lines)]
fn collect_checks(root: &std::path::Path, check_network: bool) -> Vec<Check> {
    let mut checks: Vec<Check> = Vec::new();

    let bazel = run_version("bazel", &["--version"]);
    let pinned = first_line_of(&root.join(".bazelversion"));
    let bazel_detail = match (&bazel, &pinned) {
        (Some(have), Some(want)) => {
            let ok = have.contains(want.trim());
            Some((
                ok,
                format!("{have} (pinned {want})"),
                format!("installed {have} but .bazelversion pins {want}"),
            ))
        }
        (Some(have), None) => Some((true, have.clone(), String::new())),
        _ => None,
    };
    match bazel_detail {
        Some((ok, ok_d, bad_d)) => checks.push(Check {
            name: "bazel".into(),
            ok,
            detail: if ok { ok_d } else { bad_d },
        }),
        None => checks.push(check("bazel", None)),
    }

    checks.push(check("rustc", run_version("rustc", &["--version"])));
    checks.push(check("cargo", run_version("cargo", &["--version"])));
    checks.push(check("node", run_version("node", &["--version"])));
    checks.push(check("docker", run_version("docker", &["--version"])));
    checks.push(check("git", run_version("git", &["--version"])));
    checks.push(check(
        "pre-commit",
        run_version("pre-commit", &["--version"]),
    ));

    let orbit_json = root.join(".orbit.json");
    let orbit_ok = if orbit_json.exists() {
        match std::fs::read_to_string(&orbit_json) {
            Ok(s) => match serde_json::from_str::<serde_json::Value>(&s) {
                Ok(_) => {
                    checks.push(Check {
                        name: ".orbit.json".into(),
                        ok: true,
                        detail: "valid JSON".into(),
                    });
                    true
                }
                Err(e) => {
                    checks.push(Check {
                        name: ".orbit.json".into(),
                        ok: false,
                        detail: format!("invalid JSON: {e}"),
                    });
                    false
                }
            },
            Err(e) => {
                checks.push(Check {
                    name: ".orbit.json".into(),
                    ok: false,
                    detail: e.to_string(),
                });
                false
            }
        }
    } else {
        checks.push(Check {
            name: ".orbit.json".into(),
            ok: false,
            detail: "missing".into(),
        });
        false
    };
    let _ = orbit_ok;

    let lock = root.join("Cargo.lock");
    checks.push(Check {
        name: "Cargo.lock".into(),
        ok: lock.exists(),
        detail: if lock.exists() {
            "present".into()
        } else {
            "missing".into()
        },
    });

    if check_network {
        let net = Command::new("curl")
            .args([
                "-sS",
                "--max-time",
                "5",
                "-o",
                "/dev/null",
                "https://static.crates.io",
            ])
            .status()
            .is_ok_and(|s| s.success());
        checks.push(Check {
            name: "network egress".into(),
            ok: net,
            detail: if net {
                "reachable".into()
            } else {
                "unreachable".into()
            },
        });
    }

    checks
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let checks = collect_checks(&cli.root, cli.check_network);
    let failed = checks.iter().filter(|c| !c.ok).count();

    match cli.format {
        Format::Json => {
            let payload = serde_json::json!({ "checks": checks, "failed": failed });
            println!("{}", serde_json::to_string_pretty(&payload)?);
        }
        Format::Text => {
            for c in &checks {
                let mark = if c.ok { "ok  " } else { "FAIL" };
                println!("[{mark}] {:<16} {detail}", c.name, detail = c.detail);
            }
            println!("---");
            println!("{} checks, {} failing", checks.len(), failed);
        }
    }

    if failed > 0 {
        std::process::exit(1);
    }
    Ok(())
}
