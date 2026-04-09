#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# Disallow placeholder-like markers in production paths.
# Test files and dedicated test harness crates are excluded.
if rg -n "TODO:|FIXME:|XXX:|placeholder|\bstub\b|\bscaffold\b" \
  crates docs \
  --glob '!**/tests/**' \
  --glob '!crates/mock-anthropic-service/**' \
  --glob '!**/*.snap' \
  --glob '!docs/development_and_testing.md' \
  --glob '!MOCK_PARITY_HARNESS.md' \
  --glob '!mock_parity_scenarios.json'; then
  echo "\nDisallowed placeholder markers found in non-test paths." >&2
  exit 1
fi

echo "No disallowed placeholder markers found in non-test paths."
