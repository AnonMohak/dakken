class Dakken < Formula
  desc "Typing-speed test for the terminal"
  homepage "https://github.com/AnonMohak/dakken"
  version "0.7.2"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.7.2/dakken-v0.7.2-aarch64-apple-darwin"
      sha256 "59b0f54d34fa56a92197bedef5ee5d38d899f46e304be04c152b334e2307c482"
    end
    on_intel do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.7.2/dakken-v0.7.2-x86_64-apple-darwin"
      sha256 "9b0d6cf88bf721c6228e38447b50cd02e062f39842e436289a333758cc162c19"
    end
  end

  # Static musl binaries: they run on old and new Linux systems.
  on_linux do
    on_arm do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.7.2/dakken-v0.7.2-aarch64-unknown-linux-musl"
      sha256 "5638cabc5cdf89ae211a024d3fbf476472c0ab3f8ff72037e6c13488d527e0cb"
    end
    on_intel do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.7.2/dakken-v0.7.2-x86_64-unknown-linux-musl"
      sha256 "33aa0d139d2d04de8919e9a50816ed2e676cff0841a5326c2641714758bb9a59"
    end
  end

  def install
    bin.install Dir["dakken-*"].first => "dakken"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/dakken --version")
  end
end
