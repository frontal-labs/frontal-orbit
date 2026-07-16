#!/usr/bin/env bash
# scripts/cli.sh — delegate to the orbit-cli npm package verify entrypoint.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT/orbit-cli"
exec ./scripts/verify.sh "$@"
