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
git clone https://github.com/frontal-labs/orbit.git
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

## Bazel Build Foundation (Advanced)

A hermetic, reproducible build foundation is also provided via **Bzlmod** (no
`WORKSPACE` file). All build infrastructure lives under `bazel/` and
`third_party/`; `MODULE.bazel` is the single source of truth.

### Option A — Dev container (recommended)

The dev container installs Bazel via the official apt repository (keyring
based) and layers language features (Git, Node 22, Rust 1.80, Python 3.11).

1. Open the repo in a dev container (`Dev Containers: Reopen in Container`).
2. Post-create runs `make bootstrap`, which installs pre-commit hooks and runs
   `bazel mod tidy` (non-fatal).

### Option B — Local machine

1. Install [Bazelisk](https://github.com/bazelbuild/bazelisk). The pinned
   version comes from `.bazelversion` (`7.4.0`).
2. Bootstrap:

   ```bash
   make bootstrap
   ```

### Common Bazel commands

| Command            | What it does                                  |
|--------------------|-----------------------------------------------|
| `make build`       | `bazel build //...`                           |
| `make test`        | `bazel test //...`                            |
| `make lint`        | `pre-commit run --all-files`                  |
| `make fmt`         | `bazel run //:buildifier -- -r .`             |
| `make tidy`        | `bazel mod tidy` (non-fatal)                  |
| `make doctor`      | Sanity-check the toolchain / environment      |
| `make clean`       | `bazel clean` (`EXPUNGE=1` for full expunge)  |
| `make coverage`    | `bazel coverage //...` → `coverage/lcov.info` |
| `make ci`          | build → test → lint                           |

### Local overrides

Never put machine-specific flags in `.bazelrc`. Add them to the gitignored
`.bazelrc.project`, for example:

```bash
common --output_user_root=~/.cache/bazel/frontal-orbit
```

#### Remote cache

When you have a Bazel remote cache endpoint (e.g. `bazel-remote`, GCS, S3),
uncomment the remote cache stanza in `.bazelrc.project` and fill in the URL:

```bash
build --remote_cache=grpcs://remote-cache.example.com
build --remote_upload_local_results=true
build --remote_timeout=10
```

For fully remote / CI builds you may also want to force a platform:

```bash
build --platforms=//bazel/platforms:linux_x86_64
```

## Configuration

After installation, configure your API credentials:

```bash
export ORBIT_API_KEY="sk-ant-..."
# Or use Frontal's OpenAI-compatible API gateway
export FRONTAL_API_KEY="frontal-..."
export FRONTAL_BASE_URL="https://ai.frontal.dev/v1"
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
orbit --model claude-opus-5

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
