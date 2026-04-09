# Homebrew Formula for Orbit

This directory contains the Homebrew formula for installing the Orbit CLI tool.

## Installation

Install Orbit using the local Homebrew formula:

```bash
brew install --HEAD ./homebrew/orbit.rb
```

This will:
- Build the Orbit CLI from source using Rust
- Install the `orbit` binary to your Homebrew prefix
- Enable tab completion and shell integration

## Formula Details

The `orbit.rb` formula:

- **Description**: High-performance Rust AI agent harness
- **Homepage**: https://github.com/frontal-labs/frontal-orbit
- **License**: MIT
- **Source**: Installs from the main git branch (`--HEAD`)
- **Dependencies**: Rust toolchain for building
- **Install Target**: Builds and installs from `crates/cli`

## Development

When developing locally, you can reinstall the formula after making changes:

```bash
brew reinstall --HEAD ./homebrew/orbit.rb
```

Or build directly from source:

```bash
cargo build --workspace
cargo run -p orbit-cli -- ...
```

## Verification

After installation, verify the CLI is working:

```bash
orbit --version
orbit --help
```

## Uninstallation

Remove Orbit using Homebrew:

```bash
brew uninstall orbit
```
