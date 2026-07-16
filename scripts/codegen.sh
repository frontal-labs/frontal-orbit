#!/usr/bin/env bash
# scripts/codegen.sh — delegate to //tools/codegen.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
exec cargo run -q -p tools-codegen -- "$@"
