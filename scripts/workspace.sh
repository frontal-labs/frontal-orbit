#!/usr/bin/env bash
# scripts/workspace.sh — delegate to //tools/workspace.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
exec cargo run -q -p tools-workspace -- "$@"
