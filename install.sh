#!/bin/sh
# Install dakken on macOS or Linux, with no sudo:
#
#   curl -fsSL https://raw.githubusercontent.com/AnonMohak/dakken/master/install.sh | sh
#
# It downloads the newest dakken for your system from GitHub Releases,
# checks it against the release's SHA256SUMS, puts it in ~/.local/bin, and,
# if that folder is not on PATH, adds it in your shell's config file. Run it
# again to update; in dakken, `u` in the menu also updates it.
#
# Set DAKKEN_INSTALL_DIR to use another folder. Then the script does not
# change any config file; add the folder to PATH yourself.

set -eu

repo=AnonMohak/dakken

fail() {
    echo "error: $1" >&2
    exit 1
}

case "$(uname -s)-$(uname -m)" in
    Darwin-arm64) target=aarch64-apple-darwin ;;
    Darwin-x86_64) target=x86_64-apple-darwin ;;
    Linux-x86_64 | Linux-amd64) target=x86_64-unknown-linux-musl ;;
    Linux-aarch64 | Linux-arm64) target=aarch64-unknown-linux-musl ;;
    *) fail "dakken has no build for $(uname -s) $(uname -m). On Windows, use install.ps1." ;;
esac

command -v curl > /dev/null || fail "this script needs curl"
tag=$(curl -fsSL -H 'User-Agent: dakken-install' "https://api.github.com/repos/$repo/releases/latest" |
    sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n 1)
[ -n "$tag" ] || fail "cannot find the latest release on GitHub"
name="dakken-$tag-$target"
base="https://github.com/$repo/releases/download/$tag"

set_path=yes
if [ -n "${DAKKEN_INSTALL_DIR:-}" ]; then
    dir=$DAKKEN_INSTALL_DIR
    set_path=no
else
    dir="$HOME/.local/bin"
fi
mkdir -p "$dir"

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
echo "Downloading dakken $tag..."
curl -fsSL -o "$tmp/$name" "$base/$name" || fail "cannot download $name"
curl -fsSL -o "$tmp/SHA256SUMS" "$base/SHA256SUMS" || fail "cannot download SHA256SUMS"

want=$(awk -v n="$name" '{ f = $2; sub(/^\*/, "", f); if (f == n) print $1 }' "$tmp/SHA256SUMS")
[ -n "$want" ] || fail "SHA256SUMS has no line for $name"
if command -v sha256sum > /dev/null; then
    got=$(sha256sum "$tmp/$name" | cut -d ' ' -f 1)
else
    got=$(shasum -a 256 "$tmp/$name" | cut -d ' ' -f 1)
fi
[ "$got" = "$want" ] || fail "the download does not match its checksum; nothing was installed"

chmod 755 "$tmp/$name"
mv -f "$tmp/$name" "$dir/dakken"
echo "dakken $tag is installed in $dir."

case ":$PATH:" in
    *":$dir:"*) echo 'Run "dakken" to start.' ;;
    *)
        if [ "$set_path" = no ]; then
            echo "Add $dir to PATH to run dakken from any folder."
            exit 0
        fi
        case "$(basename "${SHELL:-sh}")" in
            zsh)
                rc="$HOME/.zshrc"
                line='export PATH="$HOME/.local/bin:$PATH"'
                ;;
            bash)
                # macOS Terminal starts login shells, which read .bash_profile.
                if [ "$(uname -s)" = Darwin ]; then rc="$HOME/.bash_profile"; else rc="$HOME/.bashrc"; fi
                line='export PATH="$HOME/.local/bin:$PATH"'
                ;;
            fish)
                rc="$HOME/.config/fish/config.fish"
                line='fish_add_path $HOME/.local/bin'
                ;;
            *)
                rc="$HOME/.profile"
                line='export PATH="$HOME/.local/bin:$PATH"'
                ;;
        esac
        if ! grep -qxF "$line" "$rc" 2> /dev/null; then
            mkdir -p "$(dirname "$rc")"
            printf '\n# Added by the dakken installer\n%s\n' "$line" >> "$rc"
            echo "Added $dir to PATH in $rc."
        fi
        echo "Open a new terminal, then run \"dakken\" to start."
        ;;
esac
