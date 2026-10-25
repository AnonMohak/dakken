# Changelog

All notable changes to dakken. Versions follow [Semantic Versioning](https://semver.org).

## Unreleased

### Added

- Competitive modes: `time 30` and `time 60` show "competitive" in the menu
  and in the `o` list. They have plain words only: punctuation, numbers, the
  word list, a word file, strict mode and the challenge are gray in the menu,
  their keys do nothing, and the label is only "time 30" or "time 60". Your
  options stay saved and come back in the other modes. These are the modes of
  the speed boards on the dakken site.

## 0.6.2 - 2026-10-24

### Added

- The settings screen (`e`) tells how to change the text size: dakken cannot
  set the font, so zoom the terminal with `Ctrl` + `+` and `Ctrl` + `-`
  (`Cmd` on macOS). The README tells it too.

## 0.6.1 - 2026-10-24

### Changed

- `u` updates a Cargo install with the release file, as for a file from
  Releases: nothing compiles, so a broken Rust or Visual Studio setup cannot
  stop the update. `cargo install` stays as the fallback, and it no longer
  takes Cygwin's or Git's `link.exe` for the Visual Studio linker when
  `VCINSTALLDIR` is set but empty.

## 0.6.0 - 2026-10-24

### Added

- Practice mode: four new rows in the `o` list. `practice adaptive` gives
  short lessons of made-up words from a few letters, with a new letter when
  every letter reaches the target speed (`practice_target`, 35 WPM).
  `practice weak-keys`, `weak-pairs` and `mistakes` practise your weak
  points. Lessons follow one another; `Tab` shows a summary. Practice keeps
  its own files and does not count in the history, XP or stats. `--practice`
  starts it from the command line.
- A practice page in the profile, with the letters of each language, the
  speed of each key, your weak and strong keys, and `r` two times to reset.

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

- Momo mode has a new collection of 36 quotes, each with a mood. The
  results no longer show the series.

- The `mistakes`, `weak-keys` and `weak-pairs` word lists are now practice
  kinds in the `o` list. `-l weak-keys` (and the others) and old config
  files start that practice.

- The heatmaps and the skill map in the app use the colors of the theme.
- In modes with their own text (quote, zen, custom, daily, race),
  punctuation, numbers and the word list are gray in the menu, and `p`, `n`
  and `l` do nothing.
- The profile and its pages start at the top, in the same place on each
  page. A long word list row (`weak-pairs`) no longer moves the menu.
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
