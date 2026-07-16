# Getting Started (Bazel)

Two ways to build Frontal Orbit with Bazel.

## Option A — Dev container (recommended)

The dev container installs Bazel via the official apt repository (keyring
based) and layers language features (Git, Node 22, Rust 1.80, Python 3.11).

1. Open the repo in a dev container (`Dev Containers: Reopen in Container`).
2. Post-create runs `make bootstrap`, which installs pre-commit hooks and runs
   `bazel mod tidy` (non-fatal).

## Option B — Local machine

1. Install [Bazelisk](https://github.com/bazelbuild/bazelisk). The pinned
   version comes from `.bazelversion` (`7.4.0`).
2. Bootstrap:

   ```bash
   make bootstrap
   ```

## Common commands

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

## Local overrides

Never put machine-specific flags in `.bazelrc`. Add them to the gitignored
`.bazelrc.project`, for example:

```bash
common --output_user_root=~/.cache/bazel/frontal-orbit
```

### Remote cache

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
