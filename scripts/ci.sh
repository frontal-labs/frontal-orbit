#!/usr/bin/env bash
# Aggregate CI entrypoint: build, test, then lint.
set -uo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"
"$REPO_ROOT/scripts/build.sh"
"$REPO_ROOT/scripts/test.sh"
"$REPO_ROOT/scripts/lint.sh"
