// Resolves the native `orbit` binary for the launcher.
//
// Resolution order:
//   1. Vendored binary downloaded during postinstall:
//      <pkgRoot>/vendor/<target>/bin/orbit(.exe)
//   2. Local cargo build in the monorepo (dev/workspace case):
//      walk up from <pkgRoot> looking for <repo>/target/release/orbit(.exe)
//   3. Otherwise null (caller decides what to do).

import { existsSync, statSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { detectTarget } from "./platform.mjs";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
// lib/ -> orbit-cli/
export const PKG_ROOT = resolve(__dirname, "..");

// Allow tests to override the package root without touching the filesystem.
let ROOT_OVERRIDE = null;
export function __setPkgRootForTests(root) {
  ROOT_OVERRIDE = root;
}
function pkgRoot() {
  return ROOT_OVERRIDE ?? PKG_ROOT;
}

function isExecutableFile(p) {
  if (!existsSync(p)) return false;
  try {
    const s = statSync(p);
    if (!s.isFile()) return false;
    // On Windows `stat` mode bits are not meaningful; treat as executable.
    if (process.platform === "win32") return true;
    // Check any execute bit is set (owner/group/other).
    return Boolean(s.mode & 0o111);
  } catch {
    return false;
  }
}

export function vendorPath(root = pkgRoot()) {
  const { target, binName } = detectTarget();
  return join(root, "vendor", target, "bin", binName);
}

export function resolveVendoredBinary(root = pkgRoot()) {
  const p = vendorPath(root);
  return isExecutableFile(p) ? p : null;
}

// Walk up from `startDir` looking for a locally built `target/<profile>/<binName>`.
// Checks both `release` and `debug` so a plain `cargo build` (debug) works too.
export function resolveLocalBuild(startDir = pkgRoot()) {
  const { binName } = detectTarget();
  let dir = resolve(startDir);
  // eslint-disable-next-line no-constant-condition
  while (true) {
    for (const profile of ["release", "debug"]) {
      const candidate = join(dir, "target", profile, binName);
      if (isExecutableFile(candidate)) return candidate;
    }
    const parent = dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  return null;
}

// Full resolution: vendored first, then local cargo build.
export function resolveBinary(startDir = pkgRoot()) {
  return resolveVendoredBinary(startDir) ?? resolveLocalBuild(startDir);
}
