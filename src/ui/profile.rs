use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::column;
use super::menu::{HINT_ROWS, MARGIN, XP_BAR, keys_line, logo_lines};
use crate::app::App;
use crate::badges::BADGES;
use crate::history;
use crate::progress::{self, Level, RANKS};

/// Everything about the user: level, test stats, badges, keys. `scroll`
/// rows are hidden at the top when the terminal is too short.
pub fn draw(frame: &mut Frame, app: &App, scroll: usize) {
    let t = app.theme();
    let screen = frame.area();
    let lines = lines(app);
    let rows = visible_rows(screen.height);
    let scroll = scroll.min(lines.len().saturating_sub(rows));
    let shown: Vec<Line> = lines.into_iter().skip(scroll).take(rows).collect();
    let height = shown.len() as u16;
    let width = shown.iter().map(Line::width).max().unwrap_or(0) as u16;
    let area = column(screen, height);
    let left = (screen.width.saturating_sub(width) / 2).max(MARGIN);
    let area = Rect {
        x: screen.x + left.min(screen.width),
        width: width.min(screen.width.saturating_sub(left)),
        ..area
    };
    frame.render_widget(Paragraph::new(shown), area);
    keys_line(frame, t, height, "↑↓ scroll · tab back");
}

/// The most rows that `scroll` can hide on a terminal `height` rows high.
pub fn max_scroll(app: &App, height: u16) -> usize {
    lines(app).len().saturating_sub(visible_rows(height))
}

/// Rows for the profile, with the keys line under them.
fn visible_rows(height: u16) -> usize {
    usize::from(height.saturating_sub(HINT_ROWS)).max(1)
}

