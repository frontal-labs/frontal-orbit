#!/usr/bin/env bash
# Post-create setup for the dev container: bootstrap the Bazel monorepo.
set -euo pipefail
cd "$(dirname "$0")/.."
make bootstrap
