# Installation Guide

This guide covers installing both the Orbit CLI tool and the Orbit server.

## CLI Installation

### Homebrew (Recommended)

The easiest way to install the Orbit CLI is via Homebrew:

```bash
brew install --HEAD ./homebrew/orbit.rb
```

This will build and install the `orbit` binary from source.

### From Source

If you prefer to build from source:

```bash
# Clone the repository
git clone https://github.com/frontal-labs/frontal-orbit.git
cd frontal-orbit

# Build the workspace
cargo build --workspace

# Run the CLI
cargo run -p orbit-cli -- ...
```

## Server Installation

### Docker (Recommended)

The Orbit server is available as a Docker container:

```bash
# Build the server image
docker build -f infrastructure/docker/orbit-server.Dockerfile -t orbit-server .

# Run the server
docker run -p 8080:8080 orbit-server
```

### Docker Compose

For a complete setup with dependencies:

```bash
cd infrastructure/compose
cp .env.example .env
# Edit .env with your configuration
docker-compose up -d
```

### From Source

Build and run the server directly:

```bash
# Build the server
cargo build -p orbit-server

# Run the server
cargo run -p orbit-server
```

## Configuration

After installation, configure your API credentials:

```bash
export ORBIT_API_KEY="sk-ant-..."
# Or use Frontal's OpenAI-compatible API gateway
export FRONTAL_API_KEY="frontal-..."
export FRONTAL_BASE_URL="https://api.frontal.ai/v1"
# Or use an Anthropic proxy
export ORBIT_BASE_URL="https://your-proxy.com"
```

## Verification

Verify your installation:

```bash
# Check CLI version
orbit --version

# Test CLI functionality
orbit --help

# Test server (if running)
curl http://localhost:8080/health
```

## Quick Start

Once installed, you can start using Orbit:

```bash
# Interactive REPL
orbit --model claude-opus-4-6

# One-shot prompt
orbit prompt "explain this codebase"

# Check status
orbit status
```

## Troubleshooting

### Build Issues

If you encounter build issues:

1. Ensure you have Rust installed: `rustc --version`
2. Update Rust: `rustup update`
3. Clean build cache: `cargo clean`

### Permission Issues

If you get permission errors:

1. Check binary permissions: `ls -la $(which orbit)`
2. Reinstall with Homebrew: `brew reinstall --HEAD ./homebrew/orbit.rb`

### Server Issues

If the server won't start:

1. Check port availability: `lsof -i :8080`
2. Verify Docker is running: `docker version`
3. Check logs: `docker logs orbit-server`

For more detailed troubleshooting, see [TROUBLESHOOTING.md](TROUBLESHOOTING.md).
