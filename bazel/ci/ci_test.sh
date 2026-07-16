#!/usr/bin/env bash
# Thin wrapper: run all Bazel tests in the monorepo.
set -euo pipefail
cd "$(dirname "$0")/../.."
bazel test //...
