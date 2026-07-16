#!/usr/bin/env bash
# Run the TypeScript SDK's Node toolchain (Bun) actions inside its directory.
# Usage: sdk_ts.sh <relative-dir> <test|build|typecheck>
set -uo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIR="$1"
ACTION="${2:-test}"
cd "$REPO_ROOT/$DIR"

run() {
  if command -v bun >/dev/null 2>&1; then
    bun "$@"
  elif command -v npm >/dev/null 2>&1; then
    npm "$@"
  else
    echo "ERROR: neither bun nor npm found" >&2
    exit 1
  fi
}

case "$ACTION" in
  test) run install && run run test ;;
  build) run install && run run build ;;
  typecheck) run install && run run typecheck ;;
  *) echo "unknown action: $ACTION" >&2; exit 2 ;;
esac
