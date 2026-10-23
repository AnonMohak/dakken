use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::menu::{HINT_ROWS, MARGIN, XP_BAR, keys_line, logo_lines};
use crate::app::App;
use crate::badges::BADGES;
use crate::history;
use crate::progress::{self, Level, RANKS};
use crate::{keystats, practice, quests};
use ratatui::style::Color;

use super::ansi;
use super::theme::Theme;

/// The pages after the profile, with `→`.
const STATS_NAMES: [&str; 5] = ["stats", "history", "keys", "skills", "practice"];
pub const STATS_PAGES: usize = STATS_NAMES.len();
/// The stats page of practice, where `r` resets the adaptive practice.
pub const PRACTICE_PAGE: usize = 4;

/// Everything about the user: level, test stats, badges, keys. `scroll`
/// rows are hidden at the top when the terminal is too short.
pub fn draw(frame: &mut Frame, app: &App, scroll: usize) {
    draw_lines(frame, app, 0, lines(app), scroll);
}

/// A stats page: one of the reports that the command line prints.
pub fn draw_stats(frame: &mut Frame, app: &App, page: usize, scroll: usize) {
    draw_lines(frame, app, page + 1, stats_lines(app, page), scroll);
}

/// The most rows that `scroll` can hide on a stats page.
pub fn stats_max_scroll(app: &App, page: usize, height: u16) -> usize {
    stats_lines(app, page)
        .len()
        .saturating_sub(visible_rows(height))
}

/// The page names at the top, the one on screen in the theme color.
/// `page` 0 is the profile.
fn tabs_line(app: &App, page: usize) -> Line<'static> {
    let t = app.theme();
    let mut spans = Vec::new();
    for (i, name) in ["profile"].iter().chain(&STATS_NAMES).enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", Style::new().fg(t.sub)));
        }
        let style = if i == page {
            Style::new().fg(t.main).add_modifier(Modifier::BOLD)
        } else {
            Style::new().fg(t.sub)
        };
        spans.push(Span::styled(name.to_string(), style));
    }
    Line::from(spans)
}

/// The report of a stats page, in its colors. The report titles are bold;
/// the other rows are text.
fn stats_lines(app: &App, page: usize) -> Vec<Line<'static>> {
    let t = app.theme();
    let today = history::today();
    let records = app.history.records();
    let text = match STATS_NAMES.get(page) {
        Some(&"stats") => {
            history::stats_report(&app.history)
                + &progress::weekly_report(records, app.weekly_goal, today)
        }
        Some(&"history") => history::report(&app.history),
        Some(&"keys") => {
            keystats::heatmap(&app.key_stats, true)
                + "\n"
                + &history::day_heatmap(&app.history, today, true)
        }
        Some(&"skills") => quests::skill_map(&app.key_stats, &app.quests, true),
        _ => {
            let language = practice::language(app.options.list).name();
            let reset = if app.reset_armed {
                format!("\x1b[38;5;37mpress r again to reset {language}\x1b[0m")
            } else {
                format!("\x1b[38;5;244mr resets {language}\x1b[0m")
            };
            let all = app.all_key_stats();
            practice::report(
                &app.practice,
                app.practice_target,
                app.options.list,
                &all,
                true,
            ) + "\n  "
                + &reset
                + "\n"
        }
    };
    let head = Style::new().fg(t.main).add_modifier(Modifier::BOLD);
    (text.lines())
        .map(|row| {
            let plain = ansi::plain(row);
            let style = if plain.starts_with(' ') || plain.is_empty() {
                Style::new().fg(t.text)
            } else {
                head
            };
            themed(ansi::line(row, style), t)
        })
        .collect()
}

/// The shades of the speed calendar (ANSI 256), as the report prints them.
const SHADE_COLORS: [u8; 4] = [159, 116, 37, 23];

