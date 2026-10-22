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
use rand::seq::{IndexedRandom, SliceRandom};

use crate::badges::{self, Badges};
use crate::best_runs::BestRuns;
use crate::card;
use crate::challenge::{self, Challenge};
use crate::config::{Caret, Config};
use crate::history::{self, History, Record};
use crate::keystats::KeyStats;
use crate::layout::Layout;
use crate::mistakes::{self, Mistakes};
use crate::progress::{self, Level};
use crate::quests::{Quest, Quests};
use crate::race::{self, Race};
use crate::ui::{self, theme::Theme};
use std::cell::RefCell;
use std::rc::Rc;

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
        Mode::Momo => "momo".to_string(),
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
    /// The settings, each with its key.
    Menu,
    /// The list of modes, opened with `o` in the menu.
    Modes {
        selected: usize,
    },
    Replay(Box<Replay>),
    /// A new rank or a new badge on the full screen, before the results.
    /// `page` is an index into `Reward::celebrations`.
    Celebration {
        page: usize,
    },
    /// Everything about the user, opened with `f` in the menu.
    Profile {
        scroll: usize,
    },
}

/// One full-screen page of a reward.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Celebration {
    Rank(Level),
    Badge(&'static str),
}

/// The last test played again at the speed it was typed.
pub struct Replay {
    pub session: Session,
    start: Instant,
    /// Index of the next key in the log.
    next: usize,
}

/// What the last test gave: XP, a new level, new badges.
#[derive(Debug, Clone, PartialEq)]
pub struct Reward {
    pub xp: u64,
    pub before: Level,
    pub after: Level,
    pub badges: Vec<&'static str>,
    /// This test made the words of the week reach the weekly goal.
    pub goal_reached: bool,
    /// Boss words typed right, of the boss words reached, in a boss test.
    pub bosses: Option<(usize, usize)>,
    /// Key quests this test finished.
    pub quests: Vec<Quest>,
}

impl Reward {
    /// The full-screen pages: a new rank first, then each new badge.
    pub fn celebrations(&self) -> Vec<Celebration> {
        let rank_up = self.after.rank_index() > self.before.rank_index();
        let rank = rank_up.then_some(Celebration::Rank(self.after));
        rank.into_iter()
            .chain(self.badges.iter().map(|&b| Celebration::Badge(b)))
            .collect()
    }
}

pub struct App {
    pub mode: Mode,
    /// The day (since 1970-01-01 UTC) of the daily challenge, in daily mode.
    pub daily: Option<i64>,
    /// The race from `--race`: its words, and the ghost that types them.
    pub race: Option<Race>,
    /// Where `x` saved the last test as a race, or why it could not.
    pub race_saved: Option<String>,
    pub options: Options,
    pub theme: Theme,
    /// The theme from the `color_*` keys in the config, if any.
    pub custom_theme: Option<Theme>,
    /// Show the live WPM and accuracy during the test.
    pub live: bool,
    /// Show a pace caret at the personal-best speed.
    pub pace: bool,
    pub strict: Strict,
    pub challenge: Challenge,
    /// The round of the climb challenge. `None` starts a new climb.
    pub climb: Option<challenge::Climb>,
    pub caret: Caret,
    pub layout: Layout,
    /// Words to type each week. 0 is no goal.
    pub weekly_goal: u32,
    /// Ask GitHub once a day for a newer version.
    pub update_check: bool,
    /// Draw the screens with ASCII only (`symbols = "ascii"`).
    pub ascii: bool,
    /// The Linux text console, whose font has few symbols: the screens use
    /// ASCII there, but the saved `symbols` setting does not change.
    pub plain_console: bool,
    /// The terminal shows 24-bit color. Without it, the screens use the
    /// nearest of the 256 colors.
    pub truecolor: bool,
    /// The tag of a newer release, for the notice in the menu.
    pub update: Option<String>,
    /// `u` in the menu: quit, then update dakken.
    pub update_now: bool,
    /// The menu saves the settings here. `None` does not save them.
    pub config_path: Option<PathBuf>,
    /// Terminal width and height. Untimed tests get the words that fit on
    /// the screen.
    pub size: (u16, u16),
    pub session: Session,
    /// Where the quote of a quote test comes from.
    pub source: Option<&'static str>,
    /// Momo mode is on. Only `x` starts it; `self.mode` stays the chosen
    /// mode, which `Enter` starts.
    pub momo: bool,
    /// The quotes of the momo test, in order, as the test takes them.
    pub momo_quotes: Rc<RefCell<Vec<words::MomoQuote>>>,
    /// The text for custom mode, from `--file`.
    pub custom: Option<CustomText>,
    pub screen: Screen,
    pub history: History,
    pub mistakes: Mistakes,
    pub key_stats: KeyStats,
    /// The key times of each personal-best test, for the pace caret.
    pub best_runs: BestRuns,
    pub badges: Badges,
    pub quests: Quests,
    /// What the last finished test gave. `None` for a failed test.
    pub reward: Option<Reward>,
    /// The last climb round, and if it was passed.
    pub climb_result: Option<(challenge::Climb, bool)>,
    /// Best WPM for this label before the last finished test.
    pub previous_best: Option<f64>,
    /// Text for the main loop to put on the clipboard.
    pub copy: Option<String>,
    /// The system clipboard command put the score card on the clipboard.
    pub copied: bool,
    /// The score card is also saved in this file, for terminals where the
    /// copy does not work.
    pub card_file: Option<PathBuf>,
    /// The last finished test could not be added to the history file.
    pub save_failed: bool,
    pub quit: bool,
}

impl App {
    pub fn new(
        config: Config,
        config_path: Option<PathBuf>,
        size: (u16, u16),
        history: History,
        mistakes: Mistakes,
        key_stats: KeyStats,
        custom: Option<CustomText>,
    ) -> Self {
        let mode = match custom {
            Some(_) => Mode::Custom,
            None => config.mode,
        };
        let mut app = Self {
            options: config.options,
            theme: config.theme,
            custom_theme: config.custom_theme,
            live: config.live,
            pace: config.pace,
            strict: config.strict,
            challenge: config.challenge,
            climb: None,
            caret: config.caret,
            layout: config.layout,
            weekly_goal: config.weekly_goal,
            update_check: config.update_check,
            ascii: config.ascii,
            plain_console: std::env::var_os("TERM").is_some_and(|t| t == "linux"),
            truecolor: ui::color256::truecolor(),
            config_path,
            size,
            history,
            mistakes,
            key_stats,
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
            daily: None,
            race: None,
            race_saved: None,
            options: Options::default(),
            theme: Theme::default(),
            custom_theme: None,
            live: false,
            pace: false,
            strict: Strict::Off,
            challenge: Challenge::Off,
            climb: None,
            caret: Caret::Block,
            layout: Layout::Qwerty,
            weekly_goal: 0,
            update_check: false,
            ascii: false,
            plain_console: false,
            truecolor: true,
            update: None,
            update_now: false,
            config_path: None,
            size: (80, 24),
            session,
            source: None,
            momo: false,
            momo_quotes: Rc::default(),
            custom: None,
            screen: Screen::Typing,
            history: History::in_memory(),
            mistakes: Mistakes::in_memory(),
            key_stats: KeyStats::in_memory(),
            best_runs: BestRuns::in_memory(),
            badges: Badges::in_memory(),
            quests: Quests::in_memory(),
            reward: None,
            climb_result: None,
            previous_best: None,
            copy: None,
            copied: false,
            card_file: None,
            save_failed: false,
            quit: false,
        }
    }

