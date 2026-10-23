use ratatui::{
    Frame,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::column;
use super::menu::keys_line;
use crate::app::App;
use crate::engine::{Mode, PracticeKind};

/// The status line above the words in practice: the lesson, what it
/// practises, and the WPM of the last lesson. Adaptive practice shows all
/// the letters: the unlocked ones in the text color, the focus key in the
/// theme color, and the locked ones gray.
pub fn status(app: &App) -> Vec<Span<'static>> {
    let t = app.theme();
    let sub = Style::new().fg(t.sub);
    let accent = Style::new().fg(t.main).add_modifier(Modifier::BOLD);
    let mut spans = vec![Span::styled(
        format!("lesson {}  ", app.lessons.count + 1),
        sub,
    )];
    match (app.mode, &app.lesson_letters) {
        (Mode::Practice(PracticeKind::Adaptive), Some(letters)) => {
            for (i, &c) in letters.order.iter().enumerate() {
                let style = if Some(c) == letters.focus {
                    accent
                } else if i < letters.unlocked {
                    Style::new().fg(t.text)
                } else {
                    sub
                };
                if i > 0 {
                    spans.push(Span::raw(" "));
                }
                spans.push(Span::styled(c.to_string(), style));
            }
        }
        (Mode::Practice(PracticeKind::WeakKeys), _) => {
            let letters: Vec<String> = app.weak_letters().iter().map(char::to_string).collect();
            spans.push(Span::styled(format!("weak keys {}", or_all(&letters)), sub));
        }
        (Mode::Practice(PracticeKind::WeakPairs), _) => {
            let pairs = app.weak_pairs();
            spans.push(Span::styled(format!("weak pairs {}", or_all(&pairs)), sub));
        }
        _ => {
            let left = app.all_mistakes().len();
            spans.push(Span::styled(format!("{left} mistakes left"), sub));
        }
    }
    if let Some(&new) = app.lessons.unlocked.last() {
        spans.push(Span::styled(format!("  new letter {new}"), accent));
    }
    if let Some(wpm) = app.lessons.last_wpm {
        spans.push(Span::styled(format!("  {wpm:.0} wpm"), sub));
    }
    spans
}

/// "a b c", or "(no data yet)" for no data.
fn or_all(items: &[String]) -> String {
    if items.is_empty() {
        "(no data yet)".to_string()
    } else {
        items.join(" ")
    }
}

/// The end of a practice session: the lessons, the speed, and the letters
/// that unlocked.
pub fn draw_summary(frame: &mut Frame, app: &App) {
    let t = app.theme();
    let l = &app.lessons;
    let accent = Style::new().fg(t.main).add_modifier(Modifier::BOLD);
    let text = Style::new().fg(t.text);
    let sub = Style::new().fg(t.sub);
    let seconds = l.seconds.round() as u64;
    let lessons = if l.count == 1 { "lesson" } else { "lessons" };
    let mut lines = vec![
        Line::styled(app.label(), sub),
        Line::default(),
        Line::styled(format!("{:.0} wpm", l.wpm()), accent),
        Line::styled(format!("{:.0}% accuracy", l.accuracy()), text),
        Line::default(),
        Line::styled(
            format!(
                "{} {lessons} · {}:{:02}",
                l.count,
                seconds / 60,
                seconds % 60
            ),
            text,
        ),
    ];
    if !l.unlocked.is_empty() {
        let letters: Vec<String> = l.unlocked.iter().map(char::to_string).collect();
        lines.push(Line::default());
        lines.push(Line::from(vec![
            Span::styled("new letters ", text),
            Span::styled(letters.join(" "), accent),
        ]));
    }
    let height = lines.len() as u16;
    frame.render_widget(
        Paragraph::new(lines).centered(),
        column(frame.area(), height),
    );
    keys_line(frame, t, height, "enter practise again · tab menu · q quit");
}
