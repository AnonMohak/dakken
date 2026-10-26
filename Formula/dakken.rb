class Dakken < Formula
  desc "Typing-speed test for the terminal"
  homepage "https://github.com/AnonMohak/dakken"
  version "0.7.3"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.7.3/dakken-v0.7.3-aarch64-apple-darwin"
      sha256 "6b34f5a1302cbffc97a4748c00075bb260781f6ce5567b64efa91ff97b9b6c64"
    end
    on_intel do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.7.3/dakken-v0.7.3-x86_64-apple-darwin"
      sha256 "57b36ead67060a2e4b486981e6e4ff1cffa78de15f1fa08a0cf61d9f2e597069"
    end
  end

  # Static musl binaries: they run on old and new Linux systems.
  on_linux do
    on_arm do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.7.3/dakken-v0.7.3-aarch64-unknown-linux-musl"
      sha256 "95162b488b689b93fa91c7551e125585bf99fa94be2a6401f29d373622a0a16e"
    end
    on_intel do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.7.3/dakken-v0.7.3-x86_64-unknown-linux-musl"
      sha256 "70b9ef93ae8a5ab9fc61f8bd5ccf46d3d832659c2f463becb0ba430759785033"
    end
  end

  def install
    bin.install Dir["dakken-*"].first => "dakken"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/dakken --version")
  end
end
