class Dakken < Formula
  desc "Typing-speed test for the terminal"
  homepage "https://github.com/AnonMohak/dakken"
  version "0.6.2"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.6.2/dakken-v0.6.2-aarch64-apple-darwin"
      sha256 "203457c9939396644a50824349cab6980e0178dde899089503f8bd93e9e31ac6"
    end
    on_intel do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.6.2/dakken-v0.6.2-x86_64-apple-darwin"
      sha256 "b5261791b6237e26b7cc4449915f897808b5a0de847b8aec60bc106024bf801a"
    end
  end

  # Static musl binaries: they run on old and new Linux systems.
  on_linux do
    on_arm do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.6.2/dakken-v0.6.2-aarch64-unknown-linux-musl"
      sha256 "fdbd45cfc4df027a63b7c06ffb03cc09bf35e1a2ddec14648b9ece0d9c08eb21"
    end
    on_intel do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.6.2/dakken-v0.6.2-x86_64-unknown-linux-musl"
      sha256 "a59751f155632eabc82d3f0a3f78c51e16dd308fc7ace703fdcad1541da4b187"
    end
  end

  def install
    bin.install Dir["dakken-*"].first => "dakken"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/dakken --version")
  end
end
