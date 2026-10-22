use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::column;
use crate::app::{App, MODES, mode_label};
use crate::challenge::Challenge;
use crate::engine::{Strict, words::WordList};
use crate::progress::{self, Level};
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
pub(super) const MARGIN: u16 = 4;
/// Rows kept free under the menu for the keys line.
pub(super) const HINT_ROWS: u16 = 4;
/// Cells in the XP bar under the logo.
pub(super) const XP_BAR: usize = 16;

/// The menu: the logo, then one row for each setting with its key.
pub fn draw(frame: &mut Frame, app: &App) {
    let t = app.theme();
    let screen = frame.area();
    let mut rows: Vec<(char, Span)> = Vec::new();
    let mut setting = |key, name: &str, value: &str, color: Color| {
        let text = format!("{name} {value}");
        rows.push((key, Span::styled(text, Style::new().fg(color))));
    };
    let mode = match (app.daily, &app.race) {
        (Some(_), _) => "daily".to_string(),
        (None, Some(_)) => "race".to_string(),
        (None, None) => mode_label(app.mode),
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
    // The chosen theme, also in momo mode, where the momo theme is on.
    setting('t', "theme", app.theme.name, t.text);
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
    let challenge_color = if app.challenge == Challenge::Off {
        t.text
    } else {
        t.main
    };
    setting('g', "challenge", app.challenge.name(), challenge_color);

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
    // Momo mode and the profile are apart from the settings: their keys
    // open them at once.
    let mut settings = settings;
    settings.push(Line::default());
    settings.push(Line::from(vec![
        Span::styled(format!("x{}", " ".repeat(GAP)), sub),
        Span::styled(
            "momo mode",
            Style::new().fg(t.main).add_modifier(Modifier::BOLD),
        ),
    ]));
    let today = crate::history::today();
    let (daily, daily_style) = if crate::history::daily_done(&app.history, today) {
        ("done today", sub)
    } else {
        ("not done today", Style::new().fg(t.main))
    };
    settings.push(Line::from(vec![
        Span::styled(format!("d{}", " ".repeat(GAP)), sub),
        Span::styled("daily challenge ", Style::new().fg(t.text)),
        Span::styled(daily, daily_style),
    ]));
    settings.push(Line::from(vec![
        Span::styled(format!("f{}", " ".repeat(GAP)), sub),
        Span::styled("profile", Style::new().fg(t.text)),
    ]));
    let width = settings.iter().map(Line::width).max().unwrap_or(0) as u16;
    // The logo and the level go on top when there is room for them.
    let level = Level::from_xp(progress::total_xp(app.history.records()));
    let goal = app.weekly_goal > 0;
    let logo_height = LOGO.len() as u16 + 3 + u16::from(goal);
    let with_logo = screen.height >= settings.len() as u16 + logo_height + HINT_ROWS;
    let mut lines: Vec<Line> = Vec::new();
    if with_logo {
        lines.extend(logo_lines(t, level.rank_index()));
        lines.push(Line::default());
        lines.push(level_line(t, level));
        if goal {
            let words = progress::this_week(app.history.records(), crate::history::today());
            lines.push(week_line(t, words, app.weekly_goal));
        }
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
    keys_line(frame, t, height, "enter start · tab back · q quit");
    if let Some(tag) = &app.update {
        update_line(frame, t, height, tag);
    }
}

/// The list of modes, from `o` in the menu.
pub fn draw_modes(frame: &mut Frame, app: &App, selected: usize) {
    let t = app.theme();
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

/// "level 7 · rank homing" and a bar of the XP in this level.
fn level_line(t: Theme, level: Level) -> Line<'static> {
    let (filled, empty) = level.bar(XP_BAR);
    Line::from(vec![
        Span::styled(
            format!("level {} · rank {}  ", level.level, level.rank()),
            Style::new().fg(t.sub),
        ),
        Span::styled("━".repeat(filled), Style::new().fg(t.main)),
        Span::styled("─".repeat(empty), Style::new().fg(t.sub)),
    ])
}

/// "week 1234/5000" and a bar of the words typed this week.
fn week_line(t: Theme, words: f64, goal: u32) -> Line<'static> {
    let (filled, empty) = progress::goal_bar(words, goal, XP_BAR);
    Line::from(vec![
        Span::styled(format!("week {words:.0}/{goal}  "), Style::new().fg(t.sub)),
        Span::styled("━".repeat(filled), Style::new().fg(t.main)),
        Span::styled("─".repeat(empty), Style::new().fg(t.sub)),
    ])
}

/// The logo rows, padded to one width so they stay in shape when centered:
/// the speed lines plain, the keycap bold, both in the theme color. Each
/// rank above the first makes the speed lines one cell longer.
pub(super) fn logo_lines(t: Theme, rank: usize) -> Vec<Line<'static>> {
    let width = LOGO.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    LOGO.iter()
        .map(|row| {
            // Every row gets `rank` more cells, so the keycap stays in shape.
            let indent = row.len() - row.trim_start().len();
            let (front, back) = row.split_at(indent);
            let more = if back.starts_with('=') { "=" } else { " " };
            let pad = width - indent;
            let row = format!("{front}{}{back:<pad$}", more.repeat(rank));
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

/// "update v0.5.1 available · u to update", on the row above the keys, when
/// there is a free row for it.
fn update_line(frame: &mut Frame, t: Theme, menu_height: u16, tag: &str) {
    let screen = frame.area();
    if screen.height < menu_height + HINT_ROWS {
        return;
    }
    let notice = Line::styled(
        format!("update {tag} available · u to update"),
        Style::new().fg(t.main).add_modifier(Modifier::BOLD),
    );
    let area = Rect {
        y: screen.bottom().saturating_sub(3),
        height: 1,
        ..screen
    };
    frame.render_widget(Paragraph::new(notice).centered(), area);
}

/// The keys at the bottom, like "• tab for menu" on the typing screen. A
/// short terminal puts them on the last row, under the menu.
pub(super) fn keys_line(frame: &mut Frame, t: Theme, menu_height: u16, keys: &str) {
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
