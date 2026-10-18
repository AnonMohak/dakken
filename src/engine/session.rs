use std::time::{Duration, Instant};

use super::stats::{self, CharCounts, Press, TestResult};

/// Most letters you can type past the end of a word.
pub const MAX_EXTRA: usize = 10;

/// Time mode keeps at least this many words ready after the current one.
const WORDS_AHEAD: usize = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Type a fixed list of words with no time limit.
    Untimed,
    /// Type one quote with no time limit.
    Quote,
    Time(Duration),
    Words(usize),
}

/// Input the engine understands. The UI maps terminal keys to these.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Space,
    Backspace,
    /// Ctrl+Backspace: delete the whole word.
    DeleteWord,
}

type WordSource = Box<dyn FnMut() -> String>;

/// One typing test: the target words, what the user typed, and the timing.
pub struct Session {
    mode: Mode,
    words: Vec<Vec<char>>,
    typed: Vec<Vec<char>>,
    /// Index of the word the caret is in. Equals `words.len()` after the last
    /// word is closed with a space.
    current: usize,
    presses: Vec<Press>,
    start: Option<Instant>,
    end: Option<Instant>,
    source: Option<WordSource>,
}

impl Session {
    /// A test with random words. Time mode adds new words as you type.
    /// An untimed test gets the same word count as the default words mode.
    pub fn new(mode: Mode, mut next_word: impl FnMut() -> String + 'static) -> Self {
        let count = match mode {
            Mode::Words(n) => n,
            Mode::Untimed | Mode::Quote => 25,
            Mode::Time(_) => WORDS_AHEAD * 2,
        };
        let words: Vec<String> = (0..count).map(|_| next_word()).collect();
        let mut s = Self::from_words(mode, &words);
        if matches!(mode, Mode::Time(_)) {
            s.source = Some(Box::new(next_word));
        }
        s
    }

