use ratatui::{
    Frame,
    style::{Modifier, Style},
    text::Line,
    widgets::Paragraph,
};

use super::column;
use crate::app::{App, MODES, mode_label};
use crate::ui::theme::Theme;

/// Rows that are not mode names: title, gaps, settings and hints.
const OTHER_ROWS: u16 = 10;

pub fn draw(frame: &mut Frame, app: &App, selected: usize) {
    let t = app.theme;
    let mut lines = vec![
        Line::styled("mode", Style::new().fg(t.sub)),
        Line::default(),
    ];
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
    lines.push(toggle(t, "punctuation", app.options.punctuation));
    lines.push(toggle(t, "numbers", app.options.numbers));
    lines.push(Line::styled(
        format!("theme {}", t.name),
        Style::new().fg(t.text),
    ));
    lines.push(Line::default());
    lines.push(Line::styled(
        "↑↓ select · enter start · esc back",
        Style::new().fg(t.sub),
    ));
    lines.push(Line::styled(
        "p punctuation · n numbers · t theme",
        Style::new().fg(t.sub),
    ));
    let area = column(frame.area(), lines.len() as u16);
    frame.render_widget(Paragraph::new(lines).centered(), area);
}

fn toggle(t: Theme, name: &str, on: bool) -> Line<'static> {
    let (state, color) = if on { ("on", t.main) } else { ("off", t.text) };
    Line::styled(format!("{name} {state}"), Style::new().fg(color))
}
