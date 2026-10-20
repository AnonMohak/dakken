//! Saved test results and personal bests, one tab-separated line per test.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::engine::stats::TestResult;

#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    /// Seconds since 1970-01-01 UTC.
    pub time: u64,
    /// The mode and options, for example "time 30 · punctuation".
    pub label: String,
    pub wpm: f64,
    pub raw_wpm: f64,
    pub accuracy: f64,
    pub consistency: f64,
    pub seconds: f64,
}

impl Record {
    pub fn new(label: String, r: &TestResult) -> Self {
        Self {
            time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_secs()),
            label,
            wpm: r.wpm,
            raw_wpm: r.raw_wpm,
            accuracy: r.accuracy,
            consistency: r.consistency,
            seconds: r.duration.as_secs_f64(),
        }
    }

    fn to_line(&self) -> String {
        format!(
            "{}\t{}\t{:.2}\t{:.2}\t{:.2}\t{:.2}\t{:.2}",
            self.time,
            self.label,
            self.wpm,
            self.raw_wpm,
            self.accuracy,
            self.consistency,
            self.seconds
        )
    }

    fn from_line(line: &str) -> Option<Self> {
        let mut f = line.split('\t');
        let record = Self {
            time: f.next()?.parse().ok()?,
            label: f.next()?.to_string(),
            wpm: f.next()?.parse().ok()?,
            raw_wpm: f.next()?.parse().ok()?,
            accuracy: f.next()?.parse().ok()?,
            consistency: f.next()?.parse().ok()?,
            seconds: f.next()?.parse().ok()?,
        };
        Some(record)
    }
}

pub struct History {
    /// `None` keeps the history in memory only.
    path: Option<PathBuf>,
    records: Vec<Record>,
}

impl History {
    pub fn in_memory() -> Self {
        Self {
            path: None,
            records: Vec::new(),
        }
    }

    /// Read the history file. A missing file is an empty history, and bad
    /// lines are skipped.
    pub fn load(path: PathBuf) -> Self {
        let records = fs::read_to_string(&path)
            .unwrap_or_default()
            .lines()
            .filter_map(Record::from_line)
            .collect();
        Self {
            path: Some(path),
            records,
        }
    }

    pub fn records(&self) -> &[Record] {
        &self.records
    }

    /// The highest WPM for tests with this label.
    pub fn best(&self, label: &str) -> Option<f64> {
        self.records
            .iter()
            .filter(|r| r.label == label)
            .map(|r| r.wpm)
            .reduce(f64::max)
    }

    /// Each label with its best WPM, in label order.
    pub fn bests(&self) -> Vec<(&str, f64)> {
        let mut labels: Vec<&str> = self.records.iter().map(|r| r.label.as_str()).collect();
        labels.sort();
        labels.dedup();
        labels
            .into_iter()
            .filter_map(|l| Some((l, self.best(l)?)))
            .collect()
    }

    /// Keep the record, and add it to the file if there is one.
    pub fn add(&mut self, record: Record) -> io::Result<()> {
        let line = record.to_line();
        self.records.push(record);
        let Some(path) = &self.path else {
            return Ok(());
        };
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        writeln!(file, "{line}")
    }
}

/// `%APPDATA%\dakken` on Windows, else `$XDG_DATA_HOME/dakken` or
/// `~/.local/share/dakken`.
pub fn data_dir() -> Option<PathBuf> {
    let var = |name| std::env::var_os(name).filter(|v| !v.is_empty());
    let base = if cfg!(windows) {
        PathBuf::from(var("APPDATA")?)
    } else if let Some(xdg) = var("XDG_DATA_HOME") {
        PathBuf::from(xdg)
    } else {
        PathBuf::from(var("HOME")?).join(".local").join("share")
    };
    Some(base.join("dakken"))
}

pub fn default_path() -> Option<PathBuf> {
    Some(data_dir()?.join("history.tsv"))
}

/// A UTC date and time as "YYYY-MM-DD HH:MM".
pub fn format_time(unix: u64) -> String {
    let days = (unix / 86_400) as i64;
    let secs = unix % 86_400;
    // Howard Hinnant's days-to-civil algorithm.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}",
        secs / 3600,
        secs % 3600 / 60
    )
}

