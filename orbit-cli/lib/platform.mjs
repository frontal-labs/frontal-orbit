// Platform detection and release-asset mapping for the Orbit CLI launcher.
//
// This module maps the current Node platform/arch to the per-platform release
// asset names produced by `.github/workflows/release.yml` (and consumed by
// `homebrew/orbit.rb`). It deliberately has no side effects so it is unit
// testable.

export class UnsupportedPlatformError extends Error {
  constructor(platform, arch) {
    super(
      `Unsupported platform "${platform}/${arch}". ` +
        `Orbit supports: macos-arm64, macos-x64, linux-x64, linux-arm64, windows-x64.`,
    );
    this.name = "UnsupportedPlatformError";
    this.platform = platform;
    this.arch = arch;
  }
}

// Map of "<platform>/<arch>" -> release asset descriptor.
// `assetName` is the suffix used in the GitHub release asset
// `orbit-<assetName>.tar.gz` (or `.exe` on Windows).
const TARGETS = {
  "darwin/arm64": { target: "macos-arm64", ext: "tar.gz" },
  "darwin/x64": { target: "macos-x64", ext: "tar.gz" },
  "linux/x64": { target: "linux-x64", ext: "tar.gz" },
  "linux/arm64": { target: "linux-arm64", ext: "tar.gz" },
  "win32/x64": { target: "windows-x64", ext: "exe" },
};

// Pure helper: resolve a descriptor from explicit platform/arch.
export function targetFor(platform, arch) {
  const entry = TARGETS[`${platform}/${arch}`];
  if (!entry) {
    throw new UnsupportedPlatformError(platform, arch);
  }
  return {
    target: entry.target,
    ext: entry.ext,
    assetName: `orbit-${entry.target}.${entry.ext}`,
    // Binary name inside vendor/ (and expected name of the native binary).
    binName: platform === "win32" ? "orbit.exe" : "orbit",
  };
}

// Resolve from the running process. Throws `UnsupportedPlatformError` on
// platforms/architectures we do not ship for.
export function detectTarget() {
  return targetFor(process.platform, process.arch);
}
