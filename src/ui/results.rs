use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    symbols::Marker,
    text::{Line, Span},
    widgets::{Axis, Chart, Dataset, GraphType, Paragraph},
};

use super::{column, theme::Theme};

/// Most rows for the graph and its legend.
const GRAPH_HEIGHT: u16 = 12;
/// Fewer free rows than this hide the graph.
const MIN_GRAPH_HEIGHT: u16 = 6;
use crate::app::App;
use crate::engine::stats::TestResult;
use crate::engine::{Mode, words::WordList};

pub fn draw(frame: &mut Frame, app: &App, r: &TestResult) {
    let t = app.theme();
    let label = |s: &str| Span::styled(s.to_string(), Style::new().fg(t.sub));
    let big = |s: String| Span::styled(s, Style::new().fg(t.main).add_modifier(Modifier::BOLD));
    let value = |s: String| Span::styled(s, Style::new().fg(t.main));
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
        speed_line(r, t),
        Line::from(label(&results_label(app))),
        Line::from(label(&source_text(app))),
        best_line(app, r),
        climb_line(app),
        race_line(app, r),
        reward_line(app),
        badges_line(app, super::text_width(frame.area().width)),
        quests_line(app),
        Line::from(label(&keys_text(app))),
    ];
    let text_height = lines.len() as u16;
    let free = frame.area().height.saturating_sub(text_height + 2);
    let graph_height = if r.wpm_over_time.len() >= 2 && free >= MIN_GRAPH_HEIGHT {
        free.min(GRAPH_HEIGHT)
    } else {
        0
    };
    let gap = u16::from(graph_height > 0);
    let area = column(frame.area(), graph_height + gap + text_height);
    if graph_height > 0 {
        let graph_area = Rect {
            height: graph_height,
            ..area
        };
        draw_graph(frame, graph_area, r, t);
    }
    let text_area = Rect {
        y: area.y + graph_height + gap,
        height: text_height,
        ..area
    };
    frame.render_widget(Paragraph::new(lines).centered(), text_area);
}

/// WPM and raw WPM for each second, with a one-row legend above the chart.
fn draw_graph(frame: &mut Frame, area: Rect, r: &TestResult, t: Theme) {
    let points = |values: &[f64]| -> Vec<(f64, f64)> {
        values
            .iter()
            .enumerate()
            .map(|(i, &v)| ((i + 1) as f64, v))
            .collect()
    };
    let wpm = points(&r.wpm_over_time);
    let raw = points(&r.raw_per_second);
    let seconds = r.wpm_over_time.len() as f64;
    let top = r
        .raw_per_second
        .iter()
        .chain(&r.wpm_over_time)
        .copied()
        .fold(0.0, f64::max);
    // Round the top of the y axis up to a multiple of 20.
    let top = ((top / 20.0).ceil() * 20.0).max(20.0);

    let legend = Line::from(vec![
        Span::styled("── wpm", Style::new().fg(t.main)),
        Span::styled("   ── raw", Style::new().fg(t.sub)),
    ]);
    frame.render_widget(
        Paragraph::new(legend).centered(),
        Rect { height: 1, ..area },
    );

    let line = |name: &'static str, data, color| {
        Dataset::default()
            .name(name)
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::new().fg(color))
            .data(data)
    };
    let axis_style = Style::new().fg(t.sub);
    let chart = Chart::new(vec![line("raw", &raw, t.sub), line("wpm", &wpm, t.main)])
        .legend_position(None)
        .x_axis(
            Axis::default()
                .bounds([1.0, seconds])
                .labels(["1".to_string(), format!("{seconds:.0}s")])
                .style(axis_style),
        )
        .y_axis(
            Axis::default()
                .bounds([0.0, top])
                .labels(["0".to_string(), format!("{top:.0}")])
                .style(axis_style),
        );
    let chart_area = Rect {
        y: area.y + 1,
        height: area.height - 1,
        ..area
    };
    frame.render_widget(chart, chart_area);
}

/// "new personal best", or the best WPM before this test.
fn best_line(app: &App, r: &TestResult) -> Line<'static> {
    let t = app.theme();
    if app.session.is_failed() {
        return Line::styled(
            "test failed: a wrong key in master mode",
            Style::new().fg(t.error).add_modifier(Modifier::BOLD),
        );
    }
    let mut spans = match app.previous_best {
        Some(best) if r.wpm <= best => vec![Span::styled(
            format!("best {best:.0} wpm"),
            Style::new().fg(t.sub),
        )],
        _ => vec![Span::styled(
            "new personal best",
            Style::new().fg(t.main).add_modifier(Modifier::BOLD),
        )],
    };
    if app.session.lives() == Some(0) {
        spans.push(Span::styled(" · out of lives", Style::new().fg(t.error)));
    }
    if app.save_failed {
        spans.push(Span::styled(
            " · history not saved",
            Style::new().fg(t.error),
        ));
    }
    Line::from(spans)
}

/// The keys of the results screen, with `y` after a daily challenge and
/// `x` for a test that can be a race.
fn keys_text(app: &App) -> String {
    let mut keys = vec!["enter restart", "r replay"];
    if app.can_save_race() {
        keys.push("x save race");
    }
    match (app.daily, app.copied) {
        (Some(_), true) => keys.insert(0, "score card copied"),
        (Some(_), false) if app.reward.is_some() => keys.push("y copy score card"),
        _ => {}
    }
    keys.push("tab menu");
    keys.join(" · ")
}

