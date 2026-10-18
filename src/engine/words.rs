use rand::{Rng, RngExt, seq::IndexedRandom};

const ENGLISH: &str = include_str!("../../assets/words/english.txt");
const ENGLISH_1K: &str = include_str!("../../assets/words/english_1k.txt");
const CODE: &str = include_str!("../../assets/words/code.txt");
const QUOTES: &str = include_str!("../../assets/quotes/english.txt");

/// A quote to type, and where it comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quote {
    pub text: &'static str,
    pub source: &'static str,
}

/// The built-in quotes, all from public-domain texts. One quote on each line:
/// the text, a tab, then the source.
pub fn quotes() -> Vec<Quote> {
    QUOTES
        .lines()
        .filter_map(|line| {
            let (text, source) = line.split_once('\t')?;
            Some(Quote {
                text: text.trim(),
                source: source.trim(),
            })
        })
        .collect()
}

/// The built-in word lists.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WordList {
    /// The 200 most common English words.
    #[default]
    English,
    /// 1000 common English words.
    English1k,
    /// Keywords, operators and short pieces of code.
    Code,
}

/// The lists in menu order.
pub const WORD_LISTS: [WordList; 3] = [WordList::English, WordList::English1k, WordList::Code];

impl WordList {
    pub fn name(self) -> &'static str {
        match self {
            Self::English => "english",
            Self::English1k => "english-1k",
            Self::Code => "code",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        WORD_LISTS.into_iter().find(|l| l.name() == name)
    }

    /// The list after this one in `WORD_LISTS`, for the menu.
    pub fn next(self) -> Self {
        let i = WORD_LISTS.iter().position(|&l| l == self).unwrap_or(0);
        WORD_LISTS[(i + 1) % WORD_LISTS.len()]
    }

    /// One word on each line of the list file.
    pub fn words(self) -> Vec<&'static str> {
        let text = match self {
            Self::English => ENGLISH,
            Self::English1k => ENGLISH_1K,
            Self::Code => CODE,
        };
        text.lines()
            .map(str::trim)
            .filter(|w| !w.is_empty())
            .collect()
    }
}

/// Picks random words from a list and never gives the same word two times in a row.
pub struct WordGen<R> {
    list: Vec<&'static str>,
    rng: R,
    last: Option<&'static str>,
    punctuation: bool,
    numbers: bool,
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
            numbers: false,
            sentence_start: true,
        }
    }

    /// Add capitals and punctuation, like monkeytype's punctuation mode.
    pub fn with_punctuation(mut self, on: bool) -> Self {
        self.punctuation = on;
        self
    }

    /// Put a random number in place of some words, like monkeytype's numbers mode.
    pub fn with_numbers(mut self, on: bool) -> Self {
        self.numbers = on;
        self
    }

    pub fn next_word(&mut self) -> String {
        let word = if self.numbers && self.rng.random_range(0..100) < 10 {
            self.number()
        } else {
            self.pick().to_string()
        };
        if !self.punctuation {
            return word;
        }
        let mut word = if self.sentence_start {
            capitalize(&word)
        } else {
            word
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

    /// A number with 1 to 4 digits and no leading zero.
    fn number(&mut self) -> String {
        let len = self.rng.random_range(1..=4);
        let low = if len == 1 { 0 } else { 10u32.pow(len - 1) };
        self.rng.random_range(low..10u32.pow(len)).to_string()
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
        let list = WordList::English.words();
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
    fn quotes_are_plain_ascii_with_a_source() {
        let quotes = quotes();
        assert_eq!(quotes.len(), QUOTES.lines().count());
        for q in quotes {
            assert!(!q.source.is_empty(), "{}", q.text);
            assert!(
                q.text.chars().all(|c| c.is_ascii_graphic() || c == ' '),
                "{}",
                q.text
            );
            assert!(!q.text.contains("  "), "{}", q.text);
        }
    }

    #[test]
    fn english_1k_has_1000_unique_words_with_the_200_first() {
        let list = WordList::English1k.words();
        assert_eq!(list.len(), 1000);
        let mut sorted = list.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 1000);
        assert!(
            list.iter()
                .all(|w| w.chars().all(|c| c.is_ascii_lowercase()))
        );
        assert_eq!(list[..200], WordList::English.words()[..]);
    }

    #[test]
    fn code_words_are_unique_and_have_no_spaces() {
        let list = WordList::Code.words();
        let mut sorted = list.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), list.len());
        assert!(list.iter().all(|w| w.chars().all(|c| c.is_ascii_graphic())));
    }

    #[test]
    fn word_list_names_parse_back() {
        for list in WORD_LISTS {
            assert_eq!(WordList::parse(list.name()), Some(list));
        }
        assert_eq!(WordList::Code.next(), WordList::English);
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
        let mut generator = WordGen::new(WordList::English.words(), StdRng::seed_from_u64(7))
            .with_punctuation(true);
        (0..count).map(|_| generator.next_word()).collect()
    }

    #[test]
    fn punctuation_off_gives_plain_words() {
        let mut generator = WordGen::new(WordList::English.words(), StdRng::seed_from_u64(7));
        for _ in 0..200 {
            let word = generator.next_word();
            assert!(word.chars().all(|c| c.is_ascii_lowercase()), "{word}");
        }
    }

    #[test]
    fn punctuation_keeps_list_words_inside_the_marks() {
        let list = WordList::English.words();
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
    fn numbers_replace_about_one_word_in_ten() {
        let mut generator =
            WordGen::new(WordList::English.words(), StdRng::seed_from_u64(7)).with_numbers(true);
        let words: Vec<String> = (0..1000).map(|_| generator.next_word()).collect();
        let numbers: Vec<&String> = words
            .iter()
            .filter(|w| w.chars().all(|c| c.is_ascii_digit()))
            .collect();
        assert!((50..150).contains(&numbers.len()), "{}", numbers.len());
        for n in numbers {
            assert!(n.len() <= 4 && (n.len() == 1 || !n.starts_with('0')), "{n}");
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
