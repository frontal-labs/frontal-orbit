#!/usr/bin/env bash
# Build the monorepo. Pass TARGETS via ARGS or leave empty for //...
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"
TARGETS="${ARGS:-//...}"
bazel build "$TARGETS"
