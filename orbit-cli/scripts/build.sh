#!/usr/bin/env bash
# scripts/build.sh — build the native `orbit` binary and place it where the
# launcher (bin/orbit.js) expects it in dev: vendor/<target>/bin/orbit.
set -euo pipefail
PKG_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PKG_ROOT"

# Repo root is two levels up (orbit-cli is a top-level package dir here, but the
# cargo workspace lives at the repo root one level up if nested; resolve both).
REPO_ROOT_CANDIDATE="$(cd "$PKG_ROOT/.." && pwd)"
if [[ -f "$REPO_ROOT_CANDIDATE/Cargo.toml" && -d "$REPO_ROOT_CANDIDATE/crates/cli" ]]; then
  REPO_ROOT="$REPO_ROOT_CANDIDATE"
else
  REPO_ROOT="$PKG_ROOT"
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "ERROR: cargo not found on PATH. Install Rust to build the native binary." >&2
  exit 1
fi

echo "==> Building orbit-cli with cargo (release)"
( cd "$REPO_ROOT" && cargo build --release -p orbit-cli )

# Determine target triple dir name (mirror lib/platform.mjs: darwin/arm64 -> macos-arm64).
OS="$(uname -s)"; ARCH="$(uname -m)"
case "$OS" in
  Darwin) PLAT="macos" ;;
  Linux)  PLAT="linux" ;;
  *) echo "ERROR: unsupported build OS: $OS" >&2; exit 1 ;;
esac
case "$ARCH" in
  arm64|aarch64) A="arm64" ;;
  x86_64|amd64)  A="x64" ;;
  *) echo "ERROR: unsupported build arch: $ARCH" >&2; exit 1 ;;
esac
TARGET_DIR="$PLAT-$A"

SRC="$REPO_ROOT/target/release/orbit"
DEST="$PKG_ROOT/vendor/$TARGET_DIR/bin/orbit"
mkdir -p "$(dirname "$DEST")"
cp "$SRC" "$DEST"
chmod +x "$DEST"
echo "==> Native binary placed at $DEST"
