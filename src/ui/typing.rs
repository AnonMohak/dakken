use std::ops::Range;
use std::time::Instant;

use ratatui::{
    Frame,
    layout::{Position, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::{theme::Theme, typing_area, word_lines};
use crate::app::App;
use crate::engine::{Mode, Session};

/// Draw the test `s`: the current test, or a replay of it.
pub fn draw(frame: &mut Frame, app: &App, s: &Session, now: Instant, replay: bool) {
    // Only the words, in the middle of the screen.
    let area = typing_area(frame.area());
    let visible = word_lines(area.height);
    let t = app.theme();

    // Above the words, on the left: the momo title, the climb round, the
    // lives in survival, and the live stats when they are on. The timer is
    // on the right.
    let mut status: Vec<Vec<Span>> = Vec::new();
    let sub = |text: String| vec![Span::styled(text, Style::new().fg(t.sub))];
    if s.mode() == Mode::Momo {
        status.push(vec![Span::styled(
            "momo mode",
            Style::new().fg(t.main).add_modifier(Modifier::BOLD),
        )]);
    }
    if let (true, Some(climb)) = (app.climb_on() && !replay, app.climb) {
        status.push(sub(format!(
            "round {} · {:.0} wpm",
            climb.round, climb.target
        )));
    }
    // One heart for each life left, with a gap between them. A lost life
    // takes its heart away.
    if let Some(lives) = s.lives().filter(|&l| l > 0) {
        status.push(vec![Span::styled(
            vec![if app.ascii { "<3" } else { "♥" }; lives as usize].join(" "),
            Style::new().fg(t.main).add_modifier(Modifier::BOLD),
        )]);
    }
    if app.live && s.is_started() {
        status.push(sub(format!("{:.0} wpm", s.live_wpm(now))));
        status.push(sub(format!("{:.0}%", s.live_accuracy())));
    }
    if !status.is_empty() {
        let mut spans = Vec::new();
        for (i, part) in status.into_iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw("  "));
            }
            spans.extend(part);
        }
        let timer = Line::from(spans);
        if area.y >= 2 {
            let timer_area = Rect {
                y: area.y - 2,
                height: 1,
                ..area
            };
            frame.render_widget(Paragraph::new(timer), timer_area);
        }
    }
    // Time and momo tests show the seconds left, from before the start.
    if let (Some(left), true) = (s.remaining(now), area.y >= 2) {
        let seconds = Line::styled(
            format!("{}", left.as_secs_f64().ceil() as u64),
            Style::new().fg(t.main).add_modifier(Modifier::BOLD),
        );
        let timer_area = Rect {
            y: area.y - 2,
            height: 1,
            ..area
        };
        frame.render_widget(Paragraph::new(seconds).right_aligned(), timer_area);
    }

    let width = area.width as usize;
    let widths: Vec<usize> = s
        .words()
        .iter()
        .zip(s.typed())
        .map(|(w, t)| w.len().max(t.len()))
        .collect();
    // Momo mode shows one quote at a time, and who said it under it.
    let momo = (s.mode() == Mode::Momo)
        .then(|| app.momo_current(s))
        .flatten();
    let lines: Vec<Range<usize>> = match &momo {
        Some((quote, _)) => wrap(&widths[quote.clone()], width)
            .into_iter()
            .map(|l| l.start + quote.start..l.end + quote.start)
            .collect(),
        None => wrap(&widths, width),
    };
    let current = s.current().min(s.words().len() - 1);
    let current_line = lines.iter().position(|l| l.contains(&current)).unwrap_or(0);
    // Scroll only when the text is longer than the area. Then the current
    // line is the second line, so the line before it stays in view.
    let first = current_line
        .saturating_sub(1)
        .min(lines.len().saturating_sub(visible));
    // The pace caret replays the personal-best test for this mode and
    // options. A best from before dakken kept the key times moves at the
    // best speed.
    // In a climb, the pace caret moves at the target speed of the round.
    let climb = app.climb.filter(|_| app.climb_on() && !replay);
    let pace = if let Some(race) = app.race.as_ref().filter(|_| !replay) {
        s.run_position(&race.times, now)
    } else if let Some(climb) = climb {
        s.pace_position(climb.target, now)
    } else {
        (app.pace && !replay && s.mode() != Mode::Zen)
            .then(|| {
                let label = app.label();
                match app.best_runs.get(&label) {
                    Some(times) => s.run_position(times, now),
                    None => s.pace_position(app.history.best(&label)?, now),
                }
            })
            .flatten()
    };

    for (offset, range) in lines.iter().skip(first).take(visible).enumerate() {
        // An empty row between lines.
        let y = area.y + 2 * offset as u16;
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
            let pace_letter = pace.filter(|&(w, _)| w == i).map(|(_, l)| l);
            let word = (&s.words()[i][..], &s.typed()[i][..]);
            let boss = app.boss_on() && crate::challenge::is_boss(i);
            let marks = (i < s.current(), boss, app.underline_errors);
            word_spans(t, word, marks, pace_letter, &mut spans);
            x += widths[i];
        }
        let line_area = Rect {
            y,
            height: 1,
            ..area
        };
        frame.render_widget(Paragraph::new(Line::from(spans)), line_area);
    }

    let screen = frame.area();
    // Two rows under the last line of the quote, on the right, in gray:
    // the mood and who said it. It is not typed.
    if let Some((_, quote)) = &momo {
        let last = lines.len().saturating_sub(1);
        let offset = last.checked_sub(first).filter(|&o| o < visible);
        if let Some(offset) = offset {
            let y = area.y + 2 * offset as u16 + 2;
            if y < screen.bottom().saturating_sub(2) {
                let who = Line::styled(
                    format!(
                        "{} {}",
                        super::ascii::mood(quote.mood, app.ascii),
                        quote.speaker
                    ),
                    Style::new().fg(t.sub),
                );
                let who_area = Rect {
                    y,
                    height: 1,
                    ..area
                };
                frame.render_widget(Paragraph::new(who).right_aligned(), who_area);
            }
        }
    }
    // Zen starts with an empty screen, so it also says how to end.
    let hint_text = if replay {
        "• replay · any key to stop"
    } else if s.mode() == Mode::Zen {
        "• tab for menu · enter to finish · ctrl+r restart"
    } else {
        "• tab for menu · ctrl+r restart"
    };
    let hint = Line::styled(
        hint_text,
        Style::new().fg(t.sub).add_modifier(Modifier::ITALIC),
    );
    let hint_area = Rect {
        y: screen.bottom().saturating_sub(2),
        height: 1,
        ..screen
    };
    frame.render_widget(Paragraph::new(hint).centered(), hint_area);
}

