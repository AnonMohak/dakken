class Dakken < Formula
  desc "Typing-speed test for the terminal"
  homepage "https://github.com/AnonMohak/dakken"
  version "0.5.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.5.0/dakken-v0.5.0-aarch64-apple-darwin"
      sha256 "0f4aca16cd1bdaee73218d69a78b852c25225192d0b87a8f53229b644daf1677"
    end
    on_intel do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.5.0/dakken-v0.5.0-x86_64-apple-darwin"
      sha256 "aa452e49ef9db7c4ae706aff8cf371d66a7e592d493e5c8755200039e730534e"
    end
  end

  # Static musl binaries: they run on old and new Linux systems.
  on_linux do
    on_arm do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.5.0/dakken-v0.5.0-aarch64-unknown-linux-musl"
      sha256 "bc636b3fede451a674b77380ef200a2b0784b275c8b88758f94fbb7da295aad8"
    end
    on_intel do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.5.0/dakken-v0.5.0-x86_64-unknown-linux-musl"
      sha256 "4948ba1909fb67039d096b13e7f1c1ad2a09834dd06e95a603aa0c454373db78"
    end
  end

  def install
    bin.install Dir["dakken-*"].first => "dakken"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/dakken --version")
  end
end
