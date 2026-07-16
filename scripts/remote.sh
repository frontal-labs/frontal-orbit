#!/usr/bin/env bash
# scripts/remote.sh — delegate to //tools/remote.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
exec cargo run -q -p tools-remote -- "$@"
