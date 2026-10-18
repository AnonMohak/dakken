use ratatui::{
    Frame,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::{column, theme};
use crate::app::App;
use crate::engine::stats::TestResult;

pub fn draw(frame: &mut Frame, app: &App, r: &TestResult) {
    let label = |s: &str| Span::styled(s.to_string(), Style::new().fg(theme::SUB));
    let big =
        |s: String| Span::styled(s, Style::new().fg(theme::MAIN).add_modifier(Modifier::BOLD));
    let value = |s: String| Span::styled(s, Style::new().fg(theme::MAIN));
    let gap = || Span::raw("     ");
    let c = r.chars;

    let lines = vec![
        Line::from(vec![
            label("wpm "),
            big(format!("{:.0}", r.wpm)),
            gap(),
            label("acc "),
            big(format!("{:.0}%", r.accuracy)),
        ]),
        Line::default(),
        Line::from(vec![
            label("raw "),
            value(format!("{:.0}", r.raw_wpm)),
            gap(),
            label("consistency "),
            value(format!("{:.0}%", r.consistency)),
        ]),
        Line::from(vec![
            label("characters "),
            value(format!(
                "{}/{}/{}/{}",
                c.correct, c.incorrect, c.extra, c.missed
            )),
            gap(),
            label("time "),
            value(format!("{:.0}s", r.duration.as_secs_f64())),
        ]),
        Line::from(label(&format!("{} · english", app.label()))),
        Line::default(),
        best_line(app, r),
        Line::from(label("tab restart · esc menu · ctrl+c quit")),
    ];
    let area = column(frame.area(), lines.len() as u16);
    frame.render_widget(Paragraph::new(lines).centered(), area);
}

/// "new personal best", or the best WPM before this test.
fn best_line(app: &App, r: &TestResult) -> Line<'static> {
    let mut spans = match app.previous_best {
        Some(best) if r.wpm <= best => vec![Span::styled(
            format!("best {best:.0} wpm"),
            Style::new().fg(theme::SUB),
        )],
        _ => vec![Span::styled(
            "new personal best",
            Style::new().fg(theme::MAIN).add_modifier(Modifier::BOLD),
        )],
    };
    if app.save_failed {
        spans.push(Span::styled(
            " · history not saved",
            Style::new().fg(theme::ERROR),
        ));
    }
    Line::from(spans)
}
