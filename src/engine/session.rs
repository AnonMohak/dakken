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

/// Rules that make a test harder, like monkeytype's.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Strict {
    #[default]
    Off,
    /// A wrong key does not move the caret. Only the correct key does.
    StopOnError,
    /// Backspace does nothing.
    NoBackspace,
    /// The first wrong key fails the test.
    Master,
}

/// The strict modes in menu order.
pub const STRICT_MODES: [Strict; 4] = [
    Strict::Off,
    Strict::StopOnError,
    Strict::NoBackspace,
    Strict::Master,
];

impl Strict {
    pub fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::StopOnError => "stop-on-error",
            Self::NoBackspace => "no-backspace",
            Self::Master => "master",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        STRICT_MODES.into_iter().find(|s| s.name() == name)
    }

    /// The mode after this one in `STRICT_MODES`, for the menu.
    pub fn next(self) -> Self {
        let i = STRICT_MODES.iter().position(|&s| s == self).unwrap_or(0);
        STRICT_MODES[(i + 1) % STRICT_MODES.len()]
    }
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
    strict: Strict,
    /// A wrong key in master mode ended the test.
    failed: bool,
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
            strict: Strict::Off,
            failed: false,
        }
    }

    pub fn set_strict(&mut self, strict: Strict) {
        self.strict = strict;
    }

    /// The test ended because of a wrong key in master mode.
    pub fn is_failed(&self) -> bool {
        self.failed
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
        self.presses.push(Press {
            at: now - start,
            correct,
        });
        if !correct && self.strict == Strict::StopOnError {
            return;
        }
        typed.push(c);
        if !correct && self.strict == Strict::Master {
            self.fail(now);
            return;
        }

        // The last word ends the test when it is full length, also with errors.
        if self.current == self.words.len() - 1
            && self.source.is_none()
            && typed.len() >= target.len()
        {
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
        match self.strict {
            // A space before the word is complete is a wrong key too.
            Strict::StopOnError if !correct => return,
            Strict::Master if !correct => {
                self.fail(now);
                return;
            }
            _ => {}
        }
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

    fn fail(&mut self, now: Instant) {
        self.failed = true;
        self.end = Some(now);
    }

    fn backspace(&mut self, whole_word: bool) {
        if self.strict == Strict::NoBackspace {
            return;
        }
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

    /// Where a typist at `wpm` would be now, as (word, letter). A position on
    /// the space after a word moves to the start of the next word. `None`
    /// before the start, after the end, or past the last word.
    pub fn pace_position(&self, wpm: f64, now: Instant) -> Option<(usize, usize)> {
        if !self.is_started() || self.is_finished() {
            return None;
        }
        let mut chars = (wpm * 5.0 / 60.0 * self.elapsed(now).as_secs_f64()) as usize;
        for (i, word) in self.words.iter().enumerate() {
            if chars < word.len() {
                return Some((i, chars));
            }
            chars -= (word.len() + 1).min(chars);
            if chars == 0 {
                return (i + 1 < self.words.len()).then_some((i + 1, 0));
            }
        }
        None
    }

    /// Accuracy so far, for the live display.
    pub fn live_accuracy(&self) -> f64 {
        stats::accuracy(&self.presses)
    }

    /// The words the test checked, with `true` for a correct word: the closed
    /// words, and the last word when typing it ended the test.
    pub fn checked_words(&self) -> Vec<(String, bool)> {
        let mut end = self.current;
        if self.is_finished()
            && self.source.is_none()
            && end < self.words.len()
            && !self.typed[end].is_empty()
        {
            end += 1;
        }
        (0..end)
            .map(|i| {
                (
                    self.words[i].iter().collect(),
                    self.typed[i] == self.words[i],
                )
            })
            .collect()
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
    fn words_mode_ends_on_full_length_wrong_last_word() {
        let mut d = Driver::words(&["hi", "yo"]);
        d.text("hi y");
        assert!(!d.s.is_finished());
        d.text("x");
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
    fn words_mode_ends_on_space_after_short_last_word() {
        let mut d = Driver::words(&["hi", "yo"]);
        d.text("hi y");
        d.text(" ");
        assert!(d.s.is_finished());
        assert_eq!(d.s.result().chars.missed, 1);
    }

    #[test]
    fn checked_words_include_the_last_word_that_ended_the_test() {
        let mut d = Driver::words(&["hi", "yo", "ok"]);
        d.text("hx yo o");
        assert_eq!(
            d.s.checked_words(),
            vec![("hi".into(), false), ("yo".into(), true)]
        );
        d.text("x");
        assert_eq!(d.s.checked_words().len(), 3);
        assert_eq!(d.s.checked_words()[2], ("ok".into(), false));
    }

    #[test]
    fn pace_position_moves_at_the_given_speed() {
        let start = Instant::now();
        let mut s = Session::from_words(Mode::Words(2), &["abcd", "efgh"]);
        assert_eq!(s.pace_position(60.0, start), None);
        s.handle(Key::Char('a'), start);
        // 60 WPM is 5 letters each second.
        let at = |ms| start + Duration::from_millis(ms);
        assert_eq!(s.pace_position(60.0, at(0)), Some((0, 0)));
        assert_eq!(s.pace_position(60.0, at(600)), Some((0, 3)));
        assert_eq!(s.pace_position(60.0, at(800)), Some((1, 0)));
        assert_eq!(s.pace_position(60.0, at(1400)), Some((1, 2)));
        assert_eq!(s.pace_position(60.0, at(5000)), None);
    }

    #[test]
    fn stop_on_error_keeps_the_caret_until_the_right_key() {
        let mut d = Driver::words(&["hi", "yo"]);
        d.s.set_strict(Strict::StopOnError);
        d.text("hx");
        assert_eq!(d.typed(0), "h");
        d.text(" ");
        assert_eq!(d.s.current(), 0, "space before the word is complete");
        d.text("i yo");
        assert!(d.s.is_finished() && !d.s.is_failed());
        let r = d.s.result();
        assert!(r.accuracy < 100.0, "wrong keys lower accuracy");
        assert_eq!(r.chars.incorrect, 0, "no wrong letters stay in the text");
    }

    #[test]
    fn no_backspace_ignores_backspace() {
        let mut d = Driver::words(&["hi", "yo"]);
        d.s.set_strict(Strict::NoBackspace);
        d.text("hx");
        d.key(Key::Backspace);
        d.key(Key::DeleteWord);
        assert_eq!(d.typed(0), "hx");
    }

    #[test]
    fn master_fails_on_the_first_wrong_key() {
        let mut d = Driver::words(&["hi", "yo"]);
        d.s.set_strict(Strict::Master);
        d.text("hi y");
        assert!(!d.s.is_finished());
        d.text("x");
        assert!(d.s.is_finished() && d.s.is_failed());

        let mut d = Driver::words(&["hello", "yo"]);
        d.s.set_strict(Strict::Master);
        d.text("he ");
        assert!(d.s.is_failed(), "a skipped letter fails too");
    }

    #[test]
    fn strict_names_parse_back() {
        for s in STRICT_MODES {
            assert_eq!(Strict::parse(s.name()), Some(s));
        }
        assert_eq!(Strict::Master.next(), Strict::Off);
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
