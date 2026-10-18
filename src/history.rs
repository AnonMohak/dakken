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

/// `%APPDATA%\typer` on Windows, else `$XDG_DATA_HOME/typer` or
/// `~/.local/share/typer`.
pub fn data_dir() -> Option<PathBuf> {
    let var = |name| std::env::var_os(name).filter(|v| !v.is_empty());
    let base = if cfg!(windows) {
        PathBuf::from(var("APPDATA")?)
    } else if let Some(xdg) = var("XDG_DATA_HOME") {
        PathBuf::from(xdg)
    } else {
        PathBuf::from(var("HOME")?).join(".local").join("share")
    };
    Some(base.join("typer"))
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

/// The text for `typer --history`: personal bests, then the last 10 tests.
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
        let dir = std::env::temp_dir().join(format!("typer-test-{}", std::process::id()));
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
    fn report_lists_bests_and_recent_tests() {
        let mut h = History::in_memory();
        assert_eq!(report(&h), "No tests yet.\n");
        h.add(record("time 15", 42.0)).unwrap();
        let out = report(&h);
        assert!(out.contains("42 wpm  time 15"), "{out}");
        assert!(out.contains("1970-01-01 00:00"), "{out}");
    }
}
