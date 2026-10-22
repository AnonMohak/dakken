# Features

- Time mode (15, 30, 60, 120 s); 30 seconds is the default. The seconds
  left show on the right above the words.
- Words mode (10, 25, 50, 100 words).
- Untimed mode: type the words that fill the screen, with no time limit.
- The text fills 70% of the terminal, with an empty row between lines.
- Quote mode: one quote from a public-domain book, poem or speech. Choose
  short (up to 15 words), medium (16 to 30) or long (more than 30) quotes.
- Momo mode: press `x` in the menu to start it at once. 150 quotes,
  paraphrases and longer speeches from One Piece, Hunter x Hunter and
  Vinland Saga come one after the other for 60 seconds (the next quote
  starts as soon as you type the last letter, with no space), with the timer on
  the right above the words. The screen shows one quote at a time; two rows
  under it, on the right and in gray, a mood symbol and who said it (not
  typed): ♥ love, ≈ peace, † darkness, ▼ sorrow, ↑ change, ▲ resolve, ¤
  dreams. The results show the quotes you typed to the end. The screen says
  "momo mode" and uses its own night sakura colors (the menu keeps your
  theme). Only `x` in the menu
  starts momo mode: momo is not in the `o` list, and `Enter` in the menu
  starts the mode from `o`, with your own theme. Momo mode is a mode on its
  own: challenges (`g`) and strict modes (`m`) are always off in it.
- Custom mode: type the text of your own file, with `--file`. Or use the
  words of a file as the word list, in random order, with `--words-file`
  (`l` in the menu goes back to the chosen list).
- Zen mode: type anything with no target text. `Enter` ends the test.
- Daily challenge: the same 30 words for everyone each day (UTC), with your
  streak of days in a row in `--stats`. Each 7 days of a streak give a
  streak freeze (you keep up to 2): a freeze keeps the streak when you miss
  one day. The day changes at midnight UTC: the results say when the next
  daily challenge starts. After the test, press `y` to copy a
  score card to paste in a chat: your WPM, accuracy and streak, and a 🟦 for
  each correct word (⬛ for a wrong word). The copy uses the OSC 52 escape
  code and the clipboard command of your system (PowerShell on Windows,
  `pbcopy` on macOS, `wl-copy`, `xclip` or `xsel` on Linux). The card is
  also saved in `score-card.txt` in the data folder, for terminals where
  the copy does not work.
- Punctuation mode: capital letters at the start of sentences, and `. , ? ! ; :`,
  quotes and parentheses.
- Numbers mode: random numbers in place of some words.
- Word lists: `english` (200 common words), `english-1k` (1000 words),
  `code` (keywords, operators and short pieces of code), `spanish`, `german`,
  `french` and `portuguese` (250 common words each), `mistakes`, `weak-keys`
  and `weak-pairs`.
- Practise your weak keys: the `weak-keys` list uses words with the 5 letters
  that have the most errors (then the slowest) in your key stats.
- Skill map and key quests: `--skills` shows a keyboard with a level from 1
  to 10 for each key, from its speed and errors. dakken gives your 3 weakest
  letters a quest, for example "q under 180 ms": type that letter at least 3
  times in one test, faster than the target and with no errors. The
  `weak-keys` list practises the quest letters first, the results say when
  you finish a quest, and the key turns teal on the skill map.
- Practise your slow letter pairs: the `weak-pairs` list uses words with the
  5 slowest letter pairs (like `th` or `er`) in your key stats. `--heatmap`
  shows them.
- Practise your mistakes: the `mistakes` list uses the words you typed wrong
  in past tests. A word leaves the list when you type it right there.
- Clear errors: wrong and missed letters in red, extra letters in dark red,
  backspace into a previous word only if it has an error.
- Mistakes do not stop you: the test ends when the last word has all its
  letters, also if some are wrong. Accuracy shows the errors.
- Tokyo Night colors by default. The `terminal` theme uses your terminal's
  palette, so the test follows your shell theme. Other dark themes:
  `dracula`, `serika-dark`, `nord`, `catppuccin-mocha`, `rose-pine` and
  `one-dark`. Light themes: `catppuccin-latte` and `github-light`. You can
  also make your own `custom` theme.
- Settings are saved: the menu remembers the mode, options and theme.
- Optional live WPM and accuracy during the test (off by default).
- Practise Dvorak or Colemak on a QWERTY keyboard: each key gives the
  character at the same place on that layout. Your system layout must be
  US QWERTY for this.
- Strict modes: `stop-on-error` (only the correct key moves
  the caret), `no-backspace`, and `master` (the first wrong key fails the
  test; a failed test is not saved).
- Optional pace caret: a gray letter that replays your personal-best test for
  the same mode and options, key by key, with its fast parts and its pauses,
  so you can race it (off by default). A best from before v0.4.0 moves at the
  best speed until you set a new one.
- Challenges (`g` in the menu, or `-g NAME`): `survival` gives you 3 lives
  (`♥ ♥ ♥` above the words; a lost life takes its heart away). Each wrong
  word takes one, and the test ends with
  the last life. `boss` makes every 25th word a boss word: a long word in
  bold, with one of your slow letter pairs when it can. Each boss word you
  type right gives 50 bonus XP. `climb` is rounds of 25 words: reach the
  target WPM with 90% accuracy to pass, and the next target is 5 WPM
  higher. The first target is 80% of your recent average, and the pace caret
  moves at the target speed. A failed round ends the climb.
- Ghost races: press `x` on the results to save the test as a race file
  (in the `races` folder next to the history). Send it to a friend, who
  races its ghost with `dakken --race FILE`: the same words, with a pace
  caret that types like you did. The results say who won.
- Results: WPM, raw WPM, accuracy, consistency, and character counts, with a
  graph of WPM and raw WPM over the test, your slowest word and your fastest
  burst (the fastest single word).
- Personal bests and history for each mode and option set.
- Levels and badges: each test gives XP for its correct characters, with
  more for punctuation, numbers and strict modes, and less for errors. The
  menu shows your level, your rank (named after keycaps: `blank` from level
  1, `homing` from 5, `sculpted` from 10, `doubleshot` from 20, `artisan`
  from 35 and `legend` from 50) and an XP bar, and the logo gets longer
  speed lines at each rank. A new rank or a new badge fills the screen
  before the results (`Tab` goes on), and the results show the XP.
  `f` in the menu opens your profile: level, XP, test stats, best speeds,
  all badges, weak keys and key quests.
  `--badges` lists all 19 badges, each with a small ASCII symbol such as
  `100` for century; 4 stay hidden until you get them.
- One `.exe` with no runtime. Made and tested on Windows 11 with WezTerm.
