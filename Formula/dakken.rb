class Dakken < Formula
  desc "Typing-speed test for the terminal"
  homepage "https://github.com/AnonMohak/dakken"
  version "0.6.1"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.6.1/dakken-v0.6.1-aarch64-apple-darwin"
      sha256 "7835762472de67fc26f545aeb1f1c03833ee0abb3e99c1a6db62da61f9adc3d6"
    end
    on_intel do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.6.1/dakken-v0.6.1-x86_64-apple-darwin"
      sha256 "e6d82f9a0a5561fc6c4e65f127aedfa7fd6f124a7066efda421e8527428a9569"
    end
  end

  # Static musl binaries: they run on old and new Linux systems.
  on_linux do
    on_arm do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.6.1/dakken-v0.6.1-aarch64-unknown-linux-musl"
      sha256 "dc4374b02d75d3e3f86f63c6e7462d33b63893bdcdc4d3d1268243cba7d8ac56"
    end
    on_intel do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.6.1/dakken-v0.6.1-x86_64-unknown-linux-musl"
      sha256 "9f9858ae46ce3d282671041ba8ccaa2cdcc6b0940f6c1acb6bbadf827b48669d"
    end
  end

  def install
    bin.install Dir["dakken-*"].first => "dakken"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/dakken --version")
  end
end
