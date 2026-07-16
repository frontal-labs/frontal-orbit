#!/usr/bin/env bash
# Third-party integrity check.
#
# Verifies that:
#   1. MODULE.bazel exists and every bazel_dep pins an explicit version.
#   2. //third_party:repos.bzl (the non_module_deps extension) is present.
#
# This guards against accidentally drifting to unpinned / floating third-party
# dependencies, which would break hermetic, reproducible builds.
set -euo pipefail
# Prefer the Bazel runfiles root for the main repo (_main); fall back to the
# script-relative repo root when run outside Bazel.
if [ -n "${TEST_SRCDIR:-}" ] && [ -d "${TEST_SRCDIR}/_main" ]; then
  REPO_ROOT="${TEST_SRCDIR}/_main"
else
  REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fi
cd "$REPO_ROOT"

fail=0

if [ ! -f MODULE.bazel ]; then
  echo "FAIL: MODULE.bazel not found"
  fail=1
else
  # Every bazel_dep(...) must include a version = "..." argument.
  while IFS= read -r line; do
    if [[ "$line" == bazel_dep\(* ]] && [[ "$line" != *version* ]]; then
      echo "FAIL: unpinned bazel_dep: $line"
      fail=1
    fi
  done < <(grep -E '^[[:space:]]*bazel_dep\(' MODULE.bazel || true)
fi

if [ ! -f third_party/repos.bzl ]; then
  echo "FAIL: third_party/repos.bzl not found"
  fail=1
fi

if [ "$fail" -ne 0 ]; then
  echo "third-party check FAILED"
  exit 1
fi

echo "third-party check OK"
