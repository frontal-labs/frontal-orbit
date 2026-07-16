#!/usr/bin/env bash
# Format the monorepo: Starlark/BUILD (buildifier), TypeScript/JSON/YAML
# (Biome), and Rust (rustfmt). Each formatter is best-effort: if its toolchain
# is unavailable the step is skipped rather than failing the whole run.
set -uo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# --- Starlark / BUILD (buildifier via Bazel) -------------------------------
mapfile -t files < <(find "$REPO_ROOT" \
    -type f \( -name '*.bzl' -o -name 'BUILD' -o -name 'BUILD.bazel' -o -name 'MODULE.bazel' -o -name 'WORKSPACE' \) \
    -not -path '*/target/*' \
    -not -path '*/node_modules/*' \
    -not -path '*/bazel-bin/*' \
    -not -path '*/bazel-out/*' \
    -not -path '*/bazel-testlogs/*' \
    -not -path '*/bazel-*/*')

if [ "${#files[@]}" -gt 0 ]; then
  echo "==> buildifier"
  bazel run //:buildifier -- "${files[@]}" || echo "WARN: buildifier unavailable; skipping"
else
  echo "No Starlark files to format."
fi

# --- TypeScript / JSON / YAML (Biome) --------------------------------------
if command -v bun >/dev/null 2>&1; then
  echo "==> biome format"
  bunx @biomejs/biome format --write . || echo "WARN: biome format failed"
elif command -v npx >/dev/null 2>&1; then
  echo "==> biome format (npx)"
  npx -y @biomejs/biome format --write . || echo "WARN: biome format failed"
else
  echo "WARN: bun/npx not found; skipping Biome format"
fi

# --- Rust (rustfmt) --------------------------------------------------------
if command -v cargo >/dev/null 2>&1; then
  echo "==> cargo fmt"
  cargo fmt --all || echo "WARN: cargo fmt failed"
else
  echo "WARN: cargo not found; skipping Rust format"
fi

echo "==> format complete"