    /// Start the daily challenge for `day`: the same words for everyone.
    pub fn start_daily(&mut self, day: i64) {
        self.momo = false;
        self.daily = Some(day);
        self.mode = Mode::Words(words::DAILY_WORDS);
        self.new_test();
    }

    /// Start momo mode at once: quotes one after the other for 60 seconds.
    pub fn start_momo(&mut self) {
        self.daily = None;
        self.race = None;
        self.momo = true;
        self.restart();
    }

    /// Leave momo mode and start a test of the chosen mode.
    fn start_chosen(&mut self) {
        self.momo = false;
        self.restart();
    }

    /// The challenge of the test on screen. Momo mode is a mode on its own,
    /// so it has no challenge, whatever the menu says.
    pub fn challenge_now(&self) -> Challenge {
        if self.momo {
            Challenge::Off
        } else {
            self.challenge
        }
    }

    /// The strict mode of the test on screen. Momo mode has none, whatever
    /// the menu says.
    pub fn strict_now(&self) -> Strict {
        if self.momo { Strict::Off } else { self.strict }
    }

    /// The mode of the test on screen: momo, or the chosen mode.
    pub fn mode_now(&self) -> Mode {
        if self.momo { Mode::Momo } else { self.mode }
    }

    /// A momo session: the momo quotes in random order, one after the
    /// other, again from the start after the last one.
    fn momo_session(&mut self) -> Session {
        let used = Rc::new(RefCell::new(Vec::new()));
        self.momo_quotes = Rc::clone(&used);
        let mut quotes = words::momo_quotes();
        let mut next = 0;
        let mut words = std::collections::VecDeque::new();
        let ends = Rc::new(RefCell::new(std::collections::BTreeSet::new()));
        let quote_ends = Rc::clone(&ends);
        let mut total = 0;
        let mut session = Session::new(Mode::Momo, move || {
            loop {
                if let Some(word) = words.pop_front() {
                    return word;
                }
                if next == 0 {
                    quotes.shuffle(&mut rand::rng());
                }
                let quote = quotes[next].clone();
                next = (next + 1) % quotes.len();
                words.extend(quote.text.split_whitespace().map(String::from));
                total += quote.text.split_whitespace().count();
                quote_ends.borrow_mut().insert(total - 1);
                used.borrow_mut().push(quote);
            }
        });
        session.set_quote_ends(ends);
        session
    }

    /// The momo quotes typed to the end in this test.
    pub fn momo_done(&self) -> Vec<words::MomoQuote> {
        let mut left = self.session.checked_words().len();
        let mut done = Vec::new();
        for quote in self.momo_quotes.borrow().iter() {
            let n = quote.text.split_whitespace().count();
            if n > left {
                break;
            }
            left -= n;
            done.push(quote.clone());
        }
        done
    }

    /// The momo quote at the caret of `s`, and the range of its words.
    pub fn momo_current(&self, s: &Session) -> Option<(std::ops::Range<usize>, words::MomoQuote)> {
        let current = s.current().min(s.words().len().saturating_sub(1));
        let mut start = 0;
        for quote in self.momo_quotes.borrow().iter() {
            let end = start + quote.text.split_whitespace().count();
            if current < end {
                return Some((start..end.min(s.words().len()), quote.clone()));
            }
            start = end;
        }
        None
    }

    /// Race the ghost of a saved test: the same words, with its pace caret.
    pub fn start_race(&mut self, race: Race) {
        self.momo = false;
        self.race = Some(race);
        self.new_test();
    }

    /// The daily challenge and races have their own text, so challenges
    /// and the mode do not apply.
    fn fixed_text(&self) -> bool {
        self.daily.is_some() || self.race.is_some()
    }

    /// Save the finished test as a race file, for `--race`.
    fn save_race(&mut self) {
        if self.session.checked_words().is_empty() {
            self.race_saved = Some("race not saved: no words typed".to_string());
            return;
        }
        let result = self.session.result();
        let race = Race {
            label: self.label(),
            wpm: result.wpm,
            accuracy: result.accuracy,
            // Only the words the test reached: time mode has more ready.
            words: (self.session.checked_words().into_iter())
                .map(|(word, _)| word)
                .collect(),
            times: (self.session.presses().iter())
                .filter(|p| p.correct)
                .map(|p| p.at.as_millis() as u32)
                .collect(),
        };
        let now = self.history.records().last().map_or(0, |r| r.time);
        let saved = match race::default_dir() {
            Some(dir) => race.save(&dir, now),
            None => Err(std::io::Error::other("no data folder")),
        };
        self.race_saved = Some(match saved {
            Ok(path) => format!("race saved: {}", path.display()),
            Err(e) => format!("race not saved: {e}"),
        });
    }

    fn restart(&mut self) {
        self.new_test();
        self.screen = Screen::Typing;
    }

    fn new_test(&mut self) {
        (self.session, self.source) = match &self.custom {
            _ if self.daily.is_some() => {
                let words = words::daily_words(self.daily.unwrap_or_default());
                (Session::from_words(self.mode, &words), None)
            }
            _ if self.race.is_some() => {
                let words = self
                    .race
                    .as_ref()
                    .map(|r| r.words.clone())
                    .unwrap_or_default();
                (Session::from_words(Mode::Words(words.len()), &words), None)
            }
            _ if self.momo => (self.momo_session(), None),
            Some(custom) if self.mode == Mode::Custom => {
                (Session::from_words(self.mode, &custom.words), None)
            }
            _ => {
                let boss = self.boss_on().then(|| self.weak_pairs());
                let mode = if self.climb_on() {
                    Mode::Words(challenge::CLIMB_WORDS)
                } else {
                    self.mode
                };
                new_test(mode, self.options, self.size, self.practice_words(), boss)
            }
        };
        self.session.set_strict(self.strict_now());
        if self.climb_on() && self.climb.is_none() {
            self.climb = Some(challenge::Climb::start(self.history.records()));
        }
        if !self.fixed_text() && self.challenge_now() == Challenge::Survival {
            self.session.set_lives(challenge::LIVES);
        }
    }

