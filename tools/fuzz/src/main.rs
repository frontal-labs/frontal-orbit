//! `tools-fuzz` — discover, scaffold, and run fuzz targets.
//!
//! Fuzz targets themselves live under `tools/fuzz/fuzz_targets/<name>/` as
//! standalone `cargo-fuzz` crates (excluded from the main workspace so they
//! don't perturb `cargo build --workspace`). This binary lists them, scaffolds
//! new ones from `tools-templates`, and dispatches runs to `cargo +nightly fuzz`.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use tools_templates::render_file;

#[derive(Parser)]
#[command(name = "orbit-fuzz", about = "Manage fuzz targets")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// List discovered fuzz targets.
    List,
    /// Scaffold a new fuzz target under `fuzz_targets/`.
    New { name: String },
    /// Run a fuzz target (requires cargo-fuzz + nightly).
    Run {
        name: String,
        /// Extra args passed to `cargo fuzz run`.
        extra: Vec<String>,
    },
}

fn fuzz_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("fuzz_targets")
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::List => {
            let root = fuzz_root();
            if !root.exists() {
                println!("(no fuzz_targets dir)");
                return Ok(());
            }
            for e in std::fs::read_dir(&root)?.flatten() {
                if e.path().is_dir() {
                    println!("{}", e.file_name().to_string_lossy());
                }
            }
        }
        Cmd::New { name } => {
            let templates_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join("templates/templates");
            let mut vars = HashMap::new();
            vars.insert("name", name.clone());
            let root = fuzz_root().join(&name);
            if root.exists() {
                anyhow::bail!("{} already exists", root.display());
            }
            std::fs::create_dir_all(root.join("fuzz_targets")).context("create target dir")?;
            let cargo = render_file(&templates_dir, "fuzz/Cargo.toml", &vars)?;
            let target = render_file(&templates_dir, "fuzz/target.rs", &vars)?;
            std::fs::write(root.join("Cargo.toml"), cargo)?;
            std::fs::write(root.join("fuzz_targets").join(format!("{name}.rs")), target)?;
            println!("Scaffolded fuzz target `{name}` at {}", root.display());
            println!("Run with: cargo +nightly fuzz run {name}");
        }
        Cmd::Run { name, extra } => {
            let has_cargo_fuzz = Command::new("cargo")
                .args(["fuzz", "--help"])
                .output()
                .is_ok_and(|o| o.status.success());
            if !has_cargo_fuzz {
                anyhow::bail!(
                    "cargo-fuzz not installed. Install with `cargo +nightly install cargo-fuzz` (nightly required)."
                );
            }
            let status = Command::new("cargo")
                .arg("+nightly")
                .arg("fuzz")
                .arg("run")
                .arg(&name)
                .args(&extra)
                .status()?;
            if !status.success() {
                anyhow::bail!("fuzz run exited with {status}");
            }
        }
    }
    Ok(())
}
