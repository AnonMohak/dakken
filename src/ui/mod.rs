mod ascii;
mod celebration;
mod menu;
mod profile;
mod results;
pub mod theme;
mod typing;

use std::time::Instant;

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Style,
    text::Line,
    widgets::{Block, Paragraph},
};

use crate::app::{App, Screen};

/// Width of the text column. Wider text is hard to read.
const MAX_WIDTH: u16 = 80;
const MIN_WIDTH: u16 = 30;
const MIN_HEIGHT: u16 = 11;

pub fn draw(frame: &mut Frame, app: &App, now: Instant) {
    let area = frame.area();
    let t = app.theme();
    if let Some(bg) = t.bg {
        frame.render_widget(Block::new().style(Style::new().bg(bg)), area);
    }

    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        let msg = Paragraph::new(vec![
            Line::from("terminal too small"),
            Line::from(format!("need {MIN_WIDTH} x {MIN_HEIGHT}")),
        ])
        .style(Style::new().fg(t.sub))
        .alignment(Alignment::Center);
        frame.render_widget(msg, center(area, area.width, 2));
        return;
    }

    match &app.screen {
        Screen::Typing => typing::draw(frame, app, &app.session, now, false),
        Screen::Replay(replay) => typing::draw(frame, app, &replay.session, now, true),
        Screen::Results(result) => results::draw(frame, app, result),
        Screen::Menu => menu::draw(frame, app),
        Screen::Modes { selected } => menu::draw_modes(frame, app, *selected),
        Screen::Celebration { page } => celebration::draw(frame, app, *page),
        Screen::Profile { scroll } => profile::draw(frame, app, *scroll),
    }
    if app.ascii {
        ascii::replace_symbols(frame.buffer_mut());
    }
}

/// The most rows that the profile can scroll in the terminal now.
pub fn profile_max_scroll(app: &App) -> usize {
    profile::max_scroll(app, app.size.1)
}

/// A `width` x `height` box in the middle of `area`.
fn center(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    }
}

/// The text column: up to `MAX_WIDTH` wide, with a gap on both sides.
fn column(area: Rect, height: u16) -> Rect {
    center(area, text_width(area.width), height)
}

pub(super) fn text_width(terminal_width: u16) -> u16 {
    terminal_width.saturating_sub(4).min(MAX_WIDTH)
}

/// The word area of the typing screen: 70% of the terminal width and
/// height, in the middle. At least 4 rows stay free above and below, so the
/// words do not touch the border, the status line or the hint.
pub fn typing_area(area: Rect) -> Rect {
    let width = (u32::from(area.width) * 7 / 10) as u16;
    let height = ((u32::from(area.height) * 7 / 10) as u16).min(area.height.saturating_sub(8));
    center(area, width.max(1), height.max(1))
}

/// Word lines in an area `height` rows high: an empty row is between lines.
pub fn word_lines(height: u16) -> usize {
    (height as usize).div_ceil(2)
}

