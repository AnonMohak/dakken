<p align="center">
  <img src="assets/brand/logo.svg" width="128" alt="dakken logo: a teal keycap with speed lines">
</p>

# dakken

A typing-speed test for the terminal. One small program for Windows, macOS
and Linux, with no account and nothing else to install.

- **Many ways to type:** time, words, quotes, untimed, zen, your own text,
  and a daily challenge with the same words for everyone.
- **Practice mode:** short lessons of made-up words from a few letters,
  with a new letter when you are fast enough. Or practise your weak keys,
  slow letter pairs and past mistakes. Practice stays apart from your
  history. A skill map and key quests show your progress.
- **Progress:** XP, ranks, badges, daily streaks and a weekly goal.
- **Challenges:** survival, boss words, climb, strict modes, and ghost races
  that you can send to a friend.
- **Clear results:** WPM, accuracy, consistency, a graph, your slowest word,
  and a score card to paste in a chat.
- **Your look:** 10 themes and your own colors, three caret shapes, and
  Dvorak and Colemak practice on a QWERTY keyboard.

All the features are in [docs/features.md](docs/features.md).

## Install

**Windows** (no admin rights):

```powershell
irm https://raw.githubusercontent.com/AnonMohak/dakken/master/install.ps1 | iex
```

**macOS and Linux** (no sudo):

```sh
curl -fsSL https://raw.githubusercontent.com/AnonMohak/dakken/master/install.sh | sh
```

You can also use Scoop, Homebrew, Cargo or a file from Releases. (winget
is not ready yet: the package waits for approval, so do not use
`winget install AnonMohak.dakken` for now.) See
[docs/install.md](docs/install.md), which also tells how to update and
uninstall.

## Use

```sh
dakken            # start a test: type the first letter to begin
dakken -d         # the daily challenge: the same words for everyone today
dakken -t 60      # a 60-second test
dakken --help     # all the options
```

| Key | Action |
|---|---|
| `Tab` or `Esc` | Open the menu |
| `?` (in the menu) | Show all the keys |
| `Ctrl+R` | Start a new test while you type |
| `Enter` (on the results) | Start the next test |
| `q` (in the menu or on the results) | Quit |

More in [docs/usage.md](docs/usage.md).

## Privacy

dakken has no account and sends no data about you. It sends one request to
GitHub each day to look for a newer version (`update_check = false` in
`config.toml` turns it off). Your history, settings and badges stay in
files on your computer.

## Documentation

- [Features](docs/features.md)
- [Install, update and uninstall](docs/install.md)
- [Use: commands and keys](docs/usage.md)
- [Settings and custom themes](docs/settings.md)
- [Data files and how the stats work](docs/data.md)
- [Troubleshooting](docs/troubleshooting.md)
- [Development](docs/development.md)
- [Changelog](CHANGELOG.md)

## Word lists and quotes

The word lists (English, code, Spanish, German, French and Portuguese) were
written for this project. The quotes of quote mode are from public-domain books, poems and
speeches. The momo mode collection (`assets/quotes/momo.md`) has quotes
from One Piece (Eiichiro Oda), Hunter x Hunter (Yoshihiro Togashi) and
Vinland Saga (Makoto Yukimura). They are not public domain and belong to
their owners; the MIT license does not cover them.

## License

MIT. See [LICENSE](LICENSE).
