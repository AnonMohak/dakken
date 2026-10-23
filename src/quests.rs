//! Key quests and the skill map. A quest asks for one slow letter to be
//! faster: an average under a target time in one test, with no errors. One
//! quest on each line: the key, the target in milliseconds, the start time,
//! and the time it was done (0 while it is open), separated by tabs.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::PathBuf;

use crate::engine::stats::Press;
use crate::keystats::{KeyStat, KeyStats, MIN_PRESSES, ROWS};

/// Open quests at one time.
pub const OPEN_QUESTS: usize = 3;
/// A quest key needs this many presses in one test to count.
const QUEST_PRESSES: u64 = 3;
/// The target is this part of the key's average time now.
const QUEST_STEP: f64 = 0.85;
/// No target is lower than this.
const MIN_TARGET_MS: u32 = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quest {
    pub key: char,
    pub target_ms: u32,
    pub started: u64,
    pub done: Option<u64>,
}

impl Quest {
    /// "q under 180 ms".
    pub fn goal(&self) -> String {
        format!("{} under {} ms", self.key, self.target_ms)
    }
}

/// The skill level of a key, 1 to 10, from its speed and errors. `None`
/// with too few presses. 100 ms is level 10 for speed and 600 ms is 0, and
/// each 1% of errors takes 4% off. The lowest level is 1.
pub fn level(s: &KeyStat) -> Option<u8> {
    if s.presses < MIN_PRESSES {
        return None;
    }
    let speed = ((600.0 - s.avg_ms()?) / 50.0).clamp(0.0, 10.0);
    let accuracy = (1.0 - s.error_rate() * 4.0).clamp(0.0, 1.0);
    Some((speed * accuracy).round().clamp(1.0, 10.0) as u8)
}

pub struct Quests {
    /// `None` keeps the quests in memory only.
    path: Option<PathBuf>,
    quests: Vec<Quest>,
}

impl Quests {
    pub fn in_memory() -> Self {
        Self {
            path: None,
            quests: Vec::new(),
        }
    }

    /// Read the quests. A missing file has none, and bad lines are skipped.
    pub fn load(path: PathBuf) -> Self {
        let quests = fs::read_to_string(&path)
            .unwrap_or_default()
            .lines()
            .filter_map(parse_line)
            .collect();
        Self {
            path: Some(path),
            quests,
        }
    }

    pub fn open(&self) -> impl Iterator<Item = &Quest> {
        self.quests.iter().filter(|q| q.done.is_none())
    }

    /// The keys with a quest done.
    pub fn done_keys(&self) -> BTreeSet<char> {
        (self.quests.iter())
            .filter(|q| q.done.is_some())
            .map(|q| q.key)
            .collect()
    }

    pub fn done_count(&self) -> usize {
        self.quests.iter().filter(|q| q.done.is_some()).count()
    }

    /// Give the weakest letters without an open quest a new quest, up to
    /// `OPEN_QUESTS`. True if a quest was added.
    fn refill(&mut self, stats: &KeyStats, now: u64) -> bool {
        let mut added = false;
        for key in stats.weakest_letters(26) {
            if self.open().count() >= OPEN_QUESTS {
                break;
            }
            if self.open().any(|q| q.key == key) {
                continue;
            }
            let Some(ms) = stats.get(key).avg_ms() else {
                continue;
            };
            let target = ((ms * QUEST_STEP / 10.0).floor() * 10.0) as u32;
            self.quests.push(Quest {
                key,
                target_ms: target.max(MIN_TARGET_MS),
                started: now,
                done: None,
            });
            added = true;
        }
        added
    }

    /// Finish the open quests the `presses` of one test meet, then add new
    /// quests from `stats`. Write the file if something changed. Gives the
    /// quests done in this test.
    pub fn update(
        &mut self,
        presses: &[Press],
        stats: &KeyStats,
        now: u64,
    ) -> (Vec<Quest>, io::Result<()>) {
        let mut test = KeyStats::in_memory();
        let _ = test.add_test(presses);
        let mut done = Vec::new();
        for q in self.quests.iter_mut().filter(|q| q.done.is_none()) {
            let s = test.get(q.key);
            let fast = s.avg_ms().is_some_and(|ms| ms <= f64::from(q.target_ms));
            if s.timed >= QUEST_PRESSES && s.errors == 0 && fast {
                q.done = Some(now);
                done.push(*q);
            }
        }
        let added = self.refill(stats, now);
        let saved = if added || !done.is_empty() {
            self.save()
        } else {
            Ok(())
        };
        (done, saved)
    }

    /// Add quests if there are fewer than `OPEN_QUESTS`, for `--skills`.
    pub fn fill(&mut self, stats: &KeyStats, now: u64) -> io::Result<()> {
        if self.refill(stats, now) {
            return self.save();
        }
        Ok(())
    }

    fn save(&self) -> io::Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let text: String = (self.quests.iter())
            .map(|q| {
                format!(
                    "{}\t{}\t{}\t{}\n",
                    q.key,
                    q.target_ms,
                    q.started,
                    q.done.unwrap_or(0)
                )
            })
            .collect();
        fs::write(path, text)
    }
}

fn parse_line(line: &str) -> Option<Quest> {
    let mut f = line.split('\t');
    let mut key = f.next()?.chars();
    let (Some(k), None) = (key.next(), key.next()) else {
        return None;
    };
    let target_ms = f.next()?.parse().ok()?;
    let started = f.next()?.parse().ok()?;
    let done: u64 = f.next()?.parse().ok()?;
    Some(Quest {
        key: k,
        target_ms,
        started,
        done: (done > 0).then_some(done),
    })
}

pub fn default_path() -> Option<PathBuf> {
    Some(crate::history::data_dir()?.join("quests.tsv"))
}

