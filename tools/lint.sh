#!/usr/bin/env bash
# Frontal Orbit — monorepo linter.
#
# Runs Biome (TS/JS/JSON/YAML) and Rust clippy. Each step is best-effort so a
# missing toolchain never blocks the others. Accepts an optional path argument
# (default: repo root) so it can target a single SDK, e.g. `tools/lint.sh sdk/typescript`.
set -uo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET="${1:-$REPO_ROOT}"
cd "$REPO_ROOT"

echo "==> linting $TARGET"

if command -v bun >/dev/null 2>&1; then
  bunx @biomejs/biome check "$TARGET" || echo "WARN: biome check failed"
elif command -v npx >/dev/null 2>&1; then
  npx -y @biomejs/biome check "$TARGET" || echo "WARN: biome check failed"
else
  echo "WARN: bun/npx not found; skipping Biome check"
fi

if command -v cargo >/dev/null 2>&1; then
  cargo clippy --workspace --all-targets -- -D warnings \
    || echo "WARN: cargo clippy failed"
else
  echo "WARN: cargo not found; skipping Rust lint"
fi
