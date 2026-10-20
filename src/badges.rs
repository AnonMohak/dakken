//! Badges for goals in the history. One badge on each line: its name, a
//! tab, then the time you got it in seconds since 1970-01-01 UTC. A badge
//! stays when the history changes later.

use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::history::{self, History, Record};
use crate::progress::{self, Level, options, words};

pub struct Badge {
    pub name: &'static str,
    pub goal: &'static str,
    /// A hidden badge shows "???" until you get it.
    pub hidden: bool,
}

const fn badge(name: &'static str, goal: &'static str, hidden: bool) -> Badge {
    Badge { name, goal, hidden }
}

pub const BADGES: [Badge; 15] = [
    badge("first press", "finish your first test", false),
    badge("regular", "finish 100 tests", false),
    badge("clean", "100% accuracy on 50 words or more", false),
    badge("century", "reach 100 wpm on 25 words or more", false),
    badge("steady", "85% consistency on 50 words or more", false),
    badge("iron", "pass 100 words in master mode", false),
    badge("polyglot", "finish a test in spanish and in german", false),
    badge(
        "compiler",
        "100% accuracy on 25 words of the code list",
        false,
    ),
    badge("switch hitter", "finish a test on dvorak or colemak", false),
    badge("marathon", "finish a 120 s time test", false),
    badge(
        "week of keys",
        "do the daily challenge 7 days in a row",
        false,
    ),
    badge("goal getter", "reach your weekly goal of words", false),
    badge(
        "nemesis",
        "beat your best for a mode by 10 wpm on 25 words or more",
        true,
    ),
    badge("exorcist", "empty the mistakes list by practising it", true),
    badge("zen master", "type 5 minutes in one zen test", true),
];

/// The badges the history earns, in `BADGES` order. `cleared_mistakes` is
/// true when the last test emptied the mistakes list. A `weekly_goal` of 0
/// is no goal.
pub fn earned(records: &[Record], cleared_mistakes: bool, weekly_goal: u32) -> Vec<&'static str> {
    let any = |f: &dyn Fn(&Record) -> bool| records.iter().any(f);
    let has = |r: &Record, option: &str| options(r).any(|o| o == option);
    BADGES
        .iter()
        .filter(|b| match b.name {
            "first press" => !records.is_empty(),
            "regular" => records.len() >= 100,
            "clean" => any(&|r| r.accuracy >= 100.0 && words(r) >= 50.0),
            "century" => any(&|r| r.wpm >= 100.0 && words(r) >= 25.0),
            "steady" => any(&|r| r.consistency >= 85.0 && words(r) >= 50.0),
            "iron" => any(&|r| has(r, "master") && words(r) >= 100.0),
            "polyglot" => any(&|r| has(r, "spanish")) && any(&|r| has(r, "german")),
            "compiler" => any(&|r| has(r, "code") && r.accuracy >= 100.0 && words(r) >= 25.0),
            "switch hitter" => any(&|r| has(r, "dvorak") || has(r, "colemak")),
            "marathon" => any(&|r| r.label.split(" · ").next() == Some("time 120")),
            "week of keys" => longest_daily_run(records) >= 7,
            "goal getter" => {
                weekly_goal > 0
                    && progress::week_words(records)
                        .values()
                        .any(|&w| w >= f64::from(weekly_goal))
            }
            "nemesis" => beat_best_by(records, 10.0),
            "exorcist" => cleared_mistakes,
            "zen master" => any(&|r| r.label == "zen" && r.seconds >= 300.0),
            _ => false,
        })
        .map(|b| b.name)
        .collect()
}

/// The most days in a row with a daily challenge.
fn longest_daily_run(records: &[Record]) -> usize {
    let mut days: Vec<i64> = records
        .iter()
        .filter(|r| r.label == "daily")
        .map(|r| (r.time / 86_400) as i64)
        .collect();
    days.sort_unstable();
    days.dedup();
    let (mut best, mut run) = (0, 0);
    for (i, day) in days.iter().enumerate() {
        run = if i > 0 && days[i - 1] + 1 == *day {
            run + 1
        } else {
            1
        };
        best = best.max(run);
    }
    best
}

