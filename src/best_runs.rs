//! The pace of each personal-best test, so the pace caret can race it key by
//! key. One test on each line: the label, a tab, then the time in
//! milliseconds of each correct letter or space, separated by spaces.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::PathBuf;

use crate::engine::stats::Press;

pub struct BestRuns {
    /// `None` keeps the runs in memory only.
    path: Option<PathBuf>,
    runs: BTreeMap<String, Vec<u32>>,
}

impl BestRuns {
    pub fn in_memory() -> Self {
        Self {
            path: None,
            runs: BTreeMap::new(),
        }
    }

    /// Read the runs. A missing file gives no runs, and bad lines are skipped.
    pub fn load(path: PathBuf) -> Self {
        let runs = fs::read_to_string(&path)
            .unwrap_or_default()
            .lines()
            .filter_map(parse_line)
            .collect();
        Self {
            path: Some(path),
            runs,
        }
    }

    /// The times of the best test for this label, if there is one.
    pub fn get(&self, label: &str) -> Option<&[u32]> {
        self.runs.get(label).map(Vec::as_slice)
    }

    /// Keep the presses of a new personal best, and write the file if there
    /// is one.
    pub fn set(&mut self, label: &str, presses: &[Press]) -> io::Result<()> {
        let times = presses
            .iter()
            .filter(|p| p.correct)
            .map(|p| p.at.as_millis() as u32)
            .collect();
        self.runs.insert(label.to_string(), times);
        let Some(path) = &self.path else {
            return Ok(());
        };
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let text: String = self
            .runs
            .iter()
            .map(|(label, times)| {
                let times: Vec<String> = times.iter().map(u32::to_string).collect();
                format!("{label}\t{}\n", times.join(" "))
            })
            .collect();
        fs::write(path, text)
    }
}

fn parse_line(line: &str) -> Option<(String, Vec<u32>)> {
    let (label, times) = line.split_once('\t')?;
    let times = times
        .split_whitespace()
        .map(|t| t.parse().ok())
        .collect::<Option<Vec<u32>>>()?;
    Some((label.to_string(), times))
}

pub fn default_path() -> Option<PathBuf> {
    Some(crate::history::data_dir()?.join("best-runs.tsv"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn press(ms: u64, correct: bool) -> Press {
        Press {
            at: Duration::from_millis(ms),
            correct,
            key: Some('a'),
            word: 0,
        }
    }

    #[test]
    fn best_runs_keep_the_correct_presses_and_load_back() {
        let path = std::env::temp_dir().join(format!("dakken-runs-{}.tsv", std::process::id()));
        let mut runs = BestRuns::load(path.clone());
        assert_eq!(runs.get("time 30"), None);
        runs.set(
            "time 30",
            &[press(0, true), press(150, false), press(300, true)],
        )
        .unwrap();
        assert_eq!(runs.get("time 30"), Some(&[0, 300][..]));
        let loaded = BestRuns::load(path.clone());
        assert_eq!(loaded.get("time 30"), Some(&[0, 300][..]));
        let _ = fs::remove_file(path);
    }
}
