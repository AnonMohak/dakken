//! Words typed wrong in past tests, for the "mistakes" word list. One word on
//! each line, the newest last.

use std::fs;
use std::io;
use std::path::PathBuf;

/// The most words the list keeps. Older words are dropped first.
pub const MAX_WORDS: usize = 100;

pub struct Mistakes {
    /// `None` keeps the list in memory only.
    path: Option<PathBuf>,
    words: Vec<String>,
}

impl Mistakes {
    pub fn in_memory() -> Self {
        Self {
            path: None,
            words: Vec::new(),
        }
    }

    /// Read the list. A missing file is an empty list.
    pub fn load(path: PathBuf) -> Self {
        let words = fs::read_to_string(&path)
            .unwrap_or_default()
            .lines()
            .map(str::trim)
            .filter(|w| !w.is_empty())
            .map(String::from)
            .collect();
        Self {
            path: Some(path),
            words,
        }
    }

    pub fn words(&self) -> &[String] {
        &self.words
    }

    /// Add the `wrong` words as the newest, and remove the `fixed` words.
    /// Write the file if there is one and the list changed.
    pub fn update(&mut self, wrong: &[String], fixed: &[String]) -> io::Result<()> {
        let before = self.words.clone();
        self.words
            .retain(|w| !wrong.contains(w) && !fixed.contains(w));
        for w in wrong {
            if !self.words.contains(w) && !fixed.contains(w) {
                self.words.push(w.clone());
            }
        }
        let extra = self.words.len().saturating_sub(MAX_WORDS);
        self.words.drain(..extra);
        match &self.path {
            Some(path) if self.words != before => {
                if let Some(dir) = path.parent() {
                    fs::create_dir_all(dir)?;
                }
                let mut text = self.words.join("\n");
                text.push('\n');
                fs::write(path, text)
            }
            _ => Ok(()),
        }
    }
}

pub fn default_path() -> Option<PathBuf> {
    Some(crate::history::data_dir()?.join("mistakes.txt"))
}

/// The word to practise for a target word: punctuation and capitals from
/// punctuation mode are taken off. Numbers are not practised. Code words
/// (`keep_symbols`) stay as they are.
pub fn practice_word(target: &str, keep_symbols: bool) -> Option<String> {
    if keep_symbols {
        return Some(target.to_string());
    }
    let word = target
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase();
    let practise = !word.is_empty() && !word.chars().any(|c| c.is_ascii_digit());
    practise.then_some(word)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| w.to_string()).collect()
    }

    #[test]
    fn update_adds_newest_last_and_removes_fixed_words() {
        let mut m = Mistakes::in_memory();
        m.update(&strings(&["the", "of"]), &[]).unwrap();
        m.update(&strings(&["and", "the"]), &strings(&["of"]))
            .unwrap();
        assert_eq!(m.words(), strings(&["and", "the"]));
    }

    #[test]
    fn update_keeps_only_the_newest_words() {
        let mut m = Mistakes::in_memory();
        let many: Vec<String> = (0..MAX_WORDS + 5).map(|i| format!("w{i}")).collect();
        m.update(&many, &[]).unwrap();
        assert_eq!(m.words().len(), MAX_WORDS);
        assert_eq!(m.words()[0], "w5");
    }

    #[test]
    fn list_is_saved_and_loaded() {
        let dir = std::env::temp_dir().join(format!("dakken-mistakes-{}", std::process::id()));
        let path = dir.join("mistakes.txt");
        let _ = fs::remove_dir_all(&dir);
        let mut m = Mistakes::load(path.clone());
        m.update(&strings(&["world"]), &[]).unwrap();
        assert_eq!(Mistakes::load(path).words(), strings(&["world"]));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn practice_words_drop_punctuation_and_numbers() {
        assert_eq!(practice_word("\"Hello,\"", false), Some("hello".into()));
        assert_eq!(practice_word("(world)", false), Some("world".into()));
        assert_eq!(practice_word("world's", false), Some("world's".into()));
        assert_eq!(practice_word("1234", false), None);
        assert_eq!(practice_word("foo()", true), Some("foo()".into()));
    }
}
