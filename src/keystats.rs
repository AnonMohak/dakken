//! Speed and errors for each key, from all saved tests. One key on each line:
//! the key, presses, errors and the total time to type it in milliseconds.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::PathBuf;

use ratatui::crossterm::style::Stylize;

use crate::engine::stats::Press;

/// Fewer presses than this are not enough to judge a key.
pub const MIN_PRESSES: u64 = 5;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KeyStat {
    pub presses: u64,
    pub errors: u64,
    /// Time from the press before, added up. The first press of a test has
    /// no time.
    pub total_ms: u64,
    pub timed: u64,
}

impl KeyStat {
    pub fn error_rate(&self) -> f64 {
        if self.presses == 0 {
            return 0.0;
        }
        self.errors as f64 / self.presses as f64
    }

    pub fn avg_ms(&self) -> Option<f64> {
        (self.timed > 0).then(|| self.total_ms as f64 / self.timed as f64)
    }
}

pub struct KeyStats {
    /// `None` keeps the stats in memory only.
    path: Option<PathBuf>,
    keys: BTreeMap<char, KeyStat>,
}

impl KeyStats {
    pub fn in_memory() -> Self {
        Self {
            path: None,
            keys: BTreeMap::new(),
        }
    }

    /// Read the stats. A missing file gives no stats, and bad lines are skipped.
    pub fn load(path: PathBuf) -> Self {
        let keys = fs::read_to_string(&path)
            .unwrap_or_default()
            .lines()
            .filter_map(parse_line)
            .collect();
        Self {
            path: Some(path),
            keys,
        }
    }

    pub fn get(&self, key: char) -> KeyStat {
        self.keys.get(&key).copied().unwrap_or_default()
    }

    /// Add the presses of one test, and write the file if there is one.
    pub fn add_test(&mut self, presses: &[Press]) -> io::Result<()> {
        let mut before = None;
        for p in presses {
            if let Some(key) = p.key {
                let stat = self.keys.entry(key).or_default();
                stat.presses += 1;
                stat.errors += u64::from(!p.correct);
                if let Some(b) = before {
                    stat.total_ms += (p.at.saturating_sub(b)).as_millis() as u64;
                    stat.timed += 1;
                }
            }
            before = Some(p.at);
        }
        let Some(path) = &self.path else {
            return Ok(());
        };
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let text: String = self
            .keys
            .iter()
            .map(|(&k, s)| {
                format!(
                    "{}\t{}\t{}\t{}\t{}\n",
                    key_name(k),
                    s.presses,
                    s.errors,
                    s.total_ms,
                    s.timed
                )
            })
            .collect();
        fs::write(path, text)
    }

