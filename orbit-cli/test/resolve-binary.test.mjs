import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, writeFileSync, rmSync, chmodSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { detectTarget } from "../lib/platform.mjs";
import {
  __setPkgRootForTests,
  resolveVendoredBinary,
  resolveLocalBuild,
  resolveBinary,
} from "../lib/resolve-binary.mjs";

function makeFakeBin(dir, name = "orbit") {
  const p = join(dir, name);
  writeFileSync(p, "#!/bin/sh\necho hi\n");
  chmodSync(p, 0o755);
  return p;
}

function withFakeRoot(fn) {
  const root = mkdtempSync(join(tmpdir(), "orbit-"));
  __setPkgRootForTests(root);
  try {
    return fn(root);
  } finally {
    __setPkgRootForTests(null);
    rmSync(root, { recursive: true, force: true });
  }
}

test("resolveVendoredBinary finds vendor/<target>/bin/orbit", () => {
  withFakeRoot((root) => {
    const { target, binName } = detectTarget();
    const vendored = join(root, "vendor", target, "bin");
    mkdirSync(vendored, { recursive: true });
    const bin = makeFakeBin(vendored, binName);
    assert.equal(resolveVendoredBinary(root), bin);
  });
});

test("resolveLocalBuild walks up to target/release/orbit", () => {
  withFakeRoot((root) => {
    const { binName } = detectTarget();
    const nested = join(root, "a", "b", "c");
    mkdirSync(nested, { recursive: true });
    const release = join(root, "target", "release");
    mkdirSync(release, { recursive: true });
    const bin = makeFakeBin(release, binName);
    const found = resolveLocalBuild(nested);
    assert.equal(found, bin);
  });
});

test("resolveLocalBuild returns null when no target/release present", () => {
  withFakeRoot((root) => {
    const nested = join(root, "x", "y");
    mkdirSync(nested, { recursive: true });
    assert.equal(resolveLocalBuild(nested), null);
  });
});

test("resolveBinary prefers vendored over local build", () => {
  withFakeRoot((root) => {
    const { target, binName } = detectTarget();
    const vendoredDir = join(root, "vendor", target, "bin");
    mkdirSync(vendoredDir, { recursive: true });
    const vendored = makeFakeBin(vendoredDir, binName);
    const release = join(root, "target", "release");
    mkdirSync(release, { recursive: true });
    makeFakeBin(release, binName);
    assert.equal(resolveBinary(root), vendored);
  });
});

test("resolveBinary falls back to local build when no vendor", () => {
  withFakeRoot((root) => {
    const { binName } = detectTarget();
    const release = join(root, "target", "release");
    mkdirSync(release, { recursive: true });
    const local = makeFakeBin(release, binName);
    assert.equal(resolveBinary(root), local);
  });
});

test("resolveBinary returns null when nothing present", () => {
  withFakeRoot((root) => {
    assert.equal(resolveBinary(root), null);
  });
});
