use std::time::{Duration, Instant};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::engine::{
    Key, Mode, Session,
    stats::TestResult,
    words::{self, WordGen},
};
use rand::seq::IndexedRandom;

use crate::history::{History, Record};
use crate::ui;

/// The modes the menu offers, in menu order.
pub const MODES: [Mode; 10] = [
    Mode::Untimed,
    Mode::Quote,
    Mode::Time(Duration::from_secs(15)),
    Mode::Time(Duration::from_secs(30)),
    Mode::Time(Duration::from_secs(60)),
    Mode::Time(Duration::from_secs(120)),
    Mode::Words(10),
    Mode::Words(25),
    Mode::Words(50),
    Mode::Words(100),
];

pub fn mode_label(mode: Mode) -> String {
    match mode {
        Mode::Untimed => "untimed".to_string(),
        Mode::Quote => "quote".to_string(),
        Mode::Time(d) => format!("time {}", d.as_secs()),
        Mode::Words(n) => format!("words {n}"),
    }
}

/// Changes to the words, which the menu turns on and off.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Options {
    pub punctuation: bool,
    pub numbers: bool,
}

pub enum Screen {
    Typing,
    Results(TestResult),
    Menu { selected: usize },
}

pub struct App {
    pub mode: Mode,
    pub options: Options,
    /// Terminal width. Untimed tests get the words that fit on the screen.
    pub width: u16,
    pub session: Session,
    /// Where the quote of a quote test comes from.
    pub source: Option<&'static str>,
    pub screen: Screen,
    pub history: History,
    /// Best WPM for this label before the last finished test.
    pub previous_best: Option<f64>,
    /// The last finished test could not be added to the history file.
    pub save_failed: bool,
    pub quit: bool,
}

impl App {
    pub fn new(mode: Mode, options: Options, width: u16, history: History) -> Self {
        let (session, source) = new_test(mode, options, width);
        Self {
            options,
            width,
            history,
            source,
            ..Self::with_session(mode, session)
        }
    }

    pub fn with_session(mode: Mode, session: Session) -> Self {
        Self {
            mode,
            options: Options::default(),
            width: 80,
            session,
            source: None,
            screen: Screen::Typing,
            history: History::in_memory(),
            previous_best: None,
            save_failed: false,
            quit: false,
        }
    }

    fn restart(&mut self) {
        self.new_test();
        self.screen = Screen::Typing;
    }

    fn new_test(&mut self) {
        (self.session, self.source) = new_test(self.mode, self.options, self.width);
    }

    /// Before the test starts, give an untimed test the words for the new width.
    pub fn on_resize(&mut self, width: u16) {
        self.width = width;
        if self.mode == Mode::Untimed && !self.session.is_started() {
            self.new_test();
        }
    }

    /// The mode and its options, for example "time 30 · punctuation".
    /// Quotes have their own punctuation and numbers, so options do not apply.
    pub fn label(&self) -> String {
        let mut label = mode_label(self.mode);
        if self.mode == Mode::Quote {
            return label;
        }
        if self.options.punctuation {
            label.push_str(" · punctuation");
        }
        if self.options.numbers {
            label.push_str(" · numbers");
        }
        label
    }

    /// Show the results screen if the test ended. Call this on each frame.
    pub fn tick(&mut self, now: Instant) {
        self.session.tick(now);
        if matches!(self.screen, Screen::Typing) && self.session.is_finished() {
            let result = self.session.result();
            let label = self.label();
            self.previous_best = self.history.best(&label);
            self.save_failed = self.history.add(Record::new(label, &result)).is_err();
            self.screen = Screen::Results(result);
        }
    }

