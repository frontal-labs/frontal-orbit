#!/usr/bin/env bash
# Frontal Orbit — monorepo formatter.
#
# Runs Biome format --write (TS/JS/JSON/YAML) and cargo fmt. Best-effort per
# tool. Accepts an optional path argument (default: repo root).
set -uo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET="${1:-$REPO_ROOT}"
cd "$REPO_ROOT"

echo "==> formatting $TARGET"

if command -v bun >/dev/null 2>&1; then
  bunx @biomejs/biome format --write "$TARGET" || echo "WARN: biome format failed"
elif command -v npx >/dev/null 2>&1; then
  npx -y @biomejs/biome format --write "$TARGET" || echo "WARN: biome format failed"
else
  echo "WARN: bun/npx not found; skipping Biome format"
fi

if command -v cargo >/dev/null 2>&1; then
  cargo fmt --all || echo "WARN: cargo fmt failed"
else
  echo "WARN: cargo not found; skipping Rust format"
fi
