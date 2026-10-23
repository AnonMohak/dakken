# Data files and stats

## History

Each finished test adds one line to `history.tsv`: in `%APPDATA%\dakken` on
Windows, and in `$XDG_DATA_HOME/dakken` or `~/.local/share/dakken` on other
systems. A personal best counts only tests with the same mode and options,
for example `time 30 · punctuation`. Delete the file to clear the history.

`keys.tsv` keeps the presses, errors and time for each key and each letter
pair, for `--heatmap` and weak-keys and weak-pairs practice. `best-runs.tsv` keeps the
key times of each personal-best test, for the pace caret.
`badges.tsv` keeps each badge you got, with its date. `quests.tsv` keeps
the key quests. Saved races go in the `races` folder. XP and levels come
from the history, so they need no file of their own.
`mistakes.txt`, in the same folder, keeps the last 100 words you typed wrong.
If it is empty, mistakes practice uses the chosen word list.

## Practice

Practice has its own files, so it does not change your tests, stats or XP:

- `practice.tsv`: the unlocked letters of each language, the recent speed
  and errors of each key, and the lessons and time of all practice.
- `practice-keys.tsv`: the presses, errors and time of each key in
  practice, like `keys.tsv`.
- `practice-mistakes.txt`: the words you typed wrong in practice.

Weak-keys, weak-pairs and mistakes practice use your tests and your practice
together. Delete `practice.tsv` to start adaptive practice again in every
language.
`score-card.txt` keeps the last score card that you copied with `y`.

## How the stats work

- **WPM:** (letters in correct words + the spaces after them) ÷ 5 ÷ minutes.
- **Raw WPM:** all typed letters and spaces ÷ 5 ÷ minutes.
- **Accuracy:** correct key presses ÷ all key presses.
- **Consistency:** `100 × (1 − tanh(cv + cv³/3 + cv⁵/5))`,
  where `cv` is the standard deviation ÷ mean of the raw WPM in each second.
- **Characters:** correct / incorrect / extra / missed.
