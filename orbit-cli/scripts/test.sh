#!/usr/bin/env bash
# scripts/test.sh — run the JS test suite and a binary smoke test.
set -euo pipefail
PKG_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PKG_ROOT"

echo "==> Running node:test suite"
node --test test/*.test.mjs

# Smoke test: only if a binary can be resolved (vendored or local cargo build).
if node -e 'import("./lib/resolve-binary.mjs").then(m=>process.exit(m.resolveBinary()?0:1))'; then
  echo "==> Smoke test: orbit --version"
  node ./bin/orbit.js --version
  node ./bin/orbit.js --version | grep -qi "orbit" && echo "OK: version output contains 'Orbit'"
else
  echo "WARN: no native binary resolved; skipping smoke test."
fi
