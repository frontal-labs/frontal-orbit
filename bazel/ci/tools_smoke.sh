#!/usr/bin/env bash
# Smoke test for the //tools dev-tooling suite. Runs lightweight invocations
# of each tool to confirm they build and execute. Tagged "manual" in BUILD
# so it is not triggered by a wildcard `bazel test //...` (which would spawn
# nested cargo/bazel invocations).
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."

cargo run -q -p tools-doctor -- --root . >/dev/null
cargo run -q -p tools-version -- show >/dev/null
cargo run -q -p tools-workspace -- list >/dev/null
cargo run -q -p tools-fuzz -- list >/dev/null
cargo run -q -p tools-cache -- status >/dev/null
cargo run -q -p tools-remote -- show >/dev/null
cargo run -q -p tools-codegen -- command Smoke >/dev/null
GEN_DIR="$(mktemp -d /tmp/orbit_gen_smoke.XXXXXX)"
cargo run -q -p tools-generators -- crate smoke_lib --dest "$GEN_DIR" >/dev/null
rm -rf "$GEN_DIR"
cargo run -q -p tools-telemetry -- event smoke --message ok >/dev/null

echo "devtools smoke OK"
