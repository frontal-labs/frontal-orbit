#!/usr/bin/env bash
# Thin wrapper: run pre-commit across all files.
set -euo pipefail
cd "$(dirname "$0")/../.."
pre-commit run --all-files
