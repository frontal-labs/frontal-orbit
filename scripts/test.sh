#!/usr/bin/env bash
# Run all Bazel tests.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"
bazel test "${ARGS:-//...}"
