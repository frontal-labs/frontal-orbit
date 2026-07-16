//! `tools-generators` — scaffold new crates/commands from templates.
//!
//! Uses the `handlebars` crate for templating. Crate templates live under
//! `tools/templates/templates/crate/` and use `{{name}}` placeholders
//! (HTML escaping is disabled so generated Rust — including generics like
//! `Vec<T>` — is emitted verbatim).

use anyhow::{Context, Result};
use clap::Parser;
use handlebars::Handlebars;
use serde_json::json;
use std::path::Path;

#[derive(Parser)]
#[command(name = "orbit-generators", about = "Scaffold crates from templates")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(clap::Subcommand)]
enum Cmd {
    /// Scaffold a new crate under crates/.
    Crate {
        name: String,
        #[arg(long, value_enum, default_value_t = Kind::Lib)]
        kind: Kind,
        /// Target directory (defaults to crates/).
        #[arg(long, default_value = "crates")]
        dest: String,
    },
}

#[derive(Copy, Clone, clap::ValueEnum)]
enum Kind {
    Lib,
    Bin,
}

fn render(hb: &Handlebars, templates_dir: &Path, file: &str, name: &str) -> Result<String> {
    let content = std::fs::read_to_string(templates_dir.join(file))
        .with_context(|| format!("reading template {file}"))?;
    hb.render_template(&content, &json!({ "name": name }))
        .with_context(|| format!("rendering template {file}"))
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Crate { name, kind, dest } => {
            let templates_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join("templates/templates");

            let mut hb = Handlebars::new();
            // Emit generated Rust verbatim (generics like Vec<T> must not be HTML-escaped).
            hb.register_escape_fn(handlebars::no_escape);

            let src_template = match kind {
                Kind::Bin => "crate/crate_bin.rs",
                Kind::Lib => "crate/crate_lib.rs",
            };

            let out_dir = Path::new(&dest).join(&name);
            if out_dir.exists() {
                anyhow::bail!("{} already exists", out_dir.display());
            }
            std::fs::create_dir_all(&out_dir).context("create crate dir")?;
            std::fs::create_dir_all(out_dir.join("src")).context("create src dir")?;

            let cargo = render(&hb, &templates_dir, "crate/Cargo.toml", &name)?;
            let lib = render(&hb, &templates_dir, src_template, &name)?;
            std::fs::write(out_dir.join("Cargo.toml"), cargo).context("write Cargo.toml")?;
            let entry = match kind {
                Kind::Bin => "src/main.rs",
                Kind::Lib => "src/lib.rs",
            };
            std::fs::write(out_dir.join(entry), lib).context("write source")?;

            println!("Generated {} crate at {}", kind_as_str(kind), out_dir.display());
            println!("  - Cargo.toml");
            println!("  - {entry}");
            println!("Remember to run `cargo build` and add it to the workspace if needed.");
        }
    }
    Ok(())
}

fn kind_as_str(k: Kind) -> &'static str {
    match k {
        Kind::Lib => "library",
        Kind::Bin => "binary",
    }
}
