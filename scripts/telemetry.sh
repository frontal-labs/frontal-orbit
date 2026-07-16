#!/usr/bin/env bash
# scripts/telemetry.sh — delegate to //tools/telemetry.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
exec cargo run -q -p tools-telemetry -- "$@"
