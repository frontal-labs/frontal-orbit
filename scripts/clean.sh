#!/usr/bin/env bash
# Remove Bazel outputs. `EXPUNGE=1` performs a full expunge.
set -uo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"
if [ "${EXPUNGE:-0}" = "1" ]; then
  bazel clean --expunge
else
  bazel clean
fi
