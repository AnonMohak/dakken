class Dakken < Formula
  desc "Typing-speed test for the terminal"
  homepage "https://github.com/AnonMohak/dakken"
  version "0.5.1"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.5.1/dakken-v0.5.1-aarch64-apple-darwin"
      sha256 "374760fef6a96d27873038761091788da2f4fe392733f1d2bc4023362ef9b997"
    end
    on_intel do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.5.1/dakken-v0.5.1-x86_64-apple-darwin"
      sha256 "d6f26318a841cd00f646f607136feb09232b47782f486690824f3c74e991ff0d"
    end
  end

  # Static musl binaries: they run on old and new Linux systems.
  on_linux do
    on_arm do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.5.1/dakken-v0.5.1-aarch64-unknown-linux-musl"
      sha256 "5b5ad389c5b13129d9e34f3e4b4b8846b2583939266b4d0e4952bc6fd2938cc8"
    end
    on_intel do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.5.1/dakken-v0.5.1-x86_64-unknown-linux-musl"
      sha256 "d72d88712a6cb2e492419f2cb5035e558b25c9dd664a1e71021b16467d9ee6a0"
    end
  end

  def install
    bin.install Dir["dakken-*"].first => "dakken"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/dakken --version")
  end
end
