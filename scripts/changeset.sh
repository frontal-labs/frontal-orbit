#!/usr/bin/env bash
# Wrapper around `npx changeset` to ensure we run from the repo root.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
exec npx changeset "$@"