/// Words that fill the word area of a terminal `width` x `height`.
pub fn screen_words(width: u16, height: u16, mut next_word: impl FnMut() -> String) -> Vec<String> {
    let area = typing_area(Rect::new(
        0,
        0,
        width.max(MIN_WIDTH),
        height.max(MIN_HEIGHT),
    ));
    let (width, lines) = (area.width as usize, word_lines(area.height));
    // Fill the lines the way `typing::wrap` does: one space between words,
    // and a long word on a line of its own.
    let mut words = Vec::new();
    let (mut line, mut used) = (1, 0);
    loop {
        let word = next_word();
        let w = word.chars().count();
        if used > 0 && used + 1 + w > width {
            line += 1;
            used = w;
        } else {
            used += if used > 0 { 1 + w } else { w };
        }
        if line > lines && !words.is_empty() {
            return words;
        }
        words.push(word);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Mode, Session};
    use ratatui::{Terminal, backend::TestBackend};

    fn render(app: &App, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|f| draw(f, app, Instant::now())).unwrap();
        let buf = terminal.backend().buffer();
        (0..buf.area.height)
            .map(|y| {
                (0..buf.area.width)
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn app(words: &[&str]) -> App {
        let mode = Mode::Words(words.len());
        App::with_session(mode, Session::from_words(mode, words))
    }

    #[test]
    fn typing_screen_shows_the_words() {
        let out = render(&app(&["hello", "world"]), 60, 20);
        assert!(out.contains("hello world"), "{out}");
        assert!(out.contains("• tab for menu"), "{out}");
        // Nothing else: no mode name, counter or other hints.
        assert!(!out.contains("words 2") && !out.contains("ctrl+c"), "{out}");
    }

    #[test]
    fn any_size_and_long_extra_letters_render_without_panic() {
        let words = ["a", "bb", "ccc", "dddd", "eeeee", "ffffff", "ggggggg"];
        let mut a = app(&words);
        for c in "a bb ccc dddd".chars().chain("e".repeat(30).chars()) {
            let key = if c == ' ' {
                crate::engine::Key::Space
            } else {
                crate::engine::Key::Char(c)
            };
            a.session.handle(key, Instant::now());
        }
        for (w, h) in [(30, 10), (31, 10), (40, 12), (120, 40), (1, 1), (0, 0)] {
            render(&a, w, h);
        }
    }

    #[test]
    fn screen_words_fill_the_word_area() {
        // 100 x 30: the area is 70 x 21, so 11 lines with gaps. 14
        // four-letter words fit on a line.
        assert_eq!(screen_words(100, 30, || "abcd".to_string()).len(), 14 * 11);
        // The minimum size (30 x 11): 21 x 3, so 2 lines of 4 words.
        assert_eq!(screen_words(0, 0, || "abcd".to_string()).len(), 4 * 2);
        // The same count as the real wrapping.
        let words = screen_words(57, 20, || "abc".to_string());
        let widths: Vec<usize> = words.iter().map(|w| w.len()).collect();
        let area = typing_area(Rect::new(0, 0, 57, 20));
        assert_eq!(
            typing::wrap(&widths, area.width as usize).len(),
            word_lines(area.height)
        );
    }

    #[test]
    fn typing_area_is_70_percent_of_the_terminal() {
        let area = typing_area(Rect::new(0, 0, 200, 50));
        assert_eq!((area.x, area.y, area.width, area.height), (30, 7, 140, 35));
        assert_eq!(word_lines(35), 18);
        // A short terminal keeps 4 rows above and below.
        assert_eq!(typing_area(Rect::new(0, 0, 30, 11)).height, 3);
    }

    #[test]
    fn results_show_a_graph_when_there_is_room() {
        let mut a = app(&["hello", "world"]);
        let start = Instant::now();
        for (i, c) in "hello world".chars().enumerate() {
            let key = if c == ' ' {
                crate::engine::Key::Space
            } else {
                crate::engine::Key::Char(c)
            };
            a.session.handle(
                key,
                start + std::time::Duration::from_millis(300 * i as u64),
            );
        }
        a.tick(start + std::time::Duration::from_secs(4));
        a.skip_celebration();
        assert!(render(&a, 80, 30).contains("── raw"));
        // A short terminal keeps the stats and drops the graph.
        let out = render(&a, 80, 12);
        assert!(!out.contains("── raw") && out.contains("wpm"), "{out}");
    }

    #[test]
    fn the_time_mode_timer_is_on_the_right_from_the_start() {
        let mode = Mode::Time(std::time::Duration::from_secs(30));
        let a = App::with_session(mode, Session::from_words(mode, &["hello", "world"]));
        let out = render(&a, 60, 20);
        let row = out.lines().find(|l| l.contains("30")).expect(&out);
        let words = out.lines().find(|l| l.contains("hello")).unwrap();
        assert!(row.find("30") > words.find("world"), "{out}");
    }

    #[test]
    fn live_stats_show_only_when_on() {
        let mut a = app(&["hello", "world"]);
        a.session
            .handle(crate::engine::Key::Char('h'), Instant::now());
        assert!(!render(&a, 60, 20).contains("wpm"));
        a.live = true;
        let out = render(&a, 60, 20);
        assert!(out.contains("wpm") && out.contains("100%"), "{out}");
    }

    #[test]
    fn a_lost_life_takes_its_heart_away() {
        let mut a = app(&["hello", "world", "again"]);
        a.session.set_lives(3);
        a.session
            .handle(crate::engine::Key::Char('x'), Instant::now());
        a.session.handle(crate::engine::Key::Space, Instant::now());
        let mut terminal = Terminal::new(TestBackend::new(60, 20)).unwrap();
        terminal.draw(|f| draw(f, &a, Instant::now())).unwrap();
        let buf = terminal.backend().buffer();
        let hearts: Vec<_> = (0..buf.area.height)
            .flat_map(|y| (0..buf.area.width).map(move |x| (x, y)))
            .map(|p| &buf[p])
            .filter(|c| c.symbol() == "♥")
            .map(|c| (c.symbol().to_string(), c.fg))
            .collect();
        let t = a.theme();
        assert_eq!(
            hearts,
            [("♥".to_string(), t.main), ("♥".to_string(), t.main)]
        );
        assert!(render(&a, 60, 20).contains("♥ ♥"), "a gap between hearts");
    }

    #[test]
    fn momo_mode_shows_its_title_and_the_timer_above_the_words() {
        let mode = Mode::Momo;
        let a = App::with_session(mode, Session::from_words(mode, &["I'm", "gonna"]));
        let out = render(&a, 60, 20);
        let title = out.lines().position(|l| l.contains("momo mode"));
        let words = out.lines().position(|l| l.contains("I'm gonna"));
        assert!(title.is_some_and(|t| words.is_some_and(|w| t < w)), "{out}");
        // The timer is on the right of the title row.
        let row = out.lines().nth(title.unwrap()).unwrap();
        assert!(row.trim_end().ends_with("60"), "{out}");
        assert!(row.find("60") > row.find("momo mode"), "{out}");
    }

    #[test]
    fn momo_mode_shows_one_quote_and_who_said_it_under_it() {
        let mut a = app(&["hi"]);
        a.start_momo();
        let (first, second) = {
            let quotes = a.momo_quotes.borrow();
            (quotes[0].clone(), quotes[1].clone())
        };
        let out = render(&a, 200, 40);
        let start: String = first
            .text
            .split_whitespace()
            .take(3)
            .collect::<Vec<_>>()
            .join(" ");
        let next: String = second
            .text
            .split_whitespace()
            .take(3)
            .collect::<Vec<_>>()
            .join(" ");
        assert!(out.contains(&start), "{out}");
        assert!(!out.contains(&next), "only one quote shows:\n{out}");
        let who = format!("{} {}", first.mood, first.speaker);
        let rows: Vec<&str> = out.lines().collect();
        let who_row = rows.iter().position(|l| l.contains(&who)).expect(&out);
        let last_text = rows
            .iter()
            .rposition(|l| {
                l.trim_start().starts_with(|c: char| c.is_ascii_graphic())
                    && !l.contains(&who)
                    && !l.contains("momo mode")
                    && !l.contains("tab for menu")
            })
            .unwrap();
        assert_eq!(who_row, last_text + 2, "{out}");
        // It is on the right side.
        let row = rows[who_row];
        assert!(row.find(&who) > Some(row.len() / 3), "{out}");
    }

    #[test]
    fn ascii_symbols_leave_only_ascii_on_every_screen() {
        let mut a = app(&["hello", "world", "again"]);
        a.ascii = true;
        a.session.set_lives(3);
        a.session
            .handle(crate::engine::Key::Char('x'), Instant::now());
        a.session.handle(crate::engine::Key::Space, Instant::now());
        let typing = render(&a, 80, 24);
        assert!(typing.is_ascii(), "{typing}");
        assert!(typing.contains("<3 <3"), "{typing}");
        // The results, with a graph and the reward lines.
        let start = Instant::now();
        for (i, c) in "hello world again".chars().enumerate() {
            let key = if c == ' ' {
                crate::engine::Key::Space
            } else {
                crate::engine::Key::Char(c)
            };
            a.session.handle(
                key,
                start + std::time::Duration::from_millis(300 * i as u64),
            );
        }
        a.tick(start + std::time::Duration::from_secs(10));
        let results = render(&a, 80, 30);
        assert!(results.is_ascii(), "{results}");
        a.skip_celebration();
        for screen in [
            Screen::Menu,
            Screen::Modes { selected: 0 },
            Screen::Celebration { page: 0 },
            Screen::Profile { scroll: 0 },
        ] {
            a.screen = screen;
            a.update = Some("v9.0.0".into());
            let out = render(&a, 80, 30);
            assert!(out.is_ascii(), "{out}");
        }
        a.start_momo();
        let momo = render(&a, 100, 30);
        assert!(momo.is_ascii(), "{momo}");
        // Letters of the words stay as they are.
        let mut b = app(&["für"]);
        b.ascii = true;
        assert!(render(&b, 60, 20).contains("für"));
    }

    #[test]
    fn a_new_badge_fills_the_screen() {
        let mut a = app(&["hi"]);
        a.session
            .handle(crate::engine::Key::Char('h'), Instant::now());
        a.session
            .handle(crate::engine::Key::Char('i'), Instant::now());
        a.tick(Instant::now());
        let out = render(&a, 60, 20);
        for text in [
            "new badge",
            "[>]",
            "F I R S T   P R E S S",
            "finish your first test",
            "1 of 19 badges",
            "tab continue",
        ] {
            assert!(
                out.contains(text),
                "missing {text}:
{out}"
            );
        }
        // The results are not on this screen.
        assert!(!out.contains("wpm"), "{out}");
        // A new rank has the logo with the rank's speed lines.
        a.reward.as_mut().unwrap().after = crate::progress::Level::from_xp(20_000);
        let out = render(&a, 60, 24);
        for text in ["new rank", "H O M I N G", "level 9", "1 / 2"] {
            assert!(
                out.contains(text),
                "missing {text}:
{out}"
            );
        }
    }

    #[test]
    fn the_profile_shows_everything_about_the_user() {
        let mut a = app(&["hi"]);
        a.session
            .handle(crate::engine::Key::Char('h'), Instant::now());
        a.session
            .handle(crate::engine::Key::Char('i'), Instant::now());
        a.tick(Instant::now());
        a.screen = Screen::Profile { scroll: 0 };
        let out = render(&a, 100, 80);
        for text in [
            "level 1 · rank blank",
            "next rank homing at level 5",
            "1 test · ",
            "best wpm",
            "words 1",
            "badges 1 of 19",
            "[>] first press",
            "[?] ???",
            "weak keys",
            "key quests  0 done",
            "↑↓ scroll · tab back",
        ] {
            assert!(
                out.contains(text),
                "missing {text}:
{out}"
            );
        }
        // A short terminal shows the end of the profile after a scroll.
        a.screen = Screen::Profile { scroll: 1000 };
        let out = render(&a, 100, 20);
        assert!(
            out.contains("key quests") && !out.contains("rank blank"),
            "{out}"
        );
    }

    #[test]
    fn small_terminal_shows_a_message() {
        let out = render(&app(&["hello"]), 20, 5);
        assert!(out.contains("too small"), "{out}");
    }

    #[test]
    fn results_screen_shows_the_stats() {
        let mut a = app(&["hi"]);
        a.session
            .handle(crate::engine::Key::Char('h'), Instant::now());
        a.session
            .handle(crate::engine::Key::Char('i'), Instant::now());
        a.tick(Instant::now());
        a.skip_celebration();
        let out = render(&a, 60, 20);
        for label in [
            "wpm",
            "acc",
            "raw",
            "consistency",
            "characters",
            "personal best",
            "xp",
            "new badge: [>] first press",
        ] {
            assert!(out.contains(label), "missing {label}:\n{out}");
        }
    }

    #[test]
    fn menu_shows_the_settings_with_keys_and_the_logo() {
        let mut a = app(&["hi"]);
        a.screen = Screen::Menu;
        let out = render(&a, 60, 29);
        let row = |text: &str| out.lines().find(|l| l.contains(text)).unwrap_or_default();
        for (key, text) in [
            ("o", "mode words 1"),
            ("p", "punctuation off"),
            ("n", "numbers off"),
            ("l", "words english"),
            ("t", "theme tokyo-night"),
            ("s", "live stats off"),
            ("c", "pace caret off"),
            ("m", "strict off"),
            ("g", "challenge off"),
        ] {
            assert!(
                row(text).trim_start().starts_with(&format!("{key}      ")),
                "{out}"
            );
        }
        assert!(
            row("momo mode").trim_start().starts_with("x      "),
            "{out}"
        );
        // Keys and text are each in one column.
        let start = |text: &str| row(text).find(|c: char| c != ' ');
        assert_eq!(start("mode words 1"), start("strict off"), "{out}");
        assert!(out.contains("/_________/"), "{out}");
        assert!(out.contains("level 1 · rank blank  ────"), "{out}");
        assert!(!out.contains("week 0/"), "{out}");
        a.weekly_goal = 500;
        assert!(render(&a, 60, 29).contains("week 0/500  ────"), "{out}");
        assert!(!out.contains("time 15"), "{out}");
        assert!(!out.contains("u to update"), "{out}");
        a.update = Some("v9.0.0".to_string());
        let with_update = render(&a, 60, 29);
        assert!(
            with_update.contains("update v9.0.0 available · u to update"),
            "{with_update}"
        );
        assert!(
            out.contains("enter start · tab back · ctrl+c exit"),
            "{out}"
        );
        // The smallest terminal drops the logo but keeps every setting.
        let out = render(&a, 30, 11);
        assert!(!out.contains("_____"), "{out}");
        assert!(
            out.contains("mode words 1") && out.contains("strict off"),
            "{out}"
        );
    }

    #[test]
    fn mode_screen_lists_the_modes() {
        let mut a = app(&["hi"]);
        a.screen = Screen::Modes { selected: 13 };
        let out = render(&a, 60, 24);
        assert!(
            out.contains("  time 15") && out.contains("> words 100"),
            "{out}"
        );
        assert!(out.contains("enter select · tab back"), "{out}");
        let start = |text: &str| {
            out.lines()
                .find(|l| l.contains(text))
                .and_then(|l| l.find(text))
        };
        assert_eq!(start("mode"), start("time 15"), "{out}");
        let out = render(&a, 30, 11);
        assert!(out.contains("> words 100"), "{out}");
    }
}
