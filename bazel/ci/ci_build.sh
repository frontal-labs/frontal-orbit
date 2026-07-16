#!/usr/bin/env bash
# Thin wrapper: build the entire monorepo with Bazel.
set -euo pipefail
cd "$(dirname "$0")/../.."
bazel build //...
