#!/bin/sh
# Write the Homebrew formula and the Scoop manifests for a release, from its
# SHA256SUMS. release.yml runs it after each release, then commits the files:
#
#   sh .github/scripts/packaging.sh v0.6.0 path/to/SHA256SUMS
#
# Windows ARM64 goes in the Scoop manifests only when the release has it.

set -eu

tag=$1
sums=$2
version=${tag#v}
download="https://github.com/AnonMohak/dakken/releases/download"

hash() {
    awk -v n="dakken-$tag-$1" '{ f = $2; sub(/^\*/, "", f); if (f == n) print $1 }' "$sums"
}

need() {
    [ -n "$(hash "$1")" ] || { echo "error: SHA256SUMS has no line for $1" >&2; exit 1; }
}

for target in aarch64-apple-darwin x86_64-apple-darwin aarch64-unknown-linux-musl \
    x86_64-unknown-linux-musl x86_64-pc-windows-msvc.exe; do
    need "$target"
done

cat > Formula/dakken.rb << EOF
class Dakken < Formula
  desc "Typing-speed test for the terminal"
  homepage "https://github.com/AnonMohak/dakken"
  version "$version"
  license "MIT"

  on_macos do
    on_arm do
      url "$download/$tag/dakken-$tag-aarch64-apple-darwin"
      sha256 "$(hash aarch64-apple-darwin)"
    end
    on_intel do
      url "$download/$tag/dakken-$tag-x86_64-apple-darwin"
      sha256 "$(hash x86_64-apple-darwin)"
    end
  end

  # Static musl binaries: they run on old and new Linux systems.
  on_linux do
    on_arm do
      url "$download/$tag/dakken-$tag-aarch64-unknown-linux-musl"
      sha256 "$(hash aarch64-unknown-linux-musl)"
    end
    on_intel do
      url "$download/$tag/dakken-$tag-x86_64-unknown-linux-musl"
      sha256 "$(hash x86_64-unknown-linux-musl)"
    end
  end

  def install
    bin.install Dir["dakken-*"].first => "dakken"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/dakken --version")
  end
end
EOF

x64_url="$download/$tag/dakken-$tag-x86_64-pc-windows-msvc.exe#/dakken.exe"
x64_auto="$download/v\$version/dakken-v\$version-x86_64-pc-windows-msvc.exe#/dakken.exe"
arm_hash=$(hash aarch64-pc-windows-msvc.exe)
if [ -n "$arm_hash" ]; then
    arm_url="$download/$tag/dakken-$tag-aarch64-pc-windows-msvc.exe#/dakken.exe"
    arm_auto="$download/v\$version/dakken-v\$version-aarch64-pc-windows-msvc.exe#/dakken.exe"
    arm=",
        \"arm64\": {
            \"url\": \"$arm_url\",
            \"hash\": \"$arm_hash\"
        }"
    arm_autoupdate=",
            \"arm64\": {
                \"url\": \"$arm_auto\"
            }"
else
    arm=""
    arm_autoupdate=""
fi

for manifest in bucket/dakken.json packaging/scoop/dakken.json; do
    cat > "$manifest" << EOF
{
    "version": "$version",
    "description": "A typing-speed test for the terminal",
    "homepage": "https://github.com/AnonMohak/dakken",
    "license": "MIT",
    "architecture": {
        "64bit": {
            "url": "$x64_url",
            "hash": "$(hash x86_64-pc-windows-msvc.exe)"
        }$arm
    },
    "bin": "dakken.exe",
    "checkver": "github",
    "autoupdate": {
        "architecture": {
            "64bit": {
                "url": "$x64_auto"
            }$arm_autoupdate
        }
    }
}
EOF
done

echo "Formula and Scoop manifests are now for $tag."
