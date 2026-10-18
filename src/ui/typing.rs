use std::ops::Range;
use std::time::Instant;

use ratatui::{
    Frame,
    layout::{Position, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::{column, theme};
use crate::app::App;
use crate::engine::Mode;

/// Number of word lines on screen, like monkeytype.
const VISIBLE_LINES: usize = 3;

pub fn draw(frame: &mut Frame, app: &App, now: Instant) {
    // Rows: status, gap, 3 word lines, 2 gap, hints.
    let area = column(frame.area(), 8);
    let s = &app.session;

    let status = if s.is_started() {
        let counter = match s.mode() {
            Mode::Time(_) => {
                let left = s.remaining(now).unwrap_or_default();
                format!("{}", left.as_secs_f64().ceil() as u64)
            }
            Mode::Words(n) => format!("{}/{n}", s.current()),
        };
        Line::from(vec![
            Span::styled(counter, Style::new().fg(theme::MAIN)),
            Span::styled(
                format!("  {:.0} wpm", s.live_wpm(now)),
                Style::new().fg(theme::SUB),
            ),
        ])
    } else {
        Line::styled(app.label(), Style::new().fg(theme::SUB))
    };
    frame.render_widget(Paragraph::new(status), row(area, 0));

    let width = area.width as usize;
    let widths: Vec<usize> = s
        .words()
        .iter()
        .zip(s.typed())
        .map(|(w, t)| w.len().max(t.len()))
        .collect();
    let lines = wrap(&widths, width);
    let current = s.current().min(s.words().len() - 1);
    let current_line = lines.iter().position(|l| l.contains(&current)).unwrap_or(0);
    let first = current_line.saturating_sub(1);

    for (offset, range) in lines.iter().skip(first).take(VISIBLE_LINES).enumerate() {
        let y = area.y + 2 + offset as u16;
        let mut spans = Vec::new();
        let mut x = 0;
        for i in range.clone() {
            if i > range.start {
                spans.push(Span::raw(" "));
                x += 1;
            }
            if i == current {
                let caret_x = (x + s.typed()[i].len()).min(width.saturating_sub(1));
                frame.set_cursor_position(Position::new(area.x + caret_x as u16, y));
            }
            word_spans(&s.words()[i], &s.typed()[i], i < s.current(), &mut spans);
            x += widths[i];
        }
        let line_area = Rect {
            y,
            height: 1,
            ..area
        };
        frame.render_widget(Paragraph::new(Line::from(spans)), line_area);
    }

    let hints = Line::styled(
        "tab restart · esc menu · ctrl+c quit",
        Style::new().fg(theme::SUB),
    );
    frame.render_widget(Paragraph::new(hints).centered(), row(area, 7));
}

fn row(area: Rect, n: u16) -> Rect {
    Rect {
        y: area.y + n,
        height: 1,
        ..area
    }
}

/// Colors each letter of one word, the way monkeytype does.
fn word_spans(target: &[char], typed: &[char], done: bool, out: &mut Vec<Span<'static>>) {
    let has_error = done && typed != target;
    let base = if has_error {
        Style::new()
            .add_modifier(Modifier::UNDERLINED)
            .underline_color(theme::ERROR)
    } else {
        Style::new()
    };
    for j in 0..target.len().max(typed.len()) {
        let (ch, color) = match (target.get(j), typed.get(j)) {
            (Some(&t), Some(&c)) if t == c => (t, theme::TEXT),
            (Some(&t), Some(_)) => (t, theme::ERROR),
            (None, Some(&c)) => (c, theme::ERROR_EXTRA),
            (Some(&t), None) => (t, theme::SUB),
            (None, None) => unreachable!(),
        };
        out.push(Span::styled(ch.to_string(), base.fg(color)));
    }
}

/// Splits words into lines no wider than `width`, with one space between words.
/// A word wider than `width` gets a line of its own.
pub fn wrap(widths: &[usize], width: usize) -> Vec<Range<usize>> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut used = 0;
    for (i, &w) in widths.iter().enumerate() {
        if i > start && used + 1 + w > width {
            lines.push(start..i);
            start = i;
            used = w;
        } else {
            used += if i > start { 1 + w } else { w };
        }
    }
    if start < widths.len() {
        lines.push(start..widths.len());
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_fills_lines_up_to_the_width() {
        // "aaa bb" = 6, then "cccc" does not fit in 8.
        assert_eq!(wrap(&[3, 2, 4, 1], 8), vec![0..2, 2..4]);
    }

    #[test]
    fn wrap_puts_a_long_word_on_its_own_line() {
        assert_eq!(wrap(&[2, 20, 2], 10), vec![0..1, 1..2, 2..3]);
    }

    #[test]
    fn wrap_exact_fit_stays_on_one_line() {
        assert_eq!(wrap(&[3, 4], 8), vec![0..2]);
    }
}
