import { test } from "node:test";
import assert from "node:assert/strict";
import { detectTarget, targetFor, UnsupportedPlatformError } from "../lib/platform.mjs";

test("detectTarget maps darwin/arm64 to macos-arm64", () => {
  const t = targetFor("darwin", "arm64");
  assert.equal(t.target, "macos-arm64");
  assert.equal(t.assetName, "orbit-macos-arm64.tar.gz");
  assert.equal(t.binName, "orbit");
});

test("detectTarget maps darwin/x64 to macos-x64", () => {
  const t = targetFor("darwin", "x64");
  assert.equal(t.target, "macos-x64");
  assert.equal(t.assetName, "orbit-macos-x64.tar.gz");
});

test("detectTarget maps linux/x64 to linux-x64", () => {
  const t = targetFor("linux", "x64");
  assert.equal(t.target, "linux-x64");
  assert.equal(t.assetName, "orbit-linux-x64.tar.gz");
});

test("detectTarget maps win32/x64 to windows-x64 (.exe)", () => {
  const t = targetFor("win32", "x64");
  assert.equal(t.target, "windows-x64");
  assert.equal(t.assetName, "orbit-windows-x64.exe");
  assert.equal(t.binName, "orbit.exe");
});

test("detectTarget throws UnsupportedPlatformError for unknown", () => {
  assert.throws(() => targetFor("sunos", "sparc"), UnsupportedPlatformError);
});

test("detectTarget (process) returns a valid descriptor on the CI runner", () => {
  const t = detectTarget();
  assert.ok(t.target);
  assert.ok(t.assetName.startsWith("orbit-"));
});