/// The text for `dakken --history`: personal bests, then the last 10 tests.
pub fn report(history: &History) -> String {
    if history.records().is_empty() {
        return "No tests yet.\n".to_string();
    }
    let mut out = String::from("personal bests\n");
    for (label, wpm) in history.bests() {
        out += &format!("  {wpm:>5.0} wpm  {label}\n");
    }
    out += "\nlast tests (UTC)\n";
    for r in history.records().iter().rev().take(10) {
        out += &format!(
            "  {}  {:>5.0} wpm  {:>3.0}% acc  {}\n",
            format_time(r.time),
            r.wpm,
            r.accuracy,
            r.label
        );
    }
    out
}

/// The full history as CSV, with a header line.
pub fn to_csv(history: &History) -> String {
    let mut out =
        String::from("time_utc,unix_time,mode,wpm,raw_wpm,accuracy,consistency,seconds\n");
    for r in history.records() {
        // A label can hold a file name, so quote it.
        let label = format!("\"{}\"", r.label.replace('"', "\"\""));
        out += &format!(
            "{},{},{label},{:.2},{:.2},{:.2},{:.2},{:.2}\n",
            format_time(r.time),
            r.time,
            r.wpm,
            r.raw_wpm,
            r.accuracy,
            r.consistency,
            r.seconds
        );
    }
    out
}

/// The full history as a JSON array of objects.
pub fn to_json(history: &History) -> String {
    let items: Vec<String> = history
        .records()
        .iter()
        .map(|r| {
            format!(
                "  {{\"time_utc\": \"{}\", \"unix_time\": {}, \"mode\": {}, \"wpm\": {:.2}, \"raw_wpm\": {:.2}, \"accuracy\": {:.2}, \"consistency\": {:.2}, \"seconds\": {:.2}}}",
                format_time(r.time),
                r.time,
                json_string(&r.label),
                r.wpm,
                r.raw_wpm,
                r.accuracy,
                r.consistency,
                r.seconds
            )
        })
        .collect();
    if items.is_empty() {
        return "[]\n".to_string();
    }
    format!("[\n{}\n]\n", items.join(",\n"))
}

fn json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out += "\\\"",
            '\\' => out += "\\\\",
            c if c.is_control() => out += &format!("\\u{:04x}", c as u32),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Weeks the day heatmap shows, up to this week.
const HEATMAP_WEEKS: i64 = 26;
/// Shades from slow to fast: a faster day is darker.
const SHADES: [char; 4] = ['░', '▒', '▓', '█'];
/// Teal colors (ANSI 256), the default theme color, from light to dark,
/// for the shades.
const SHADE_COLORS: [u8; 4] = [159, 116, 37, 23];

/// Today as days since 1970-01-01 UTC.
pub fn today() -> i64 {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    (secs / 86_400) as i64
}

/// A calendar of the last weeks up to `today` (days since 1970-01-01 UTC),
/// one row for each weekday. Each day with tests is shaded by its average
/// WPM, darker for faster. A day with no tests is empty.
pub fn day_heatmap(history: &History, today: i64, color: bool) -> String {
    let mut sums: std::collections::HashMap<i64, (f64, usize)> = Default::default();
    for r in history.records() {
        let day = (r.time / 86_400) as i64;
        let e = sums.entry(day).or_default();
        e.0 += r.wpm;
        e.1 += 1;
    }
    // 1970-01-01 was a Thursday; 0 is Monday.
    let weekday = |day: i64| (day + 3).rem_euclid(7);
    let first = today - weekday(today) - (HEATMAP_WEEKS - 1) * 7;
    let avg = |day: i64| sums.get(&day).map(|(sum, n)| sum / *n as f64);
    let shown: Vec<f64> = (first..=today).filter_map(avg).collect();
    let mut out = String::from("speed by day (UTC), last 26 weeks\n\n");
    if shown.is_empty() {
        out += "No tests in these weeks.\n";
        return out;
    }
    let low = shown.iter().copied().fold(f64::INFINITY, f64::min);
    let high = shown.iter().copied().fold(0.0, f64::max);
    let level = |wpm: f64| {
        if high <= low {
            return SHADES.len() - 1;
        }
        (((wpm - low) / (high - low)) * SHADES.len() as f64).min(SHADES.len() as f64 - 1.0) as usize
    };
    for (row, name) in ["Mon", "", "Wed", "", "Fri", "", "Sun"].iter().enumerate() {
        out += &format!("  {name:<4}");
        for week in 0..HEATMAP_WEEKS {
            let day = first + week * 7 + row as i64;
            let cell = match (day <= today).then(|| avg(day)).flatten() {
                None => "  ".to_string(),
                Some(wpm) => {
                    let l = level(wpm);
                    let shade = SHADES[l].to_string().repeat(2);
                    if color {
                        format!(
                            "\x1b[38;5;{0}m\x1b[48;5;{0}m{shade}\x1b[0m",
                            SHADE_COLORS[l]
                        )
                    } else {
                        shade
                    }
                }
            };
            out += &cell;
        }
        out += "\n";
    }
    out += &format!(
        "\n  slower {} faster   ({low:.0} to {high:.0} wpm; empty: no tests)\n",
        SHADES.iter().collect::<String>()
    );
    out
}

