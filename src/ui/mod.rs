mod menu;
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
    let t = app.theme;
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
        Screen::Typing => typing::draw(frame, app, now),
        Screen::Results(result) => results::draw(frame, app, result),
        Screen::Menu { selected } => menu::draw(frame, app, *selected),
    }
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

fn text_width(terminal_width: u16) -> u16 {
    terminal_width.saturating_sub(4).min(MAX_WIDTH)
}

/// Words that fill the word lines of a terminal `terminal_width` wide.
pub fn screen_words(terminal_width: u16, mut next_word: impl FnMut() -> String) -> Vec<String> {
    let width = text_width(terminal_width.max(MIN_WIDTH)) as usize;
    let mut words = vec![next_word()];
    loop {
        let word = next_word();
        let mut widths: Vec<usize> = words.iter().map(|w| w.chars().count()).collect();
        widths.push(word.chars().count());
        if typing::wrap(&widths, width).len() > typing::VISIBLE_LINES {
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
    fn screen_words_fill_the_visible_lines() {
        // Text width 76 holds 15 four-letter words on each line.
        assert_eq!(screen_words(80, || "abcd".to_string()).len(), 45);
        // The minimum width (30) gives width 26: 5 words on each line.
        assert_eq!(screen_words(0, || "abcd".to_string()).len(), 15);
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
        assert!(render(&a, 80, 30).contains("── raw"));
        // A short terminal keeps the stats and drops the graph.
        let out = render(&a, 80, 12);
        assert!(!out.contains("── raw") && out.contains("wpm"), "{out}");
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
        let out = render(&a, 60, 20);
        for label in [
            "wpm",
            "acc",
            "raw",
            "consistency",
            "characters",
            "personal best",
        ] {
            assert!(out.contains(label), "missing {label}:\n{out}");
        }
    }

    #[test]
    fn menu_lists_the_modes() {
        let mut a = app(&["hi"]);
        a.screen = Screen::Menu { selected: 0 };
        let out = render(&a, 60, 24);
        assert!(
            out.contains("time 15") && out.contains("words 100"),
            "{out}"
        );
        assert!(
            out.contains("punctuation off") && out.contains("numbers off"),
            "{out}"
        );
        assert!(
            out.contains("theme terminal") && out.contains("words english"),
            "{out}"
        );
        assert!(
            out.contains("c  pace caret off") && out.contains("m  strict off"),
            "{out}"
        );
        // The smallest terminal still shows the selected mode, every setting
        // and the hint.
        a.screen = Screen::Menu { selected: 13 };
        let out = render(&a, 30, 11);
        assert!(
            out.contains("> words 100") && out.contains("pace caret") && out.contains("tab back"),
            "{out}"
        );
    }
}
