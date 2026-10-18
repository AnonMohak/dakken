use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::engine::{
    Key, Mode, QuoteLength, Session, Strict,
    stats::TestResult,
    words::{self, WordGen, WordList},
};
use rand::seq::IndexedRandom;

use crate::config::Config;
use crate::history::{History, Record};
use crate::mistakes::{self, Mistakes};
use crate::ui::{self, theme::Theme};

/// The modes the menu offers, in menu order.
pub const MODES: [Mode; 14] = [
    Mode::Untimed,
    Mode::Quote(QuoteLength::All),
    Mode::Quote(QuoteLength::Short),
    Mode::Quote(QuoteLength::Medium),
    Mode::Quote(QuoteLength::Long),
    Mode::Zen,
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
        Mode::Quote(QuoteLength::All) => "quote".to_string(),
        Mode::Quote(length) => format!("quote {}", length.name()),
        Mode::Custom => "custom".to_string(),
        Mode::Zen => "zen".to_string(),
        Mode::Time(d) => format!("time {}", d.as_secs()),
        Mode::Words(n) => format!("words {n}"),
    }
}

/// The opposite of `mode_label`.
pub fn parse_mode(label: &str) -> Option<Mode> {
    match label.split_once(' ') {
        None if label == "untimed" => Some(Mode::Untimed),
        None if label == "quote" => Some(Mode::Quote(QuoteLength::All)),
        Some(("quote", length)) => Some(Mode::Quote(QuoteLength::parse(length)?)),
        None if label == "zen" => Some(Mode::Zen),
        Some(("time", n)) => Some(Mode::Time(Duration::from_secs(n.parse().ok()?))),
        Some(("words", n)) => Some(Mode::Words(n.parse().ok()?)),
        _ => None,
    }
    .filter(|&m| m != Mode::Time(Duration::ZERO) && m != Mode::Words(0))
}

/// The text of a file for custom mode.
pub struct CustomText {
    /// The file name, without the folder.
    pub name: String,
    pub words: Vec<String>,
}

impl CustomText {
    /// Read a text file. Line breaks and tabs become spaces.
    pub fn load(path: &Path) -> io::Result<Self> {
        let text = fs::read_to_string(path)?;
        let words: Vec<String> = text.split_whitespace().map(String::from).collect();
        if words.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "the file has no text",
            ));
        }
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        Ok(Self { name, words })
    }
}

/// Changes to the words, which the menu turns on and off.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Options {
    pub punctuation: bool,
    pub numbers: bool,
    pub list: WordList,
}

pub enum Screen {
    Typing,
    Results(TestResult),
    Menu { selected: usize },
}

pub struct App {
    pub mode: Mode,
    pub options: Options,
    pub theme: Theme,
    /// Show the live WPM and accuracy during the test.
    pub live: bool,
    /// Show a pace caret at the personal-best speed.
    pub pace: bool,
    pub strict: Strict,
    /// The menu saves the settings here. `None` does not save them.
    pub config_path: Option<PathBuf>,
    /// Terminal width. Untimed tests get the words that fit on the screen.
    pub width: u16,
    pub session: Session,
    /// Where the quote of a quote test comes from.
    pub source: Option<&'static str>,
    /// The text for custom mode, from `--file`.
    pub custom: Option<CustomText>,
    pub screen: Screen,
    pub history: History,
    pub mistakes: Mistakes,
    /// Best WPM for this label before the last finished test.
    pub previous_best: Option<f64>,
    /// The last finished test could not be added to the history file.
    pub save_failed: bool,
    pub quit: bool,
}

impl App {
    pub fn new(
        config: Config,
        config_path: Option<PathBuf>,
        width: u16,
        history: History,
        mistakes: Mistakes,
        custom: Option<CustomText>,
    ) -> Self {
        let mode = match custom {
            Some(_) => Mode::Custom,
            None => config.mode,
        };
        let mut app = Self {
            options: config.options,
            theme: config.theme,
            live: config.live,
            pace: config.pace,
            strict: config.strict,
            config_path,
            width,
            history,
            mistakes,
            custom,
            // `new_test` below makes the real test.
            ..Self::with_session(mode, Session::from_words(mode, &["dakken"]))
        };
        app.new_test();
        app
    }

