#!/usr/bin/env bash
# scripts/clean.sh — remove vendored binaries and any *.tgz in the package dir.
set -euo pipefail
PKG_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PKG_ROOT"

echo "==> Removing vendor/ and *.tgz"
rm -rf vendor
rm -f ./*.tgz
echo "done."
