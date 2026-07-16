#!/usr/bin/env bash
# scripts/fuzz.sh — delegate to //tools/fuzz.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
exec cargo run -q -p tools-fuzz -- "$@"
