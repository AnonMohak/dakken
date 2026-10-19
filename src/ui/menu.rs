use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::column;
use crate::app::{App, MODES, mode_label};
use crate::engine::{Strict, words::WordList};
use crate::ui::theme::Theme;

/// Rows that are not mode names: gaps, settings and the hint.
const OTHER_ROWS: u16 = 10;

pub fn draw(frame: &mut Frame, app: &App, selected: usize) {
    let t = app.theme;
    let mut lines = Vec::new();
    // In a short terminal, show only the modes around the selected one.
    let rows = (frame.area().height.saturating_sub(OTHER_ROWS) as usize).clamp(1, MODES.len());
    let first = selected.saturating_sub(rows / 2).min(MODES.len() - rows);
    for (i, &mode) in MODES.iter().enumerate().skip(first).take(rows) {
        let line = if i == selected {
            Line::styled(
                format!("> {}", mode_label(mode)),
                Style::new().fg(t.main).add_modifier(Modifier::BOLD),
            )
        } else {
            Line::styled(format!("  {}", mode_label(mode)), Style::new().fg(t.text))
        };
        lines.push(line);
    }
    lines.push(Line::default());
    // Each setting shows the key that changes it.
    let on_off = |on: bool| if on { ("on", t.main) } else { ("off", t.text) };
    let (state, color) = on_off(app.options.punctuation);
    lines.push(setting(t, 'p', "punctuation", state, color));
    let (state, color) = on_off(app.options.numbers);
    lines.push(setting(t, 'n', "numbers", state, color));
    let list = match app.options.list {
        WordList::WeakKeys => {
            let letters: Vec<String> = app.weak_letters().iter().map(char::to_string).collect();
            if letters.is_empty() {
                "weak-keys (no data yet)".to_string()
            } else {
                format!("weak-keys ({})", letters.join(" "))
            }
        }
        WordList::WeakPairs => {
            let pairs = app.weak_pairs();
            if pairs.is_empty() {
                "weak-pairs (no data yet)".to_string()
            } else {
                format!("weak-pairs ({})", pairs.join(" "))
            }
        }
        list => list.name().to_string(),
    };
    lines.push(setting(t, 'l', "words", &list, t.text));
    lines.push(setting(t, 't', "theme", t.name, t.text));
    let (state, color) = on_off(app.live);
    lines.push(setting(t, 's', "live stats", state, color));
    let (state, color) = on_off(app.pace);
    lines.push(setting(t, 'c', "pace caret", state, color));
    let strict_color = if app.strict == Strict::Off {
        t.text
    } else {
        t.main
    };
    lines.push(setting(t, 'm', "strict", app.strict.name(), strict_color));
    lines.push(Line::default());
    lines.push(Line::styled(
        "enter start · tab back",
        Style::new().fg(t.sub),
    ));
    let height = lines.len() as u16;
    let area = column(frame.area(), height);
    frame.render_widget(Paragraph::new(lines).centered(), area);

    // At the bottom, like "• tab for menu" on the typing screen, when it
    // does not cover the menu.
    let screen = frame.area();
    if screen.height >= height + 4 {
        let hint = Line::styled(
            "• ctrl+c to exit",
            Style::new().fg(t.sub).add_modifier(Modifier::ITALIC),
        );
        let hint_area = Rect {
            y: screen.bottom().saturating_sub(2),
            height: 1,
            ..screen
        };
        frame.render_widget(Paragraph::new(hint).centered(), hint_area);
    }
}

/// One setting line, for example "p  punctuation off" with the key in gray.
fn setting(t: Theme, key: char, name: &str, value: &str, color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{key}  "), Style::new().fg(t.sub)),
        Span::styled(format!("{name} {value}"), Style::new().fg(color)),
    ])
}
