class Orbit < Formula
  desc "High-performance Rust AI agent harness"
  homepage "https://github.com/frontal-labs/frontal-orbit"
  license "MIT"

  # Use --HEAD for unreleased development versions:
  #   brew install --HEAD ./homebrew/orbit.rb
  head "https://github.com/frontal-labs/frontal-orbit.git", branch: "main"

  # TODO: Add stable release URL and checksum when a release is published:
  # url "https://github.com/frontal-labs/frontal-orbit/archive/refs/tags/v0.1.1.tar.gz"
  # sha256 "<INSERT_CHECKSUM_HERE>"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: "crates/cli")
  end

  test do
    assert_match "orbit", shell_output("#{bin}/orbit --version")
  end
end