/// Days the stats graph shows, the newest days with tests.
const STATS_DAYS: usize = 30;
/// Width of the longest bar in the stats graph.
const BAR_WIDTH: usize = 40;

/// The text for `dakken --stats`: averages for each label, then the average
/// WPM of each day as a bar graph.
pub fn stats_report(history: &History) -> String {
    if history.records().is_empty() {
        return "No tests yet.\n".to_string();
    }
    let mut out = String::from("averages\n");
    out += &format!(
        "  {:>5}  {:>7}  {:>7}  {:>8}  mode\n",
        "tests", "avg wpm", "avg acc", "best wpm"
    );
    for (label, best) in history.bests() {
        let records: Vec<&Record> = history
            .records()
            .iter()
            .filter(|r| r.label == label)
            .collect();
        let n = records.len() as f64;
        let wpm = records.iter().map(|r| r.wpm).sum::<f64>() / n;
        let acc = records.iter().map(|r| r.accuracy).sum::<f64>() / n;
        out += &format!(
            "  {:>5}  {wpm:>7.0}  {acc:>6.0}%  {best:>8.0}  {label}\n",
            records.len()
        );
    }

    if let Some(best) = history.best("daily") {
        let streak = daily_streak(history, today());
        let s = if streak.days == 1 { "" } else { "s" };
        let freezes = match streak.freezes {
            0 => String::new(),
            1 => ", 1 streak freeze".to_string(),
            n => format!(", {n} streak freezes"),
        };
        out += &format!(
            "
daily challenge: {} day{s} in a row{freezes}, best {best:.0} wpm
",
            streak.days
        );
    }

    let days = daily_wpm(history);
    let shown = &days[days.len().saturating_sub(STATS_DAYS)..];
    let top = shown.iter().map(|(_, wpm)| *wpm).fold(0.0, f64::max);
    out += "\naverage wpm each day (UTC)\n";
    for (day, wpm) in shown {
        out += &format!("  {day}  {} {wpm:.0}\n", bar(*wpm, top, BAR_WIDTH));
    }
    out
}

/// Each 7 days of a streak give one streak freeze, up to this many.
pub const MAX_FREEZES: usize = 2;

/// A daily-challenge streak and the freezes kept for it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Streak {
    /// Days with a daily challenge in the streak.
    pub days: usize,
    pub freezes: usize,
}

/// The daily-challenge streak up to `today`. A missed day uses a streak
/// freeze if there is one, else it ends the streak. Today is not missed
/// until it is over.
pub fn daily_streak(history: &History, today: i64) -> Streak {
    let days: std::collections::HashSet<i64> = history
        .records()
        .iter()
        .filter(|r| r.label == "daily")
        .map(|r| (r.time / 86_400) as i64)
        .collect();
    let Some(&first) = days.iter().min() else {
        return Streak::default();
    };
    let mut streak = Streak::default();
    for day in first..=today {
        if days.contains(&day) {
            streak.days += 1;
            if streak.days % 7 == 0 {
                streak.freezes = (streak.freezes + 1).min(MAX_FREEZES);
            }
        } else if day == today {
            // Today is not over, so it is not missed yet.
        } else if streak.freezes > 0 && streak.days > 0 {
            streak.freezes -= 1;
        } else {
            streak = Streak::default();
        }
    }
    streak
}

