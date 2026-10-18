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
    let t = app.theme;
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
        Line::from(label(&results_label(app))),
        Line::from(label(
            &app.source.map_or(String::new(), |s| format!("— {s}")),
        )),
        best_line(app, r),
        Line::from(label("enter restart · tab menu · ctrl+c quit")),
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
    let t = app.theme;
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
    if app.save_failed {
        spans.push(Span::styled(
            " · history not saved",
            Style::new().fg(t.error),
        ));
    }
    Line::from(spans)
}

/// The mode and options, with the word list. Quotes have no word list.
fn results_label(app: &App) -> String {
    match app.mode {
        Mode::Quote => app.label(),
        _ if app.options.list == WordList::English => format!("{} · english", app.label()),
        _ => app.label(),
    }
}