    /// The `n` weakest letters a to z with enough presses: the most errors,
    /// and then the slowest.
    pub fn weakest_letters(&self, n: usize) -> Vec<char> {
        let mut letters: Vec<(char, KeyStat)> = self
            .keys
            .iter()
            .filter(|(k, s)| k.is_ascii_lowercase() && s.presses >= MIN_PRESSES)
            .map(|(&k, &s)| (k, s))
            .collect();
        letters.sort_by(|a, b| {
            let score = |s: &KeyStat| (s.error_rate(), s.avg_ms().unwrap_or(0.0));
            score(&b.1)
                .partial_cmp(&score(&a.1))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        letters.into_iter().take(n).map(|(k, _)| k).collect()
    }
}

fn key_name(key: char) -> String {
    match key {
        ' ' => "space".to_string(),
        '\t' => "tab".to_string(),
        k => k.to_string(),
    }
}

fn parse_line(line: &str) -> Option<(char, KeyStat)> {
    let mut f = line.split('\t');
    let name = f.next()?;
    let key = match name {
        "space" => ' ',
        "tab" => '\t',
        _ => {
            let mut chars = name.chars();
            let k = chars.next()?;
            if chars.next().is_some() {
                return None;
            }
            k
        }
    };
    let mut num = || f.next()?.parse::<u64>().ok();
    let stat = KeyStat {
        presses: num()?,
        errors: num()?,
        total_ms: num()?,
        timed: num()?,
    };
    Some((key, stat))
}

pub fn default_path() -> Option<PathBuf> {
    Some(crate::history::data_dir()?.join("keys.tsv"))
}

const ROWS: [&str; 3] = ["qwertyuiop", "asdfghjkl", "zxcvbnm"];

/// The text for `dakken --heatmap`: a keyboard colored by error rate, then
/// the slowest keys and the keys with the most errors.
pub fn heatmap(stats: &KeyStats, color: bool) -> String {
    let mut out = String::from("errors on each key\n\n");
    for (indent, row) in ROWS.iter().enumerate() {
        out += &" ".repeat(2 + indent * 2);
        for key in row.chars() {
            let s = stats.get(key);
            let cell = format!(" {key} ");
            if !color {
                out += &cell;
            } else if s.presses < MIN_PRESSES {
                out += &cell.dark_grey().to_string();
            } else if s.error_rate() < 0.02 {
                out += &cell.black().on_green().to_string();
            } else if s.error_rate() < 0.05 {
                out += &cell.black().on_yellow().to_string();
            } else {
                out += &cell.black().on_red().to_string();
            }
            out += " ";
        }
        out += "\n";
    }
    out += "\n  green: under 2%   yellow: 2% to 5%   red: 5% or more   gray: not enough data\n";

    let mut known: Vec<(char, KeyStat)> = stats
        .keys
        .iter()
        .filter(|(_, s)| s.presses >= MIN_PRESSES)
        .map(|(&k, &s)| (k, s))
        .collect();
    if known.is_empty() {
        out += "\nNo key has enough presses yet. Do some tests first.\n";
        return out;
    }
    known.sort_by(|a, b| {
        b.1.avg_ms()
            .partial_cmp(&a.1.avg_ms())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out += "\nslowest keys\n";
    for (k, s) in known.iter().take(5) {
        if let Some(ms) = s.avg_ms() {
            out += &format!("  {:<6} {ms:>4.0} ms\n", key_name(*k));
        }
    }
    known.sort_by(|a, b| {
        b.1.error_rate()
            .partial_cmp(&a.1.error_rate())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out += "\nmost errors\n";
    for (k, s) in known.iter().take(5) {
        out += &format!(
            "  {:<6} {:>4.1}%  ({} of {})\n",
            key_name(*k),
            s.error_rate() * 100.0,
            s.errors,
            s.presses
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn press(ms: u64, key: char, correct: bool) -> Press {
        Press {
            at: Duration::from_millis(ms),
            correct,
            key: Some(key),
        }
    }

    #[test]
    fn presses_add_up_for_each_key() {
        let mut s = KeyStats::in_memory();
        s.add_test(&[
            press(0, 'a', true),
            press(200, 'b', false),
            press(300, 'a', true),
        ])
        .unwrap();
        assert_eq!(
            s.get('a'),
            KeyStat {
                presses: 2,
                errors: 0,
                total_ms: 100,
                timed: 1
            }
        );
        assert_eq!(s.get('b').errors, 1);
        assert_eq!(s.get('b').avg_ms(), Some(200.0));
    }

    #[test]
    fn stats_are_saved_and_loaded() {
        let dir = std::env::temp_dir().join(format!("dakken-keys-{}", std::process::id()));
        let path = dir.join("keys.tsv");
        let _ = fs::remove_dir_all(&dir);
        let mut s = KeyStats::load(path.clone());
        s.add_test(&[press(0, ' ', true), press(150, 'x', false)])
            .unwrap();
        let loaded = KeyStats::load(path);
        assert_eq!(loaded.get(' ').presses, 1);
        assert_eq!(loaded.get('x'), s.get('x'));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn weakest_letters_have_the_most_errors() {
        let mut s = KeyStats::in_memory();
        let mut presses = Vec::new();
        for i in 0..10 {
            presses.push(press(i * 100, 'a', true));
            presses.push(press(i * 100 + 10, 'b', i % 2 == 0));
            presses.push(press(i * 100 + 20, 'c', i != 0));
        }
        presses.push(press(5000, 'z', false));
        s.add_test(&presses).unwrap();
        assert_eq!(s.weakest_letters(2), ['b', 'c'], "z has too few presses");
    }

    #[test]
    fn heatmap_shows_the_keyboard_and_lists() {
        let mut s = KeyStats::in_memory();
        assert!(heatmap(&s, false).contains("enough presses yet"));
        let presses: Vec<Press> = (0..6).map(|i| press(i * 100, 'q', i > 0)).collect();
        s.add_test(&presses).unwrap();
        let out = heatmap(&s, false);
        assert!(out.contains(" q   w   e "), "{out}");
        assert!(
            out.contains("slowest keys") && out.contains("q       100 ms"),
            "{out}"
        );
        assert!(out.contains("16.7%  (1 of 6)"), "{out}");
    }
}
