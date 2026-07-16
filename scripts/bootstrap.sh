#!/usr/bin/env bash
# Resolve the repository root from this script's location.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "==> Bootstrapping frontal-orbit Bazel monorepo"

if ! command -v bazel >/dev/null 2>&1 && ! command -v bazelisk >/dev/null 2>&1; then
  echo "ERROR: bazel/bazelisk not found on PATH. Install Bazel first." >&2
  exit 1
fi

# Pre-commit keeps hooks reproducible.
if command -v pre-commit >/dev/null 2>&1; then
  pre-commit install
else
  echo "WARN: pre-commit not installed; skipping hook install." >&2
fi

# Non-fatal: network / cargo-env failures must not block bootstrap.
bazel mod tidy || echo "WARN: 'bazel mod tidy' failed (non-fatal)."

echo "==> Bootstrap complete."
