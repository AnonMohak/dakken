# dakken

A typing-speed test for the terminal, in the style of [monkeytype](https://monkeytype.com).

- Untimed mode (the default): type the words on the screen, with no time limit.
- Time mode (15, 30, 60, 120 s) and words mode (10, 25, 50, 100 words).
- Quote mode: one quote from a public-domain book, poem or speech.
- Punctuation mode: capital letters at the start of sentences, and `. , ? ! ; :`,
  quotes and parentheses.
- Numbers mode: random numbers in place of some words.
- Word lists: `english` (200 common words), `english-1k` (1000 words),
  `code` (keywords, operators and short pieces of code) and `mistakes`.
- Practise your mistakes: the `mistakes` list uses the words you typed wrong
  in past tests. A word leaves the list when you type it right there.
- Monkeytype rules: wrong and missed letters in red, extra letters in dark red,
  backspace into a previous word only if it has an error.
- Mistakes do not stop you: the test ends when the last word has all its
  letters, also if some are wrong. Accuracy shows the errors.
- Colors from your terminal's palette, so the test follows your shell theme.
  Other themes: `serika-dark`, `nord` and `dracula`.
- Settings are saved: the menu remembers the mode, options and theme.
- Optional live WPM and accuracy during the test (off by default).
- Optional pace caret: a gray letter that moves at your personal-best speed
  for the same mode and options, so you can race it (off by default).
- Results: WPM, raw WPM, accuracy, consistency, and character counts, with a
  graph of WPM and raw WPM over the test.
- Personal bests and history for each mode and option set.
- One `.exe` with no runtime. Made and tested on Windows 11 with WezTerm.

## Install

**Download:** get the file for your system from
[Releases](https://github.com/AnonMohak/dakken/releases). On Windows, rename it
to `dakken.exe` and put it in a folder on your `PATH`.

**Build from source:** you need [Rust](https://rustup.rs). On Windows:
`winget install Rustlang.Rustup`.

```sh
cargo install --git https://github.com/AnonMohak/dakken.git
```

Or from a clone: `cargo install --path .`

## Use

```sh
dakken            # untimed test: the words on the screen
dakken -t 60      # 60-second test
dakken -w 25      # 25-word test
dakken -q         # quote test
dakken -l code    # untimed test with the code word list
dakken -p         # untimed test with punctuation
dakken -p -n      # untimed test with punctuation and numbers
dakken --theme nord  # use a color theme
dakken --history  # print personal bests and the last 10 tests
dakken --stats    # print averages and a graph of WPM by day
dakken --keys     # print raw key events (to check your terminal)
```

The test starts when you type the first letter.

| Key | Action |
|---|---|
| `Tab` or `Esc` | Open or close the menu (`↑`/`↓` or `j`/`k`, then `Enter`) |
| `Enter` (on the results) | Start a new test |
| `p` (in the menu) | Turn punctuation on or off |
| `n` (in the menu) | Turn numbers on or off |
| `l` (in the menu) | Go to the next word list |
| `t` (in the menu) | Go to the next color theme |
| `s` (in the menu) | Show or hide the live WPM and accuracy |
| `c` (in the menu) | Show or hide the pace caret |
| `Ctrl+Backspace`, `Ctrl+W` | Delete the word |
| `Ctrl+C` | Quit |

## Bigger font in WezTerm

A terminal app cannot set the font size. In WezTerm, dakken sets the user var
`dakken_active` to `1` at start and `0` at exit. Add this to `.wezterm.lua`,
before `return config`, to get a font 2 points bigger while dakken runs:

```lua
wezterm.on("user-var-changed", function(window, pane, name, value)
  if name ~= "dakken_active" then
    return
  end
  local overrides = window:get_config_overrides() or {}
  if value == "1" then
    overrides.font_size = config.font_size + 2
  else
    overrides.font_size = nil
  end
  window:set_config_overrides(overrides)
end)
```

## Settings

The menu saves the mode, punctuation, numbers, word list and theme to `config.toml`, in
the same folder as the history. The next start uses them. Command-line flags
change them for one run only, until you change a setting in the menu.

```toml
mode = "time 30"     # untimed, quote, time N or words N
theme = "terminal"   # terminal, serika-dark, nord or dracula
punctuation = false
numbers = false
words = "english"    # english, english-1k, code or mistakes
live = false         # live WPM and accuracy during the test
pace = false         # pace caret at your personal-best speed
```

## History

Each finished test adds one line to `history.tsv`: in `%APPDATA%\dakken` on
Windows, and in `$XDG_DATA_HOME/dakken` or `~/.local/share/dakken` on other
systems. A personal best counts only tests with the same mode and options,
for example `time 30 · punctuation`. Delete the file to clear the history.

`mistakes.txt`, in the same folder, keeps the last 100 words you typed wrong.
If it is empty, the `mistakes` list uses the English words.

## How the stats work

- **WPM:** (letters in correct words + the spaces after them) ÷ 5 ÷ minutes.
- **Raw WPM:** all typed letters and spaces ÷ 5 ÷ minutes.
- **Accuracy:** correct key presses ÷ all key presses.
- **Consistency:** monkeytype's formula, `100 × (1 − tanh(cv + cv³/3 + cv⁵/5))`,
  where `cv` is the standard deviation ÷ mean of the raw WPM in each second.
- **Characters:** correct / incorrect / extra / missed.

## Build problem on Windows

If `cargo build` fails with `link: extra operand`, the environment variables
`VCINSTALLDIR` and `VSINSTALLDIR` are set globally. Rust then uses the wrong
`link.exe` (from Git or Cygwin). Remove these two variables from your system
settings, or remove them in the shell before you build:

```powershell
Remove-Item Env:VCINSTALLDIR, Env:VSINSTALLDIR
```

## Development

```sh
cargo test      # engine, app and screen tests
cargo clippy --all-targets
```

CI runs the tests on Windows, Linux and macOS. To publish a release, push a
tag such as `v0.2.0`: the release workflow builds the binaries and adds them
to the GitHub Release.

The code has three parts:

- `src/engine/`: typing rules, stats and word list. No terminal code.
- `src/app.rs`: screens and key handling.
- `src/ui/`: drawing with [ratatui](https://ratatui.rs).

## License

MIT. The word list is a list of common English words made for this project.
The quotes are from public-domain texts.
