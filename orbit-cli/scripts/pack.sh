#!/usr/bin/env bash
# scripts/pack.sh — dry-run `npm pack` and print the published file list.
set -euo pipefail
PKG_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PKG_ROOT"

echo "==> npm pack --dry-run (files that would be published)"
npm pack --dry-run 2>&1 | sed -n '/npm notice/,$p'
