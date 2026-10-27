class Dakken < Formula
  desc "Typing-speed test for the terminal"
  homepage "https://github.com/AnonMohak/dakken"
  version "0.7.4"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.7.4/dakken-v0.7.4-aarch64-apple-darwin"
      sha256 "7aab52aaa2dd5975f52fd3cc654a955a06e7e9402223531bc643f63ea87d5b34"
    end
    on_intel do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.7.4/dakken-v0.7.4-x86_64-apple-darwin"
      sha256 "7f1e85fe4fc8cf0a30463d4d79689476b661aaa7bfa56f989e2ca9bce67d0186"
    end
  end

  # Static musl binaries: they run on old and new Linux systems.
  on_linux do
    on_arm do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.7.4/dakken-v0.7.4-aarch64-unknown-linux-musl"
      sha256 "fdd4046b58d33a958d842ef978448feea1e0308344efe58a2afcb53981d1c2a4"
    end
    on_intel do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.7.4/dakken-v0.7.4-x86_64-unknown-linux-musl"
      sha256 "bb61f5df36540fa6d4403b9ead34c2440f21fedddf4a534628e31e8e35a5a3ca"
    end
  end

  def install
    bin.install Dir["dakken-*"].first => "dakken"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/dakken --version")
  end
end
