#!/usr/bin/env bash
set -euo pipefail
cd "${BUILD_WORKSPACE_DIRECTORY:-.}"
ACTION="${1:-test}"
cd orbit-cli

case "$ACTION" in
  test)
    echo "==> Running orbit-cli test suite"
    node --test test/*.test.mjs
    ;;
  lint)
    if command -v bunx >/dev/null 2>&1; then
      bunx @biomejs/biome check . || echo "WARN: biome check failed"
    elif command -v npx >/dev/null 2>&1; then
      npx -y @biomejs/biome check . || echo "WARN: biome check failed"
    else
      echo "WARN: bun/npx not found; skipping Biome check"
    fi
    ;;
  build)
    echo "==> Building orbit-cli"
    ./scripts/build.sh
    ;;
  *) echo "unknown action: $ACTION" >&2; exit 2 ;;
esac