fn lines(app: &App) -> Vec<Line<'static>> {
    let t = app.theme();
    let records = app.history.records();
    let head = Style::new().fg(t.main).add_modifier(Modifier::BOLD);
    let text = Style::new().fg(t.text);
    let sub = Style::new().fg(t.sub);
    let mut out: Vec<Line> = Vec::new();

    // The level, the XP and the next rank.
    let xp = progress::total_xp(records);
    let level = Level::from_xp(xp);
    let logo = logo_lines(t, level.rank_index());
    let logo_rows = logo.len();
    out.extend(logo);
    out.push(Line::default());
    out.push(Line::styled(
        format!("level {} · rank {}", level.level, level.rank()),
        head,
    ));
    let (filled, empty) = level.bar(XP_BAR);
    out.push(Line::from(vec![
        Span::styled("━".repeat(filled), Style::new().fg(t.main)),
        Span::styled("─".repeat(empty), sub),
        Span::styled(
            format!(
                "  {} / {} xp to level {}",
                level.xp,
                level.needed,
                level.level + 1
            ),
            text,
        ),
    ]));
    let next = RANKS.get(level.rank_index() + 1).map_or_else(
        || "top rank".to_string(),
        |(at, name)| format!("next rank {name} at level {at}"),
    );
    out.push(Line::styled(format!("{xp} xp in all · {next}"), sub));

    // The tests.
    out.push(Line::default());
    out.push(Line::styled("tests", head));
    if records.is_empty() {
        out.push(Line::styled("  no tests yet", sub));
    } else {
        let n = records.len() as f64;
        let seconds: f64 = records.iter().map(|r| r.seconds).sum();
        let s = if records.len() == 1 { "" } else { "s" };
        out.push(Line::styled(
            format!("  {} test{s} · {} typed", records.len(), duration(seconds)),
            text,
        ));
        let wpm = records.iter().map(|r| r.wpm).sum::<f64>() / n;
        let acc = records.iter().map(|r| r.accuracy).sum::<f64>() / n;
        out.push(Line::styled(
            format!("  average {wpm:.0} wpm · {acc:.0}% accuracy"),
            text,
        ));
    }
    let streak = history::daily_streak(&app.history, history::today());
    if streak.days > 0 {
        let s = if streak.days == 1 { "" } else { "s" };
        let freezes = match streak.freezes {
            0 => String::new(),
            1 => " · 1 streak freeze".to_string(),
            n => format!(" · {n} streak freezes"),
        };
        out.push(Line::styled(
            format!("  daily streak {} day{s}{freezes}", streak.days),
            text,
        ));
    }
    if app.weekly_goal > 0 {
        let words = progress::this_week(records, history::today());
        let (filled, empty) = progress::goal_bar(words, app.weekly_goal, XP_BAR);
        out.push(Line::from(vec![
            Span::styled("  ", text),
            Span::styled("━".repeat(filled), Style::new().fg(t.main)),
            Span::styled("─".repeat(empty), sub),
            Span::styled(
                format!("  {words:.0} of {} words this week", app.weekly_goal),
                text,
            ),
        ]));
    }

    // The best speed of each mode.
    let bests = app.history.bests();
    if !bests.is_empty() {
        out.push(Line::default());
        out.push(Line::styled("best wpm", head));
        for (label, wpm) in bests {
            out.push(Line::from(vec![
                Span::styled(format!("  {wpm:>4.0}  "), Style::new().fg(t.main)),
                Span::styled(label.to_string(), text),
            ]));
        }
    }

    // Each badge: earned ones with the day, hidden ones as "???".
    let got = (BADGES.iter())
        .filter(|b| app.badges.time(b.name).is_some())
        .count();
    out.push(Line::default());
    out.push(Line::styled(
        format!("badges {got} of {}", BADGES.len()),
        head,
    ));
    let width = BADGES.iter().map(|b| b.name.len()).max().unwrap_or(0);
    for b in &BADGES {
        out.push(match app.badges.time(b.name) {
            Some(time) => Line::from(vec![
                Span::styled(
                    format!("  {} {:<width$}  ", b.symbol, b.name),
                    Style::new().fg(t.main),
                ),
                Span::styled(format!("{}  ", b.goal), text),
                Span::styled(history::format_time(time)[..10].to_string(), sub),
            ]),
            None if b.hidden => Line::styled(format!("  [?] {:<width$}  hidden", "???"), sub),
            None => Line::styled(
                format!("  {} {:<width$}  {}", b.symbol, b.name, b.goal),
                sub,
            ),
        });
    }

    // The weak keys and the key quests.
    out.push(Line::default());
    out.push(Line::styled("keys", head));
    let letters: Vec<String> = app.weak_letters().iter().map(char::to_string).collect();
    let pairs = app.weak_pairs();
    let or_none = |v: &[String]| {
        if v.is_empty() {
            "no data yet".to_string()
        } else {
            v.join(" ")
        }
    };
    out.push(Line::styled(
        format!("  weak keys   {}", or_none(&letters)),
        text,
    ));
    out.push(Line::styled(
        format!("  weak pairs  {}", or_none(&pairs)),
        text,
    ));
    out.push(Line::styled(
        format!("  key quests  {} done", app.quests.done_count()),
        text,
    ));
    for q in app.quests.open() {
        let now = (app.key_stats.get(q.key).avg_ms())
            .map_or(String::new(), |ms| format!(" (now {ms:.0} ms)"));
        out.push(Line::styled(format!("    {}{now}", q.goal()), sub));
    }
    // The logo goes in the middle, over the rest.
    let rest = out[logo_rows..].iter().map(Line::width).max().unwrap_or(0);
    let pad = rest.saturating_sub(out[0].width()) / 2;
    for line in &mut out[..logo_rows] {
        line.spans.insert(0, Span::raw(" ".repeat(pad)));
    }
    out
}

/// "1h 05m", "12m 30s" or "45s".
fn duration(seconds: f64) -> String {
    let s = seconds.round() as u64;
    match (s / 3600, s / 60 % 60, s % 60) {
        (0, 0, s) => format!("{s}s"),
        (0, m, s) => format!("{m}m {s:02}s"),
        (h, m, _) => format!("{h}h {m:02}m"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_use_the_largest_units() {
        assert_eq!(duration(45.0), "45s");
        assert_eq!(duration(750.0), "12m 30s");
        assert_eq!(duration(3900.0), "1h 05m");
    }
}