/// True if a test beat the best before it, for the same label, by `margin`.
/// Tests shorter than 25 words do not count.
fn beat_best_by(records: &[Record], margin: f64) -> bool {
    let mut bests: HashMap<&str, f64> = HashMap::new();
    for r in records.iter().filter(|r| words(r) >= 25.0) {
        let best = bests.entry(&r.label).or_insert(r.wpm);
        if r.wpm >= *best + margin {
            return true;
        }
        *best = best.max(r.wpm);
    }
    false
}

pub struct Badges {
    /// `None` keeps the badges in memory only.
    path: Option<PathBuf>,
    got: Vec<(String, u64)>,
}

impl Badges {
    pub fn in_memory() -> Self {
        Self {
            path: None,
            got: Vec::new(),
        }
    }

    /// Read the badges. A missing file has none, and bad lines are skipped.
    pub fn load(path: PathBuf) -> Self {
        let got = fs::read_to_string(&path)
            .unwrap_or_default()
            .lines()
            .filter_map(|line| {
                let (name, time) = line.split_once('\t')?;
                Some((name.to_string(), time.parse().ok()?))
            })
            .collect();
        Self {
            path: Some(path),
            got,
        }
    }

    /// When you got the badge, if you have it.
    pub fn time(&self, name: &str) -> Option<u64> {
        self.got.iter().find(|(n, _)| n == name).map(|&(_, t)| t)
    }

    /// Keep the `earned` badges that are new, and add them to the file if
    /// there is one. Gives the new badges, also when the file write fails.
    pub fn add(&mut self, earned: &[&'static str]) -> (Vec<&'static str>, io::Result<()>) {
        let have: HashSet<&str> = self.got.iter().map(|(n, _)| n.as_str()).collect();
        let new: Vec<&'static str> = earned
            .iter()
            .copied()
            .filter(|n| !have.contains(n))
            .collect();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        self.got.extend(new.iter().map(|n| (n.to_string(), now)));
        let saved = match &self.path {
            Some(path) if !new.is_empty() => append(path, &new, now),
            _ => Ok(()),
        };
        (new, saved)
    }
}

fn append(path: &PathBuf, names: &[&str], time: u64) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    for name in names {
        writeln!(file, "{name}\t{time}")?;
    }
    Ok(())
}

pub fn default_path() -> Option<PathBuf> {
    Some(history::data_dir()?.join("badges.tsv"))
}

/// Teal (ANSI 256), like the heatmap, and gray.
const TEAL: u8 = 37;
const GRAY: u8 = 244;