/// The average WPM of each day that has tests, oldest first.
fn daily_wpm(history: &History) -> Vec<(String, f64)> {
    let mut days: Vec<(String, f64, usize)> = Vec::new();
    for r in history.records() {
        let day = format_time(r.time)[..10].to_string();
        match days.iter_mut().find(|(d, _, _)| *d == day) {
            Some((_, sum, n)) => {
                *sum += r.wpm;
                *n += 1;
            }
            None => days.push((day, r.wpm, 1)),
        }
    }
    days.sort_by(|a, b| a.0.cmp(&b.0));
    days.into_iter()
        .map(|(day, sum, n)| (day, sum / n as f64))
        .collect()
}

/// A bar `width` cells long for `top`, with eighth-cell steps.
fn bar(value: f64, top: f64, width: usize) -> String {
    const PARTS: [char; 8] = [' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉'];
    if top <= 0.0 {
        return String::new();
    }
    let eighths = (value / top * (width * 8) as f64).round() as usize;
    let mut s = "█".repeat(eighths / 8);
    let part = eighths % 8;
    if part > 0 {
        s.push(PARTS[part]);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(label: &str, wpm: f64) -> Record {
        Record {
            time: 0,
            label: label.to_string(),
            wpm,
            raw_wpm: wpm,
            accuracy: 100.0,
            consistency: 80.0,
            seconds: 30.0,
        }
    }

    #[test]
    fn daily_streak_counts_days_in_a_row() {
        let mut h = History::in_memory();
        for day in [10, 11, 12, 14] {
            let mut r = record("daily", 100.0);
            r.time = day * 86_400 + 60;
            h.add(r).unwrap();
        }
        h.add(record("time 30", 90.0)).unwrap();
        let days = |today| daily_streak(&h, today).days;
        assert_eq!(days(14), 1);
        // Day 15 has no test yet, so the streak to day 14 still counts.
        assert_eq!(days(15), 1);
        assert_eq!(days(13), 3);
        assert_eq!(days(17), 0);
    }

    #[test]
    fn streak_freezes_keep_a_streak_over_a_missed_day() {
        let mut h = History::in_memory();
        let mut play = |day: i64| {
            let mut r = record("daily", 100.0);
            r.time = day as u64 * 86_400 + 60;
            h.add(r).unwrap();
        };
        // 7 days give a freeze, which day 8 uses.
        for day in 1..=7 {
            play(day);
        }
        play(9);
        let streak = |h: &History, today| daily_streak(h, today);
        assert_eq!(
            streak(&h, 9),
            Streak {
                days: 8,
                freezes: 0
            }
        );
        // No freeze is left for day 10.
        assert_eq!(streak(&h, 11).days, 0);
        // 14 days give 2 freezes; more are not kept.
        let mut h = History::in_memory();
        for day in 1..=21 {
            let mut r = record("daily", 100.0);
            r.time = day * 86_400 + 60;
            h.add(r).unwrap();
        }
        assert_eq!(streak(&h, 21).freezes, MAX_FREEZES);
        assert_eq!(
            streak(&h, 24),
            Streak {
                days: 21,
                freezes: 0
            }
        );
    }

    #[test]
    fn best_is_per_label() {
        let mut h = History::in_memory();
        h.add(record("time 30", 50.0)).unwrap();
        h.add(record("time 30", 70.0)).unwrap();
        h.add(record("time 30 · numbers", 90.0)).unwrap();
        assert_eq!(h.best("time 30"), Some(70.0));
        assert_eq!(h.best("words 10"), None);
        assert_eq!(
            h.bests(),
            vec![("time 30", 70.0), ("time 30 · numbers", 90.0)]
        );
    }

    #[test]
    fn records_are_saved_and_loaded() {
        let dir = std::env::temp_dir().join(format!("dakken-test-{}", std::process::id()));
        let path = dir.join("history.tsv");
        let _ = fs::remove_dir_all(&dir);
        let mut h = History::load(path.clone());
        assert!(h.records().is_empty());
        h.add(record("untimed · punctuation", 61.5)).unwrap();
        fs::write(&path, fs::read_to_string(&path).unwrap() + "not a record\n").unwrap();
        let loaded = History::load(path);
        assert_eq!(loaded.records(), &[record("untimed · punctuation", 61.5)]);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn time_is_formatted_as_utc() {
        assert_eq!(format_time(0), "1970-01-01 00:00");
        assert_eq!(format_time(1_709_164_800 + 3_723), "2024-02-29 01:02");
    }

    #[test]
    fn day_heatmap_shades_faster_days_darker() {
        let mut h = History::in_memory();
        // Day 20000 (2024-10-04) is a Friday.
        let today = 20_000;
        assert!(day_heatmap(&h, today, false).contains("No tests"));
        for (day, wpm) in [(20_000, 80.0), (19_999, 40.0), (19_998, 60.0)] {
            let mut r = record("time 30", wpm);
            r.time = day * 86_400 + 3600;
            h.add(r).unwrap();
        }
        let out = day_heatmap(&h, today, false);
        let row = |name: &str| {
            out.lines()
                .find(|l| l.starts_with(&format!("  {name}")))
                .unwrap()
                .to_string()
        };
        // The last column is this week: Wed 60 wpm, Thu 40, Fri 80.
        assert!(row("Wed").ends_with("▓▓"), "{out}");
        assert!(row("Fri").ends_with("██"), "{out}");
        let thu = out.lines().nth(5).unwrap();
        assert!(thu.ends_with("░░"), "{out}");
        assert!(row("Mon").trim() == "Mon", "no tests on Monday:\n{out}");
        assert!(out.contains("40 to 80 wpm"), "{out}");
    }

    #[test]
    fn bars_scale_to_the_top_value() {
        assert_eq!(bar(10.0, 10.0, 4), "████");
        assert_eq!(bar(5.0, 10.0, 4), "██");
        assert_eq!(bar(1.0, 8.0, 1), "▏");
        assert_eq!(bar(0.0, 0.0, 4), "");
    }

    #[test]
    fn stats_average_each_label_and_day() {
        let mut h = History::in_memory();
        assert_eq!(stats_report(&h), "No tests yet.\n");
        h.add(record("time 15", 40.0)).unwrap();
        h.add(record("time 15", 60.0)).unwrap();
        let mut next_day = record("words 10", 80.0);
        next_day.time = 86_400;
        h.add(next_day).unwrap();
        assert_eq!(
            daily_wpm(&h),
            vec![
                ("1970-01-01".to_string(), 50.0),
                ("1970-01-02".to_string(), 80.0)
            ]
        );
        let out = stats_report(&h);
        assert!(
            out.contains("      2       50     100%        60  time 15"),
            "{out}"
        );
        assert!(
            out.contains("1970-01-02  ████████████████████████████████████████ 80"),
            "{out}"
        );
    }

    #[test]
    fn export_as_csv_and_json() {
        let mut h = History::in_memory();
        assert_eq!(to_json(&h), "[]\n");
        h.add(record("custom · \"my\" notes.txt", 50.0)).unwrap();
        let csv = to_csv(&h);
        assert!(csv.starts_with("time_utc,unix_time,mode,"), "{csv}");
        assert!(
            csv.contains("1970-01-01 00:00,0,\"custom · \"\"my\"\" notes.txt\",50.00,50.00,100.00,80.00,30.00"),
            "{csv}"
        );
        let json = to_json(&h);
        assert!(
            json.contains("\"mode\": \"custom · \\\"my\\\" notes.txt\""),
            "{json}"
        );
        assert!(json.contains("\"wpm\": 50.00"), "{json}");
        assert_eq!(json_string("a\\b\u{1}"), "\"a\\\\b\\u0001\"");
    }

    #[test]
    fn report_lists_bests_and_recent_tests() {
        let mut h = History::in_memory();
        assert_eq!(report(&h), "No tests yet.\n");
        h.add(record("time 15", 42.0)).unwrap();
        let out = report(&h);
        assert!(out.contains("42 wpm  time 15"), "{out}");
        assert!(out.contains("1970-01-01 00:00"), "{out}");
    }
}
