class Dakken < Formula
  desc "Monkeytype-style typing test for the terminal"
  homepage "https://github.com/AnonMohak/dakken"
  version "0.2.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.2.0/dakken-v0.2.0-aarch64-apple-darwin"
      sha256 "cb0b13b18012760eaf78e3542c85ea3526f1180538c0c711421140728de76fda"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/AnonMohak/dakken/releases/download/v0.2.0/dakken-v0.2.0-x86_64-unknown-linux-gnu"
      sha256 "55b89e16505adf9d9d7fc43aab056a8fc1edf7da1c21fb606880e4b97f412ec1"
    end
  end

  def install
    bin.install Dir["dakken-*"].first => "dakken"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/dakken --version")
  end
end
