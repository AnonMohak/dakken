# dakken: ideas to gamify the test

dakken already has a daily challenge, streaks, personal bests and a pace
caret. These ideas build on them. Rewards use the theme accent (teal by
default), never green or yellow. Nothing interrupts the typing screen.

## 1. Progress and levels (done on the `gamify` branch)

- **XP and levels.** Each test gives XP for its correct characters. Accuracy
  and harder modes give more XP. The menu shows the level under the logo.
- **Ranks with keycap names.** `blank` → `homing` → `sculpted` →
  `doubleshot` → `artisan` → `legend`.
- **A keycap logo that grows.** The menu logo gets longer speed lines as the
  rank goes up.

## 2. Achievements (done on the `gamify` branch)

- A short line on the results screen shows each new badge.
- `dakken --badges` lists all badges. Locked badges are gray, and hidden
  badges show `???` until you get them.
- Example badges: `clean` (100% accuracy on 50 words or more), `century`
  (100 WPM), `iron` (pass 100 words in `master` mode), `polyglot` (a test in
  each language), `nemesis` (beat your best by 10 WPM), `exorcist` (make the
  `mistakes` list empty).
- Not done: `night owl` (a test between 00:00 and 04:00). It needs the local
  time zone, and the history keeps UTC only.

## 3. Streaks and daily play (done on the `gamify` branch)

- **Streak freeze.** One freeze for each 7 days of a streak, up to 2. A
  freeze keeps the streak if you miss one day. `--stats` shows them.
- **Daily score card.** After `--daily`, `y` copies a short card to paste
  in chats: WPM, accuracy, the streak, and a 🟦 for each correct word (⬛ for
  a wrong word). The copy uses the OSC 52 escape code.
- **Weekly goal.** `weekly_goal = 5000` in `config.toml`. The menu and
  `--stats` show a bar, the results say when you reach it, and the
  `goal getter` badge rewards it.

## 4. Challenge modes

- **Ghost race.** Race a replay file from a friend (`--race friend.replay`).
- **Boss words.** Every 25 words, one hard word (long, or from your
  `weak-pairs`). Type it clean for bonus XP.
- **Survival.** Start with 3 lives. Each wrong word takes one life.
- **Climb.** The target WPM goes up by 5 for each round that you pass.

## 5. Skill trees from your own data

- The key heatmap as a skill map: each key has a level from its accuracy and
  speed.
- **Key quests.** For example "make `q` faster than 150 ms". A finished
  quest makes the key teal on the heatmap.
- This makes the `weak-keys` practice a goal, not only a drill.

## 6. Social (needs a server, so last)

- A leaderboard for the daily challenge only: everyone types the same words.
- Upload the replay with the score, so the server can check the score.

## Order

Groups 1 to 3 are done. Next: group 4 (challenge modes), then group 5 (skill
trees). Group 6 needs a server, so it is last.