/// The text for `dakken --badges`: the level and its XP, then each badge.
pub fn report(history: &History, badges: &Badges, color: bool) -> String {
    let paint = |code: u8, s: &str| {
        if color {
            format!("\x1b[38;5;{code}m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    };
    let xp = progress::total_xp(history.records());
    let level = Level::from_xp(xp);
    let (filled, empty) = level.bar(30);
    let mut out = format!("level {} · {}\n", level.level, level.rank());
    out += &format!(
        "  {}{}  {} / {} xp to level {}\n",
        paint(TEAL, &"━".repeat(filled)),
        paint(GRAY, &"─".repeat(empty)),
        level.xp,
        level.needed,
        level.level + 1
    );
    out += &format!("  {xp} xp in all\n");

    let got = BADGES
        .iter()
        .filter(|b| badges.time(b.name).is_some())
        .count();
    out += &format!("\nbadges: {got} of {}\n", BADGES.len());
    let width = BADGES.iter().map(|b| b.name.len()).max().unwrap_or(0);
    for b in &BADGES {
        let line = match badges.time(b.name) {
            Some(time) => paint(
                TEAL,
                &format!(
                    "■ {:<width$}  {}  ({})",
                    b.name,
                    b.goal,
                    &history::format_time(time)[..10]
                ),
            ),
            None if b.hidden => paint(GRAY, &format!("□ {:<width$}  hidden", "???")),
            None => paint(GRAY, &format!("□ {:<width$}  {}", b.name, b.goal)),
        };
        out += &format!("  {line}\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(label: &str, wpm: f64) -> Record {
        Record {
            time: 0,
            label: label.to_string(),
            wpm,
            raw_wpm: wpm,
            accuracy: 97.0,
            consistency: 70.0,
            seconds: 60.0,
            bonus: 0,
        }
    }

    #[test]
    fn badges_come_from_the_history() {
        assert!(earned(&[], false, 0).is_empty());
        let mut clean = record("words 50", 60.0);
        clean.accuracy = 100.0;
        let records = vec![
            clean,
            record("time 30 · master", 120.0),
            record("time 30 · spanish", 40.0),
        ];
        assert_eq!(
            earned(&records, false, 0),
            ["first press", "clean", "century", "iron"]
        );
        let mut more = records.clone();
        more.push(record("time 30 · punctuation · german · dvorak", 40.0));
        let names = earned(&more, true, 0);
        assert!(names.contains(&"polyglot"), "{names:?}");
        assert!(names.contains(&"switch hitter"), "{names:?}");
        assert!(names.contains(&"exorcist"), "{names:?}");
        assert!(!names.contains(&"nemesis"), "{names:?}");
    }

    #[test]
    fn nemesis_needs_a_best_beaten_by_10_wpm() {
        let a = [record("time 30", 50.0), record("time 30", 59.0)];
        assert!(!beat_best_by(&a, 10.0));
        let b = [record("time 30", 50.0), record("time 60", 70.0)];
        assert!(!beat_best_by(&b, 10.0));
        let c = [record("time 30", 50.0), record("time 30", 60.0)];
        assert!(beat_best_by(&c, 10.0));
    }

    #[test]
    fn daily_run_counts_days_in_a_row() {
        let daily = |day: u64| Record {
            time: day * 86_400 + 60,
            ..record("daily", 50.0)
        };
        let records: Vec<Record> = [1, 2, 3, 3, 5, 6, 7, 8, 9, 10, 11]
            .into_iter()
            .map(daily)
            .collect();
        assert_eq!(longest_daily_run(&records), 7);
        assert!(earned(&records, false, 300).contains(&"goal getter"));
        assert!(!earned(&records, false, 400).contains(&"goal getter"));
        assert!(earned(&records, false, 0).contains(&"week of keys"));
    }

    #[test]
    fn new_badges_are_saved_once() {
        let dir = std::env::temp_dir().join(format!("dakken-badges-{}", std::process::id()));
        let path = dir.join("badges.tsv");
        let _ = fs::remove_dir_all(&dir);
        let mut b = Badges::load(path.clone());
        let (new, saved) = b.add(&["clean", "century"]);
        assert_eq!(new, ["clean", "century"]);
        saved.unwrap();
        let (new, _) = b.add(&["clean", "iron"]);
        assert_eq!(new, ["iron"]);
        let loaded = Badges::load(path);
        assert!(loaded.time("clean").is_some() && loaded.time("iron").is_some());
        assert_eq!(loaded.time("steady"), None);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn report_shows_level_and_hides_hidden_badges() {
        let mut h = History::in_memory();
        h.add(record("time 60", 120.0)).unwrap();
        let mut b = Badges::in_memory();
        b.add(&earned(h.records(), false, 0)).1.unwrap();
        let out = report(&h, &b, false);
        assert!(out.starts_with("level 2 · blank\n"), "{out}");
        assert!(out.contains("badges: 2 of 15"), "{out}");
        assert!(out.contains("■ century "), "{out}");
        assert!(out.contains("□ clean "), "{out}");
        assert!(out.contains("□ ???"), "{out}");
        assert!(!out.contains("nemesis"), "{out}");
    }
}
