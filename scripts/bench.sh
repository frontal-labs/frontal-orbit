#!/usr/bin/env bash
# scripts/bench.sh — delegate to //tools/benchmark.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
exec cargo run -q -p tools-benchmark -- "$@"
