import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import {
  mkdtempSync,
  mkdirSync,
  writeFileSync,
  rmSync,
  chmodSync,
  cpSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { dirname } from "node:path";
import { detectTarget } from "../lib/platform.mjs";

const __dirname = dirname(fileURLToPath(import.meta.url));
const PKG_ROOT = resolve(__dirname, "..");
const SKIP = process.platform === "win32";

// Build a throwaway copy of the package (bin/ + lib/) containing a fake
// vendored binary, then run that copy's launcher to verify argv + exit-code
// passthrough without touching the real filesystem.
test("launcher forwards argv and exit code to the resolved binary", { skip: SKIP }, () => {
  const { target, binName } = detectTarget();
  const fakeRoot = mkdtempSync(join(tmpdir(), "orbit-launch-"));
  try {
    // Copy package sources.
    cpSync(join(PKG_ROOT, "bin"), join(fakeRoot, "bin"), { recursive: true });
    cpSync(join(PKG_ROOT, "lib"), join(fakeRoot, "lib"), { recursive: true });

    // Drop a fake vendored binary that echoes args and exits per first arg.
    const vendored = join(fakeRoot, "vendor", target, "bin");
    mkdirSync(vendored, { recursive: true });
    const fakeBin = join(vendored, binName);
    writeFileSync(
      fakeBin,
      '#!/bin/sh\necho "ARGS:$@"\n[ "$1" = "fail" ] && exit 7\nexit 0\n',
    );
    chmodSync(fakeBin, 0o755);

    const launcher = join(fakeRoot, "bin", "orbit.js");

    const ok = spawnSync("node", [launcher, "hello", "world"], {
      encoding: "utf8",
    });
    assert.match(ok.stdout, /ARGS:hello world/);
    assert.equal(ok.status, 0);

    const fail = spawnSync("node", [launcher, "fail"], { encoding: "utf8" });
    assert.equal(fail.status, 7);
  } finally {
    rmSync(fakeRoot, { recursive: true, force: true });
  }
});
