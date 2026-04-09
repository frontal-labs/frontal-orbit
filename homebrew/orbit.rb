class Orbit < Formula
  desc "High-performance Rust AI agent harness"
  homepage "https://github.com/frontal-labs/frontal-orbit"
  license "MIT"
  head "https://github.com/frontal-labs/frontal-orbit.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: "crates/cli")
  end

  test do
    assert_match "orbit", shell_output("#{bin}/orbit --version")
  end
end
