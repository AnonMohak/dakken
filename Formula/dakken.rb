class Dakken < Formula
  desc "Typing-speed test for the terminal"
  homepage "https://github.com/AnonMohak/dakken"
  version "0.6.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.6.0/dakken-v0.6.0-aarch64-apple-darwin"
      sha256 "ccfe12be35f7fa557dc406b7db953635035524a677852ea404a8a21a0b7adb17"
    end
    on_intel do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.6.0/dakken-v0.6.0-x86_64-apple-darwin"
      sha256 "98ae97cb23aaffc8d375eeeed0712295d44fb20af6db26ad1f894ef23f89a165"
    end
  end

  # Static musl binaries: they run on old and new Linux systems.
  on_linux do
    on_arm do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.6.0/dakken-v0.6.0-aarch64-unknown-linux-musl"
      sha256 "4754d568735a1840073a2971e83af39452cf875dea8a19a8a8d81668da20d6a9"
    end
    on_intel do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.6.0/dakken-v0.6.0-x86_64-unknown-linux-musl"
      sha256 "74d260fdc789c4949575aa25a9c017b7a8c55e2f19e5f0ac21e05f9e32de7f22"
    end
  end

  def install
    bin.install Dir["dakken-*"].first => "dakken"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/dakken --version")
  end
end
