# typer

A typing-speed test for the terminal, in the style of [monkeytype](https://monkeytype.com).

- Untimed mode (the default): type the words on the screen, with no time limit.
- Time mode (15, 30, 60, 120 s) and words mode (10, 25, 50, 100 words).
- Quote mode: one quote from a public-domain book, poem or speech.
- Punctuation mode: capital letters at the start of sentences, and `. , ? ! ; :`,
  quotes and parentheses.
- Numbers mode: random numbers in place of some words.
- Monkeytype rules: wrong letters in red, extra letters in dark red,
  backspace into a previous word only if it has an error.
- Colors from your terminal's palette, so the test follows your shell theme.
- Results: WPM, raw WPM, accuracy, consistency, and character counts, with a
  graph of WPM and raw WPM over the test.
- Personal bests and history for each mode and option set.
- One `.exe` with no runtime. Made and tested on Windows 11 with WezTerm.

## Install

You need [Rust](https://rustup.rs). On Windows: `winget install Rustlang.Rustup`.

```sh
git clone https://github.com/AnonMohak/cli-typing.git
cd cli-typing
cargo install --path .
```

## Use

```sh
typer            # untimed test: the words on the screen
typer -t 60      # 60-second test
typer -w 25      # 25-word test
typer -q         # quote test
typer -p         # untimed test with punctuation
typer -p -n      # untimed test with punctuation and numbers
typer --history  # print personal bests and the last 10 tests
typer --keys     # print raw key events (to check your terminal)
```

The test starts when you type the first letter.

| Key | Action |
|---|---|
| `Tab` | Restart the test |
| `Esc` | Open the mode menu (`↑`/`↓` or `j`/`k`, then `Enter`) |
| `p` (in the menu) | Turn punctuation on or off |
| `n` (in the menu) | Turn numbers on or off |
| `Ctrl+Backspace`, `Ctrl+W` | Delete the word |
| `Ctrl+C` | Quit |

## History

Each finished test adds one line to `history.tsv`: in `%APPDATA%\typer` on
Windows, and in `$XDG_DATA_HOME/typer` or `~/.local/share/typer` on other
systems. A personal best counts only tests with the same mode and options,
for example `time 30 · punctuation`. Delete the file to clear the history.

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

The code has three parts:

- `src/engine/`: typing rules, stats and word list. No terminal code.
- `src/app.rs`: screens and key handling.
- `src/ui/`: drawing with [ratatui](https://ratatui.rs).

## License

MIT. The word list is a list of common English words made for this project.
The quotes are from public-domain texts.