    /// Remember the words typed wrong. In the mistakes list, words typed
    /// right are removed.
    fn update_mistakes(&mut self) -> std::io::Result<()> {
        // Quote, custom and zen tests have their own text, not a word list.
        let own_text = matches!(
            self.mode_now(),
            Mode::Quote(_) | Mode::Momo | Mode::Custom | Mode::Zen
        );
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

    /// The XP and new badges of the test just added to the history.
    fn give_reward(&mut self, xp_before: u64, had_mistakes: bool) -> Reward {
        let own_text = matches!(
            self.mode_now(),
            Mode::Quote(_) | Mode::Momo | Mode::Custom | Mode::Zen
        );
        let practising = !own_text && self.options.list == WordList::Mistakes;
        let cleared = practising && had_mistakes && self.mistakes.words().is_empty();
        let now = self.history.records().last().map_or(0, |r| r.time);
        let (done_quests, saved) =
            (self.quests).update(self.session.presses(), &self.key_stats, now);
        self.save_failed |= saved.is_err();
        let result = self.session.result();
        let climb_round = match (self.climb_on(), self.climb) {
            (true, Some(c)) if c.passed(result.wpm, result.accuracy) => c.round,
            _ => 0,
        };
        let moment = badges::Moment {
            cleared_mistakes: cleared,
            beat_ghost: self.race.as_ref().is_some_and(|r| result.wpm > r.wpm),
            climb_round,
            quests_done: self.quests.done_count(),
            local_hour: crate::localtime::hour(),
        };
        let records = self.history.records();
        let earned = badges::earned(records, &moment, self.weekly_goal);
        let goal = f64::from(self.weekly_goal);
        let week = progress::this_week(records, history::today());
        let last = records.last().map_or(0.0, progress::words);
        let goal_reached = self.weekly_goal > 0 && week >= goal && week - last < goal;
        let (new, saved) = self.badges.add(&earned);
        self.save_failed |= saved.is_err();
        let xp_after = progress::total_xp(self.history.records());
        Reward {
            xp: xp_after - xp_before,
            before: Level::from_xp(xp_before),
            after: Level::from_xp(xp_after),
            badges: new,
            goal_reached,
            bosses: self.boss_on().then(|| self.bosses()),
            quests: done_quests,
        }
    }

    /// A finished test with a target text can be saved as a race.
    pub fn can_save_race(&self) -> bool {
        self.reward.is_some() && self.mode_now() != Mode::Zen && self.race_saved.is_none()
    }

    /// The theme on the screen: the momo theme on the momo test, its replay
    /// and its results, else the chosen theme. The menu always has the
    /// chosen theme.
    pub fn theme(&self) -> Theme {
        let menu = matches!(
            self.screen,
            Screen::Menu | Screen::Modes { .. } | Screen::Profile { .. }
        );
        if self.momo && !menu {
            ui::theme::MOMO
        } else {
            self.theme
        }
    }

    /// This test is a round of the climb challenge.
    pub fn climb_on(&self) -> bool {
        let own_text = matches!(self.mode_now(), Mode::Momo | Mode::Custom | Mode::Zen);
        self.challenge_now() == Challenge::Climb && !self.fixed_text() && !own_text
    }

    /// Boss words are in this test: the boss challenge, with random words.
    pub fn boss_on(&self) -> bool {
        let own_text = matches!(
            self.mode_now(),
            Mode::Quote(_) | Mode::Momo | Mode::Custom | Mode::Zen
        );
        self.challenge_now() == Challenge::Boss && !self.fixed_text() && !own_text
    }

    /// The boss words typed right, and the boss words reached.
    fn bosses(&self) -> (usize, usize) {
        let checked = self.session.checked_words();
        let bosses: Vec<bool> = (checked.iter().enumerate())
            .filter(|(i, _)| challenge::is_boss(*i))
            .map(|(_, (_, correct))| *correct)
            .collect();
        (bosses.iter().filter(|&&c| c).count(), bosses.len())
    }

    /// The score card of the daily challenge just finished.
    fn score_card(&self, day: i64, wpm: f64, accuracy: f64) -> String {
        let words: Vec<bool> = self
            .session
            .checked_words()
            .into_iter()
            .map(|(_, correct)| correct)
            .collect();
        let streak = history::daily_streak(&self.history, day).days;
        card::score_card(day, wpm, accuracy, &words, streak)
    }

    /// Save the score card next to the history, for terminals where the
    /// copy does not work. Gives the file, or `None` with no data folder.
    fn save_card(&self, card: &str) -> Option<PathBuf> {
        let path = self
            .config_path
            .as_deref()?
            .with_file_name("score-card.txt");
        std::fs::write(&path, card).ok()?;
        Some(path)
    }

    /// The letters the weak-keys list practises: the letters of the open
    /// key quests first, then the weakest letters, 5 in all.
    pub fn weak_letters(&self) -> Vec<char> {
        let mut letters: Vec<char> = self.quests.open().map(|q| q.key).collect();
        for c in self.key_stats.weakest_letters(5) {
            if !letters.contains(&c) {
                letters.push(c);
            }
        }
        letters.truncate(5);
        letters
    }

    /// The letter pairs the weak-pairs list practises.
    pub fn weak_pairs(&self) -> Vec<String> {
        self.key_stats.weakest_pairs(5)
    }

    /// The words of the mistakes, weak-keys and weak-pairs lists. `None`
    /// uses the list's own words: English for mistakes, all of english-1k for
    /// weak keys and weak pairs.
    fn practice_words(&self) -> Option<Vec<String>> {
        let words: Vec<String> = match self.options.list {
            WordList::Mistakes => self.mistakes.words().to_vec(),
            WordList::WeakKeys => {
                let weak = self.weak_letters();
                WordList::WeakKeys
                    .words()
                    .into_iter()
                    .filter(|w| w.chars().any(|c| weak.contains(&c)))
                    .map(String::from)
                    .collect()
            }
            WordList::WeakPairs => {
                let weak = self.weak_pairs();
                WordList::WeakPairs
                    .words()
                    .into_iter()
                    .filter(|w| weak.iter().any(|p| w.contains(p.as_str())))
                    .map(String::from)
                    .collect()
            }
            _ => Vec::new(),
        };
        (!words.is_empty()).then_some(words)
    }

    /// Save the mode, options and theme. Settings are not important enough to
    /// stop the test, so a failed save is ignored.
    fn save_config(&self) {
        if let Some(path) = &self.config_path {
            // Custom mode needs a file from the command line, so keep the
            // saved mode instead.
            let mode = match self.mode {
                _ if self.fixed_text() => Config::load(path).mode,
                Mode::Custom => Config::load(path).mode,
                mode => mode,
            };
            let config = Config {
                mode,
                options: self.options,
                theme: self.theme,
                custom_theme: self.custom_theme,
                live: self.live,
                pace: self.pace,
                strict: self.strict,
                challenge: self.challenge,
                caret: self.caret,
                layout: self.layout,
                weekly_goal: self.weekly_goal,
                update_check: self.update_check,
                ascii: self.ascii,
            };
            let _ = config.save(path);
        }
    }

    /// Before the test starts, give an untimed test the words for the new width.
    pub fn on_resize(&mut self, width: u16, height: u16) {
        self.size = (width, height);
        if self.mode == Mode::Untimed && !self.session.is_started() {
            self.new_test();
        }
    }

    /// The mode and its options, for example "time 30 · punctuation".
    /// Quotes have their own punctuation and numbers, so options do not apply.
    pub fn label(&self) -> String {
        // One fixed text, so options do not apply.
        if self.daily.is_some() {
            return "daily".to_string();
        }
        if self.race.is_some() {
            return "race".to_string();
        }
        // Rounds of fixed length, so the mode does not apply.
        if self.climb_on() {
            return "climb".to_string();
        }
        let mut parts = vec![mode_label(self.mode_now())];
        if let (Mode::Custom, Some(custom)) = (self.mode, &self.custom) {
            parts.push(custom.name.clone());
        }
        // These modes have their own text, so word options do not apply.
        let own_text = matches!(
            self.mode_now(),
            Mode::Quote(_) | Mode::Momo | Mode::Custom | Mode::Zen
        );
        if !own_text && self.options.punctuation {
            parts.push("punctuation".to_string());
        }
        if !own_text && self.options.numbers {
            parts.push("numbers".to_string());
        }
        if self.strict_now() != Strict::Off {
            parts.push(self.strict_now().name().to_string());
        }
        if self.challenge_now() != Challenge::Off {
            parts.push(self.challenge_now().name().to_string());
        }
        // The defaults add nothing, so old history labels stay the same.
        if !own_text && self.options.list != WordList::English {
            parts.push(self.options.list.name().to_string());
        }
        if self.layout != Layout::Qwerty {
            parts.push(self.layout.name().to_string());
        }
        parts.join(" · ")
    }

    /// Show the results screen if the test ended. Call this on each frame.
    pub fn tick(&mut self, now: Instant) {
        if let Screen::Replay(replay) = &mut self.screen {
            let log = self.session.log();
            let since = now.saturating_duration_since(replay.start);
            while let Some(&(at, key)) = log.get(replay.next).filter(|(at, _)| *at <= since) {
                replay.session.handle(key, replay.start + at);
                replay.next += 1;
            }
            replay.session.tick(now);
            // Stay one second on the end, then go back to the results.
            let end = log.last().map_or(Duration::ZERO, |(at, _)| *at);
            if replay.next == log.len() && since >= end + Duration::from_secs(1) {
                self.screen = Screen::Results(self.session.result());
            }
            return;
        }
        self.session.tick(now);
        if matches!(self.screen, Screen::Typing) && self.session.is_finished() {
            let result = self.session.result();
            let label = self.label();
            self.previous_best = self.history.best(&label);
            let xp_before = progress::total_xp(self.history.records());
            let had_mistakes = !self.mistakes.words().is_empty();
            // A failed test is not a real result, so it is not saved.
            let failed = self.session.is_failed();
            if !failed {
                let new_best = self.previous_best.is_none_or(|best| result.wpm > best);
                let presses = self.session.presses();
                self.save_failed = (new_best && self.mode != Mode::Zen)
                    && self.best_runs.set(&label, presses).is_err();
                let mut record = Record::new(label, &result);
                if self.boss_on() {
                    record.bonus = self.bosses().0 as u64 * challenge::BOSS_XP;
                }
                self.save_failed |= self.history.add(record).is_err();
                self.save_failed |= self.key_stats.add_test(self.session.presses()).is_err();
            }
            self.save_failed |= self.update_mistakes().is_err();
            self.reward = (!failed).then(|| self.give_reward(xp_before, had_mistakes));
            self.climb_result = None;
            if let (true, Some(climb)) = (self.climb_on(), self.climb) {
                let passed = !failed && climb.passed(result.wpm, result.accuracy);
                self.climb_result = Some((climb, passed));
                self.climb = passed.then(|| climb.next());
            }
            self.copied = false;
            self.card_file = None;
            self.race_saved = None;
            let celebrate = self
                .reward
                .as_ref()
                .is_some_and(|r| !r.celebrations().is_empty());
            self.screen = if celebrate {
                Screen::Celebration { page: 0 }
            } else {
                Screen::Results(result)
            };
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
                    if let Some(k) = engine_key(key, self.layout) {
                        self.session.handle(k, now);
                        self.tick(now);
                    }
                }
            },
            Screen::Replay(_) => self.screen = Screen::Results(self.session.result()),
            Screen::Results(ref result) => match key.code {
                KeyCode::Enter => self.restart(),
                KeyCode::Char('r') if !self.session.log().is_empty() => {
                    self.screen = Screen::Replay(Box::new(Replay {
                        session: self.session.replay(),
                        start: now,
                        next: 0,
                    }));
                }
                KeyCode::Char('x') if self.can_save_race() => self.save_race(),
                KeyCode::Char('q') => self.quit = true,
                KeyCode::Char('y') if self.reward.is_some() => {
                    if let Some(day) = self.daily {
                        let card = self.score_card(day, result.wpm, result.accuracy);
                        self.card_file = self.save_card(&card);
                        self.copy = Some(card);
                    }
                }
                KeyCode::Tab | KeyCode::Esc => self.open_menu(),
                _ => {}
            },
            Screen::Celebration { page } => {
                if matches!(key.code, KeyCode::Tab | KeyCode::Esc) {
                    let pages = self.reward.as_ref().map_or(0, |r| r.celebrations().len());
                    self.screen = if page + 1 < pages {
                        Screen::Celebration { page: page + 1 }
                    } else {
                        Screen::Results(self.session.result())
                    };
                }
            }
            Screen::Profile { scroll } => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    let scroll = scroll.saturating_sub(1);
                    self.screen = Screen::Profile { scroll };
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let scroll = (scroll + 1).min(ui::profile_max_scroll(self));
                    self.screen = Screen::Profile { scroll };
                }
                KeyCode::Tab | KeyCode::Esc => self.screen = Screen::Menu,
                _ => {}
            },
            Screen::Modes { selected } => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    let selected = (selected + MODES.len() - 1) % MODES.len();
                    self.screen = Screen::Modes { selected };
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let selected = (selected + 1) % MODES.len();
                    self.screen = Screen::Modes { selected };
                }
                KeyCode::Enter => {
                    self.daily = None;
                    self.race = None;
                    self.momo = false;
                    self.mode = MODES[selected];
                    self.save_config();
                    self.new_test();
                    self.screen = Screen::Menu;
                }
                KeyCode::Tab | KeyCode::Esc => self.screen = Screen::Menu,
                _ => {}
            },
            Screen::Menu => match key.code {
                KeyCode::Enter => self.start_chosen(),
                KeyCode::Char('x') => self.start_momo(),
                KeyCode::Char('q') => self.quit = true,
                KeyCode::Char('f') => self.screen = Screen::Profile { scroll: 0 },
                KeyCode::Char('u') if self.update.is_some() => {
                    self.update_now = true;
                    self.quit = true;
                }
                KeyCode::Char('o') => {
                    let selected = MODES.iter().position(|&m| m == self.mode).unwrap_or(0);
                    self.screen = Screen::Modes { selected };
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
                    self.theme = self.theme.next(self.custom_theme);
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
                KeyCode::Char('g') => {
                    self.challenge = self.challenge.next();
                    self.climb = None;
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
        self.screen = Screen::Menu;
    }

    /// Press tab through the celebration pages, to the results.
    #[cfg(test)]
    pub fn skip_celebration(&mut self) {
        while let Screen::Celebration { .. } = self.screen {
            self.on_key(
                KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE),
                Instant::now(),
            );
        }
    }
}

