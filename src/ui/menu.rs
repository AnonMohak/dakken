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

/// Rows that are not mode names: gaps, settings and the two key rows (11),
/// and 4 rows kept free for the "ctrl+c to exit" hint at the bottom.
const OTHER_ROWS: u16 = 15;

/// The dakken logo, a keycap with speed lines, next to the mode list.
const LOGO: [&str; 7] = [
    r"       __________",
    r" ==   /  \      /|",
    r"==   /    \    / |",
    r" =  /     /   /  /",
    r"   /     /   /  /",
    r"  /_________/  /",
    r"  \_________\ /",
];
/// Width of the key column without the logo: the longest key, "enter".
const KEY_WIDTH: usize = 5;
/// Space between the key column and the text column.
const GAP: usize = 4;
/// Space kept free on the left and the right of the menu.
const MARGIN: usize = 4;

pub fn draw(frame: &mut Frame, app: &App, selected: usize) {
    let t = app.theme;
    let screen = frame.area();
    // Each row is a key column and a text column: (key, text spans).
    let mut rows: Vec<(Option<&str>, Vec<Span>)> = Vec::new();
    // In a short terminal, show only the modes around the selected one.
    let modes = (screen.height.saturating_sub(OTHER_ROWS) as usize).clamp(1, MODES.len());
    let first = selected.saturating_sub(modes / 2).min(MODES.len() - modes);
    for (i, &mode) in MODES.iter().enumerate().skip(first).take(modes) {
        let span = if i == selected {
            Span::styled(
                format!("> {}", mode_label(mode)),
                Style::new().fg(t.main).add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(format!("  {}", mode_label(mode)), Style::new().fg(t.text))
        };
        rows.push((None, vec![span]));
    }
    rows.push((None, Vec::new()));
    // Each setting shows the key that changes it.
    let on_off = |on: bool| if on { ("on", t.main) } else { ("off", t.text) };
    let mut setting = |key, name: &str, value: &str, color: Color| {
        rows.push((
            Some(key),
            vec![Span::styled(
                format!("{name} {value}"),
                Style::new().fg(color),
            )],
        ));
    };
    let (state, color) = on_off(app.options.punctuation);
    setting("p", "punctuation", state, color);
    let (state, color) = on_off(app.options.numbers);
    setting("n", "numbers", state, color);
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
    setting("l", "words", &list, t.text);
    setting("t", "theme", t.name, t.text);
    let (state, color) = on_off(app.live);
    setting("s", "live stats", state, color);
    let (state, color) = on_off(app.pace);
    setting("c", "pace caret", state, color);
    let strict_color = if app.strict == Strict::Off {
        t.text
    } else {
        t.main
    };
    setting("m", "strict", app.strict.name(), strict_color);
    rows.push((None, Vec::new()));
    let sub = Style::new().fg(t.sub);
    rows.push((Some("enter"), vec![Span::styled("start", sub)]));
    rows.push((Some("tab"), vec![Span::styled("back", sub)]));

    // The logo needs all its rows next to the modes, and room on the sides.
    let text_width = rows
        .iter()
        .map(|(_, spans)| {
            spans
                .iter()
                .map(|s| s.content.chars().count())
                .sum::<usize>()
        })
        .max()
        .unwrap_or(0);
    let logo_width = LOGO.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let with_logo =
        modes >= LOGO.len() && logo_width + GAP + text_width + 2 * MARGIN <= screen.width as usize;
    let key_width = if with_logo { logo_width } else { KEY_WIDTH };
    let logo_top = modes.saturating_sub(LOGO.len()) / 2;

    let lines: Vec<Line> = rows
        .into_iter()
        .enumerate()
        .map(|(i, (key, text))| {
            let mut spans = match key {
                Some(k) => vec![Span::styled(format!("{k:<key_width$}"), sub)],
                None if with_logo && (logo_top..logo_top + LOGO.len()).contains(&i) => {
                    logo_line(t, LOGO[i - logo_top], key_width)
                }
                None => vec![Span::raw(" ".repeat(key_width))],
            };
            // The "> " of the selected mode sits in the gap, so the mode
            // names line up with the settings.
            let marked = key.is_none()
                && text
                    .first()
                    .is_some_and(|s| s.content.starts_with(['>', ' ']));
            let gap = if marked { GAP - 2 } else { GAP };
            spans.push(Span::raw(" ".repeat(gap)));
            spans.extend(text);
            Line::from(spans)
        })
        .collect();

    let height = lines.len() as u16;
    let width = (key_width + GAP + text_width) as u16;
    // Centered, with at least the margin on the left.
    let left = ((screen.width.saturating_sub(width)) / 2).max(MARGIN as u16);
    let area = Rect {
        x: screen.x + left.min(screen.width),
        width: width.min(screen.width.saturating_sub(left)),
        ..column(screen, height)
    };
    frame.render_widget(Paragraph::new(lines), area);

    // At the bottom, like "• tab for menu" on the typing screen, when it
    // does not cover the menu.
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

/// One row of the logo, padded to `width`, in the theme color: the speed
/// lines plain, the keycap bold.
fn logo_line(t: Theme, row: &str, width: usize) -> Vec<Span<'static>> {
    let row = format!("{row:<width$}");
    let split = row
        .find(|c: char| c != '=' && c != ' ')
        .unwrap_or(row.len());
    let (lines, cap) = row.split_at(split);
    vec![
        Span::styled(lines.to_string(), Style::new().fg(t.main)),
        Span::styled(
            cap.to_string(),
            Style::new().fg(t.main).add_modifier(Modifier::BOLD),
        ),
    ]
}
