//! Race files: one test's words and key times, to race as a ghost with
//! `--race`. A small text file:
//!
//! ```text
//! dakken race 1
//! <label>\t<wpm>\t<accuracy>
//! <the words, separated by spaces>
//! <the time in milliseconds of each correct letter or space>
//! ```

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::history::{self, format_time};

const HEADER: &str = "dakken race 1";

#[derive(Debug, Clone, PartialEq)]
pub struct Race {
    /// The mode and options of the test, for example "time 30".
    pub label: String,
    pub wpm: f64,
    pub accuracy: f64,
    pub words: Vec<String>,
    /// Milliseconds of each correct letter or space, for the ghost caret.
    pub times: Vec<u32>,
}

impl Race {
    pub fn to_text(&self) -> String {
        let times: Vec<String> = self.times.iter().map(u32::to_string).collect();
        format!(
            "{HEADER}\n{}\t{:.2}\t{:.2}\n{}\n{}\n",
            self.label,
            self.wpm,
            self.accuracy,
            self.words.join(" "),
            times.join(" ")
        )
    }

    pub fn parse(text: &str) -> Option<Self> {
        let mut lines = text.lines();
        if lines.next()?.trim() != HEADER {
            return None;
        }
        let mut info = lines.next()?.split('\t');
        let label = info.next()?.to_string();
        let wpm = info.next()?.parse().ok()?;
        let accuracy = info.next()?.parse().ok()?;
        let words: Vec<String> = lines.next()?.split_whitespace().map(String::from).collect();
        let times = lines
            .next()
            .unwrap_or_default()
            .split_whitespace()
            .map(|t| t.parse().ok())
            .collect::<Option<Vec<u32>>>()?;
        (!words.is_empty()).then_some(Self {
            label,
            wpm,
            accuracy,
            words,
            times,
        })
    }

    pub fn load(path: &Path) -> io::Result<Self> {
        let text = fs::read_to_string(path)?;
        Self::parse(&text).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "this is not a dakken race file")
        })
    }

    /// Write the race to `dir`, with a name from the time and the WPM.
    pub fn save(&self, dir: &Path, unix_time: u64) -> io::Result<PathBuf> {
        fs::create_dir_all(dir)?;
        let time = format_time(unix_time).replace([' ', ':'], "-");
        let path = dir.join(format!("race-{time}-{:.0}wpm.txt", self.wpm));
        fs::write(&path, self.to_text())?;
        Ok(path)
    }
}

/// The folder for saved races.
pub fn default_dir() -> Option<PathBuf> {
    Some(history::data_dir()?.join("races"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn race() -> Race {
        Race {
            label: "words 10 · punctuation".to_string(),
            wpm: 72.4,
            accuracy: 98.0,
            words: vec!["Hello,".into(), "world.".into()],
            times: vec![0, 120, 250],
        }
    }

    #[test]
    fn text_round_trips() {
        let r = race();
        assert_eq!(Race::parse(&r.to_text()), Some(r));
        assert_eq!(Race::parse("not a race"), None);
        assert_eq!(Race::parse("dakken race 1\nx\t1\t2\n\n"), None);
    }

    #[test]
    fn save_names_the_file_by_time_and_speed() {
        let dir = std::env::temp_dir().join(format!("dakken-race-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let path = race().save(&dir, 0).unwrap();
        assert!(
            path.ends_with("race-1970-01-01-00-00-72wpm.txt"),
            "{path:?}"
        );
        assert_eq!(Race::load(&path).unwrap(), race());
        fs::remove_dir_all(&dir).unwrap();
    }
}
