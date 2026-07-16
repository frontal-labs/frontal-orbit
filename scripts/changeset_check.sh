#!/usr/bin/env bash
# Enforce a changeset for source/config changes (docs-only changes are exempt).
# Mirrors the convention used by `scripts/third_party_check.sh`.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

# A changeset is already staged (or the config changed): nothing to do.
if git diff --name-only --cached | grep -qE '^(.changeset/|.changeset/config.json)'; then
  exit 0
fi

# Docs, CI, infra, and tooling changes do not require a changeset.
if ! git diff --name-only --cached | grep -qvE '^(docs/|.*\.md$|\.github/|infrastructure/|scripts/|tools/|.*\.(ya?ml|toml|json)$)'; then
  exit 0
fi

# Source/src changes require a changeset entry.
if [ -z "$(ls .changeset/*.md 2>/dev/null | grep -v README)" ]; then
  echo "No .changeset entry found. Run: make changeset (or npx changeset)" >&2
  exit 1
fi
