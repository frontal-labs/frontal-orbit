#!/usr/bin/env bash
# scripts/generators.sh — delegate to //tools/generators.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
exec cargo run -q -p tools-generators -- "$@"
