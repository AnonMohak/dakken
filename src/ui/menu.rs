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

/// The dakken logo, a keycap with speed lines, above the settings.
const LOGO: [&str; 7] = [
    r"       __________",
    r" ==   /  \      /|",
    r"==   /    \    / |",
    r" =  /     /   /  /",
    r"   /     /   /  /",
    r"  /_________/  /",
    r"  \_________\ /",
];
/// Space between the key column and the text column.
const GAP: usize = 6;
/// Space kept free on the left and the right of the menu.
const MARGIN: u16 = 4;
/// Rows kept free under the menu for the keys line.
const HINT_ROWS: u16 = 4;

/// The menu: the logo, then one row for each setting with its key.
pub fn draw(frame: &mut Frame, app: &App) {
    let t = app.theme;
    let screen = frame.area();
    let mut rows: Vec<(char, Span)> = Vec::new();
    let mut setting = |key, name: &str, value: &str, color: Color| {
        let text = format!("{name} {value}");
        rows.push((key, Span::styled(text, Style::new().fg(color))));
    };
    let mode = match app.daily {
        Some(_) => "daily".to_string(),
        None => mode_label(app.mode),
    };
    setting('o', "mode", &mode, t.main);
    // Each setting shows the key that changes it.
    let on_off = |on: bool| if on { ("on", t.main) } else { ("off", t.text) };
    let (state, color) = on_off(app.options.punctuation);
    setting('p', "punctuation", state, color);
    let (state, color) = on_off(app.options.numbers);
    setting('n', "numbers", state, color);
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
    setting('l', "words", &list, t.text);
    setting('t', "theme", t.name, t.text);
    let (state, color) = on_off(app.live);
    setting('s', "live stats", state, color);
    let (state, color) = on_off(app.pace);
    setting('c', "pace caret", state, color);
    let strict_color = if app.strict == Strict::Off {
        t.text
    } else {
        t.main
    };
    setting('m', "strict", app.strict.name(), strict_color);

    let sub = Style::new().fg(t.sub);
    let settings: Vec<Line> = rows
        .into_iter()
        .map(|(key, text)| {
            Line::from(vec![
                Span::styled(format!("{key}{}", " ".repeat(GAP)), sub),
                text,
            ])
        })
        .collect();
    let width = settings.iter().map(Line::width).max().unwrap_or(0) as u16;
    // The logo goes on top when there is room for it.
    let logo_height = LOGO.len() as u16 + 1;
    let with_logo = screen.height >= settings.len() as u16 + logo_height + HINT_ROWS;
    let mut lines: Vec<Line> = Vec::new();
    if with_logo {
        lines.extend(logo_lines(t));
        lines.push(Line::default());
    }
    let logo_rows = lines.len();
    lines.extend(settings);

    let height = lines.len() as u16;
    let area = column(screen, height);
    let left = (screen.width.saturating_sub(width) / 2).max(MARGIN);
    let settings_area = Rect {
        x: screen.x + left.min(screen.width),
        y: area.y + logo_rows as u16,
        width: width.min(screen.width.saturating_sub(left)),
        height: height - logo_rows as u16,
    };
    if with_logo {
        let logo_area = Rect {
            height: logo_rows as u16,
            ..area
        };
        frame.render_widget(
            Paragraph::new(lines[..logo_rows].to_vec()).centered(),
            logo_area,
        );
    }
    frame.render_widget(Paragraph::new(lines[logo_rows..].to_vec()), settings_area);
    keys_line(frame, t, height, "enter start · tab back · ctrl+c exit");
}

/// The list of modes, from `o` in the menu.
pub fn draw_modes(frame: &mut Frame, app: &App, selected: usize) {
    let t = app.theme;
    let screen = frame.area();
    // In a short terminal, show only the modes around the selected one.
    let rows = (screen.height.saturating_sub(HINT_ROWS + 2) as usize).clamp(1, MODES.len());
    let first = selected.saturating_sub(rows / 2).min(MODES.len() - rows);
    // The title lines up with the mode names, after the "> " column.
    let mut lines = vec![
        Line::styled("  mode", Style::new().fg(t.sub)),
        Line::default(),
    ];
    for (i, &mode) in MODES.iter().enumerate().skip(first).take(rows) {
        lines.push(if i == selected {
            Line::styled(
                format!("> {}", mode_label(mode)),
                Style::new().fg(t.main).add_modifier(Modifier::BOLD),
            )
        } else {
            Line::styled(format!("  {}", mode_label(mode)), Style::new().fg(t.text))
        });
    }
    let height = lines.len() as u16;
    let width = lines.iter().map(Line::width).max().unwrap_or(0) as u16;
    let area = column(screen, height);
    let left = (screen.width.saturating_sub(width) / 2).max(MARGIN);
    let area = Rect {
        x: screen.x + left.min(screen.width),
        width: width.min(screen.width.saturating_sub(left)),
        ..area
    };
    frame.render_widget(Paragraph::new(lines), area);
    keys_line(frame, t, height, "↑↓ choose · enter select · tab back");
}

/// The logo rows, padded to one width so they stay in shape when centered:
/// the speed lines plain, the keycap bold, both in the theme color.
fn logo_lines(t: Theme) -> Vec<Line<'static>> {
    let width = LOGO.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    LOGO.iter()
        .map(|row| {
            let row = format!("{row:<width$}");
            let split = row
                .find(|c: char| c != '=' && c != ' ')
                .unwrap_or(row.len());
            let (speed, cap) = row.split_at(split);
            Line::from(vec![
                Span::styled(speed.to_string(), Style::new().fg(t.main)),
                Span::styled(
                    cap.to_string(),
                    Style::new().fg(t.main).add_modifier(Modifier::BOLD),
                ),
            ])
        })
        .collect()
}

/// The keys at the bottom, like "• tab for menu" on the typing screen. A
/// short terminal puts them on the last row, under the menu.
fn keys_line(frame: &mut Frame, t: Theme, menu_height: u16, keys: &str) {
    let screen = frame.area();
    if screen.height <= menu_height {
        return;
    }
    let below = if screen.height >= menu_height + HINT_ROWS {
        2
    } else {
        1
    };
    let hint = Line::styled(
        keys.to_string(),
        Style::new().fg(t.sub).add_modifier(Modifier::ITALIC),
    );
    let area = Rect {
        y: screen.bottom().saturating_sub(below),
        height: 1,
        ..screen
    };
    frame.render_widget(Paragraph::new(hint).centered(), area);
}
