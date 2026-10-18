use rand::{Rng, RngExt, seq::IndexedRandom};

const ENGLISH: &str = include_str!("../../assets/words/english.txt");

/// The built-in English list: the 200 most common words.
pub fn english() -> Vec<&'static str> {
    ENGLISH
        .lines()
        .map(str::trim)
        .filter(|w| !w.is_empty())
        .collect()
}

/// Picks random words from a list and never gives the same word two times in a row.
pub struct WordGen<R> {
    list: Vec<&'static str>,
    rng: R,
    last: Option<&'static str>,
    punctuation: bool,
    /// The next word starts a sentence and gets a capital letter.
    sentence_start: bool,
}

impl<R: Rng> WordGen<R> {
    pub fn new(list: Vec<&'static str>, rng: R) -> Self {
        assert!(!list.is_empty(), "word list is empty");
        Self {
            list,
            rng,
            last: None,
            punctuation: false,
            sentence_start: true,
        }
    }

    /// Add capitals and punctuation, like monkeytype's punctuation mode.
    pub fn with_punctuation(mut self, on: bool) -> Self {
        self.punctuation = on;
        self
    }

    pub fn next_word(&mut self) -> String {
        let word = self.pick();
        if !self.punctuation {
            return word.to_string();
        }
        let mut word = if self.sentence_start {
            capitalize(word)
        } else {
            word.to_string()
        };
        self.sentence_start = false;
        match self.rng.random_range(0..100) {
            0..10 => self.end_sentence(&mut word, '.'),
            10..13 => self.end_sentence(&mut word, '?'),
            13..15 => self.end_sentence(&mut word, '!'),
            15..25 => word.push(','),
            25..27 => word.push(';'),
            27..29 => word.push(':'),
            29..32 => word = format!("\"{word}\""),
            32..34 => word = format!("({word})"),
            _ => {}
        }
        word
    }

    fn end_sentence(&mut self, word: &mut String, mark: char) {
        word.push(mark);
        self.sentence_start = true;
    }

    fn pick(&mut self) -> &'static str {
        loop {
            let word = *self.list.choose(&mut self.rng).expect("list is not empty");
            if self.list.len() == 1 || Some(word) != self.last {
                self.last = Some(word);
                return word;
            }
        }
    }
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};

    #[test]
    fn english_list_has_200_unique_words() {
        let list = english();
        assert_eq!(list.len(), 200);
        let mut sorted = list.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 200);
        assert!(
            list.iter()
                .all(|w| w.chars().all(|c| c.is_ascii_lowercase()))
        );
    }

    #[test]
    fn never_repeats_a_word_in_a_row() {
        let mut generator = WordGen::new(vec!["a", "b"], StdRng::seed_from_u64(1));
        let mut prev = generator.next_word();
        for _ in 0..100 {
            let next = generator.next_word();
            assert_ne!(prev, next);
            prev = next;
        }
    }

    fn punctuated(count: usize) -> Vec<String> {
        let mut generator =
            WordGen::new(english(), StdRng::seed_from_u64(7)).with_punctuation(true);
        (0..count).map(|_| generator.next_word()).collect()
    }

    #[test]
    fn punctuation_off_gives_plain_words() {
        let mut generator = WordGen::new(english(), StdRng::seed_from_u64(7));
        for _ in 0..200 {
            let word = generator.next_word();
            assert!(word.chars().all(|c| c.is_ascii_lowercase()), "{word}");
        }
    }

    #[test]
    fn punctuation_keeps_list_words_inside_the_marks() {
        let list = english();
        for word in punctuated(500) {
            let core = word
                .trim_matches(|c: char| !c.is_ascii_alphabetic())
                .to_lowercase();
            assert!(list.contains(&core.as_str()), "{word}");
        }
    }

    #[test]
    fn sentences_start_with_a_capital_letter() {
        let words = punctuated(500);
        assert!(words[0].starts_with(|c: char| c.is_ascii_uppercase()));
        for pair in words.windows(2) {
            let ends = pair[0].ends_with(['.', '?', '!']);
            let starts = pair[1]
                .trim_start_matches(['"', '('])
                .starts_with(|c: char| c.is_ascii_uppercase());
            assert_eq!(ends, starts, "{} {}", pair[0], pair[1]);
        }
    }

    #[test]
    fn punctuation_uses_every_mark() {
        let text = punctuated(1000).join(" ");
        for mark in ['.', '?', '!', ',', ';', ':', '"', '(', ')'] {
            assert!(text.contains(mark), "no {mark}");
        }
    }
}
