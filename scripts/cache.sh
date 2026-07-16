#!/usr/bin/env bash
# scripts/cache.sh — delegate to //tools/cache.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
exec cargo run -q -p tools-cache -- "$@"
