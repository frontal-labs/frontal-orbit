#!/usr/bin/env bash
# scripts/doctor.sh — delegate to the canonical tool //tools/doctor.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
exec cargo run -q -p tools-doctor -- "$@"
