class Dakken < Formula
  desc "Typing-speed test for the terminal"
  homepage "https://github.com/AnonMohak/dakken"
  version "0.4.1"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.4.1/dakken-v0.4.1-aarch64-apple-darwin"
      sha256 "c176be25592b85231b9f9cf33b69d1a757f4193132dd1e0cd97b72743278f9dc"
    end
    on_intel do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.4.1/dakken-v0.4.1-x86_64-apple-darwin"
      sha256 "fc5a8c429e3a98506eddd54f986f0bf9eaade9897f6dac788cd06004871fdae0"
    end
  end

  # Static musl binaries: they run on old and new Linux systems.
  on_linux do
    on_arm do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.4.1/dakken-v0.4.1-aarch64-unknown-linux-musl"
      sha256 "e07240bb7fa8780e6261871fa14d90839849b6ec32efffcf9f5845f7eb284657"
    end
    on_intel do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.4.1/dakken-v0.4.1-x86_64-unknown-linux-musl"
      sha256 "469069a1d7671b43115fc8beb8974915790533a151b4239e603e1bb3c980939d"
    end
  end

  def install
    bin.install Dir["dakken-*"].first => "dakken"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/dakken --version")
  end
end
