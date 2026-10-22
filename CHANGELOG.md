# Changelog

All notable changes to dakken. Versions follow [Semantic Versioning](https://semver.org).

## Unreleased

### Added

- Settings screen (`e` in the menu): theme, live stats, pace caret, caret,
  layout, weekly goal, symbols, underline errors and the update check.
- Help screen (`?` in the menu) with all the keys, and a hint before the
  first test.
- Stats, history, keys and skills pages in the app: `→` on the profile.
- `d` in the menu starts the daily challenge; the menu shows if you did it
  today. The results and `--stats` show when the next daily starts.
- `a` on the results starts the same words again; `Ctrl+R` starts a new test
  while you type; `q` quits on the menu and the results.
- Score cards (`y`) for all tests, not only the daily challenge. The card is
  also saved in `score-card.txt`.
- `--words-file`: random words from your own file.
- French and Portuguese word lists.
- `underline_errors`: underline wrong letters, not only color them.
- Full-screen pages for a new rank or a new badge, and the profile on `f`.
- Four new badges.
- `symbols = "ascii"` for fonts without the symbols.
- Install scripts: `install.ps1` for Windows, `install.sh` for macOS and
  Linux. A Scoop bucket in this repository.
- Windows on ARM release binary.

### Changed

- The menu is shorter: the display options moved to the settings screen
  (`t`, `s` and `c` still work as shortcuts).
- The daily challenge words are the same on every system.
- Terminals without 24-bit color get the nearest of the 256 colors.
- The release workflow updates the Homebrew formula and the Scoop manifests.

### Fixed

- `y` copies through the system clipboard command too (PowerShell, `pbcopy`,
  `wl-copy`, `xclip`, `xsel`), for terminals without OSC 52.
- `Ctrl+H` deletes a word, because many terminals send it for
  `Ctrl+Backspace`.
- The Linux text console gets ASCII symbols.
- `symbols = "ascii"` also replaces quote marks and `×` in momo quotes.
- `--heatmap`, `--badges` and `--skills` follow `NO_COLOR`.

## 0.5.1 - 2026-10-21

### Added

- Update notice in the menu, and `u` to update with the tool that installed
  dakken. A file from Releases updates itself.

## 0.5.0 - 2026-10-21

### Added

- XP, levels, ranks and badges.
- Daily score card, streak freezes and a weekly goal of words.
- Challenges: survival, boss words and climb.
- Ghost races: save a test with `x` and race it with `--race`.
- Skill map (`--skills`) and key quests.
- Momo mode (`x` in the menu): 60 seconds of quotes.

### Changed

- Time 30 is the default mode.

## 0.4.1 - 2026-10-19

### Changed

- Static C runtime on Windows and static musl binaries on Linux, so the
  files run with nothing else installed.

## 0.4.0 - 2026-10-19

### Added

- Daily challenge (`--daily`) with a streak in `--stats`.
- `weak-pairs` word list from your slowest letter pairs.
- The pace caret replays your personal-best test key by key.
- Menu with the logo, a mode screen on `o`, and new themes: tokyo-night
  (the default), catppuccin-mocha, rose-pine, one-dark, catppuccin-latte
  and github-light.
- Linux ARM64 and Intel macOS builds.
- winget install.

## 0.3.0 - 2026-10-18

### Added

- Word lists: english-1k, code, spanish, german, mistakes and weak-keys.
- Zen mode, custom text with `--file`, and quote lengths.
- Strict modes: stop-on-error, no-backspace and master.
- Pace caret, live WPM and accuracy, and replay of the last test.
- `--stats`, `--heatmap` and per-key stats, and history export as CSV or
  JSON.
- Dvorak and Colemak practice on a QWERTY keyboard, caret styles and a
  custom theme.
- `--check-update`.

## 0.2.0 - 2026-10-18

First release: time, words, quote and untimed modes, punctuation and
numbers, results with a WPM graph, personal bests and history, themes and a
config file.
