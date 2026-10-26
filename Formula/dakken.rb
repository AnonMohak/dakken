class Dakken < Formula
  desc "Typing-speed test for the terminal"
  homepage "https://github.com/AnonMohak/dakken"
  version "0.7.1"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.7.1/dakken-v0.7.1-aarch64-apple-darwin"
      sha256 "e7ec42fdf235e1ac9c33ebd944b661b81aa368f39a9d7c1e47c24b3b31fee5be"
    end
    on_intel do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.7.1/dakken-v0.7.1-x86_64-apple-darwin"
      sha256 "5e8de3800b0b68a8b6e55bc4902ff41f47113b1364f84b19fe9da60032a4c84b"
    end
  end

  # Static musl binaries: they run on old and new Linux systems.
  on_linux do
    on_arm do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.7.1/dakken-v0.7.1-aarch64-unknown-linux-musl"
      sha256 "ee937e1e0881e9d97a6ae246ea86779fa4a03d41ba37b9b754172f1f3a525369"
    end
    on_intel do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.7.1/dakken-v0.7.1-x86_64-unknown-linux-musl"
      sha256 "d062a2c1f2811d4a932a1bac2454400244960c23c8bedd57b2c8b1c01bbb417e"
    end
  end

  def install
    bin.install Dir["dakken-*"].first => "dakken"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/dakken --version")
  end
end