    pub fn on_key(&mut self, key: KeyEvent, now: Instant) {
        // Windows sends a Release event for each key too. Use only Press.
        if key.kind != KeyEventKind::Press {
            return;
        }
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.quit = true;
            return;
        }
        match self.screen {
            Screen::Typing => match key.code {
                KeyCode::Tab => self.restart(),
                KeyCode::Esc => self.open_menu(),
                _ => {
                    if let Some(k) = engine_key(key) {
                        self.session.handle(k, now);
                        self.tick(now);
                    }
                }
            },
            Screen::Results(_) => match key.code {
                KeyCode::Tab | KeyCode::Enter => self.restart(),
                KeyCode::Esc => self.open_menu(),
                _ => {}
            },
            Screen::Menu { selected } => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    let selected = (selected + MODES.len() - 1) % MODES.len();
                    self.screen = Screen::Menu { selected };
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let selected = (selected + 1) % MODES.len();
                    self.screen = Screen::Menu { selected };
                }
                KeyCode::Enter => {
                    self.mode = MODES[selected];
                    self.restart();
                }
                // New words are needed, so these start a new test.
                KeyCode::Char('p') => {
                    self.options.punctuation = !self.options.punctuation;
                    self.new_test();
                }
                KeyCode::Char('n') => {
                    self.options.numbers = !self.options.numbers;
                    self.new_test();
                }
                KeyCode::Esc => {
                    self.screen = if self.session.is_finished() {
                        Screen::Results(self.session.result())
                    } else {
                        Screen::Typing
                    };
                }
                _ => {}
            },
        }
    }

    fn open_menu(&mut self) {
        let selected = MODES.iter().position(|&m| m == self.mode).unwrap_or(0);
        self.screen = Screen::Menu { selected };
    }
}

/// A new test, and the source of its quote in quote mode.
fn new_test(mode: Mode, options: Options, width: u16) -> (Session, Option<&'static str>) {
    if mode == Mode::Quote {
        let quote = *words::quotes()
            .choose(&mut rand::rng())
            .expect("the quote list is not empty");
        let words: Vec<&str> = quote.text.split_whitespace().collect();
        return (Session::from_words(mode, &words), Some(quote.source));
    }
    (new_session(mode, options, width), None)
}

fn new_session(mode: Mode, options: Options, width: u16) -> Session {
    let mut words = WordGen::new(words::english(), rand::rng())
        .with_punctuation(options.punctuation)
        .with_numbers(options.numbers);
    if mode == Mode::Untimed {
        return Session::from_words(mode, &ui::screen_words(width, || words.next_word()));
    }
    Session::new(mode, move || words.next_word())
}

