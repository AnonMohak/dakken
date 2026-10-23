//! Adaptive practice: which letters are unlocked in each language, the
//! recent speed of each key, and the focus key. A new letter unlocks when
//! every unlocked letter is fast enough.

// Practice mode uses this from the next steps on.
#![allow(dead_code)]

use std::collections::BTreeMap;

use crate::engine::stats::Press;
use crate::engine::words::WordList;

/// The letters a new practice starts with.
pub const START_LETTERS: usize = 6;

/// The speed in WPM that each letter needs before the next letter unlocks.
pub const DEFAULT_TARGET: u32 = 35;

/// A letter with more errors than this is not ready, at any speed.
pub const MAX_ERROR_RATE: f64 = 0.1;

/// Fewer timed presses than this are not enough to judge a key.
pub const MIN_PRESSES: u64 = 10;

/// How much one lesson moves the recent speed of a key.
const SMOOTHING: f64 = 0.3;

/// The language of adaptive practice for a word list. The lists that are not
/// a language (code) use English.
pub fn language(list: WordList) -> WordList {
    match list {
        WordList::Spanish | WordList::German | WordList::French | WordList::Portuguese => list,
        _ => WordList::English,
    }
}

/// The words the letter model of a language learns from. English uses the
/// 1000-word list, which has more letter patterns.
pub fn model_words(language: WordList) -> Vec<&'static str> {
    match language {
        WordList::English => WordList::English1k.words(),
        _ => language.words(),
    }
}

/// The recent speed and errors of one key, from the last lessons.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct KeySpeed {
    /// Time from the press before, in milliseconds.
    pub ms: f64,
    pub error_rate: f64,
    /// All timed presses so far.
    pub presses: u64,
}

impl KeySpeed {
    /// Words per minute at this speed, with 5 characters in a word.
    pub fn wpm(&self) -> f64 {
        if self.ms <= 0.0 {
            return 0.0;
        }
        12_000.0 / self.ms
    }

    pub fn known(&self) -> bool {
        self.presses >= MIN_PRESSES
    }

    /// Fast and correct enough for the next letter to unlock.
    pub fn ready(&self, target: u32) -> bool {
        self.known() && self.wpm() >= f64::from(target) && self.error_rate <= MAX_ERROR_RATE
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Progress {
    /// The number of unlocked letters, by language name.
    pub unlocked: BTreeMap<String, usize>,
    /// The recent speed of each key, for all languages.
    pub speeds: BTreeMap<char, KeySpeed>,
}

impl Progress {
    /// The unlocked letters of a language, from its unlock `order`.
    pub fn letters<'a>(&self, language: WordList, order: &'a [char]) -> &'a [char] {
        let n = self.unlocked.get(language.name()).copied();
        &order[..n.unwrap_or(START_LETTERS).clamp(1, order.len())]
    }

    pub fn speed(&self, key: char) -> KeySpeed {
        self.speeds.get(&key).copied().unwrap_or_default()
    }

    /// The letter to practise most: the newest letter with too few presses,
    /// or else the slowest letter that is not ready. `None` when all are
    /// ready.
    pub fn focus(&self, letters: &[char], target: u32) -> Option<char> {
        if let Some(&new) = letters.iter().rev().find(|&&c| !self.speed(c).known()) {
            return Some(new);
        }
        letters
            .iter()
            .copied()
            .filter(|&c| !self.speed(c).ready(target))
            .min_by(|&a, &b| self.speed(a).wpm().total_cmp(&self.speed(b).wpm()))
    }

    /// Add the presses of one lesson. If every unlocked letter is then
    /// ready, the next letter of `order` unlocks and is returned.
    pub fn add_lesson(
        &mut self,
        language: WordList,
        order: &[char],
        target: u32,
        presses: &[Press],
    ) -> Option<char> {
        let mut lesson: BTreeMap<char, (u64, u64, u64)> = BTreeMap::new();
        for pair in presses.windows(2) {
            let Some(key) = pair[1].key else { continue };
            if !key.is_alphabetic() {
                continue;
            }
            let ms = pair[1].at.saturating_sub(pair[0].at).as_millis() as u64;
            let (n, errors, total) = lesson.entry(key).or_default();
            *n += 1;
            *errors += u64::from(!pair[1].correct);
            *total += ms;
        }
        for (key, (n, errors, total)) in lesson {
            let ms = total as f64 / n as f64;
            let error_rate = errors as f64 / n as f64;
            let s = self.speeds.entry(key).or_default();
            if s.presses == 0 {
                (s.ms, s.error_rate) = (ms, error_rate);
            } else {
                s.ms += SMOOTHING * (ms - s.ms);
                s.error_rate += SMOOTHING * (error_rate - s.error_rate);
            }
            s.presses += n;
        }
        let count = self.letters(language, order).len();
        if count >= order.len() || self.focus(&order[..count], target).is_some() {
            return None;
        }
        self.unlocked.insert(language.name().to_string(), count + 1);
        Some(order[count])
    }

