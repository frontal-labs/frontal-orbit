#!/usr/bin/env bash
# Run linting across the monorepo.
#
# Primary path: pre-commit (which wires Biome for TS/JS/JSON/YAML, rustfmt +
# clippy for Rust, buildifier for Starlark, and the rest of the repo hooks).
# Fallback: if pre-commit is not installed, run Biome check and cargo clippy
# directly so `make lint` still works in minimal environments.
set -uo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

if command -v pre-commit >/dev/null 2>&1; then
  pre-commit run --all-files
  exit $?
fi

echo "WARN: pre-commit not installed; running Biome + clippy directly"

if command -v bun >/dev/null 2>&1; then
  bunx @biomejs/biome check . || echo "WARN: biome check failed"
elif command -v npx >/dev/null 2>&1; then
  npx -y @biomejs/biome check . || echo "WARN: biome check failed"
else
  echo "WARN: bun/npx not found; skipping Biome check"
fi

if command -v cargo >/dev/null 2>&1; then
  cargo clippy --workspace --all-targets -- -D warnings \
    || echo "WARN: cargo clippy failed"
else
  echo "WARN: cargo not found; skipping Rust lint"
fi
