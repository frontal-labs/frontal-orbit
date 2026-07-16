#!/usr/bin/env bash
# Run the Rust SDK (orbit-sdk) Cargo actions from the repo root.
# Usage: sdk_rust.sh <test|fmt-check|clippy>
set -uo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ACTION="${1:-test}"
cd "$REPO_ROOT"

case "$ACTION" in
  test) cargo test -p orbit-sdk ;;
  fmt-check) cargo fmt -p orbit-sdk -- --check ;;
  clippy) cargo clippy -p orbit-sdk --all-targets -- -D warnings ;;
  *) echo "unknown action: $ACTION" >&2; exit 2 ;;
esac
