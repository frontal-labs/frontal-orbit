#!/usr/bin/env bash
# scripts/download.sh — force (re)download the native binary for this platform
# into vendor/, bypassing the postinstall skip logic.
set -euo pipefail
PKG_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PKG_ROOT"
export ORBIT_FORCE_DOWNLOAD=1
exec node ./scripts/postinstall.mjs