/// Colors each letter of one word: wrong, extra and missed letters are red.
/// The `pace` letter gets a gray background. A boss word is bold, and its
/// letters to type are in the theme color. With `underline`, the red letters
/// are also underlined, for people who do not see the red.
fn word_spans(
    theme: Theme,
    (target, typed): (&[char], &[char]),
    (done, boss, underline): (bool, bool, bool),
    pace: Option<usize>,
    out: &mut Vec<Span<'static>>,
) {
    for j in 0..target.len().max(typed.len()) {
        let (ch, color) = match (target.get(j), typed.get(j)) {
            (Some(&t), Some(&c)) if t == c => (t, theme.text),
            (Some(&t), Some(_)) => (t, theme.error),
            (None, Some(&c)) => (c, theme.error_extra),
            // Letters left out of a finished word are errors too.
            (Some(&t), None) if done => (t, theme.error),
            (Some(&t), None) if boss => (t, theme.main),
            (Some(&t), None) => (t, theme.sub),
            (None, None) => unreachable!(),
        };
        let style = Style::new().fg(color);
        let error = color == theme.error || color == theme.error_extra;
        let style = if underline && error {
            style.add_modifier(Modifier::UNDERLINED)
        } else {
            style
        };
        let style = if boss {
            style.add_modifier(Modifier::BOLD)
        } else {
            style
        };
        let style = if pace == Some(j) {
            style.bg(theme.sub)
        } else {
            style
        };
        out.push(Span::styled(ch.to_string(), style));
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