    pub fn with_session(mode: Mode, session: Session) -> Self {
        Self {
            mode,
            options: Options::default(),
            theme: Theme::default(),
            live: false,
            pace: false,
            strict: Strict::Off,
            config_path: None,
            width: 80,
            session,
            source: None,
            custom: None,
            screen: Screen::Typing,
            history: History::in_memory(),
            mistakes: Mistakes::in_memory(),
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
        (self.session, self.source) = match &self.custom {
            Some(custom) if self.mode == Mode::Custom => {
                (Session::from_words(self.mode, &custom.words), None)
            }
            _ => new_test(self.mode, self.options, self.width, self.mistakes.words()),
        };
        self.session.set_strict(self.strict);
    }

    /// Remember the words typed wrong. In the mistakes list, words typed
    /// right are removed.
    fn update_mistakes(&mut self) -> std::io::Result<()> {
        // Quote, custom and zen tests have their own text, not a word list.
        let own_text = matches!(self.mode, Mode::Quote(_) | Mode::Custom | Mode::Zen);
        let list = (!own_text).then_some(self.options.list);
        let practising = list == Some(WordList::Mistakes);
        let (mut wrong, mut fixed) = (Vec::new(), Vec::new());
        for (word, correct) in self.session.checked_words() {
            match (mistakes::practice_word(&word, list), correct) {
                (Some(w), false) => wrong.push(w),
                (Some(w), true) if practising => fixed.push(w),
                _ => {}
            }
        }
        self.mistakes.update(&wrong, &fixed)
    }

    /// Save the mode, options and theme. Settings are not important enough to
    /// stop the test, so a failed save is ignored.
    fn save_config(&self) {
        if let Some(path) = &self.config_path {
            // Custom mode needs a file from the command line, so keep the
            // saved mode instead.
            let mode = match self.mode {
                Mode::Custom => Config::load(path).mode,
                mode => mode,
            };
            let config = Config {
                mode,
                options: self.options,
                theme: self.theme,
                live: self.live,
                pace: self.pace,
                strict: self.strict,
            };
            let _ = config.save(path);
        }
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
        if let (Mode::Custom, Some(custom)) = (self.mode, &self.custom) {
            label.push_str(" · ");
            label.push_str(&custom.name);
        }
        // These modes have their own text, so word options do not apply.
        if matches!(self.mode, Mode::Quote(_) | Mode::Custom | Mode::Zen) {
            if self.strict != Strict::Off {
                label.push_str(" · ");
                label.push_str(self.strict.name());
            }
            return label;
        }
        if self.options.punctuation {
            label.push_str(" · punctuation");
        }
        if self.options.numbers {
            label.push_str(" · numbers");
        }
        if self.strict != Strict::Off {
            label.push_str(" · ");
            label.push_str(self.strict.name());
        }
        // English is the default, so old history labels stay the same.
        if self.options.list != WordList::English {
            label.push_str(" · ");
            label.push_str(self.options.list.name());
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
            // A failed test is not a real result, so it is not saved.
            self.save_failed =
                !self.session.is_failed() && self.history.add(Record::new(label, &result)).is_err();
            self.save_failed |= self.update_mistakes().is_err();
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
                KeyCode::Tab | KeyCode::Esc => self.open_menu(),
                KeyCode::Enter if self.mode == Mode::Zen => {
                    self.session.finish(now);
                    self.tick(now);
                }
                _ => {
                    if let Some(k) = engine_key(key) {
                        self.session.handle(k, now);
                        self.tick(now);
                    }
                }
            },
            Screen::Results(_) => match key.code {
                KeyCode::Enter => self.restart(),
                KeyCode::Tab | KeyCode::Esc => self.open_menu(),
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
                    self.save_config();
                    self.restart();
                }
                // New words are needed, so these start a new test.
                KeyCode::Char('p') => {
                    self.options.punctuation = !self.options.punctuation;
                    self.save_config();
                    self.new_test();
                }
                KeyCode::Char('n') => {
                    self.options.numbers = !self.options.numbers;
                    self.save_config();
                    self.new_test();
                }
                KeyCode::Char('l') => {
                    self.options.list = self.options.list.next();
                    self.save_config();
                    self.new_test();
                }
                KeyCode::Char('t') => {
                    self.theme = self.theme.next();
                    self.save_config();
                }
                KeyCode::Char('s') => {
                    self.live = !self.live;
                    self.save_config();
                }
                KeyCode::Char('c') => {
                    self.pace = !self.pace;
                    self.save_config();
                }
                KeyCode::Char('m') => {
                    self.strict = self.strict.next();
                    self.save_config();
                    self.new_test();
                }
                KeyCode::Tab | KeyCode::Esc => {
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
fn new_test(
    mode: Mode,
    options: Options,
    width: u16,
    mistakes: &[String],
) -> (Session, Option<&'static str>) {
    if let Mode::Quote(length) = mode {
        let quotes: Vec<_> = words::quotes()
            .into_iter()
            .filter(|q| length.fits(q.text.split_whitespace().count()))
            .collect();
        let quote = *quotes
            .choose(&mut rand::rng())
            .expect("each quote length has quotes");
        let words: Vec<&str> = quote.text.split_whitespace().collect();
        return (Session::from_words(mode, &words), Some(quote.source));
    }
    (new_session(mode, options, width, mistakes), None)
}

fn new_session(mode: Mode, options: Options, width: u16, mistakes: &[String]) -> Session {
    let list: Vec<String> = if options.list == WordList::Mistakes && !mistakes.is_empty() {
        mistakes.to_vec()
    } else {
        options.list.words().into_iter().map(String::from).collect()
    };
    let mut words = WordGen::new(list, rand::rng())
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
        let config = Config::default();
        let mut a = App::new(
            config,
            None,
            40,
            History::in_memory(),
            Mistakes::in_memory(),
            None,
        );
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
        let config = Config {
            mode: Mode::Quote(QuoteLength::All),
            ..Config::default()
        };
        let mut a = App::new(
            config,
            None,
            80,
            History::in_memory(),
            Mistakes::in_memory(),
            None,
        );
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
        a.mode = Mode::Quote(QuoteLength::All);
        a.session = Session::from_words(a.mode, &["one", "two"]);
        type_text(&mut a, "one");
        a.on_key(
            press(KeyCode::Char(' '), KeyModifiers::NONE),
            Instant::now(),
        );
        type_text(&mut a, "two");
        assert!(matches!(a.screen, Screen::Results(_)));
    }

    #[test]
    fn mode_labels_parse_back() {
        for mode in MODES {
            assert_eq!(parse_mode(&mode_label(mode)), Some(mode));
        }
        assert_eq!(parse_mode("time 0"), None);
        assert_eq!(parse_mode("words"), None);
    }

    #[test]
    fn menu_changes_are_saved_to_the_config() {
        let dir = std::env::temp_dir().join(format!("dakken-config-{}", std::process::id()));
        let path = dir.join("config.toml");
        let mut a = app(&["hi"]);
        a.config_path = Some(path.clone());
        a.on_key(press(KeyCode::Esc, KeyModifiers::NONE), Instant::now());
        a.on_key(
            press(KeyCode::Char('t'), KeyModifiers::NONE),
            Instant::now(),
        );
        a.on_key(
            press(KeyCode::Char('p'), KeyModifiers::NONE),
            Instant::now(),
        );
        a.on_key(press(KeyCode::Down, KeyModifiers::NONE), Instant::now());
        a.on_key(press(KeyCode::Enter, KeyModifiers::NONE), Instant::now());
        let saved = Config::load(&path);
        assert_eq!(saved.theme, Theme::default().next());
        assert!(saved.options.punctuation);
        assert_eq!(saved.mode, MODES[1]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn tab_opens_and_closes_the_menu() {
        let mut a = app(&["hi"]);
        a.on_key(press(KeyCode::Tab, KeyModifiers::NONE), Instant::now());
        assert!(matches!(a.screen, Screen::Menu { .. }));
        a.on_key(press(KeyCode::Tab, KeyModifiers::NONE), Instant::now());
        assert!(matches!(a.screen, Screen::Typing));
    }

    #[test]
    fn l_in_menu_changes_the_word_list() {
        let mut a = app(&["hi"]);
        a.on_key(press(KeyCode::Esc, KeyModifiers::NONE), Instant::now());
        a.on_key(
            press(KeyCode::Char('l'), KeyModifiers::NONE),
            Instant::now(),
        );
        assert_eq!(a.options.list, WordList::English1k);
        assert!(a.label().ends_with(" · english-1k"), "{}", a.label());
        let list = WordList::English1k.words();
        assert!(a.session.words().iter().all(|w| {
            let w: String = w.iter().collect();
            list.contains(&w.as_str())
        }));
    }

    #[test]
    fn wrong_words_are_practised_until_typed_right() {
        let mut a = app(&["hello", "world"]);
        type_text(&mut a, "hellp");
        a.on_key(
            press(KeyCode::Char(' '), KeyModifiers::NONE),
            Instant::now(),
        );
        type_text(&mut a, "world");
        assert_eq!(a.mistakes.words(), ["hello"]);

        // The mistakes list gives the saved words, and a right word leaves it.
        a.options.list = WordList::Mistakes;
        a.restart();
        let word: String = a.session.words()[0].iter().collect();
        assert_eq!(word, "hello");
        a.session = Session::from_words(a.mode, &["hello"]);
        type_text(&mut a, "hello");
        assert!(a.mistakes.words().is_empty());
    }

    #[test]
    fn failed_master_tests_are_not_saved() {
        let mut a = app(&["hi"]);
        a.strict = Strict::Master;
        a.session.set_strict(Strict::Master);
        type_text(&mut a, "x");
        assert!(matches!(a.screen, Screen::Results(_)));
        assert!(a.session.is_failed());
        assert!(a.history.records().is_empty());
        assert!(a.label().ends_with(" · master"));
    }

    #[test]
    fn m_in_menu_changes_the_strict_mode() {
        let mut a = app(&["hi"]);
        a.on_key(press(KeyCode::Esc, KeyModifiers::NONE), Instant::now());
        a.on_key(
            press(KeyCode::Char('m'), KeyModifiers::NONE),
            Instant::now(),
        );
        assert_eq!(a.strict, Strict::StopOnError);
    }

    #[test]
    fn custom_text_is_typed_in_order_and_restarts() {
        let dir = std::env::temp_dir().join(format!("dakken-custom-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("notes.txt");
        std::fs::write(&path, "Hello,\tworld\n  again ").unwrap();
        let custom = CustomText::load(&path).unwrap();
        std::fs::write(&path, " \n").unwrap();
        assert!(CustomText::load(&path).is_err(), "a file with no text");
        std::fs::remove_dir_all(&dir).unwrap();

        let mut config = Config::default();
        config.options.punctuation = true;
        let mut a = App::new(
            config,
            None,
            80,
            History::in_memory(),
            Mistakes::in_memory(),
            Some(custom),
        );
        assert_eq!(a.mode, Mode::Custom);
        assert_eq!(a.label(), "custom · notes.txt");
        let words: Vec<String> = a
            .session
            .words()
            .iter()
            .map(|w| w.iter().collect())
            .collect();
        assert_eq!(words, ["Hello,", "world", "again"]);
        type_text(&mut a, "Hello, world again");
        assert!(matches!(a.screen, Screen::Results(_)));
        a.on_key(press(KeyCode::Enter, KeyModifiers::NONE), Instant::now());
        assert_eq!(a.session.words().len(), 3, "Enter restarts the same text");
    }

    #[test]
    fn enter_finishes_a_zen_test() {
        let mut a = app(&["hi"]);
        a.mode = Mode::Zen;
        a.session = Session::zen();
        a.options.punctuation = true;
        type_text(&mut a, "free text");
        assert!(matches!(a.screen, Screen::Typing));
        a.on_key(press(KeyCode::Enter, KeyModifiers::NONE), Instant::now());
        assert!(matches!(a.screen, Screen::Results(_)));
        assert_eq!(a.history.records()[0].label, "zen");
    }

    #[test]
    fn quote_lengths_pick_quotes_of_that_length() {
        for (length, ok) in [
            (QuoteLength::Short, 1..=15),
            (QuoteLength::Medium, 16..=30),
            (QuoteLength::Long, 31..=usize::MAX),
        ] {
            let config = Config {
                mode: Mode::Quote(length),
                ..Config::default()
            };
            let mut a = App::new(
                config,
                None,
                80,
                History::in_memory(),
                Mistakes::in_memory(),
                None,
            );
            for _ in 0..20 {
                a.new_test();
                assert!(ok.contains(&a.session.words().len()), "{length:?}");
            }
            assert_eq!(a.label(), format!("quote {}", length.name()));
        }
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
