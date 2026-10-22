# Troubleshooting

**Boxes or question marks in place of symbols.** Your font does not have
some symbols, such as `━`, `♥` or `▲`. Set `symbols = "ascii"` in
`config.toml`, or use a font such as Cascadia Mono, JetBrains Mono or a Nerd
Font. In the Linux text console, dakken uses ASCII automatically.

**Wrong or strange colors.** The themes use 24-bit color. When the
`COLORTERM` environment variable does not say `truecolor` or `24bit`,
dakken uses the nearest of the 256 colors. If your terminal shows 24-bit
color but does not set `COLORTERM`, set `COLORTERM=truecolor`. You can also
use the `terminal` theme, which uses the colors of your terminal:
`dakken --theme terminal`. The printed reports (`--heatmap`, `--badges`,
`--skills`) have no color with `NO_COLOR` set.

**`y` does not copy the score card.** dakken tries the OSC 52 escape code
and the clipboard command of your system. On Linux, install `wl-clipboard`
(Wayland) or `xclip` (X11). In tmux, add `set -g set-clipboard on` to
`~/.tmux.conf`. Over SSH, only OSC 52 can work. The card is always in
`score-card.txt` in the data folder.

**A key does not do what it should.** Run `dakken --keys` and press the key
to see what your terminal sends. Some terminals send `Ctrl+H` for
`Ctrl+Backspace`; dakken deletes the word for both.

**No update notice.** The update check needs `curl`. Windows 10 (1803 and
newer), macOS and most Linux systems have it. `dakken --check-update`
shows the error, if there is one.

**"terminal too small".** dakken needs a terminal of at least 30 columns
and 11 rows. Make the window bigger or the font smaller.

### Build problem on Windows

If `cargo build` fails with `link: extra operand`, the environment variables
`VCINSTALLDIR` and `VSINSTALLDIR` are set globally. Rust then uses the wrong
`link.exe` (from Git or Cygwin). Remove these two variables from your system
settings, or remove them in the shell before you build:

```powershell
Remove-Item Env:VCINSTALLDIR, Env:VSINSTALLDIR
```