/// A new test, and the source of its quote in quote mode.
fn new_test(
    mode: Mode,
    options: Options,
    size: (u16, u16),
    practice: Option<Vec<String>>,
    boss: Option<Vec<String>>,
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
    (new_session(mode, options, size, practice, boss), None)
}

/// A session of random words. With `boss` (the weak letter pairs), every
/// 25th word is a boss word.
fn new_session(
    mode: Mode,
    options: Options,
    (width, height): (u16, u16),
    practice: Option<Vec<String>>,
    boss: Option<Vec<String>>,
) -> Session {
    let list: Vec<String> =
        practice.unwrap_or_else(|| options.list.words().into_iter().map(String::from).collect());
    let bosses = boss.map(|pairs| challenge::boss_words(&list, &pairs));
    let mut words = WordGen::new(list, rand::rng())
        .with_punctuation(options.punctuation)
        .with_numbers(options.numbers);
    let mut index = 0;
    let mut next_word = move || {
        index += 1;
        match &bosses {
            Some(bosses) if challenge::is_boss(index - 1) => bosses
                .choose(&mut rand::rng())
                .cloned()
                .unwrap_or_else(|| words.next_word()),
            _ => words.next_word(),
        }
    };
    if mode == Mode::Untimed {
        let words = ui::screen_words(width, height, &mut next_word);
        return Session::from_words(mode, &words);
    }
    Session::new(mode, next_word)
}

