// Downloads the native `orbit` binary from a GitHub release into vendor/.
//
// Dependency-free: Node stdlib only, shelling out to `tar`/`shasum` on
// macOS/Linux. Windows ships a raw `.exe` (no tarball, no sha), handled as a
// special case.
//
// No top-level side effects — everything is exported as functions so the
// module is safe to import in tests.

import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  renameSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { dirname, join } from "node:path";
import { detectTarget, UnsupportedPlatformError } from "./platform.mjs";

export const RELEASE_BASE =
  "https://github.com/frontal-labs/orbit/releases/download";

// Build the release asset URL for a given version + target descriptor.
export function releaseAssetUrl(version, assetName) {
  const tag = `v${version}`;
  return `${RELEASE_BASE}/${tag}/${assetName}`;
}

export async function fetchUrl(url, { redirects = 5 } = {}) {
  const res = await fetch(url, { redirect: "follow" });
  if (res.status === 404) {
    const err = new Error(`Release asset not found: ${url}`);
    err.code = "NOT_FOUND";
    throw err;
  }
  if (!res.ok) {
    throw new Error(`Download failed (${res.status}) for ${url}`);
  }
  const buf = Buffer.from(await res.arrayBuffer());
  return buf;
}

// Verify a buffer against a sha256 hex string (case-insensitive).
export function verifySha256(buf, expectedHex) {
  const actual = createHash("sha256").update(buf).digest("hex").toLowerCase();
  return actual === String(expectedHex).trim().toLowerCase();
}

// Download and read the `.sha256` sidecar for an asset, if it exists.
export async function fetchSha256(version, assetName) {
  try {
    const buf = await fetchUrl(releaseAssetUrl(version, `${assetName}.sha256`));
    return buf.toString("utf8").trim().split(/\s+/)[0] || null;
  } catch (e) {
    if (e.code === "NOT_FOUND") return null;
    throw e;
  }
}

// Extract a `.tar.gz` containing `bin/orbit` into `destDir/vendor/<target>/`.
// We shell out to `tar` because a from-scratch tar parser is unnecessary for
// this single-archive use case and would add risk.
function extractTarball(tarballPath, target, destDir) {
  const outDir = join(destDir, "vendor", target);
  mkdirSync(outDir, { recursive: true });
  execFileSync("tar", ["-xzf", tarballPath, "-C", outDir], { stdio: "inherit" });
}

async function downloadTo({ version, destDir, log = () => {} }) {
  const { target, assetName, ext, binName } = detectTarget();
  const url = releaseAssetUrl(version, assetName);
  log(`Downloading ${assetName} (${target}) from ${url}`);
  const buf = await fetchUrl(url);

  // Windows ships a raw .exe with no sha sidecar.
  if (ext !== "exe") {
    const expected = await fetchSha256(version, assetName);
    if (expected) {
      if (!verifySha256(buf, expected)) {
        throw new Error(`SHA-256 mismatch for ${assetName}`);
      }
      log(`Verified SHA-256 for ${assetName}`);
    } else {
      log(`No SHA-256 sidecar published for ${assetName}; skipping verify`);
    }
  }

  if (ext === "exe") {
    const binDir = join(destDir, "vendor", target, "bin");
    mkdirSync(binDir, { recursive: true });
    const binPath = join(binDir, binName);
    writeFileSync(binPath, buf);
    return binPath;
  }

  const tmpTarball = join(destDir, ".orbit-download.tar.gz");
  writeFileSync(tmpTarball, buf);
  try {
    extractTarball(tmpTarball, target, destDir);
  } finally {
    rmSync(tmpTarball, { force: true });
  }
  const binPath = join(destDir, "vendor", target, "bin", binName);
  if (!existsSync(binPath)) {
    throw new Error(`Expected binary not found after extraction: ${binPath}`);
  }
  return binPath;
}

// Public entry point. Returns the resolved binary path under vendor/.
export async function downloadRelease({ version, destDir, log = () => {} }) {
  if (!version || /dev$/.test(version) || version === "0.0.0") {
    throw new Error(
      `Refusing to download for dev version "${version}". Use a tagged release version.`,
    );
  }
  let binPath;
  try {
    binPath = await downloadTo({ version, destDir, log });
  } catch (e) {
    if (e instanceof UnsupportedPlatformError) throw e;
    throw e;
  }
  // Make executable on POSIX.
  if (process.platform !== "win32" && existsSync(binPath)) {
    try {
      execFileSync("chmod", ["+x", binPath]);
    } catch {
      /* best effort */
    }
  }
  return binPath;
}
