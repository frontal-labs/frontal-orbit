//! `tools-version` — bump the monorepo version across the workspace.
//!
//! Keeps `Cargo.toml` `[workspace.package].version` and `MODULE.bazel`
//! `module(version=...)` in sync, and can stamp `CHANGELOG.md`.
//!
//! Crates that use `version.workspace = true` inherit automatically, so only
//! the two root files need editing.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use semver::Version;
use std::path::Path;

#[derive(Parser)]
#[command(name = "orbit-version", about = "Bump the monorepo version")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
    #[arg(long, default_value = "Cargo.toml")]
    manifest: String,
    #[arg(long, default_value = "MODULE.bazel")]
    module: String,
    #[arg(long)]
    dry_run: bool,
}

#[derive(Subcommand)]
enum Cmd {
    /// Show the current version.
    Show,
    /// Bump a release part.
    Bump {
        #[arg(value_enum)]
        part: Part,
    },
    /// Set an explicit version.
    Set { version: String },
    /// Run `changeset version` then sync Cargo.toml/MODULE.bazel.
    Changeset,
}

#[derive(clap::ValueEnum, Clone, Copy)]
enum Part {
    Major,
    Minor,
    Patch,
}

fn read(path: &str) -> Result<String> {
    std::fs::read_to_string(path).with_context(|| format!("reading {path}"))
}

fn write(path: &str, content: &str, dry_run: bool) -> Result<()> {
    if dry_run {
        println!("--- would write {path} ---");
        println!("{content}");
        return Ok(());
    }
    std::fs::write(path, content).with_context(|| format!("writing {path}"))
}

fn current_version(manifest: &str) -> Result<Version> {
    let text = read(manifest)?;
    let line = text
        .lines()
        .find(|l| l.trim_start().starts_with("version") && l.contains('='))
        .context("no `version =` line found in manifest")?;
    let v = line.split('"').nth(1).context("malformed version line")?;
    Version::parse(v).with_context(|| format!("parsing version {v}"))
}

fn bump(v: &Version, part: Part) -> Version {
    let mut v = v.clone();
    match part {
        Part::Major => {
            v.major += 1;
            v.minor = 0;
            v.patch = 0;
        }
        Part::Minor => {
            v.minor += 1;
            v.patch = 0;
        }
        Part::Patch => v.patch += 1,
    }
    v
}

fn set_manifest_version(text: &str, new: &Version) -> String {
    text.lines()
        .map(|l| {
            if l.trim_start().starts_with("version") && l.contains('=') && l.contains('"') {
                // Preserve indentation.
                let indent = l.len() - l.trim_start().len();
                format!("{}{} = \"{}\"", " ".repeat(indent), "version", new)
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn set_module_version(text: &str, new: &Version) -> String {
    text.lines()
        .map(|l| {
            if l.trim_start().starts_with("module(") {
                // Replace version = "..." inside the module() block (next line).
                l.to_string()
            } else if l.trim_start().starts_with("version") && l.contains('"') {
                let indent = l.len() - l.trim_start().len();
                format!("{}{} = \"{}\"", " ".repeat(indent), "version", new)
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let cur = current_version(&cli.manifest)?;

    let next = match &cli.cmd {
        Cmd::Show => {
            println!("{cur}");
            return Ok(());
        }
        Cmd::Bump { part } => bump(&cur, *part),
        Cmd::Set { version } => {
            Version::parse(version).with_context(|| format!("parsing {version}"))?
        }
        Cmd::Changeset => anyhow::bail!(
            "`version changeset` is not implemented yet.\n\
             Run `bun changeset version` to update the npm package versions, then \
             `version set <version>` to sync Cargo.toml and MODULE.bazel."
        ),
    };

    println!("{cur} -> {next}");

    let manifest = read(&cli.manifest)?;
    let manifest_new = set_manifest_version(&manifest, &next);
    write(&cli.manifest, &manifest_new, cli.dry_run)?;

    if Path::new(&cli.module).exists() {
        let module = read(&cli.module)?;
        let module_new = set_module_version(&module, &next);
        write(&cli.module, &module_new, cli.dry_run)?;
    } else {
        println!("(skipped {}: not found)", cli.module);
    }

    // Stamp CHANGELOG if present.
    let changelog = "CHANGELOG.md";
    if !cli.dry_run && Path::new(changelog).exists() {
        let body = read(changelog)?;
        if !body.contains(&format!("## [{next}]")) {
            let stamped =
                format!("## [{next}] - unreleased\n\n- Version bump via orbit-version.\n\n{body}");
            write(changelog, &stamped, cli.dry_run)?;
        }
    }
    Ok(())
}
