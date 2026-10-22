use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::column;
use super::menu::{HINT_ROWS, MARGIN, keys_line};
use crate::app::{App, SETTINGS};

/// The settings that are not test options, one row each, from `e` in the
/// menu. The selected row is in the theme color.
pub fn draw(frame: &mut Frame, app: &App, selected: usize) {
    let t = app.theme();
    let screen = frame.area();
    let width = SETTINGS.iter().map(|s| s.name().len()).max().unwrap_or(0) + 2;
    let mut lines = vec![
        Line::styled("  settings", Style::new().fg(t.sub)),
        Line::default(),
    ];
    // In a short terminal, show only the rows around the selected one.
    let rows = (screen.height.saturating_sub(HINT_ROWS + 2) as usize).clamp(1, SETTINGS.len());
    let first = selected.saturating_sub(rows / 2).min(SETTINGS.len() - rows);
    for (i, &setting) in SETTINGS.iter().enumerate().skip(first).take(rows) {
        let name = format!("{:<width$}", setting.name());
        let value = app.setting_value(setting);
        lines.push(if i == selected {
            let style = Style::new().fg(t.main).add_modifier(Modifier::BOLD);
            Line::from(vec![
                Span::styled(format!("> {name}"), style),
                Span::styled(format!("< {value} >"), style),
            ])
        } else {
            Line::from(vec![
                Span::styled(format!("  {name}"), Style::new().fg(t.text)),
                Span::styled(format!("  {value}"), Style::new().fg(t.sub)),
            ])
        });
    }
    let height = lines.len() as u16;
    let width = lines.iter().map(Line::width).max().unwrap_or(0) as u16 + 2;
    let area = column(screen, height);
    let left = (screen.width.saturating_sub(width) / 2).max(MARGIN);
    let area = Rect {
        x: screen.x + left.min(screen.width),
        width: width.min(screen.width.saturating_sub(left)),
        ..area
    };
    frame.render_widget(Paragraph::new(lines), area);
    keys_line(frame, t, height, "↑↓ choose · ←→ change · tab back");
}