fn engine_key(key: KeyEvent) -> Option<Key> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    match key.code {
        KeyCode::Backspace if ctrl || alt => Some(Key::DeleteWord),
        KeyCode::Backspace => Some(Key::Backspace),
        KeyCode::Char('w') if ctrl && !alt => Some(Key::DeleteWord),
        KeyCode::Char(' ') => Some(Key::Space),
        // AltGr arrives as Ctrl+Alt, so allow that for letters on some layouts.
        KeyCode::Char(c) if ctrl == alt => Some(Key::Char(c)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::KeyEventState;

    fn press(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    fn app(words: &[&str]) -> App {
        let mode = Mode::Words(words.len());
        App::with_session(mode, Session::from_words(mode, words))
    }

    fn type_text(app: &mut App, text: &str) {
        for c in text.chars() {
            app.on_key(press(KeyCode::Char(c), KeyModifiers::NONE), Instant::now());
        }
    }

    #[test]
    fn release_events_are_ignored() {
        let mut a = app(&["hi"]);
        let mut release = press(KeyCode::Char('h'), KeyModifiers::NONE);
        release.kind = KeyEventKind::Release;
        release.state = KeyEventState::NONE;
        a.on_key(release, Instant::now());
        assert!(a.session.typed()[0].is_empty());
    }

    #[test]
    fn ctrl_backspace_deletes_the_word() {
        let mut a = app(&["hello"]);
        type_text(&mut a, "hel");
        a.on_key(
            press(KeyCode::Backspace, KeyModifiers::CONTROL),
            Instant::now(),
        );
        assert!(a.session.typed()[0].is_empty());
    }

    #[test]
    fn finishing_the_test_shows_results() {
        let mut a = app(&["hi"]);
        type_text(&mut a, "hi");
        assert!(matches!(a.screen, Screen::Results(_)));
    }

    #[test]
    fn finished_tests_go_into_the_history() {
        let mut a = app(&["hi"]);
        type_text(&mut a, "hi");
        assert_eq!(a.previous_best, None);
        a.session = Session::from_words(a.mode, &["hi"]);
        a.screen = Screen::Typing;
        type_text(&mut a, "hi");
        assert_eq!(a.history.records().len(), 2);
        assert!(a.previous_best.is_some());
        assert!(!a.save_failed);
    }

    #[test]
    fn menu_selects_a_new_mode() {
        let mut a = app(&["hi"]);
        a.on_key(press(KeyCode::Esc, KeyModifiers::NONE), Instant::now());
        assert!(matches!(a.screen, Screen::Menu { .. }));
        a.on_key(press(KeyCode::Up, KeyModifiers::NONE), Instant::now());
        a.on_key(press(KeyCode::Enter, KeyModifiers::NONE), Instant::now());
        assert!(matches!(a.screen, Screen::Typing));
        assert_eq!(a.mode, MODES[MODES.len() - 1]);
    }

    #[test]
    fn p_in_menu_toggles_punctuation_and_starts_a_new_test() {
        let mut a = app(&["hi", "there"]);
        type_text(&mut a, "h");
        a.on_key(press(KeyCode::Esc, KeyModifiers::NONE), Instant::now());
        a.on_key(
            press(KeyCode::Char('p'), KeyModifiers::NONE),
            Instant::now(),
        );
        assert!(a.options.punctuation);
        assert!(matches!(a.screen, Screen::Menu { .. }));
        assert!(!a.session.is_started());
        assert!(a.label().ends_with(" · punctuation"));
        a.on_key(press(KeyCode::Esc, KeyModifiers::NONE), Instant::now());
        assert!(matches!(a.screen, Screen::Typing));
        a.on_key(press(KeyCode::Esc, KeyModifiers::NONE), Instant::now());
        a.on_key(
            press(KeyCode::Char('p'), KeyModifiers::NONE),
            Instant::now(),
        );
        assert!(!a.options.punctuation);
    }

    #[test]
    fn n_in_menu_toggles_numbers() {
        let mut a = app(&["hi"]);
        a.on_key(press(KeyCode::Esc, KeyModifiers::NONE), Instant::now());
        a.on_key(
            press(KeyCode::Char('n'), KeyModifiers::NONE),
            Instant::now(),
        );
        assert!(a.options.numbers && !a.options.punctuation);
        assert!(a.label().ends_with(" · numbers"));
    }

    #[test]
    fn untimed_test_fits_on_the_screen_and_follows_resize() {
        let mut a = App::new(Mode::Untimed, Options::default(), 40, History::in_memory());
        let narrow = a.session.words().len();
        a.on_resize(120);
        assert!(a.session.words().len() > narrow);
        type_text(&mut a, "x");
        let started = a.session.words().len();
        a.on_resize(40);
        assert_eq!(a.session.words().len(), started);
    }

    #[test]
    fn quote_mode_types_a_whole_quote() {
        let mut a = App::new(Mode::Quote, Options::default(), 80, History::in_memory());
        let source = a.source.expect("a quote has a source");
        let text: String = a
            .session
            .words()
            .iter()
            .map(|w| w.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            words::quotes()
                .iter()
                .any(|q| q.text == text && q.source == source),
            "{text}"
        );
        a.options.punctuation = true;
        assert_eq!(a.label(), "quote");
        let mut a = app(&["one", "two"]);
        a.mode = Mode::Quote;
        a.session = Session::from_words(Mode::Quote, &["one", "two"]);
        type_text(&mut a, "one");
        a.on_key(
            press(KeyCode::Char(' '), KeyModifiers::NONE),
            Instant::now(),
        );
        type_text(&mut a, "two");
        assert!(matches!(a.screen, Screen::Results(_)));
    }

    #[test]
    fn ctrl_c_quits_from_any_screen() {
        let mut a = app(&["hi"]);
        a.on_key(press(KeyCode::Esc, KeyModifiers::NONE), Instant::now());
        a.on_key(
            press(KeyCode::Char('c'), KeyModifiers::CONTROL),
            Instant::now(),
        );
        assert!(a.quit);
    }
}
