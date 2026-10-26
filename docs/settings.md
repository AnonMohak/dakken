# Settings

The menu saves all the settings to `config.toml`, in the same folder as the
history. The next start uses them. Command-line flags change them for one run
only, until you change a setting in the menu. The daily challenge (`-d`) is
never saved as the mode.

```toml
mode = "time 30"     # untimed, quote, quote short/medium/long, zen, time N, words N or practice adaptive/weak-keys/weak-pairs/mistakes
theme = "sumi"  # sumi, tokyo-night, terminal, dracula, nord, github-light, ...
punctuation = false
numbers = false
words = "english"    # english, english-1k, code, spanish, german, french or portuguese
live = false         # live WPM and accuracy during the test
pace = false         # pace caret that replays your personal-best test
strict = "off"       # off, stop-on-error, no-backspace or master
challenge = "off"    # off, survival, boss or climb
caret = "block"      # block, bar or underline
layout = "qwerty"    # qwerty, dvorak or colemak
weekly_goal = 0      # words to type each week (Monday to Sunday, UTC); 0 is no goal
update_check = true  # ask GitHub once a day for a newer version
symbols = "unicode"  # or "ascii": the app screens use only ASCII characters (always in the Linux text console)
underline_errors = false  # underline wrong letters, not only color them
practice_target = 35  # WPM each letter needs in adaptive practice before the next unlocks (1 to 300)
```

With a weekly goal, the menu shows the words of this week under your level,
`--stats` shows a bar for the goal, and the results say when a test reaches
it. Reach the goal once for the `goal getter` badge.

### Custom theme

Set any of these keys in `config.toml` to make a `custom` theme. A color is
a hex value such as `"#ff79c6"` or a terminal color name such as `"cyan"`,
`"darkgray"` or `"lightred"`. A key that is not set keeps the color of the
`terminal` theme. `color_bg = "none"` keeps the terminal background.

```toml
theme = "custom"
color_bg = "none"
color_main = "#ff79c6"
color_sub = "darkgray"
color_text = "reset"
color_error = "lightred"
color_error_extra = "red"
```
