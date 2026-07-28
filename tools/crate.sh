#!/usr/bin/env bash
set -euo pipefail
cd "${BUILD_WORKSPACE_DIRECTORY:-.}"
CRATE="$1"
ACTION="${2:-test}"
shift 2

case "$ACTION" in
  test) cargo test -p "$CRATE" "$@" ;;
  fmt-check) cargo fmt -p "$CRATE" -- --check ;;
  clippy) cargo clippy -p "$CRATE" --all-targets -- -D warnings ;;
  run) cargo run -q -p "$CRATE" -- "$@" ;;
  build) cargo build -p "$CRATE" "$@" ;;
  *) echo "unknown action: $ACTION" >&2; exit 2 ;;
esac