//! XP, levels and ranks, all from the history. Nothing extra is saved, so
//! tests from before this feature count too.

use crate::history::Record;

/// Each rank with the level that gives it, lowest first. The names come from
/// keycaps.
pub const RANKS: [(u32, &str); 6] = [
    (1, "blank"),
    (5, "homing"),
    (10, "sculpted"),
    (20, "doubleshot"),
    (35, "artisan"),
    (50, "legend"),
];

/// XP from level n to level n + 1 is n times this.
const XP_STEP: u64 = 500;

/// The words of a test, with one word = 5 correct characters.
pub fn words(r: &Record) -> f64 {
    r.wpm * r.seconds / 60.0
}

/// The parts of a label after the mode, for example ["punctuation", "master"].
pub fn options(r: &Record) -> impl Iterator<Item = &str> {
    r.label.split(" · ").skip(1)
}

/// XP for one test: its correct characters, more for harder options, less
/// for errors. Zen has no target text to check, so it gives half.
pub fn test_xp(r: &Record) -> u64 {
    let chars = words(r) * 5.0;
    let mut factor: f64 = 1.0;
    for option in options(r) {
        factor += match option {
            "punctuation" | "numbers" | "stop-on-error" | "no-backspace" => 0.25,
            "master" => 0.5,
            _ => 0.0,
        };
    }
    if r.label == "zen" {
        factor = 0.5;
    }
    // Accuracy counts twice: 90% accuracy gives 81% of the XP.
    let accuracy = (r.accuracy / 100.0).clamp(0.0, 1.0);
    (chars * factor * accuracy * accuracy).round().max(0.0) as u64
}

pub fn total_xp(records: &[Record]) -> u64 {
    records.iter().map(test_xp).sum()
}

/// The week of a day (days since 1970-01-01 UTC). Weeks start on Monday;
/// 1970-01-01 was a Thursday.
pub fn week(day: i64) -> i64 {
    (day + 3).div_euclid(7)
}

/// The words typed in each week, by week.
pub fn week_words(records: &[Record]) -> std::collections::HashMap<i64, f64> {
    let mut weeks = std::collections::HashMap::new();
    for r in records {
        *weeks.entry(week((r.time / 86_400) as i64)).or_insert(0.0) += words(r);
    }
    weeks
}

/// The words typed in the week of `today`.
pub fn this_week(records: &[Record], today: i64) -> f64 {
    week_words(records)
        .get(&week(today))
        .copied()
        .unwrap_or(0.0)
}

/// A bar `width` cells long for `value` of `goal`: filled and empty cells.
pub fn goal_bar(value: f64, goal: u32, width: usize) -> (usize, usize) {
    let filled = if goal == 0 {
        0
    } else {
        ((value / f64::from(goal) * width as f64) as usize).min(width)
    };
    (filled, width - filled)
}

/// The weekly goal line for `--stats`, or nothing when there is no goal.
pub fn weekly_report(records: &[Record], goal: u32, today: i64) -> String {
    if goal == 0 {
        return String::new();
    }
    let words = this_week(records, today);
    let (filled, empty) = goal_bar(words, goal, 30);
    let done = if words >= f64::from(goal) {
        "  goal reached"
    } else {
        ""
    };
    format!(
        "
weekly goal (UTC, from Monday)
  {}{}  {words:.0} of {goal} words{done}
",
        "━".repeat(filled),
        "─".repeat(empty)
    )
}

/// A level and the XP in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Level {
    pub level: u32,
    /// XP since the start of this level.
    pub xp: u64,
    /// XP from the start of this level to the next level.
    pub needed: u64,
}

impl Level {
    pub fn from_xp(mut xp: u64) -> Self {
        let mut level = 1;
        while xp >= u64::from(level) * XP_STEP {
            xp -= u64::from(level) * XP_STEP;
            level += 1;
        }
        Self {
            level,
            xp,
            needed: u64::from(level) * XP_STEP,
        }
    }

    /// 0 for the first rank, up to 5 for the last.
    pub fn rank_index(self) -> usize {
        RANKS
            .iter()
            .rposition(|&(from, _)| self.level >= from)
            .unwrap_or(0)
    }

    pub fn rank(self) -> &'static str {
        RANKS[self.rank_index()].1
    }

    /// A bar `width` cells long: filled cells for the XP in this level.
    pub fn bar(self, width: usize) -> (usize, usize) {
        let filled = (self.xp * width as u64 / self.needed.max(1)) as usize;
        (filled.min(width), width - filled.min(width))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(label: &str, wpm: f64, accuracy: f64) -> Record {
        Record {
            time: 0,
            label: label.to_string(),
            wpm,
            raw_wpm: wpm,
            accuracy,
            consistency: 80.0,
            seconds: 60.0,
        }
    }

    #[test]
    fn xp_counts_characters_options_and_accuracy() {
        // 60 wpm for 60 s is 300 characters.
        assert_eq!(test_xp(&record("time 60", 60.0, 100.0)), 300);
        assert_eq!(test_xp(&record("time 60", 60.0, 90.0)), 243);
        assert_eq!(
            test_xp(&record("time 60 · punctuation · master", 60.0, 100.0)),
            525
        );
        assert_eq!(test_xp(&record("zen", 60.0, 100.0)), 150);
        // A file name is not an option.
        assert_eq!(test_xp(&record("custom · notes.txt", 60.0, 100.0)), 300);
    }

    #[test]
    fn levels_need_more_xp_each_time() {
        assert_eq!(
            Level::from_xp(0),
            Level {
                level: 1,
                xp: 0,
                needed: 500
            }
        );
        assert_eq!(Level::from_xp(499).level, 1);
        assert_eq!(Level::from_xp(500).level, 2);
        // 500 + 1000 to reach level 3.
        assert_eq!(
            Level::from_xp(1_600),
            Level {
                level: 3,
                xp: 100,
                needed: 1_500
            }
        );
    }

    #[test]
    fn ranks_follow_the_level() {
        let at = |level: u32| Level {
            level,
            xp: 0,
            needed: 1,
        };
        assert_eq!(at(1).rank(), "blank");
        assert_eq!(at(4).rank(), "blank");
        assert_eq!(at(5).rank(), "homing");
        assert_eq!(at(49).rank(), "artisan");
        assert_eq!(at(80).rank(), "legend");
        assert_eq!(at(80).rank_index(), 5);
    }

    #[test]
    fn weeks_start_on_monday() {
        // Day 20000 (2024-10-04) is a Friday, and 19996 is the Monday.
        assert_eq!(week(19_996), week(20_002));
        assert_ne!(week(19_995), week(19_996));
        let mut r = record("time 60", 60.0, 100.0);
        r.time = 19_996 * 86_400;
        let mut old = r.clone();
        old.time -= 86_400;
        let records = [r.clone(), r, old];
        assert_eq!(this_week(&records, 20_000), 120.0);
        let out = weekly_report(&records, 240, 20_000);
        assert!(out.contains("120 of 240 words"), "{out}");
        assert!(!out.contains("reached"), "{out}");
        assert!(weekly_report(&records, 100, 20_000).contains("goal reached"));
        assert_eq!(weekly_report(&records, 0, 20_000), "");
        assert_eq!(goal_bar(50.0, 100, 10), (5, 5));
        assert_eq!(goal_bar(500.0, 100, 10), (10, 0));
    }

    #[test]
    fn bar_fills_with_the_xp() {
        let l = Level {
            level: 2,
            xp: 500,
            needed: 1_000,
        };
        assert_eq!(l.bar(10), (5, 5));
        assert_eq!(Level::from_xp(0).bar(10), (0, 10));
    }
}
