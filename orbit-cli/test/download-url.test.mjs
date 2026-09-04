import { test } from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { downloadRelease, releaseAssetUrl, verifySha256 } from "../lib/download.mjs";

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

test("downloadRelease refuses a release with no SHA-256 sidecar", async () => {
  const realFetch = globalThis.fetch;
  // Serve the asset, but 404 the .sha256 sidecar.
  globalThis.fetch = async (url) =>
    String(url).endsWith(".sha256")
      ? new Response("not found", { status: 404 })
      : new Response(Buffer.from("binary"), { status: 200 });
  try {
    await assert.rejects(
      downloadRelease({ version: "9.9.9", destDir: mkdtempSync(join(tmpdir(), "orbit-dl-")) }),
      /No SHA-256 sidecar/,
      "an unverified binary must not be installed",
    );
  } finally {
    globalThis.fetch = realFetch;
  }
});

test("downloadRelease refuses a dev version outright", async () => {
  await assert.rejects(
    downloadRelease({ version: "0.0.0-dev", destDir: "/tmp" }),
    /Refusing to download for dev version/,
  );
});
