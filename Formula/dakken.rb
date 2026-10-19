class Dakken < Formula
  desc "Typing-speed test for the terminal"
  homepage "https://github.com/AnonMohak/dakken"
  version "0.4.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.4.0/dakken-v0.4.0-aarch64-apple-darwin"
      sha256 "c3916ae5d169e6160ac3cad8cdc0d12b1d7f270fef3dddb75582bc2c11afc44f"
    end
    on_intel do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.4.0/dakken-v0.4.0-x86_64-apple-darwin"
      sha256 "7bf1dcee53bf942e14bdcbc41ae43e73d4a86ae6b706907cf2dd311cd8e9ecb3"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.4.0/dakken-v0.4.0-aarch64-unknown-linux-gnu"
      sha256 "40749dc5e04e4b3b0d3a163ac60a8150ccc04b774af26a65677c7873a461e8a1"
    end
    on_intel do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.4.0/dakken-v0.4.0-x86_64-unknown-linux-gnu"
      sha256 "0e12e2f631b77b65f4b7a61435055eb2938a56f413698d8fcf3e15b4c9b602fd"
    end
  end

  def install
    bin.install Dir["dakken-*"].first => "dakken"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/dakken --version")
  end
end
