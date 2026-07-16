#!/usr/bin/env bash
# scripts/verify.sh — CI/presubmit checks for the npm package.
set -euo pipefail
PKG_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PKG_ROOT"

echo "==> Syntax-checking JS/mjs sources"
node --check bin/orbit.js
for f in lib/*.mjs scripts/postinstall.mjs; do
  node --check "$f"
done

echo "==> Running test suite"
./scripts/test.sh

echo "==> Checking published file list excludes vendor/ and test/"
npm pack --dry-run --json 2>/dev/null | node -e '
let raw = "";
process.stdin.on("data", d => raw += d);
process.stdin.on("end", () => {
  let data;
  try { data = JSON.parse(raw); } catch (e) { console.log("skip: no json"); process.exit(0); }
  const files = (Array.isArray(data) ? data[0].files : data.files).map(f => f.path);
  const bad = files.filter(p => p.startsWith("vendor/") || p.startsWith("test/"));
  if (bad.length) {
    console.error("ERROR: unexpected files in package:", bad);
    process.exit(1);
  }
  for (const need of ["bin/orbit.js","lib/platform.mjs","lib/resolve-binary.mjs","lib/download.mjs","scripts/postinstall.mjs"]) {
    if (!files.includes(need)) { console.error("ERROR: missing required file:", need); process.exit(1); }
  }
  console.log("OK: package file list is correct");
});
'
echo "==> verify.sh complete"
