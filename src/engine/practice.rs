//! Made-up words for adaptive practice. A letter model learns which letter
//! follows the two letters before it in a word list. It then makes words
//! with only the unlocked letters, so the words look like the language but
//! use few keys.

use std::collections::BTreeMap;

use rand::{Rng, RngExt};

/// Marks the start and the end of a word in the model.
const START: char = '^';
const END: char = '$';

/// The shortest and the longest made-up word.
pub const MIN_LEN: usize = 3;
pub const MAX_LEN: usize = 8;

/// How many tries a word gets to contain the focus key.
const FOCUS_TRIES: usize = 30;

pub struct LetterModel {
    /// The letters after two letters, with counts. `START` stands in for the
    /// letters before the first letter.
    next: BTreeMap<(char, char), Vec<(char, u32)>>,
    /// The letters after one letter, for when two letters have no data.
    next1: BTreeMap<char, Vec<(char, u32)>>,
    /// The letters by how often they occur, the most common first.
    order: Vec<char>,
}

impl LetterModel {
    /// Learn from the words of a list. Capital letters count as small ones,
    /// and words with other characters than letters are skipped.
    pub fn new<'a>(words: impl IntoIterator<Item = &'a str>) -> Self {
        let mut next: BTreeMap<(char, char), BTreeMap<char, u32>> = BTreeMap::new();
        let mut next1: BTreeMap<char, BTreeMap<char, u32>> = BTreeMap::new();
        let mut counts: BTreeMap<char, u32> = BTreeMap::new();
        for word in words {
            let word = word.to_lowercase();
            if word.is_empty() || !word.chars().all(char::is_alphabetic) {
                continue;
            }
            let (mut a, mut b) = (START, START);
            for c in word.chars().chain([END]) {
                *next.entry((a, b)).or_default().entry(c).or_default() += 1;
                *next1.entry(b).or_default().entry(c).or_default() += 1;
                if c != END {
                    *counts.entry(c).or_default() += 1;
                }
                (a, b) = (b, c);
            }
        }
        let list = |m: BTreeMap<char, u32>| m.into_iter().collect::<Vec<_>>();
        let mut order: Vec<(char, u32)> = counts.into_iter().collect();
        order.sort_by(|x, y| y.1.cmp(&x.1).then(x.0.cmp(&y.0)));
        Self {
            next: next.into_iter().map(|(k, m)| (k, list(m))).collect(),
            next1: next1.into_iter().map(|(k, m)| (k, list(m))).collect(),
            order: order.into_iter().map(|(c, _)| c).collect(),
        }
    }

    /// The letters in the order they unlock: the most common first.
    pub fn order(&self) -> &[char] {
        &self.order
    }

    /// One made-up word with only `letters`, `MIN_LEN` to `MAX_LEN` long.
    pub fn word(&self, letters: &[char], rng: &mut impl Rng) -> String {
        let mut word = String::new();
        let (mut a, mut b) = (START, START);
        for len in 0..MAX_LEN {
            let allowed = |&&(c, _): &&(char, u32)| {
                if c == END {
                    len >= MIN_LEN
                } else {
                    letters.contains(&c)
                }
            };
            let options: Vec<(char, u32)> = [self.next.get(&(a, b)), self.next1.get(&b)]
                .into_iter()
                .flatten()
                .map(|o| o.iter().filter(allowed).copied().collect::<Vec<_>>())
                .find(|o| !o.is_empty())
                .unwrap_or_default();
            let c = match pick(&options, rng) {
                Some(END) => break,
                Some(c) => c,
                None if len >= MIN_LEN => break,
                None => letters[rng.random_range(0..letters.len())],
            };
            word.push(c);
            (a, b) = (b, c);
        }
        word
    }

    /// `count` made-up words with only `letters`. With a `focus` letter,
    /// each word tries a few times to contain it.
    pub fn words(
        &self,
        letters: &[char],
        focus: Option<char>,
        count: usize,
        rng: &mut impl Rng,
    ) -> Vec<String> {
        assert!(!letters.is_empty(), "no letters to practise");
        (0..count)
            .map(|_| {
                let mut word = self.word(letters, rng);
                if let Some(focus) = focus {
                    for _ in 1..FOCUS_TRIES {
                        if word.contains(focus) {
                            break;
                        }
                        word = self.word(letters, rng);
                    }
                }
                word
            })
            .collect()
    }
}

/// A letter from `options`, weighted by the counts.
fn pick(options: &[(char, u32)], rng: &mut impl Rng) -> Option<char> {
    let total: u32 = options.iter().map(|&(_, n)| n).sum();
    if total == 0 {
        return None;
    }
    let mut r = rng.random_range(0..total);
    for &(c, n) in options {
        if r < n {
            return Some(c);
        }
        r -= n;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::words::WordList;
    use rand::{SeedableRng, rngs::StdRng};

    fn english() -> LetterModel {
        LetterModel::new(WordList::English1k.words())
    }

    #[test]
    fn order_starts_with_the_common_letters() {
        let model = english();
        let order = model.order();
        assert_eq!(order.len(), 26, "{order:?}");
        assert_eq!(order[0], 'e');
        for rare in ['q', 'j', 'z', 'x'] {
            let i = order.iter().position(|&c| c == rare).unwrap();
            assert!(i >= 20, "{rare} is at {i}");
        }
    }

    #[test]
    fn words_use_only_the_unlocked_letters() {
        let model = english();
        let mut rng = StdRng::seed_from_u64(3);
        for n in [1, 2, 6, 10, 26] {
            let letters = &model.order()[..n];
            for word in model.words(letters, None, 200, &mut rng) {
                assert!(word.chars().all(|c| letters.contains(&c)), "{word}");
                let len = word.chars().count();
                assert!((MIN_LEN..=MAX_LEN).contains(&len), "{word}");
            }
        }
    }

    #[test]
    fn most_words_contain_the_focus_key() {
        let model = english();
        let letters = &model.order()[..6];
        let focus = letters[5];
        let mut rng = StdRng::seed_from_u64(5);
        let words = model.words(letters, Some(focus), 100, &mut rng);
        let with = words.iter().filter(|w| w.contains(focus)).count();
        assert!(with >= 90, "{with} of 100 have {focus}");
    }

    #[test]
    fn words_vary_and_look_like_the_language() {
        let model = english();
        let mut rng = StdRng::seed_from_u64(9);
        let words = model.words(model.order(), None, 100, &mut rng);
        let mut unique = words.clone();
        unique.sort();
        unique.dedup();
        assert!(unique.len() >= 60, "{words:?}");
        // English words often hold a vowel; made-up ones should too.
        let vowels = words
            .iter()
            .filter(|w| w.contains(['a', 'e', 'i', 'o', 'u', 'y']))
            .count();
        assert!(vowels >= 90, "{words:?}");
    }

    #[test]
    fn other_languages_keep_their_letters() {
        let model = LetterModel::new(WordList::German.words());
        assert!(model.order().contains(&'ß') && model.order().contains(&'ü'));
        assert!(model.order().iter().all(|c| !c.is_uppercase()));
        let model = LetterModel::new(WordList::Spanish.words());
        assert!(model.order().contains(&'ñ'));
    }

    #[test]
    fn code_words_with_symbols_are_skipped() {
        let model = LetterModel::new(["fn", "a+b", "let", "{}"]);
        assert_eq!(model.order(), ['e', 'f', 'l', 'n', 't']);
    }
}
