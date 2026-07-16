#!/usr/bin/env bash
# Cache status smoke test for Bazel CI.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
cargo run -q -p tools-cache -- bazel status >/dev/null
echo "cache bazel status OK"