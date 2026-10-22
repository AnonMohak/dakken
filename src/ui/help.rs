use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::column;
use super::menu::{HINT_ROWS, MARGIN, keys_line};
use crate::app::App;

/// Each part of the help: a title and its keys.
const HELP: [(&str, &[(&str, &str)]); 5] = [
    (
        "while you type",
        &[
            ("tab, esc", "open the menu"),
            ("ctrl+r", "start a new test"),
            (
                "ctrl+backspace",
                "delete the word (also ctrl+w, ctrl+h, alt+backspace)",
            ),
            ("enter", "end the test, in zen mode"),
            ("ctrl+c", "quit, on every screen"),
        ],
    ),
    (
        "on the results",
        &[
            ("enter", "next test, with new words"),
            ("a", "the same words again"),
            ("r", "replay the test at the speed you typed it"),
            ("x", "save the test as a race file for --race"),
            ("y", "copy the score card"),
            ("tab, esc", "open the menu"),
            ("q", "quit"),
        ],
    ),
    (
        "in the menu",
        &[
            ("enter", "start a test of the chosen mode"),
            ("o p n l", "mode, punctuation, numbers, word list"),
            ("m g", "strict mode, challenge"),
            ("d", "daily challenge: the same words for everyone today"),
            ("x", "momo mode"),
            ("f", "profile: level, stats, badges, keys"),
            ("e", "settings: theme, caret, layout, goal, symbols"),
            ("t s c", "shortcuts: theme, live stats, pace caret"),
            ("q", "quit"),
        ],
    ),
    (
        "on lists and pages",
        &[
            ("↑↓ or j k", "move or scroll"),
            ("←→ or enter", "change a setting"),
            ("tab, esc", "go back"),
        ],
    ),
    (
        "boxes in place of symbols?",
        &[("e", "set symbols to ascii in the settings")],
    ),
];

/// All the keys, from `?` in the menu. `scroll` rows are hidden at the top
/// when the terminal is too short.
pub fn draw(frame: &mut Frame, app: &App, scroll: usize) {
    let t = app.theme();
    let screen = frame.area();
    let lines = lines(app);
    let rows = visible_rows(screen.height);
    let scroll = scroll.min(lines.len().saturating_sub(rows));
    // The width of all the rows, not only the ones on screen, so the page
    // does not move to the side when it scrolls.
    let width = lines.iter().map(Line::width).max().unwrap_or(0) as u16;
    let shown: Vec<Line> = lines.into_iter().skip(scroll).take(rows).collect();
    let height = shown.len() as u16;
    let area = column(screen, height);
    let left = (screen.width.saturating_sub(width) / 2).max(MARGIN);
    let area = Rect {
        x: screen.x + left.min(screen.width),
        width: width.min(screen.width.saturating_sub(left)),
        ..area
    };
    frame.render_widget(Paragraph::new(shown), area);
    keys_line(frame, t, height, "↑↓ scroll · tab back");
}

/// The most rows that `scroll` can hide on a terminal `height` rows high.
pub fn max_scroll(app: &App, height: u16) -> usize {
    lines(app).len().saturating_sub(visible_rows(height))
}

fn visible_rows(height: u16) -> usize {
    usize::from(height.saturating_sub(HINT_ROWS)).max(1)
}

fn lines(app: &App) -> Vec<Line<'static>> {
    let t = app.theme();
    let head = Style::new().fg(t.main).add_modifier(Modifier::BOLD);
    let width = (HELP.iter())
        .flat_map(|(_, keys)| keys.iter().map(|(k, _)| k.chars().count()))
        .max()
        .unwrap_or(0);
    let mut out = Vec::new();
    for (i, (title, keys)) in HELP.iter().enumerate() {
        if i > 0 {
            out.push(Line::default());
        }
        out.push(Line::styled(*title, head));
        for (key, text) in *keys {
            out.push(Line::from(vec![
                Span::styled(format!("  {key:<width$}  "), Style::new().fg(t.main)),
                Span::styled(*text, Style::new().fg(t.text)),
            ]));
        }
    }
    out
}
