import { test } from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { releaseAssetUrl, verifySha256 } from "../lib/download.mjs";

test("releaseAssetUrl builds the exact GitHub release asset URL", () => {
  assert.equal(
    releaseAssetUrl("0.1.1", "orbit-macos-arm64.tar.gz"),
    "https://github.com/frontal-labs/orbit/releases/download/v0.1.1/orbit-macos-arm64.tar.gz",
  );
  assert.equal(
    releaseAssetUrl("1.2.3", "orbit-windows-x64.exe"),
    "https://github.com/frontal-labs/orbit/releases/download/v1.2.3/orbit-windows-x64.exe",
  );
});

test("verifySha256 matches expected hex (case-insensitive)", () => {
  const data = Buffer.from("orbit");
  const hex = createHash("sha256").update(data).digest("hex");
  assert.equal(verifySha256(data, hex), true);
  assert.equal(verifySha256(data, hex.toUpperCase()), true);
  assert.equal(verifySha256(data, "deadbeef"), false);
});
