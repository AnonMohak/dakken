class Dakken < Formula
  desc "Monkeytype-style typing test for the terminal"
  homepage "https://github.com/AnonMohak/dakken"
  version "0.3.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.3.0/dakken-v0.3.0-aarch64-apple-darwin"
      sha256 "53b3f9efe507caf11cca251f615582ba0a890bdb62e51edcd3026f3457965902"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.3.0/dakken-v0.3.0-x86_64-unknown-linux-gnu"
      sha256 "bcb422e62f6c3e97f993fdb682ad2f757b81d78876c384e40f054523306021e6"
    end
  end

  def install
    bin.install Dir["dakken-*"].first => "dakken"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/dakken --version")
  end
end
