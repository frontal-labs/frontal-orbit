# Container Workflows

Container workflows for building and running the Orbit Rust workspace.

## Files in this repo

- `Dockerfile` - multi-stage production-style image that installs `orbit` into `/usr/local/bin/orbit`.
- `Dockerfile.dev` - development image with `cargo-watch` for iterative workflows.

## Build images

From the repository root:

### Docker

```bash
docker build -t orbit:latest -f Dockerfile .
docker build -t orbit-dev:latest -f Dockerfile.dev .
```

### Podman

```bash
podman build -t orbit:latest -f Dockerfile .
podman build -t orbit-dev:latest -f Dockerfile.dev .
```

## Run tests in a container (bind-mount source)

### Docker

```bash
docker run --rm -it \
  -v "$PWD":/workspace \
  -e CARGO_TARGET_DIR=/tmp/orbit-target \
  -w /workspace \
  orbit-dev:latest \
  cargo test --workspace
```

### Podman

```bash
podman run --rm -it \
  -v "$PWD":/workspace:Z \
  -e CARGO_TARGET_DIR=/tmp/orbit-target \
  -w /workspace \
  orbit-dev:latest \
  cargo test --workspace
```

## Run the CLI in containerized mode

### Using the production image

```bash
docker run --rm -it orbit:latest --help
```

### Mount a working directory for sessions/config

```bash
docker run --rm -it \
  -v "$PWD":/workspace \
  -w /workspace \
  orbit:latest prompt "summarize this repository"
```

## Notes

- `CARGO_TARGET_DIR=/tmp/orbit-target` prevents container-owned artifacts in your host `target/`.
- On SELinux-enabled hosts, keep Podman `:Z` volume labels.
- For host-native workflows, use `cargo build --workspace` and `cargo test --workspace` directly.
