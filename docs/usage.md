# Use

```sh
dakken            # 30-second test (or the mode you chose in the menu)
dakken -t 60      # 60-second test
dakken -w 25      # 25-word test
dakken -q         # quote test
dakken -q short   # short quote test
dakken -f notes.txt  # type the text of a file
dakken --words-file verbs.txt  # random words from a file
dakken -z         # zen: type freely, Enter to finish
dakken -d         # daily challenge: the same words for everyone today
dakken -l code    # test with the code word list
dakken -l weak-pairs  # words with your slowest letter pairs
dakken -p         # test with punctuation
dakken -p -n      # test with punctuation and numbers
dakken --theme nord  # use a color theme
dakken --strict master  # the first wrong key fails the test
dakken -g survival  # 3 lives: each wrong word takes one
dakken --race race.txt  # race the ghost of a saved test
dakken --caret bar  # a thin bar caret (or block, underline)
dakken --layout dvorak  # practise Dvorak (or colemak) on a QWERTY keyboard
dakken --history  # print personal bests and the last 10 tests
dakken --stats    # print averages and a graph of WPM by day
dakken --heatmap  # keyboard colored by errors, slowest keys, speed by day
dakken --badges   # print your level, XP and badges
dakken --skills   # print the level of each key and your key quests
dakken --export csv > history.csv  # save the history as CSV (or json)
dakken --check-update  # ask GitHub if there is a newer release (needs curl)
dakken --keys     # print raw key events (to check your terminal)
```

The test starts when you type the first letter.

While you type:

| Key | Action |
|---|---|
| `Tab` or `Esc` | Open the menu |
| `Enter` (in zen mode) | End the test |
| `Ctrl+R` | Start a new test at once |
| `Ctrl+Backspace`, `Alt+Backspace`, `Ctrl+W`, `Ctrl+H` | Delete the word |
| `Ctrl+C` | Quit (on every screen) |

On the results:

| Key | Action |
|---|---|
| `Enter` | Start the next test, with new words |
| `a` | Start the same words again (not in momo or zen mode, or in a climb) |
| `r` | Replay the test at the speed you typed it (any key stops it) |
| `y` | Copy the score card: WPM, accuracy and a square for each word (the daily challenge card also has your streak) |
| `x` | Save the test as a race file for `--race` |
| `Tab` or `Esc` | Open the menu |
| `q` | Quit |

In the menu, each setting shows its key on the left:

| Key | Action |
|---|---|
| `o` | Choose the mode: untimed, quote, zen, time or words |
| `p` | Turn punctuation on or off |
| `n` | Turn numbers on or off |
| `l` | Go to the next word list |
| `m` | Go to the next strict mode |
| `g` | Go to the next challenge |
| `x` | Start momo mode |
| `d` | Start the daily challenge (the menu shows if you did it today) |
| `q` | Quit |
| `f` | Open your profile (`↑`/`↓` or `j`/`k` scroll, `←`/`→` go to the pages stats, history, keys and skills, `Tab` goes back) |
| `e` | Open the settings |
| `?` | Show all the keys |
| `t`, `s`, `c` | Shortcuts: next theme, live WPM on or off, pace caret on or off |
| `Enter` | Start a new test |
| `Tab` or `Esc` | Go back to the test or the results |

On the settings screen (`e`), choose a row with `↑`/`↓` and change it with
`←`/`→` or `Enter`. The changes are saved at once:

| Setting | Values |
|---|---|
| theme | all the themes, and `custom` if you made one |
| live stats | live WPM and accuracy during the test, on or off |
| pace caret | on or off |
| caret | block, bar or underline |
| layout | qwerty, dvorak or colemak |
| weekly goal | off, 1000, 2500, 5000, 10000, 25000 or 50000 words |
| symbols | unicode or ascii |
| underline errors | underline the wrong letters too, for people who do not see the red |
| update check | on or off |

On the mode screen (`o`):

| Key | Action |
|---|---|
| `↑`/`↓` or `j`/`k` | Move the selection |
| `Enter` | Choose the mode and go back to the menu |
| `Tab` or `Esc` | Go back to the menu with no change |

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
