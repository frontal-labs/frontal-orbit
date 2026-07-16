// Postinstall: download the native `orbit` binary for this platform.
//
// Resilient by design: failures warn and exit 0 so that `npm install` in
// unrelated/CI/offline contexts never breaks. The launcher itself is the
// authority that reports a missing binary to the user at run time.
//
// Skip conditions (exit 0 quietly):
//   - ORBIT_SKIP_DOWNLOAD=1
//   - npm_config_offline (npm/pnpm/yarn offline)
//   - npm_config_ignore_scripts (already handled by npm, but be safe)
//   - dev version (0.0.0-dev / *-dev) -> expect a local cargo build
//   - a valid vendored or local binary already exists

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { detectTarget } from "../lib/platform.mjs";
import { resolveBinary, PKG_ROOT } from "../lib/resolve-binary.mjs";
import { downloadRelease } from "../lib/download.mjs";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const ROOT = resolve(__dirname, "..");

function log(msg) {
  process.stdout.write(`[orbit:postinstall] ${msg}\n`);
}

function shouldSkip() {
  if (process.env.ORBIT_SKIP_DOWNLOAD === "1") {
    return "ORBIT_SKIP_DOWNLOAD=1";
  }
  if (process.env.npm_config_offline === "true") {
    return "offline install";
  }
  if (process.env.npm_config_ignore_scripts === "true") {
    return "ignore-scripts";
  }
  if (process.env.ORBIT_FORCE_DOWNLOAD === "1") {
    return null; // explicitly forced
  }
  return null;
}

function readVersion() {
  try {
    const pkg = JSON.parse(readFileSync(resolve(ROOT, "package.json"), "utf8"));
    return pkg.version ?? "0.0.0-dev";
  } catch {
    return "0.0.0-dev";
  }
}

async function main() {
  const skipReason = shouldSkip();
  if (skipReason) {
    log(`skipping download (${skipReason}).`);
    return;
  }

  let target;
  try {
    target = detectTarget();
  } catch (e) {
    log(`warning: ${e.message}`);
    log("Skipping native binary download; the CLI will not run until built locally.");
    return;
  }

  const version = await readVersion();

  // Dev version: do not hit the network; expect local cargo build.
  if (/dev$/.test(version) || version === "0.0.0") {
    if (resolveBinary()) {
      log(`dev version detected; using existing local build at ${resolveBinary()}.`);
    } else {
      log(
        `dev version detected (${version}); skipping download.\n` +
          `Build locally with: cargo build --release -p orbit-cli`,
      );
    }
    return;
  }

  // Already downloaded (or locally built)?
  const existing = resolveBinary();
  if (existing) {
    log(`native binary already present at ${existing}; nothing to do.`);
    return;
  }

  log(`installing native binary for ${target.target} (v${version})...`);
  try {
    const bin = await downloadRelease({ version, destDir: ROOT, log });
    log(`installed native binary to ${bin}`);
  } catch (e) {
    log(`warning: failed to download native binary: ${e.message}`);
    log(
      "The `orbit` command will not work until a binary is available.\n" +
        "Build locally with `cargo build --release -p orbit-cli` or run\n" +
        "  (cd orbit-cli && ./scripts/download.sh)  after tagging a release.",
    );
    // Do NOT fail install.
  }
}

main().then(
  () => process.exit(0),
  () => process.exit(0),
);
