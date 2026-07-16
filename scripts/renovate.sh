#!/usr/bin/env bash
# Wrapper around `renovate` so it can run locally against the repo config.
# Requires a GitHub token with write access (RENOVATE_TOKEN or GITHUB_TOKEN).
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

TOKEN="${RENOVATE_TOKEN:-${GITHUB_TOKEN:-}}"
if [[ -z "${TOKEN}" ]]; then
  echo "error: set RENOVATE_TOKEN or GITHUB_TOKEN before running renovate" >&2
  exit 1
fi

export RENOVATE_TOKEN="${TOKEN}"
export RENOVATE_REPOSITORIES="${RENOVATE_REPOSITORIES:-$(git remote get-url origin 2>/dev/null | sed -E 's#.*[:/]([^/]+/[^/.]+)(\.git)?$#\1#')}"

exec npx --yes renovate --config=renovate.json "$@"