    /// A test with a fixed list of words. The test ends after the last word.
    pub fn from_words<S: AsRef<str>>(mode: Mode, words: &[S]) -> Self {
        assert!(!words.is_empty(), "a test needs at least one word");
        Self {
            mode,
            words: words.iter().map(|w| w.as_ref().chars().collect()).collect(),
            typed: vec![Vec::new(); words.len()],
            current: 0,
            presses: Vec::new(),
            start: None,
            end: None,
            source: None,
        }
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn words(&self) -> &[Vec<char>] {
        &self.words
    }

    pub fn typed(&self) -> &[Vec<char>] {
        &self.typed
    }

    pub fn current(&self) -> usize {
        self.current
    }

    pub fn is_started(&self) -> bool {
        self.start.is_some()
    }

    pub fn is_finished(&self) -> bool {
        self.end.is_some()
    }

    /// Time since the first key press, stopped at the end of the test.
    pub fn elapsed(&self, now: Instant) -> Duration {
        match (self.start, self.end) {
            (Some(start), Some(end)) => end - start,
            (Some(start), None) => now.saturating_duration_since(start),
            _ => Duration::ZERO,
        }
    }

    /// Time left in time mode. `None` in the other modes.
    pub fn remaining(&self, now: Instant) -> Option<Duration> {
        match self.mode {
            Mode::Time(limit) => Some(limit.saturating_sub(self.elapsed(now))),
            Mode::Untimed | Mode::Quote | Mode::Words(_) => None,
        }
    }

    /// Ends a time-mode test when its time is over. Call this on each frame.
    pub fn tick(&mut self, now: Instant) {
        if let (Mode::Time(limit), Some(start), None) = (self.mode, self.start, self.end)
            && now >= start + limit
        {
            self.end = Some(start + limit);
        }
    }

    pub fn handle(&mut self, key: Key, now: Instant) {
        self.tick(now);
        if self.is_finished() {
            return;
        }
        match key {
            Key::Char(c) => self.type_char(c, now),
            Key::Space => self.type_space(now),
            Key::Backspace => self.backspace(false),
            Key::DeleteWord => self.backspace(true),
        }
    }

    fn type_char(&mut self, c: char, now: Instant) {
        let target = &self.words[self.current];
        let typed = &mut self.typed[self.current];
        if typed.len() >= target.len() + MAX_EXTRA {
            return;
        }
        let start = *self.start.get_or_insert(now);
        let correct = target.get(typed.len()) == Some(&c);
        typed.push(c);
        self.presses.push(Press {
            at: now - start,
            correct,
        });

        if self.current == self.words.len() - 1 && self.source.is_none() && *typed == *target {
            self.end = Some(now);
        }
    }

    fn type_space(&mut self, now: Instant) {
        if self.typed[self.current].is_empty() {
            return;
        }
        let start = *self.start.get_or_insert(now);
        let correct = self.typed[self.current] == self.words[self.current];
        self.presses.push(Press {
            at: now - start,
            correct,
        });
        self.current += 1;

        match self.source.as_mut() {
            None if self.current == self.words.len() => self.end = Some(now),
            None => {}
            Some(next_word) => {
                while self.words.len() < self.current + WORDS_AHEAD {
                    self.words.push(next_word().chars().collect());
                    self.typed.push(Vec::new());
                }
            }
        }
    }

    fn backspace(&mut self, whole_word: bool) {
        if self.typed[self.current].is_empty() {
            // Go back only into a previous word that has an error.
            let prev = match self.current.checked_sub(1) {
                Some(p) if self.typed[p] != self.words[p] => p,
                _ => return,
            };
            self.current = prev;
            if !whole_word {
                return;
            }
        }
        let typed = &mut self.typed[self.current];
        if whole_word {
            typed.clear();
        } else {
            typed.pop();
        }
    }

    /// Characters that count for WPM: letters of correct words, the spaces
    /// after them, and a correct start of the word in progress.
    fn correct_chars(&self) -> usize {
        let mut n = 0;
        for i in 0..self.current {
            if self.typed[i] == self.words[i] {
                n += self.words[i].len() + 1;
            }
        }
        if let Some(typed) = self.typed.get(self.current)
            && self.words[self.current].starts_with(typed)
        {
            n += typed.len();
        }
        n
    }

    fn char_counts(&self) -> CharCounts {
        let mut c = CharCounts::default();
        let last = self.current.min(self.words.len() - 1);
        for i in 0..=last {
            let (target, typed) = (&self.words[i], &self.typed[i]);
            for (j, ch) in typed.iter().enumerate() {
                match target.get(j) {
                    Some(t) if t == ch => c.correct += 1,
                    Some(_) => c.incorrect += 1,
                    None => c.extra += 1,
                }
            }
            if i < self.current {
                c.missed += target.len().saturating_sub(typed.len());
            }
        }
        c
    }

    /// WPM so far, for the live display.
    pub fn live_wpm(&self, now: Instant) -> f64 {
        stats::wpm(self.correct_chars(), self.elapsed(now))
    }

    /// The final numbers. Call after the test is finished.
    pub fn result(&self) -> TestResult {
        let duration = self.elapsed(self.end.unwrap_or_else(Instant::now));
        let raw_per_second = stats::raw_per_second(&self.presses, duration);
        TestResult {
            wpm: stats::wpm(self.correct_chars(), duration),
            raw_wpm: stats::wpm(self.presses.len(), duration),
            accuracy: stats::accuracy(&self.presses),
            consistency: stats::consistency(&raw_per_second),
            wpm_over_time: stats::wpm_over_time(&self.presses, duration),
            chars: self.char_counts(),
            duration,
            raw_per_second,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drives a session with a fake clock: each key is 100 ms after the last.
    struct Driver {
        s: Session,
        now: Instant,
    }

    impl Driver {
        fn new(s: Session) -> Self {
            Self {
                s,
                now: Instant::now(),
            }
        }

        fn words(words: &[&str]) -> Self {
            Self::new(Session::from_words(Mode::Words(words.len()), words))
        }

        fn key(&mut self, k: Key) {
            self.now += Duration::from_millis(100);
            self.s.handle(k, self.now);
        }

        fn text(&mut self, text: &str) {
            for c in text.chars() {
                self.key(if c == ' ' { Key::Space } else { Key::Char(c) });
            }
        }

        fn typed(&self, i: usize) -> String {
            self.s.typed()[i].iter().collect()
        }
    }

    #[test]
    fn first_key_starts_the_test() {
        let mut d = Driver::words(&["hi"]);
        assert!(!d.s.is_started());
        d.text("h");
        assert!(d.s.is_started());
        assert_eq!(d.s.elapsed(d.now), Duration::ZERO);
    }

    #[test]
    fn space_moves_to_next_word() {
        let mut d = Driver::words(&["hi", "yo"]);
        d.text("hi ");
        assert_eq!(d.s.current(), 1);
    }

    #[test]
    fn space_at_word_start_does_nothing() {
        let mut d = Driver::words(&["hi", "yo"]);
        d.text(" ");
        assert_eq!(d.s.current(), 0);
        assert!(!d.s.is_started());
        d.text("hi  ");
        assert_eq!(d.s.current(), 1);
    }

    #[test]
    fn letters_past_word_end_are_extra_and_limited() {
        let mut d = Driver::words(&["a", "b"]);
        d.text(&"a".repeat(30));
        assert_eq!(d.typed(0).len(), 1 + MAX_EXTRA);
        d.text(" b");
        assert_eq!(d.s.result().chars.extra, MAX_EXTRA);
    }

    #[test]
    fn backspace_goes_back_only_into_a_word_with_errors() {
        let mut d = Driver::words(&["hi", "yo", "ok"]);
        d.text("hi ");
        d.key(Key::Backspace);
        assert_eq!(d.s.current(), 1, "correct word is locked");

        d.text("yx ");
        d.key(Key::Backspace);
        assert_eq!(d.s.current(), 1, "wrong word can be fixed");
        assert_eq!(d.typed(1), "yx");
        d.key(Key::Backspace);
        assert_eq!(d.typed(1), "y");
    }

    #[test]
    fn delete_word_clears_current_or_wrong_previous_word() {
        let mut d = Driver::words(&["hi", "yo", "ok"]);
        d.text("hx ");
        d.text("yo");
        d.key(Key::DeleteWord);
        assert_eq!(d.typed(1), "");
        d.key(Key::DeleteWord);
        assert_eq!(d.s.current(), 0);
        assert_eq!(d.typed(0), "");
    }

    #[test]
    fn words_mode_ends_on_last_correct_letter() {
        let mut d = Driver::words(&["hi", "yo"]);
        d.text("hi y");
        assert!(!d.s.is_finished());
        d.text("o");
        assert!(d.s.is_finished());
        d.text("xxx");
        assert_eq!(d.typed(1), "yo", "keys after the end are ignored");
    }

    #[test]
    fn words_mode_ends_on_space_after_wrong_last_word() {
        let mut d = Driver::words(&["hi", "yo"]);
        d.text("hi yx");
        assert!(!d.s.is_finished());
        d.text(" ");
        assert!(d.s.is_finished());
        assert_eq!(
            d.s.result().chars,
            CharCounts {
                correct: 3,
                incorrect: 1,
                extra: 0,
                missed: 0
            }
        );
    }

    #[test]
    fn missed_letters_count_when_word_is_skipped() {
        let mut d = Driver::words(&["hello", "yo"]);
        d.text("he yo");
        assert_eq!(d.s.result().chars.missed, 3);
    }

    #[test]
    fn perfect_test_result() {
        let mut d = Driver::words(&["hi", "yo"]);
        d.text("hi yo");
        let r = d.s.result();
        // 5 presses, 100 ms apart; first press is t=0, so duration = 400 ms.
        assert_eq!(r.duration, Duration::from_millis(400));
        assert_eq!(r.accuracy, 100.0);
        // Correct chars: "hi" + space + "yo" = 5 -> 1 word in 400 ms = 150 WPM.
        assert!((r.wpm - 150.0).abs() < 1e-9, "wpm {}", r.wpm);
        assert!((r.raw_wpm - 150.0).abs() < 1e-9);
        assert_eq!(
            r.chars,
            CharCounts {
                correct: 4,
                incorrect: 0,
                extra: 0,
                missed: 0
            }
        );
    }

    #[test]
    fn errors_lower_accuracy_and_wpm_but_not_raw() {
        let mut d = Driver::words(&["hi", "yo"]);
        d.text("hx yo");
        let r = d.s.result();
        // Wrong letter and the space after the wrong word are both errors.
        assert_eq!(r.accuracy, 60.0);
        assert!((r.raw_wpm - 150.0).abs() < 1e-9);
        // Only "yo" counts: 2 chars in 400 ms = 60 WPM.
        assert!((r.wpm - 60.0).abs() < 1e-9);
    }

    #[test]
    fn time_mode_ends_at_the_limit_and_adds_words() {
        let mut n = 0;
        let s = Session::new(Mode::Time(Duration::from_secs(1)), move || {
            n += 1;
            "ab".to_string() + &"c".repeat(n % 2)
        });
        let mut d = Driver::new(s);
        let start_len = d.s.words().len();
        for _ in 0..60 {
            let w: String = d.s.words()[d.s.current()].iter().collect();
            if d.s.is_finished() {
                break;
            }
            d.text(&w);
            d.text(" ");
        }
        assert!(d.s.is_finished());
        assert_eq!(d.s.result().duration, Duration::from_secs(1));
        assert_eq!(d.s.remaining(d.now), Some(Duration::ZERO));
        assert!(d.s.words().len() >= start_len);
    }

    #[test]
    fn time_mode_tick_ends_test_without_keys() {
        let s = Session::new(Mode::Time(Duration::from_secs(15)), || "word".into());
        let mut d = Driver::new(s);
        d.text("w");
        assert!(!d.s.is_finished());
        d.s.tick(d.now + Duration::from_secs(15));
        assert!(d.s.is_finished());
        assert_eq!(d.s.result().duration, Duration::from_secs(15));
    }

    #[test]
    fn time_mode_keeps_words_ahead_of_caret() {
        let s = Session::new(Mode::Time(Duration::from_secs(600)), || "a".into());
        let mut d = Driver::new(s);
        for _ in 0..200 {
            d.text("a ");
        }
        assert_eq!(d.s.current(), 200);
        assert!(d.s.words().len() >= 200 + WORDS_AHEAD);
    }
}
