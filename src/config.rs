//! Saved settings in a small TOML file: `key = value` lines.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::app::{Options, mode_label, parse_mode};
use crate::engine::{Mode, words::WordList};
use crate::ui::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub mode: Mode,
    pub options: Options,
    pub theme: Theme,
    /// Show the live WPM and accuracy during the test.
    pub live: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mode: Mode::Untimed,
            options: Options::default(),
            theme: Theme::default(),
            live: false,
        }
    }
}

impl Config {
    /// Read the settings. Unknown keys and bad values keep their defaults.
    pub fn parse(text: &str) -> Self {
        let mut config = Self::default();
        for line in text.lines() {
            let line = line.split('#').next().unwrap_or_default();
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim().trim_matches('"');
            match key.trim() {
                "mode" => config.mode = parse_mode(value).unwrap_or(config.mode),
                "theme" => config.theme = Theme::find(value).unwrap_or(config.theme),
                "live" => config.live = value == "true",
                "punctuation" => config.options.punctuation = value == "true",
                "numbers" => config.options.numbers = value == "true",
                "words" => {
                    config.options.list = WordList::parse(value).unwrap_or(config.options.list)
                }
                _ => {}
            }
        }
        config
    }

    pub fn to_text(self) -> String {
        format!(
            "# dakken settings. The menu saves this file.\n\
             mode = \"{}\"\n\
             theme = \"{}\"\n\
             punctuation = {}\n\
             numbers = {}\n\
             words = \"{}\"\n\
             live = {}\n",
            mode_label(self.mode),
            self.theme.name,
            self.options.punctuation,
            self.options.numbers,
            self.options.list.name(),
            self.live
        )
    }

    /// A missing or unreadable file gives the default settings.
    pub fn load(path: &Path) -> Self {
        Self::parse(&fs::read_to_string(path).unwrap_or_default())
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(path, self.to_text())
    }
}

pub fn default_path() -> Option<PathBuf> {
    Some(crate::history::data_dir()?.join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn text_round_trips() {
        let config = Config {
            mode: Mode::Time(Duration::from_secs(60)),
            options: Options {
                punctuation: true,
                numbers: false,
                list: WordList::Code,
            },
            theme: Theme::find("nord").unwrap(),
            live: true,
        };
        assert_eq!(Config::parse(&config.to_text()), config);
    }

    #[test]
    fn bad_lines_keep_the_defaults() {
        let config =
            Config::parse("mode = \"time x\"\ntheme = nope\nnot a setting\nnumbers = true # yes\n");
        assert_eq!(config.mode, Mode::Untimed);
        assert_eq!(config.theme, Theme::default());
        assert!(config.options.numbers);
    }
}