/// The text for `dakken --skills`: a keyboard with the level of each key,
/// teal where a quest is done, then the open quests.
pub fn skill_map(stats: &KeyStats, quests: &Quests, color: bool) -> String {
    let done = quests.done_keys();
    let mut out = String::from("skill map: the level of each key, 1 to 10\n\n");
    let mut levels = Vec::new();
    for (indent, row) in ROWS.iter().enumerate() {
        out += &" ".repeat(2 + indent * 3);
        for key in row.chars() {
            let level = level(&stats.get(key));
            levels.extend(level);
            let cell = match level {
                Some(l) => format!(" {key}{l:>3} "),
                None => format!(" {key}  · "),
            };
            out += &match (color, done.contains(&key), level.is_some()) {
                // Teal (ANSI 256), like the badges.
                (true, true, _) => format!("\x1b[30;48;5;43m{cell}\x1b[0m"),
                (true, false, false) => format!("\x1b[38;5;244m{cell}\x1b[0m"),
                _ => cell,
            };
            out += " ";
        }
        out += "\n";
    }
    if levels.is_empty() {
        out += "\nNo key has enough presses yet. Do some tests first.\n";
        return out;
    }
    let avg = levels.iter().map(|&l| f64::from(l)).sum::<f64>() / levels.len() as f64;
    let done = if color {
        "\x1b[30;48;5;43m   \x1b[0m quest done"
    } else {
        "teal: quest done"
    };
    out += &format!("\n  average level {avg:.1} · {done} · ·: not enough data\n");
    out += &format!("\nkey quests ({} done)\n", quests.done_count());
    let open: Vec<&Quest> = quests.open().collect();
    if open.is_empty() {
        out += "  No open quests.\n";
    }
    for q in open {
        let now = stats
            .get(q.key)
            .avg_ms()
            .map_or(String::new(), |ms| format!(" (now {ms:.0} ms)"));
        out += &format!(
            "  {}{now}: in one test, at least {QUEST_PRESSES} times with no errors\n",
            q.goal()
        );
    }
    out += "  Practise them with -l weak-keys.\n";
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn stat(presses: u64, errors: u64, avg_ms: u64) -> KeyStat {
        KeyStat {
            presses,
            errors,
            total_ms: avg_ms * presses,
            timed: presses,
        }
    }

    /// Presses of `keys`, each `ms` after the one before.
    fn presses(keys: &str, ms: u64, wrong: Option<char>) -> Vec<Press> {
        keys.chars()
            .enumerate()
            .map(|(i, k)| Press {
                at: Duration::from_millis(i as u64 * ms),
                correct: Some(k) != wrong,
                key: Some(k),
                word: 0,
            })
            .collect()
    }

    #[test]
    fn levels_come_from_speed_and_errors() {
        assert_eq!(level(&stat(2, 0, 100)), None);
        assert_eq!(level(&stat(10, 0, 100)), Some(10));
        assert_eq!(level(&stat(10, 0, 350)), Some(5));
        assert_eq!(level(&stat(10, 0, 900)), Some(1));
        // 5% errors take 20% off.
        assert_eq!(level(&stat(20, 1, 100)), Some(8));
    }

    #[test]
    fn quests_go_to_weak_letters_and_finish_on_a_fast_clean_test() {
        let mut stats = KeyStats::in_memory();
        // q is slow (300 ms), the others fast.
        stats.add_test(&presses("aqaqaqaqaqaq", 150, None)).unwrap();
        stats.add_test(&presses("qqqqqqq", 300, None)).unwrap();
        let mut quests = Quests::in_memory();
        quests.fill(&stats, 1).unwrap();
        let open: Vec<Quest> = quests.open().copied().collect();
        assert_eq!(open.len(), 2, "only a and q have enough presses");
        let q = *open.iter().find(|q| q.key == 'q').unwrap();
        assert!(q.target_ms < 250, "{q:?}");
        // Slow or wrong presses do not finish it.
        let (done, _) = quests.update(&presses("qqqq", 400, None), &stats, 2);
        assert!(done.is_empty());
        let (done, _) = quests.update(&presses("qqqq", 50, Some('q')), &stats, 2);
        assert!(done.is_empty());
        let (done, _) = quests.update(&presses("qqqq", 50, None), &stats, 3);
        assert_eq!(done.len(), 1);
        assert_eq!(done[0].key, 'q');
        assert!(quests.done_keys().contains(&'q'));
        // q is still the weakest letter, so it gets a new quest.
        assert!(quests.open().any(|q| q.key == 'q'));
    }

    #[test]
    fn quests_are_saved_and_loaded() {
        let dir = std::env::temp_dir().join(format!("dakken-quests-{}", std::process::id()));
        let path = dir.join("quests.tsv");
        let _ = fs::remove_dir_all(&dir);
        let mut stats = KeyStats::in_memory();
        stats.add_test(&presses("zzzzzzz", 300, None)).unwrap();
        let mut quests = Quests::load(path.clone());
        quests.fill(&stats, 5).unwrap();
        quests
            .update(&presses("zzzz", 50, None), &stats, 9)
            .1
            .unwrap();
        let loaded = Quests::load(path);
        assert_eq!(loaded.quests, quests.quests);
        assert_eq!(loaded.done_count(), 1);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn skill_map_shows_levels_and_quests() {
        let mut stats = KeyStats::in_memory();
        let quests = Quests::in_memory();
        assert!(skill_map(&stats, &quests, false).contains("No key has enough"));
        stats.add_test(&presses("aaaaaaa", 100, None)).unwrap();
        let out = skill_map(&stats, &quests, false);
        assert!(out.contains(" a 10 "), "{out}");
        assert!(out.contains(" s  · "), "{out}");
        assert!(out.contains("average level 10.0"), "{out}");
    }
}
