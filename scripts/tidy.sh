#!/usr/bin/env bash
# Run `bazel mod tidy`. Non-fatal: network / cargo-env failures are reported
# but must not block development.
set -uo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"
bazel mod tidy || echo "WARN: 'bazel mod tidy' failed (non-fatal)."
