//! Challenges: game rules on top of a test. The menu changes them with `g`.

use crate::history::Record;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Challenge {
    #[default]
    Off,
    /// Start with `LIVES` lives. Each wrong word takes one, and the test
    /// ends with the last life.
    Survival,
    /// Every `BOSS_EVERY`th word is a hard word. Typing it right gives
    /// `BOSS_XP` bonus XP.
    Boss,
    /// Rounds of `CLIMB_WORDS` words. Pass a round at the target WPM with
    /// `CLIMB_ACCURACY` accuracy, and the next target is `CLIMB_STEP` higher.
    Climb,
}

/// The challenges in menu order.
pub const CHALLENGES: [Challenge; 4] = [
    Challenge::Off,
    Challenge::Survival,
    Challenge::Boss,
    Challenge::Climb,
];

pub const LIVES: u8 = 3;
pub const BOSS_EVERY: usize = 25;
pub const BOSS_XP: u64 = 50;
pub const CLIMB_WORDS: usize = 25;
pub const CLIMB_STEP: f64 = 5.0;
pub const CLIMB_ACCURACY: f64 = 90.0;

/// One climb: the round and its target WPM.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Climb {
    pub round: u32,
    pub target: f64,
}

impl Climb {
    /// The first round: 80% of the average of the last 10 tests, down to a
    /// multiple of 5, and at least 20 WPM. 30 WPM with no history.
    pub fn start(records: &[Record]) -> Self {
        let last = &records[records.len().saturating_sub(10)..];
        let target = if last.is_empty() {
            30.0
        } else {
            let avg = last.iter().map(|r| r.wpm).sum::<f64>() / last.len() as f64;
            ((avg * 0.8 / CLIMB_STEP).floor() * CLIMB_STEP).max(20.0)
        };
        Self { round: 1, target }
    }

    pub fn passed(self, wpm: f64, accuracy: f64) -> bool {
        wpm >= self.target && accuracy >= CLIMB_ACCURACY
    }

    pub fn next(self) -> Self {
        Self {
            round: self.round + 1,
            target: self.target + CLIMB_STEP,
        }
    }
}

/// Boss words have at least this many letters, if the list has such words.
const BOSS_LETTERS: usize = 7;

/// True if word `index` of a boss test is a boss word: the 25th, the 50th...
pub fn is_boss(index: usize) -> bool {
    index % BOSS_EVERY == BOSS_EVERY - 1
}

/// The words for boss words: the long words of the list. Words with one of
/// the `weak_pairs` come first, if there are some.
pub fn boss_words(list: &[String], weak_pairs: &[String]) -> Vec<String> {
    let longest = list.iter().map(|w| w.chars().count()).max().unwrap_or(0);
    let long: Vec<String> = list
        .iter()
        .filter(|w| w.chars().count() >= BOSS_LETTERS.min(longest))
        .cloned()
        .collect();
    let weak: Vec<String> = long
        .iter()
        .filter(|w| weak_pairs.iter().any(|p| w.contains(p.as_str())))
        .cloned()
        .collect();
    if weak.is_empty() { long } else { weak }
}

impl Challenge {
    pub fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Survival => "survival",
            Self::Boss => "boss",
            Self::Climb => "climb",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        CHALLENGES.into_iter().find(|c| c.name() == name)
    }

    /// The challenge after this one in `CHALLENGES`, for the menu.
    pub fn next(self) -> Self {
        let i = CHALLENGES.iter().position(|&c| c == self).unwrap_or(0);
        CHALLENGES[(i + 1) % CHALLENGES.len()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boss_words_are_long_and_prefer_weak_pairs() {
        let list: Vec<String> = ["a", "therapy", "keyboard", "another"]
            .map(String::from)
            .to_vec();
        assert_eq!(boss_words(&list, &[]), ["therapy", "keyboard", "another"]);
        let weak = ["th".to_string()];
        assert_eq!(boss_words(&list, &weak), ["therapy", "another"]);
        // A list of short words uses its longest words.
        let short: Vec<String> = ["ab", "abc"].map(String::from).to_vec();
        assert_eq!(boss_words(&short, &[]), ["abc"]);
        assert!(is_boss(24) && is_boss(49) && !is_boss(25) && !is_boss(0));
    }

    #[test]
    fn climb_starts_below_your_average_and_goes_up() {
        assert_eq!(Climb::start(&[]).target, 30.0);
        let record = |wpm| Record {
            time: 0,
            label: "time 30".to_string(),
            wpm,
            raw_wpm: wpm,
            accuracy: 100.0,
            consistency: 80.0,
            seconds: 30.0,
            bonus: 0,
        };
        // 80% of 72 is 57.6, so the first target is 55.
        let c = Climb::start(&[record(70.0), record(74.0)]);
        assert_eq!(
            c,
            Climb {
                round: 1,
                target: 55.0
            }
        );
        assert_eq!(Climb::start(&[record(10.0)]).target, 20.0);
        assert!(c.passed(55.0, 90.0));
        assert!(!c.passed(54.9, 100.0) && !c.passed(80.0, 89.0));
        assert_eq!(
            c.next(),
            Climb {
                round: 2,
                target: 60.0
            }
        );
    }

    #[test]
    fn names_parse_back_and_next_cycles() {
        for c in CHALLENGES {
            assert_eq!(Challenge::parse(c.name()), Some(c));
        }
        assert_eq!(Challenge::parse("nope"), None);
        assert_eq!(Challenge::Climb.next(), Challenge::Off);
        assert_eq!(Challenge::Off.next(), Challenge::Survival);
    }
}