/// Who won the race, or where the race file went after `x`.
fn race_line(app: &App, r: &TestResult) -> Line<'static> {
    let t = app.theme();
    if let Some(saved) = &app.race_saved {
        return Line::styled(saved.clone(), Style::new().fg(t.sub));
    }
    let Some(race) = &app.race else {
        return Line::default();
    };
    let gap = r.wpm - race.wpm;
    if gap > 0.0 {
        Line::styled(
            format!("you beat the ghost by {gap:.0} wpm"),
            Style::new().fg(t.main).add_modifier(Modifier::BOLD),
        )
    } else {
        Line::styled(
            format!("the ghost won by {:.0} wpm", -gap),
            Style::new().fg(t.sub),
        )
    }
}

/// The climb round: passed with the next target, or the end of the climb.
fn climb_line(app: &App) -> Line<'static> {
    let t = app.theme();
    match app.climb_result {
        Some((c, true)) => Line::styled(
            format!(
                "round {} passed · next {:.0} wpm",
                c.round,
                c.target + crate::challenge::CLIMB_STEP
            ),
            Style::new().fg(t.main).add_modifier(Modifier::BOLD),
        ),
        Some((c, false)) => Line::styled(
            format!(
                "climb over at round {} ({:.0} wpm, {:.0}% accuracy needed)",
                c.round,
                c.target,
                crate::challenge::CLIMB_ACCURACY
            ),
            Style::new().fg(t.error),
        ),
        None => Line::default(),
    }
}

/// The XP of the test, then a new level and new badges in the accent color.
fn reward_line(app: &App) -> Line<'static> {
    let t = app.theme();
    let Some(reward) = &app.reward else {
        return Line::default();
    };
    let accent = Style::new().fg(t.main).add_modifier(Modifier::BOLD);
    let mut spans = vec![Span::styled(
        format!("+{} xp", reward.xp),
        Style::new().fg(t.sub),
    )];
    let after = reward.after;
    if after.level > reward.before.level {
        spans.push(Span::styled(" · ", Style::new().fg(t.sub)));
        let rank_up = after.rank_index() > reward.before.rank_index();
        let text = if rank_up {
            format!("level {} · new rank {}", after.level, after.rank())
        } else {
            format!("level {}", after.level)
        };
        spans.push(Span::styled(text, accent));
    }
    if let Some((beaten, total)) = reward.bosses {
        spans.push(Span::styled(" · ", Style::new().fg(t.sub)));
        spans.push(Span::styled(
            format!("bosses {beaten}/{total}"),
            Style::new().fg(t.main),
        ));
    }
    if reward.goal_reached {
        spans.push(Span::styled(" · ", Style::new().fg(t.sub)));
        spans.push(Span::styled("weekly goal reached", accent));
    }
    Line::from(spans)
}

/// The key quests this test finished.
fn quests_line(app: &App) -> Line<'static> {
    let done = app.reward.as_ref().map_or(&[][..], |r| &r.quests[..]);
    if done.is_empty() {
        return Line::default();
    }
    let goals: Vec<String> = done.iter().map(|q| q.goal()).collect();
    let s = if done.len() == 1 { "" } else { "s" };
    Line::styled(
        format!("quest{s} done: {}", goals.join(", ")),
        Style::new()
            .fg(app.theme().main)
            .add_modifier(Modifier::BOLD),
    )
}

/// The new badges, or how many there are when the names do not fit.
fn badges_line(app: &App, width: u16) -> Line<'static> {
    let badges = app.reward.as_ref().map_or(&[][..], |r| &r.badges[..]);
    let s = if badges.len() == 1 { "" } else { "s" };
    let text = match badges {
        [] => String::new(),
        _ => {
            let names: Vec<String> = badges
                .iter()
                .map(|b| crate::badges::with_symbol(b))
                .collect();
            format!("new badge{s}: {}", names.join(", "))
        }
    };
    let text = if text.chars().count() > width as usize {
        format!("{} new badges · dakken --badges", badges.len())
    } else {
        text
    };
    Line::styled(
        text,
        Style::new()
            .fg(app.theme().main)
            .add_modifier(Modifier::BOLD),
    )
}

/// Where the quote came from. In momo mode: the quotes typed to the end
/// and who said them.
fn source_text(app: &App) -> String {
    if app.momo {
        let done = app.momo_done();
        let speakers: Vec<String> = done
            .iter()
            .map(|q| format!("{} {}", super::ascii::mood(q.mood, app.ascii), q.speaker))
            .collect();
        return match done.len() {
            0 => "no quote to the end".to_string(),
            1 => format!("1 quote — {}, {}", speakers[0], done[0].series),
            n => format!("{n} quotes — {}", speakers.join(", ")),
        };
    }
    app.source.map_or(String::new(), |s| format!("— {s}"))
}

/// The mode and options, with the word list. Quotes have no word list.
fn results_label(app: &App) -> String {
    match app.mode_now() {
        Mode::Quote(_) | Mode::Momo | Mode::Custom | Mode::Zen => app.label(),
        _ if app.options.list == WordList::English => format!("{} · english", app.label()),
        _ => app.label(),
    }
}

/// The slowest word and the fastest burst, or nothing if no word counts.
fn speed_line(r: &TestResult, t: Theme) -> Line<'static> {
    let (Some((word, slow)), Some(fast)) = (&r.slowest_word, r.fastest_burst) else {
        return Line::default();
    };
    Line::styled(
        format!("slowest word \"{word}\" {slow:.0} wpm · fastest burst {fast:.0} wpm"),
        Style::new().fg(t.sub),
    )
}
