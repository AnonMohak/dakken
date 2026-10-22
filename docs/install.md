# Install, update and uninstall

## Install

**Install script (Windows, no admin rights):**

```powershell
irm https://raw.githubusercontent.com/AnonMohak/dakken/master/install.ps1 | iex
```

It downloads the newest `dakken.exe` for your system (x64 or ARM64) from
Releases, checks it against the release's `SHA256SUMS`, puts it in `%LOCALAPPDATA%\Programs\dakken`, and
adds that folder to your user `PATH`. To read the script before you run it,
open [install.ps1](../install.ps1).

**Install script (macOS and Linux, no sudo):**

```sh
curl -fsSL https://raw.githubusercontent.com/AnonMohak/dakken/master/install.sh | sh
```

It downloads the newest dakken for your system, checks it against
`SHA256SUMS`, puts it in `~/.local/bin`, and, if that folder is not on your
`PATH`, adds it in your shell's config file (`~/.zshrc`, `~/.bashrc`,
`~/.bash_profile` on macOS, fish's `config.fish`, or `~/.profile`). Read
it first in [install.sh](../install.sh).

**winget (Windows):**

```powershell
winget install AnonMohak.dakken
```

**Scoop (Windows):**

```powershell
scoop bucket add dakken https://github.com/AnonMohak/dakken
scoop install dakken/dakken
```

An install from the older manifest URL keeps working with `scoop update`.

**Homebrew (macOS and Linux, on Intel or ARM):**

```sh
brew tap AnonMohak/dakken https://github.com/AnonMohak/dakken
brew install AnonMohak/dakken/dakken
```

**Download:** get the file for your system from
[Releases](https://github.com/AnonMohak/dakken/releases). On Windows, rename it
to `dakken.exe` and put it in a folder on your `PATH`.

Each file is one program with nothing else to install: the Windows file needs
no Visual C++ Redistributable, and the Linux files are static, so they run on
old and new systems.

The files are not signed, so a file downloaded with a browser can get a
warning. The install scripts, winget, Scoop and Homebrew do not get it.

- Windows ("Windows protected your PC"): click **More info**, then **Run
  anyway**.
- macOS ("cannot be opened because the developer cannot be verified"):
  run `xattr -d com.apple.quarantine ./dakken`, then start it again.
- macOS and Linux: make the file executable with `chmod +x ./dakken`.

**Build from source:** you need [Rust](https://rustup.rs). On Windows:
`winget install Rustlang.Rustup`.

```sh
cargo install --git https://github.com/AnonMohak/dakken.git
```

Or from a clone: `cargo install --path .`

## Update

Use the same tool you installed with:

| Installed with | Update |
|---|---|
| winget | `winget upgrade AnonMohak.dakken` |
| Scoop | `scoop update dakken` |
| Homebrew | `brew update`, then `brew upgrade dakken` |
| Cargo | the same `cargo install` command again (add `--force` for `--git`) |
| A file from Releases | download the new file and replace the old one |

dakken asks GitHub once a day for a newer release, in the background. When
there is one, the menu says `update v0.5.1 available · u to update`. Press
`u`: dakken closes and runs the update command of the tool that installed it
(Scoop, winget, Homebrew or Cargo). On Windows, the update runs in a new
window, because a running program cannot be replaced there; start dakken
again when it is done. A file from Releases updates itself: dakken
downloads the new file for your system, checks it against the release's
`SHA256SUMS`, and puts it in place of the old one. Set `update_check = false` in `config.toml` to turn the check off (it
sends one request to GitHub each day). `dakken --check-update` asks at once.

Your history, settings and badges stay in the data folder, so an update
keeps them.

## Uninstall

Use the same tool you installed with:

| Installed with | Uninstall |
|---|---|
| winget | `winget uninstall AnonMohak.dakken` |
| Scoop | `scoop uninstall dakken` |
| Homebrew | `brew uninstall dakken` |
| Cargo | `cargo uninstall dakken` |
| `install.ps1` | delete the folder `%LOCALAPPDATA%\Programs\dakken`, and remove it from your user `PATH` (Settings → System → About → Advanced system settings → Environment Variables) |
| `install.sh` | delete `~/.local/bin/dakken`, and remove the line after `# Added by the dakken installer` from your shell config file |
| A file from Releases | delete the file |

Your history, settings and badges stay. To delete them too, delete the
data folder: `%APPDATA%\dakken` on Windows, and `~/.local/share/dakken`
(or `$XDG_DATA_HOME/dakken`) on macOS and Linux.