    /// Start a language again from the first letters. The key speeds stay.
    pub fn reset(&mut self, language: WordList) {
        self.unlocked.remove(language.name());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const ORDER: [char; 8] = ['e', 'r', 't', 'a', 'o', 'i', 'n', 's'];

    /// A lesson that types `text` with `ms` between presses; the letters in
    /// `wrong` are typed wrong.
    fn lesson(text: &str, ms: u64, wrong: &[char]) -> Vec<Press> {
        text.chars()
            .enumerate()
            .map(|(i, c)| Press {
                at: Duration::from_millis(i as u64 * ms),
                correct: !wrong.contains(&c),
                key: Some(c),
                word: 0,
            })
            .collect()
    }

    fn practised(text: &str, ms: u64) -> Progress {
        let mut p = Progress::default();
        p.add_lesson(WordList::English, &ORDER, 35, &lesson(text, ms, &[]));
        p
    }

    #[test]
    fn languages_map_to_practice_languages() {
        assert_eq!(language(WordList::English1k), WordList::English);
        assert_eq!(language(WordList::Code), WordList::English);
        assert_eq!(language(WordList::German), WordList::German);
        assert_eq!(model_words(WordList::English).len(), 1000);
    }

    #[test]
    fn a_new_practice_starts_with_six_letters() {
        let p = Progress::default();
        assert_eq!(p.letters(WordList::English, &ORDER), &ORDER[..6]);
        assert_eq!(p.letters(WordList::English, &ORDER[..3]), &ORDER[..3]);
    }

    #[test]
    fn wpm_comes_from_the_time_between_presses() {
        // 12000 / 200 ms = 60 WPM.
        let p = practised(&"e".repeat(20), 200);
        let e = p.speed('e');
        assert_eq!((e.presses, e.wpm()), (19, 60.0));
        assert!(e.ready(35) && !e.ready(70));
    }

    #[test]
    fn errors_stop_a_fast_key_from_being_ready() {
        let mut p = Progress::default();
        let text = "er".repeat(20);
        p.add_lesson(WordList::English, &ORDER, 35, &lesson(&text, 100, &['r']));
        assert!(p.speed('e').ready(35));
        assert!(!p.speed('r').ready(35));
    }

    #[test]
    fn focus_is_the_newest_unknown_letter_then_the_slowest() {
        let letters = &ORDER[..6];
        let p = Progress::default();
        assert_eq!(p.focus(letters, 35), Some('i'));
        // All known: e and r are fast, the rest slow, t the slowest.
        let mut p = practised(&"er".repeat(20), 100);
        p.add_lesson(
            WordList::English,
            &ORDER,
            35,
            &lesson(&"aoi".repeat(12), 500, &[]),
        );
        p.add_lesson(
            WordList::English,
            &ORDER,
            35,
            &lesson(&"t".repeat(15), 900, &[]),
        );
        assert_eq!(p.focus(letters, 35), Some('t'));
    }

    #[test]
    fn the_next_letter_unlocks_when_all_are_ready() {
        let mut p = Progress::default();
        let slow = lesson(&"eaoirt".repeat(10), 1000, &[]);
        assert_eq!(p.add_lesson(WordList::English, &ORDER, 35, &slow), None);
        // The recent speed moves part of the way each lesson, so fast
        // lessons unlock 'n' after a few lessons.
        let fast = lesson(&"eaoirt".repeat(10), 100, &[]);
        let unlocked = (0..10).find_map(|_| p.add_lesson(WordList::English, &ORDER, 35, &fast));
        assert_eq!(unlocked, Some('n'));
        assert_eq!(p.letters(WordList::English, &ORDER).len(), 7);
        // The new letter has no presses, so it is the focus and nothing
        // more unlocks.
        assert_eq!(p.focus(&ORDER[..7], 35), Some('n'));
        assert_eq!(p.add_lesson(WordList::English, &ORDER, 35, &fast), None);
        // Each language has its own letters.
        assert_eq!(p.letters(WordList::German, &ORDER).len(), 6);
        p.reset(WordList::English);
        assert_eq!(p.letters(WordList::English, &ORDER).len(), 6);
        assert!(p.speed('n').presses == 0 && p.speed('e').known());
    }

    #[test]
    fn nothing_unlocks_after_the_last_letter() {
        let mut p = Progress::default();
        let fast = lesson(&"er".repeat(20), 100, &[]);
        let order = ['e', 'r'];
        assert_eq!(p.add_lesson(WordList::English, &order, 35, &fast), None);
        assert_eq!(p.letters(WordList::English, &order), &order);
    }
}
