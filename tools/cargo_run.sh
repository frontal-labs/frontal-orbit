#!/usr/bin/env bash
# Shared Bazel sh_binary wrapper that runs a cargo workspace tool.
# Invoked by each //tools/<name>:<name> target with the cargo package name.
# Runs from the real workspace checkout so cargo resolves the full workspace.
set -euo pipefail
PKG="$1"
shift
cd "${BUILD_WORKSPACE_DIRECTORY:-.}"
exec cargo run -q -p "$PKG" -- "$@"
