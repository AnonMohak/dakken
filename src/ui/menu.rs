use ratatui::{
    Frame,
    style::{Modifier, Style},
    text::Line,
    widgets::Paragraph,
};

use super::{column, theme};
use crate::app::{MODES, Options, mode_label};

pub fn draw(frame: &mut Frame, selected: usize, options: Options) {
    let mut lines = vec![
        Line::styled("mode", Style::new().fg(theme::SUB)),
        Line::default(),
    ];
    for (i, &mode) in MODES.iter().enumerate() {
        let line = if i == selected {
            Line::styled(
                format!("> {}", mode_label(mode)),
                Style::new().fg(theme::MAIN).add_modifier(Modifier::BOLD),
            )
        } else {
            Line::styled(
                format!("  {}", mode_label(mode)),
                Style::new().fg(theme::TEXT),
            )
        };
        lines.push(line);
    }
    lines.push(Line::default());
    lines.push(toggle("punctuation", options.punctuation));
    lines.push(toggle("numbers", options.numbers));
    lines.push(Line::default());
    lines.push(Line::styled(
        "↑↓ select · enter start · esc back",
        Style::new().fg(theme::SUB),
    ));
    lines.push(Line::styled(
        "p punctuation · n numbers",
        Style::new().fg(theme::SUB),
    ));
    let area = column(frame.area(), lines.len() as u16);
    frame.render_widget(Paragraph::new(lines).centered(), area);
}

fn toggle(name: &str, on: bool) -> Line<'static> {
    let (state, color) = if on {
        ("on", theme::MAIN)
    } else {
        ("off", theme::TEXT)
    };
    Line::styled(format!("{name} {state}"), Style::new().fg(color))
}