fn engine_key(key: KeyEvent, layout: Layout) -> Option<Key> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    match key.code {
        KeyCode::Backspace if ctrl || alt => Some(Key::DeleteWord),
        KeyCode::Backspace => Some(Key::Backspace),
        KeyCode::Char('w') if ctrl && !alt => Some(Key::DeleteWord),
        // Many Linux and macOS terminals send Ctrl+Backspace as Ctrl+H.
        KeyCode::Char('h') if ctrl && !alt => Some(Key::DeleteWord),
        KeyCode::Char(' ') => Some(Key::Space),
        // AltGr arrives as Ctrl+Alt, so allow that for letters on some layouts.
        KeyCode::Char(c) if ctrl == alt => Some(Key::Char(layout.map(c))),
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

    #[test]
    fn q_quits_on_the_menu_and_the_results_but_types_in_a_test() {
        let mut a = app(&["quiet"]);
        a.on_key(
            press(KeyCode::Char('q'), KeyModifiers::NONE),
            Instant::now(),
        );
        assert!(!a.quit);
        type_text(&mut a, "uiet");
        a.skip_celebration();
        assert!(matches!(a.screen, Screen::Results(_)));
        a.on_key(
            press(KeyCode::Char('q'), KeyModifiers::NONE),
            Instant::now(),
        );
        assert!(a.quit);
        let mut b = app(&["hi"]);
        b.on_key(press(KeyCode::Tab, KeyModifiers::NONE), Instant::now());
        b.on_key(
            press(KeyCode::Char('q'), KeyModifiers::NONE),
            Instant::now(),
        );
        assert!(b.quit);
    }

    #[test]
    fn ctrl_backspace_in_every_form_deletes_a_word() {
        for key in [
            press(KeyCode::Backspace, KeyModifiers::CONTROL),
            press(KeyCode::Backspace, KeyModifiers::ALT),
            press(KeyCode::Char('w'), KeyModifiers::CONTROL),
            press(KeyCode::Char('h'), KeyModifiers::CONTROL),
        ] {
            assert_eq!(engine_key(key, Layout::Qwerty), Some(Key::DeleteWord));
        }
        // AltGr arrives as Ctrl+Alt, so it still types its letter.
        let altgr = KeyModifiers::CONTROL | KeyModifiers::ALT;
        assert_eq!(
            engine_key(press(KeyCode::Char('h'), altgr), Layout::Qwerty),
            Some(Key::Char('h'))
        );
    }

    #[test]
    fn accented_letters_from_dead_keys_type_the_word() {
        // A dead key gives no event; the next key gives the composed letter.
        let mut a = app(&["über", "señor"]);
        type_text(&mut a, "über señor");
        assert!(a.session.is_finished());
        assert_eq!(a.session.result().chars.incorrect, 0);
    }

    #[test]
    fn daily_challenge_has_its_own_label_and_fixed_words() {
        let mut a = app(&["hi"]);
        a.options.punctuation = true;
        a.start_daily(20_380);
        assert_eq!(a.label(), "daily");
        assert_eq!(a.mode, Mode::Words(words::DAILY_WORDS));
        let first = a.session.words().to_vec();
        a.restart();
        assert_eq!(a.session.words(), first);
        // Choosing a mode in the menu ends the daily challenge.
        a.open_menu();
        a.on_key(
            press(KeyCode::Char('o'), KeyModifiers::NONE),
            Instant::now(),
        );
        a.on_key(press(KeyCode::Enter, KeyModifiers::NONE), Instant::now());
        assert_eq!(a.daily, None);
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
    fn a_new_badge_shows_on_the_full_screen_before_the_results() {
        let mut a = app(&["hi"]);
        type_text(&mut a, "hi");
        assert!(matches!(a.screen, Screen::Celebration { page: 0 }));
        // Other keys do nothing, so a late key does not skip the page.
        type_text(&mut a, "x");
        a.on_key(press(KeyCode::Enter, KeyModifiers::NONE), Instant::now());
        assert!(matches!(a.screen, Screen::Celebration { page: 0 }));
        a.on_key(press(KeyCode::Tab, KeyModifiers::NONE), Instant::now());
        assert!(matches!(a.screen, Screen::Results(_)));
        // With no new badge, the results come at once.
        let mut b = app(&["hi"]);
        let all: Vec<&str> = badges::BADGES.iter().map(|b| b.name).collect();
        let _ = b.badges.add(&all);
        type_text(&mut b, "hi");
        assert!(matches!(b.screen, Screen::Results(_)));
    }

    #[test]
    fn a_new_rank_comes_before_the_new_badges() {
        let reward = Reward {
            xp: 0,
            before: Level::from_xp(0),
            after: Level::from_xp(100_000),
            badges: vec!["first press", "clean"],
            goal_reached: false,
            bosses: None,
            quests: Vec::new(),
        };
        assert_eq!(
            reward.celebrations(),
            [
                Celebration::Rank(Level::from_xp(100_000)),
                Celebration::Badge("first press"),
                Celebration::Badge("clean"),
            ]
        );
        // A new level in the same rank is not a page.
        let level_up = Reward {
            after: Level::from_xp(600),
            badges: Vec::new(),
            ..reward
        };
        assert!(level_up.celebrations().is_empty());
    }

    #[test]
    fn f_in_the_menu_opens_the_profile() {
        let mut a = app(&["hi"]);
        a.size = (80, 10);
        a.open_menu();
        a.on_key(
            press(KeyCode::Char('f'), KeyModifiers::NONE),
            Instant::now(),
        );
        assert!(matches!(a.screen, Screen::Profile { scroll: 0 }));
        a.on_key(press(KeyCode::Up, KeyModifiers::NONE), Instant::now());
        assert!(matches!(a.screen, Screen::Profile { scroll: 0 }));
        for _ in 0..500 {
            a.on_key(press(KeyCode::Down, KeyModifiers::NONE), Instant::now());
        }
        // The scroll stops at the end of the profile.
        let end = ui::profile_max_scroll(&a);
        assert!(end > 0);
        assert!(matches!(a.screen, Screen::Profile { scroll } if scroll == end));
        a.on_key(press(KeyCode::Tab, KeyModifiers::NONE), Instant::now());
        assert!(matches!(a.screen, Screen::Menu));
    }

    #[test]
    fn finishing_the_test_shows_results() {
        let mut a = app(&["hi"]);
        type_text(&mut a, "hi");
        a.skip_celebration();
        assert!(matches!(a.screen, Screen::Results(_)));
    }

    #[test]
    fn finished_tests_go_into_the_history() {
        let mut a = app(&["hi"]);
        type_text(&mut a, "hi");
        assert_eq!(a.previous_best, None);
        // The first test is the best, so the pace caret can race its key times.
        assert_eq!(a.best_runs.get(&a.label()).map(<[u32]>::len), Some(2));
        a.session = Session::from_words(a.mode, &["hi"]);
        a.screen = Screen::Typing;
        type_text(&mut a, "hi");
        assert_eq!(a.history.records().len(), 2);
        assert!(a.previous_best.is_some());
        assert!(!a.save_failed);
    }

    #[test]
    fn finished_tests_give_xp_and_badges_once() {
        let mut a = app(&["hi"]);
        type_text(&mut a, "hi");
        let reward = a.reward.clone().expect("a finished test has a reward");
        assert_eq!(reward.before.level, 1);
        assert_eq!(reward.badges, ["first press"]);
        a.session = Session::from_words(a.mode, &["hi"]);
        a.screen = Screen::Typing;
        type_text(&mut a, "hi");
        assert!(a.reward.as_ref().is_some_and(|r| r.badges.is_empty()));
    }

    #[test]
    fn results_say_when_the_weekly_goal_is_reached() {
        let mut a = app(&["hello", "world"]);
        // One word is 5 characters, so "hello world" is 2 words.
        a.weekly_goal = 2;
        type_text(&mut a, "hello world");
        assert!(a.reward.as_ref().is_some_and(|r| r.goal_reached));
        // The next test is over the goal, but did not reach it.
        a.session = Session::from_words(a.mode, &["hello", "world"]);
        a.screen = Screen::Typing;
        type_text(&mut a, "hello world");
        assert!(a.reward.as_ref().is_some_and(|r| !r.goal_reached));
    }

    #[test]
    fn g_in_menu_turns_on_survival() {
        let mut a = app(&["hi", "there"]);
        a.open_menu();
        a.on_key(
            press(KeyCode::Char('g'), KeyModifiers::NONE),
            Instant::now(),
        );
        assert_eq!(a.challenge, Challenge::Survival);
        assert_eq!(a.session.lives(), Some(challenge::LIVES));
        assert!(a.label().ends_with(" · survival"), "{}", a.label());
        // The daily challenge has no lives.
        a.start_daily(20_381);
        assert_eq!(a.session.lives(), None);
    }

    #[test]
    fn boss_words_give_bonus_xp() {
        let words: Vec<String> = (0..30).map(|i| format!("w{i}")).collect();
        let mut a = app(&words.iter().map(String::as_str).collect::<Vec<_>>());
        a.challenge = Challenge::Boss;
        assert!(a.boss_on());
        // Word 25 (index 24) is the boss: type it right, and one other word wrong.
        let mut text = words.clone();
        text[3] = "xx".to_string();
        type_text(&mut a, &text.join(" "));
        let reward = a.reward.clone().expect("a reward");
        assert_eq!(reward.bosses, Some((1, 1)));
        assert_eq!(a.history.records()[0].bonus, challenge::BOSS_XP);
        assert!(reward.xp >= challenge::BOSS_XP);
    }

    #[test]
    fn a_boss_test_has_a_long_word_every_25_words() {
        let options = Options {
            list: WordList::English1k,
            ..Options::default()
        };
        let s = new_session(Mode::Words(50), options, (80, 24), None, Some(Vec::new()));
        for i in [24, 49] {
            assert!(s.words()[i].len() >= 7, "{:?}", s.words()[i]);
        }
    }

    #[test]
    fn climb_rounds_raise_the_target_until_one_fails() {
        let mut a = app(&["hi"]);
        a.challenge = Challenge::Climb;
        a.restart();
        assert_eq!(a.label(), "climb");
        assert_eq!(a.session.words().len(), challenge::CLIMB_WORDS);
        assert_eq!(a.climb.map(|c| (c.round, c.target)), Some((1, 30.0)));
        // Typing at once is very fast, so the round passes.
        let text: Vec<String> = (a.session.words().iter())
            .map(|w| w.iter().collect())
            .collect();
        type_text(&mut a, &text.join(" "));
        assert!(matches!(a.climb_result, Some((_, true))));
        assert_eq!(a.climb.map(|c| c.round), Some(2));
        // A round with errors fails, and the next test starts a new climb.
        a.restart();
        let wrong: Vec<String> = (0..challenge::CLIMB_WORDS).map(|_| "x".into()).collect();
        // The space after the short last word ends the test.
        type_text(&mut a, &(wrong.join(" ") + " "));
        assert!(matches!(a.climb_result, Some((_, false))));
        assert_eq!(a.climb, None);
        a.restart();
        assert_eq!(a.climb.map(|c| c.round), Some(1));
    }

    #[test]
    fn a_race_uses_the_saved_words_and_its_own_label() {
        let mut a = app(&["hi"]);
        a.challenge = Challenge::Survival;
        a.start_race(Race {
            label: "words 2".to_string(),
            wpm: 50.0,
            accuracy: 100.0,
            words: vec!["two".into(), "words".into()],
            times: vec![100, 200],
        });
        assert_eq!(a.label(), "race");
        assert_eq!(a.session.words().len(), 2);
        assert_eq!(a.session.lives(), None, "challenges do not apply");
        type_text(&mut a, "two words");
        a.skip_celebration();
        assert!(matches!(a.screen, Screen::Results(_)));
        assert!(a.can_save_race());
    }

    #[test]
    fn open_quests_lead_the_weak_keys() {
        let mut a = app(&["hi"]);
        let presses: Vec<_> = "zzzzzzzaaaaaaa"
            .chars()
            .enumerate()
            .map(|(i, k)| crate::engine::stats::Press {
                at: Duration::from_millis(i as u64 * 200),
                correct: k != 'a' || i % 2 == 0,
                key: Some(k),
                word: 0,
            })
            .collect();
        a.key_stats.add_test(&presses).unwrap();
        // a has errors, so it is the weakest letter.
        assert_eq!(a.weak_letters(), ['a', 'z']);
        a.quests.fill(&a.key_stats, 0).unwrap();
        let open: Vec<char> = a.quests.open().map(|q| q.key).collect();
        assert_eq!(a.weak_letters()[..open.len()], open[..]);
    }

    #[test]
    fn momo_mode_types_momo_quotes_in_the_momo_theme() {
        let mut a = App::new(
            Config::default(),
            None,
            (80, 24),
            History::in_memory(),
            Mistakes::in_memory(),
            KeyStats::in_memory(),
            None,
        );
        a.start_momo();
        assert_eq!(
            a.session.remaining(Instant::now()),
            Some(crate::engine::session::MOMO_TIME)
        );
        // The words are whole quotes, one after the other.
        let first = a.momo_quotes.borrow()[0].clone();
        let text: Vec<String> = (a.session.words().iter())
            .map(|w| w.iter().collect())
            .collect();
        assert!(text.join(" ").starts_with(&first.text), "{text:?}");
        assert!(a.momo_quotes.borrow().len() >= 2);
        assert_eq!(a.label(), "momo");
        // Momo mode is never the saved mode.
        assert_eq!(parse_mode("momo"), None);
        assert_eq!(a.theme().name, "momo");
        // The chosen theme stays saved, and comes back after momo mode.
        assert_eq!(a.theme.name, Theme::default().name);
        // Boss words and climb do not apply to momo quotes.
        a.challenge = Challenge::Climb;
        assert!(!a.climb_on());
        a.challenge = Challenge::Boss;
        assert!(!a.boss_on());
        a.challenge = Challenge::Off;
        a.start_chosen();
        assert_eq!(a.theme().name, Theme::default().name);
        assert_eq!(a.label(), "time 30");
    }

    #[test]
    fn x_in_the_menu_starts_momo_mode_and_counts_whole_quotes() {
        let mut a = app(&["hi"]);
        a.open_menu();
        a.on_key(
            press(KeyCode::Char('x'), KeyModifiers::NONE),
            Instant::now(),
        );
        assert!(a.momo);
        assert_eq!(a.session.mode(), Mode::Momo);
        assert_eq!(a.mode, Mode::Words(1), "the chosen mode stays");
        assert!(matches!(a.screen, Screen::Typing));
        assert!(a.momo_done().is_empty());
        let first = a.momo_quotes.borrow()[0].clone();
        let n = first.text.split_whitespace().count();
        assert_eq!(a.momo_current(&a.session).map(|(r, _)| r), Some(0..n));
        // No space after the quote: the next quote starts at once.
        type_text(&mut a, &first.text);
        assert_eq!(a.momo_done(), std::slice::from_ref(&first));
        // The caret is in the next quote now.
        let (range, next) = a.momo_current(&a.session).expect("a next quote");
        assert_eq!(range.start, n);
        assert_eq!(next, a.momo_quotes.borrow()[1]);
    }

    #[test]
    fn in_the_menu_enter_starts_the_chosen_mode_and_x_starts_momo() {
        let mut a = app(&["hi"]);
        let now = Instant::now();
        a.start_momo();
        a.session.handle(Key::Char('x'), now);
        a.tick(now + crate::engine::session::MOMO_TIME);
        a.skip_celebration();
        assert!(matches!(a.screen, Screen::Results(_)));
        // On the results, enter restarts momo and x saves a race, as for
        // other tests.
        assert!(a.can_save_race());
        a.on_key(press(KeyCode::Enter, KeyModifiers::NONE), now);
        assert!(a.momo && matches!(a.screen, Screen::Typing));
        // The o list still has the mode from before momo.
        assert_eq!(a.mode, Mode::Words(1));
        // Enter in the menu starts that mode, not momo.
        a.open_menu();
        a.on_key(press(KeyCode::Enter, KeyModifiers::NONE), now);
        assert!(!a.momo);
        assert_eq!(a.session.mode(), Mode::Words(1));
        // Only x in the menu starts momo.
        a.open_menu();
        a.on_key(press(KeyCode::Char('x'), KeyModifiers::NONE), now);
        assert!(a.momo && a.session.mode() == Mode::Momo);
    }

    #[test]
    fn the_menu_keeps_the_chosen_theme_in_momo_mode() {
        let mut a = app(&["hi"]);
        a.start_momo();
        assert_eq!(a.theme().name, "momo");
        a.open_menu();
        assert_eq!(a.theme().name, Theme::default().name);
        a.on_key(
            press(KeyCode::Char('o'), KeyModifiers::NONE),
            Instant::now(),
        );
        assert_eq!(a.theme().name, Theme::default().name);
        // Back to the momo test: the momo theme again.
        a.on_key(press(KeyCode::Tab, KeyModifiers::NONE), Instant::now());
        a.on_key(press(KeyCode::Tab, KeyModifiers::NONE), Instant::now());
        assert!(matches!(a.screen, Screen::Typing));
        assert_eq!(a.theme().name, "momo");
    }

    #[test]
    fn momo_mode_has_no_challenge_and_no_strict_mode() {
        for (challenge, strict) in crate::challenge::CHALLENGES
            .into_iter()
            .zip(crate::engine::STRICT_MODES)
        {
            let mut a = app(&["hi"]);
            a.challenge = challenge;
            a.strict = strict;
            a.start_momo();
            // A wrong key does nothing special: no fail, no stop.
            a.on_key(
                press(KeyCode::Char('#'), KeyModifiers::NONE),
                Instant::now(),
            );
            assert!(
                !a.session.is_failed() && a.session.typed()[0] == ['#'],
                "{strict:?}"
            );
            a.on_key(
                press(KeyCode::Backspace, KeyModifiers::NONE),
                Instant::now(),
            );
            assert!(
                a.session.typed()[0].is_empty(),
                "backspace works: {strict:?}"
            );
            assert_eq!(a.session.lives(), None, "{challenge:?}");
            assert!(!a.boss_on() && !a.climb_on(), "{challenge:?}");
            assert_eq!(a.label(), "momo", "{challenge:?}");
            assert_eq!(a.session.mode(), Mode::Momo);
            // The menu settings stay for the other modes.
            assert_eq!((a.challenge, a.strict), (challenge, strict));
        }
    }

    #[test]
    fn u_in_the_menu_quits_to_update_only_with_a_newer_version() {
        let mut a = app(&["hi"]);
        a.open_menu();
        a.on_key(
            press(KeyCode::Char('u'), KeyModifiers::NONE),
            Instant::now(),
        );
        assert!(!a.quit && !a.update_now);
        a.update = Some("v9.0.0".to_string());
        a.on_key(
            press(KeyCode::Char('u'), KeyModifiers::NONE),
            Instant::now(),
        );
        assert!(a.quit && a.update_now);
    }

    #[test]
    fn winning_a_race_gives_ghostbuster() {
        let mut a = app(&["hi"]);
        a.start_race(Race {
            label: "words 2".to_string(),
            wpm: 1.0,
            accuracy: 100.0,
            words: vec!["two".into(), "words".into()],
            times: vec![100, 200],
        });
        // Typing at once is faster than 1 wpm.
        type_text(&mut a, "two words");
        let reward = a.reward.clone().expect("a reward");
        assert!(
            reward.badges.contains(&"ghostbuster"),
            "{:?}",
            reward.badges
        );
    }

    #[test]
    fn y_copies_the_daily_score_card() {
        let mut a = app(&["hi"]);
        a.start_daily(20_381);
        let text: Vec<String> = a
            .session
            .words()
            .iter()
            .map(|w| w.iter().collect())
            .collect();
        type_text(&mut a, &text.join(" "));
        a.skip_celebration();
        assert!(matches!(a.screen, Screen::Results(_)));
        a.on_key(
            press(KeyCode::Char('y'), KeyModifiers::NONE),
            Instant::now(),
        );
        let card = a.copy.take().expect("y gives a card to copy");
        assert!(
            card.starts_with(
                "dakken daily 2025-10-20
"
            ),
            "{card}"
        );
        assert_eq!(card.matches('🟦').count(), words::DAILY_WORDS, "{card}");
        // The main loop copies it; with no data folder there is no file.
        assert!(!a.copied);
        assert_eq!(a.card_file, None);
        // Other tests have no card.
        let mut b = app(&["hi"]);
        type_text(&mut b, "hi");
        b.on_key(
            press(KeyCode::Char('y'), KeyModifiers::NONE),
            Instant::now(),
        );
        assert_eq!(b.copy, None);
    }

    #[test]
    fn menu_selects_a_new_mode() {
        let mut a = app(&["hi"]);
        a.on_key(press(KeyCode::Esc, KeyModifiers::NONE), Instant::now());
        assert!(matches!(a.screen, Screen::Menu));
        a.on_key(
            press(KeyCode::Char('o'), KeyModifiers::NONE),
            Instant::now(),
        );
        assert!(matches!(a.screen, Screen::Modes { selected: 0 }));
        a.on_key(press(KeyCode::Up, KeyModifiers::NONE), Instant::now());
        a.on_key(press(KeyCode::Enter, KeyModifiers::NONE), Instant::now());
        // The mode changes and the menu shows it; enter then starts the test.
        assert!(matches!(a.screen, Screen::Menu));
        assert_eq!(a.mode, MODES[MODES.len() - 1]);
        a.on_key(press(KeyCode::Enter, KeyModifiers::NONE), Instant::now());
        assert!(matches!(a.screen, Screen::Typing));
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
        assert!(matches!(a.screen, Screen::Menu));
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
        let config = Config {
            mode: Mode::Untimed,
            ..Config::default()
        };
        let mut a = App::new(
            config,
            None,
            (40, 24),
            History::in_memory(),
            Mistakes::in_memory(),
            KeyStats::in_memory(),
            None,
        );
        let narrow = a.session.words().len();
        a.on_resize(120, 24);
        assert!(a.session.words().len() > narrow);
        type_text(&mut a, "x");
        let started = a.session.words().len();
        a.on_resize(40, 24);
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
            (80, 24),
            History::in_memory(),
            Mistakes::in_memory(),
            KeyStats::in_memory(),
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
        a.skip_celebration();
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
        a.on_key(
            press(KeyCode::Char('o'), KeyModifiers::NONE),
            Instant::now(),
        );
        a.on_key(press(KeyCode::Down, KeyModifiers::NONE), Instant::now());
        a.on_key(press(KeyCode::Enter, KeyModifiers::NONE), Instant::now());
        let saved = Config::load(&path);
        assert_eq!(saved.theme, Theme::default().next(None));
        assert!(saved.options.punctuation);
        assert_eq!(saved.mode, MODES[1]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn tab_opens_and_closes_the_menu() {
        let mut a = app(&["hi"]);
        a.on_key(press(KeyCode::Tab, KeyModifiers::NONE), Instant::now());
        assert!(matches!(a.screen, Screen::Menu));
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
            (80, 24),
            History::in_memory(),
            Mistakes::in_memory(),
            KeyStats::in_memory(),
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
        a.skip_celebration();
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
        a.skip_celebration();
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
                (80, 24),
                History::in_memory(),
                Mistakes::in_memory(),
                KeyStats::in_memory(),
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
    fn finished_tests_add_key_stats() {
        let mut a = app(&["hi"]);
        type_text(&mut a, "hx");
        assert_eq!(a.key_stats.get('h').presses, 1);
        assert_eq!(a.key_stats.get('i').errors, 1);
    }

    #[test]
    fn weak_keys_list_uses_words_with_the_weakest_letters() {
        use crate::engine::stats::Press;
        let mut a = app(&["hi"]);
        let presses: Vec<Press> = (0..10)
            .map(|i| Press {
                at: Duration::from_millis(i * 100),
                correct: i % 2 == 0,
                key: Some('q'),
                word: 0,
            })
            .collect();
        a.key_stats.add_test(&presses).unwrap();
        assert_eq!(a.weak_letters(), ['q']);
        a.options.list = WordList::WeakKeys;
        a.restart();
        assert!(
            a.session.words().iter().all(|w| w.contains(&'q')),
            "every word has a q"
        );
    }

    #[test]
    fn r_replays_the_test_and_returns_to_the_results() {
        let mut a = app(&["hi"]);
        let start = Instant::now();
        a.on_key(press(KeyCode::Char('h'), KeyModifiers::NONE), start);
        let later = start + Duration::from_millis(500);
        a.on_key(press(KeyCode::Char('i'), KeyModifiers::NONE), later);
        a.skip_celebration();
        a.on_key(press(KeyCode::Char('r'), KeyModifiers::NONE), later);
        let Screen::Replay(replay) = &a.screen else {
            panic!("r starts the replay");
        };
        assert!(!replay.session.is_started());
        a.tick(later + Duration::from_millis(100));
        let Screen::Replay(replay) = &a.screen else {
            panic!("the replay runs");
        };
        assert_eq!(replay.session.typed()[0], ['h']);
        a.tick(later + Duration::from_secs(2));
        assert!(matches!(a.screen, Screen::Results(_)), "the replay ended");
    }

    #[test]
    fn layout_changes_the_typed_letters() {
        let mut a = app(&["aoeu"]);
        a.layout = Layout::Dvorak;
        type_text(&mut a, "asdf");
        a.skip_celebration();
        assert!(
            matches!(a.screen, Screen::Results(_)),
            "asdf on dvorak is aoeu"
        );
        assert!(a.label().ends_with(" · dvorak"), "{}", a.label());
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
