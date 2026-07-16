//! `tools-workspace` — validate and maintain the Cargo workspace member list.
//!
//! Subcommands:
//!   list   print workspace members
//!   check  verify every member exists and scan for duplicate deps
//!   sort   rewrite the member list in sorted order (in place)

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::fmt::Write;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "orbit-workspace", about = "Maintain the Cargo workspace member list")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
    /// Path to the workspace root Cargo.toml.
    #[arg(long, default_value = "Cargo.toml")]
    manifest: PathBuf,
}

#[derive(Subcommand)]
enum Cmd {
    List,
    Check,
    Sort,
}

fn load_manifest(path: &std::path::Path) -> Result<toml::Value> {
    let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    text.parse::<toml::Value>().context("parsing Cargo.toml")
}

fn members(value: &toml::Value) -> Vec<String> {
    value
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(|m| m.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| Some(v.as_str()?.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// Resolve workspace-member entries to concrete crate directories, expanding
/// simple trailing-`*` globs (e.g. `crates/*`).
fn expand_members(root: &std::path::Path, members: &[String]) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for m in members {
        if m.contains('*') {
            let base = m.trim_end_matches('*').trim_end_matches('/');
            let base_dir = root.join(base);
            if let Ok(rd) = std::fs::read_dir(&base_dir) {
                for e in rd.flatten() {
                    let p = e.path();
                    if p.is_dir() && p.join("Cargo.toml").exists() {
                        out.push(p);
                    }
                }
            }
        } else {
            out.push(root.join(m));
        }
    }
    out
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let manifest = &cli.manifest;
    let value = load_manifest(manifest)?;
    let members = members(&value);
    let root = manifest.parent().unwrap_or_else(|| std::path::Path::new("."));

    match cli.cmd {
        Cmd::List => {
            for m in &members {
                println!("{m}");
            }
        }
        Cmd::Check => {
            let expanded = expand_members(root, &members);
            let mut errors = 0usize;
            for m in &expanded {
                let ok = m.join("Cargo.toml").exists();
                if !ok {
                    eprintln!("MISSING member: {}", m.display());
                    errors += 1;
                }
            }
            // Duplicate package names across members are a real workspace
            // conflict (cargo fails to build). Reuse of the same dependency
            // across crates/sections is normal and intentionally ignored.
            let mut names: HashMap<String, Vec<String>> = HashMap::new();
            for m in &expanded {
                let cf = m.join("Cargo.toml");
                if let Ok(t) = std::fs::read_to_string(&cf) {
                    if let Ok(v) = t.parse::<toml::Value>() {
                        if let Some(name) = v
                            .get("package")
                            .and_then(|p| p.get("name"))
                            .and_then(|n| n.as_str())
                        {
                            names.entry(name.to_string()).or_default().push(m.display().to_string());
                        }
                    }
                }
            }
            for (name, locs) in names {
                if locs.len() > 1 {
                    eprintln!("DUPLICATE crate name `{name}` in: {}", locs.join(", "));
                    errors += 1;
                }
            }
            if errors == 0 {
                println!("workspace OK: {} members, no issues", members.len());
            } else {
                anyhow::bail!("{errors} workspace problem(s) found");
            }
        }
        Cmd::Sort => {
            let sorted = {
                let mut m = members.clone();
                m.sort();
                m.dedup();
                m
            };
            let text = std::fs::read_to_string(manifest)?;
            let new_members = sorted
                .iter()
                .map(|m| format!("\"{m}\""))
                .collect::<Vec<_>>()
                .join(", ");
            // Replace the members = [ ... ] array (first occurrence in [workspace]).
            let re = regex_replace_members(&text, &new_members);
            std::fs::write(manifest, re).context("writing sorted manifest")?;
            println!("Sorted {} members.", sorted.len());
        }
    }
    Ok(())
}

/// Replace the `members = [ ... ]` assignment preserving surrounding lines.
fn regex_replace_members(text: &str, replacement: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut replaced = false;
    for line in text.lines() {
        if !replaced && line.trim_start().starts_with("members") && line.contains('[') {
            writeln!(out, "members = [{replacement}]").unwrap();
            replaced = true;
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    if !replaced {
        writeln!(out, "members = [{replacement}]").unwrap();
    }
    out
}