/// The fixed colors of a report in the colors of the theme: teal becomes
/// the theme color, dark teal and red the two error colors, gray the gray
/// of the theme. A day of the speed calendar keeps its shade character in the
/// theme color.
fn themed(mut line: Line<'static>, t: Theme) -> Line<'static> {
    for span in &mut line.spans {
        let s = span.style;
        if let (Some(Color::Indexed(f)), Some(Color::Indexed(b))) = (s.fg, s.bg)
            && f == b
            && SHADE_COLORS.contains(&f)
        {
            span.style = s.fg(t.main).bg(t.bg.unwrap_or(Color::Reset));
            continue;
        }
        span.style.fg = s.fg.map(|c| report_color(c, t));
        span.style.bg = s.bg.map(|c| report_color(c, t));
    }
    line
}

fn report_color(color: Color, t: Theme) -> Color {
    match color {
        Color::Indexed(6 | 14 | 37 | 43) => t.main,
        Color::Indexed(23) => t.error_extra,
        Color::Indexed(1 | 9) => t.error,
        Color::Indexed(8 | 244) => t.sub,
        // Black or white letters on a colored key.
        Color::Indexed(0 | 15) => t.bg.unwrap_or(Color::Black),
        color => color,
    }
}

/// The widest row of the profile and of every stats page, so each page
/// starts in the same column.
fn pages_width(app: &App) -> u16 {
    (0..STATS_PAGES)
        .map(|page| stats_lines(app, page))
        .chain([lines(app)])
        .flat_map(|lines| lines.iter().map(Line::width).collect::<Vec<_>>())
        .max()
        .unwrap_or(0) as u16
}

/// Draw page `page` (0 is the profile): the page names fixed at the top,
/// in the middle of the screen, and under them `lines`, with `scroll` rows
/// hidden at the top when the terminal is too short. Every page starts in
/// the same column, so the arrows and scrolling move nothing to the side.
fn draw_lines(frame: &mut Frame, app: &App, page: usize, lines: Vec<Line<'static>>, scroll: usize) {
    let t = app.theme();
    let screen = frame.area();
    let tabs = Rect {
        y: screen.y + TOP.min(screen.height),
        height: 1.min(screen.height.saturating_sub(TOP)),
        ..screen
    };
    frame.render_widget(Paragraph::new(tabs_line(app, page)).centered(), tabs);
    let rows = visible_rows(screen.height);
    let scroll = scroll.min(lines.len().saturating_sub(rows));
    let width = pages_width(app);
    let shown: Vec<Line> = lines.into_iter().skip(scroll).take(rows).collect();
    let height = shown.len() as u16;
    let left = (screen.width.saturating_sub(width) / 2).max(MARGIN);
    let top = TOP + TABS_ROWS;
    let area = Rect {
        x: screen.x + left.min(screen.width),
        y: screen.y + top.min(screen.height),
        width: width.min(screen.width.saturating_sub(left)),
        height: height.min(screen.height.saturating_sub(top)),
    };
    frame.render_widget(Paragraph::new(shown), area);
    keys_line(
        frame,
        t,
        height + TABS_ROWS,
        "←→ pages · ↑↓ scroll · tab back",
    );
}

/// The most rows that `scroll` can hide on a terminal `height` rows high.
pub fn max_scroll(app: &App, height: u16) -> usize {
    lines(app).len().saturating_sub(visible_rows(height))
}

/// Free rows above the page names.
const TOP: u16 = 1;
/// The page names and the empty row under them.
const TABS_ROWS: u16 = 2;

/// Rows for a page, under the page names and with the keys line under it.
fn visible_rows(height: u16) -> usize {
    usize::from(height.saturating_sub(HINT_ROWS + TOP + TABS_ROWS)).max(1)
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
    // The rows of the logo, under the page names.
    let logo_rows = out.len()..out.len() + logo.len();
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
    let rest = out[logo_rows.end..]
        .iter()
        .map(Line::width)
        .max()
        .unwrap_or(0);
    let pad = rest.saturating_sub(out[logo_rows.start].width()) / 2;
    for line in &mut out[logo_rows] {
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
