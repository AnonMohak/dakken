use ratatui::{
    Frame,
    style::{Modifier, Style},
    text::Line,
    widgets::Paragraph,
};

use super::column;
use super::menu::{keys_line, logo_lines};
use crate::app::{App, Celebration};
use crate::badges::{BADGES, Badges};

/// A new rank or a new badge on the full screen, before the results.
pub fn draw(frame: &mut Frame, app: &App, page: usize) {
    let t = app.theme();
    let pages = app.reward.as_ref().map_or(Vec::new(), |r| r.celebrations());
    let Some(&celebration) = pages.get(page) else {
        return;
    };
    let accent = Style::new().fg(t.main).add_modifier(Modifier::BOLD);
    let sub = Style::new().fg(t.sub);
    let mut lines: Vec<Line> = Vec::new();
    match celebration {
        Celebration::Rank(level) => {
            lines.extend(logo_lines(t, level.rank_index()));
            lines.push(Line::default());
            lines.push(Line::styled("new rank", sub));
            lines.push(Line::default());
            lines.push(Line::styled(spaced(level.rank()), accent));
            lines.push(Line::default());
            lines.push(Line::styled(
                format!("level {}", level.level),
                Style::new().fg(t.text),
            ));
        }
        Celebration::Badge(name) => {
            let badge = BADGES.iter().find(|b| b.name == name);
            let symbol = badge.map_or("[*]", |b| b.symbol);
            let width = symbol.chars().count() + 4;
            lines.push(Line::styled("new badge", sub));
            lines.push(Line::default());
            lines.push(Line::styled(format!("┌{}┐", "─".repeat(width)), accent));
            lines.push(Line::styled(format!("│  {symbol}  │"), accent));
            lines.push(Line::styled(format!("└{}┘", "─".repeat(width)), accent));
            lines.push(Line::default());
            lines.push(Line::styled(spaced(name), accent));
            lines.push(Line::default());
            if let Some(b) = badge {
                lines.push(Line::styled(b.goal, Style::new().fg(t.text)));
            }
            let got = got(&app.badges);
            lines.push(Line::styled(
                format!("{got} of {} badges", BADGES.len()),
                sub,
            ));
        }
    }
    if pages.len() > 1 {
        lines.push(Line::default());
        lines.push(Line::styled(format!("{} / {}", page + 1, pages.len()), sub));
    }
    let height = lines.len() as u16;
    frame.render_widget(
        Paragraph::new(lines).centered(),
        column(frame.area(), height),
    );
    keys_line(frame, t, height, "tab continue");
}

/// "S C U L P T E D": the name in capitals, with a space between letters.
fn spaced(name: &str) -> String {
    let letters: Vec<String> = name.to_uppercase().chars().map(String::from).collect();
    letters.join(" ")
}

/// The badges earned up to now.
fn got(badges: &Badges) -> usize {
    BADGES
        .iter()
        .filter(|b| badges.time(b.name).is_some())
        .count()
}
