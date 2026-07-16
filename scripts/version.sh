#!/usr/bin/env bash
# scripts/version.sh — delegate to //tools/version.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
exec cargo run -q -p tools-version -- "$@"
