import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, mkdirSync, writeFileSync, rmSync, chmodSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { dirname } from "node:path";
import { detectTarget, UnsupportedPlatformError } from "../lib/platform.mjs";

const __dirname = dirname(fileURLToPath(import.meta.url));
const PKG_ROOT = resolve(__dirname, "..");
const POSTINSTALL = join(PKG_ROOT, "scripts", "postinstall.mjs");
const SKIP = process.platform === "win32";

// Run the real postinstall in a throwaway package copy with a fake vendored
// binary, asserting dev-version + skip-env short-circuits without network.
test("postinstall skips on ORBIT_SKIP_DOWNLOAD without touching network", { skip: SKIP }, () => {
  const { target, binName } = detectTarget();
  const fakeRoot = mkdtempSync(join(tmpdir(), "orbit-pi-"));
  try {
    const vendorDir = join(fakeRoot, "vendor", target, "bin");
    mkdirSync(vendorDir, { recursive: true });
    const fakeBin = join(vendorDir, binName);
    writeFileSync(fakeBin, "#!/bin/sh\necho hi\n");
    chmodSync(fakeBin, 0o755);

    // package.json with a dev version + a real vendored binary already present.
    writeFileSync(
      join(fakeRoot, "package.json"),
      JSON.stringify({ name: "@frontal-labs/orbit", version: "0.0.0-dev" }),
    );

    const res = spawnSync("node", [POSTINSTALL], {
      cwd: fakeRoot,
      env: { ...process.env, ORBIT_SKIP_DOWNLOAD: "1", PATH: process.env.PATH },
      encoding: "utf8",
    });
    assert.equal(res.status, 0);
    assert.match(res.stdout, /skipping download/);
  } finally {
    rmSync(fakeRoot, { recursive: true, force: true });
  }
});

test("postinstall skips download for dev version (no network)", { skip: SKIP }, () => {
  const fakeRoot = mkdtempSync(join(tmpdir(), "orbit-pi2-"));
  try {
    writeFileSync(
      join(fakeRoot, "package.json"),
      JSON.stringify({ name: "@frontal-labs/orbit", version: "0.0.0-dev" }),
    );
    const res = spawnSync("node", [POSTINSTALL], {
      cwd: fakeRoot,
      env: { ...process.env, PATH: process.env.PATH },
      encoding: "utf8",
    });
    assert.equal(res.status, 0);
    assert.match(res.stdout, /dev version/);
  } finally {
    rmSync(fakeRoot, { recursive: true, force: true });
  }
});

test("postinstall fails gracefully (exit 0) when offline download fails", { skip: SKIP }, () => {
  const fakeRoot = mkdtempSync(join(tmpdir(), "orbit-pi3-"));
  try {
    // Real, non-dev version but no network: download will fail; must exit 0.
    writeFileSync(
      join(fakeRoot, "package.json"),
      JSON.stringify({ name: "@frontal-labs/orbit", version: "9.9.9" }),
    );
    const res = spawnSync("node", [POSTINSTALL], {
      cwd: fakeRoot,
      env: { ...process.env, PATH: process.env.PATH, ORBIT_FORCE_DOWNLOAD: "1" },
      encoding: "utf8",
      timeout: 15000,
    });
    // Either it downloaded (network available) or warned+exited 0.
    assert.equal(res.status, 0);
  } finally {
    rmSync(fakeRoot, { recursive: true, force: true });
  }
});
