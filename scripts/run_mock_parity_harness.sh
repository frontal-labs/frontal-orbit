#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

cargo test -p orbit-cli --test mock_parity_harness -- --nocapture
