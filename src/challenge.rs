//! Challenges: game rules on top of a test. The menu changes them with `g`.

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
}

/// The challenges in menu order.
pub const CHALLENGES: [Challenge; 3] = [Challenge::Off, Challenge::Survival, Challenge::Boss];

pub const LIVES: u8 = 3;
pub const BOSS_EVERY: usize = 25;
pub const BOSS_XP: u64 = 50;
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
    fn names_parse_back_and_next_cycles() {
        for c in CHALLENGES {
            assert_eq!(Challenge::parse(c.name()), Some(c));
        }
        assert_eq!(Challenge::parse("nope"), None);
        assert_eq!(Challenge::Boss.next(), Challenge::Off);
        assert_eq!(Challenge::Off.next(), Challenge::Survival);
    }
}
