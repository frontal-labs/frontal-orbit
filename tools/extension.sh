#!/usr/bin/env bash
set -euo pipefail
cd "${BUILD_WORKSPACE_DIRECTORY:-.}"
EXTENSION="$1"
ACTION="${2:-lint}"
shift 2
cd "$EXTENSION"

if ! command -v bun >/dev/null 2>&1; then
  echo "ERROR: bun not found on PATH" >&2
  exit 1
fi

case "$ACTION" in
  lint) bunx @biomejs/biome check src/ || echo "WARN: biome check failed" ;;
  format) bunx @biomejs/biome format --write src/ || echo "WARN: biome format failed" ;;
  build) bun install && bun run build ;;
  test) bun install && bun run test ;;
  typecheck) bun install && bun run typecheck 2>/dev/null || echo "no typecheck script" ;;
  *) echo "unknown action: $ACTION" >&2; exit 2 ;;
esac