class Dakken < Formula
  desc "Typing-speed test for the terminal"
  homepage "https://github.com/AnonMohak/dakken"
  version "0.7.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.7.0/dakken-v0.7.0-aarch64-apple-darwin"
      sha256 "6321acd28624a8990d635ed46a5c1d7df67a6ca62719b6ebe5e24afed9d8f5b0"
    end
    on_intel do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.7.0/dakken-v0.7.0-x86_64-apple-darwin"
      sha256 "4a7546ba60b4e0fc0a29644f96cc5b6b0d123d0d763993aea3e47afdf534d60d"
    end
  end

  # Static musl binaries: they run on old and new Linux systems.
  on_linux do
    on_arm do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.7.0/dakken-v0.7.0-aarch64-unknown-linux-musl"
      sha256 "b3d69353d6b7519aab102575fa39b265dd52211e0f91ee052876678f1cf724c9"
    end
    on_intel do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.7.0/dakken-v0.7.0-x86_64-unknown-linux-musl"
      sha256 "f6145b3796e5856380dd062c402fe8eb15db852faf9006e749ca1fe0b20094d7"
    end
  end

  def install
    bin.install Dir["dakken-*"].first => "dakken"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/dakken --version")
  end
end
