#!/usr/bin/env bash
# scripts/coverage.sh — delegate to the canonical tool //tools/coverage.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
exec ./tools/coverage/coverage.sh "$@"
