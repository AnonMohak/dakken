//! Saved settings in a small TOML file: `key = value` lines.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::app::{Options, mode_label, parse_mode};
use crate::challenge::Challenge;
use crate::engine::{Mode, Strict, words::WordList};
use crate::layout::Layout;
use crate::ui::theme::Theme;
use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub mode: Mode,
    pub options: Options,
    pub theme: Theme,
    /// Show the live WPM and accuracy during the test.
    pub live: bool,
    /// Show a pace caret at the personal-best speed.
    pub pace: bool,
    pub strict: Strict,
    pub challenge: Challenge,
    pub caret: Caret,
    pub layout: Layout,
    /// The theme from the `color_*` keys, if the file has any.
    pub custom_theme: Option<Theme>,
    /// Words to type each week. 0 is no goal.
    pub weekly_goal: u32,
    /// Ask GitHub once a day for a newer version, for the menu.
    pub update_check: bool,
    /// Draw the app screens with ASCII only, for terminals whose font lacks
    /// the symbols.
    pub ascii: bool,
}

/// The shape of the text cursor.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Caret {
    #[default]
    Block,
    Bar,
    Underline,
}

impl Caret {
    pub const ALL: [Caret; 3] = [Caret::Block, Caret::Bar, Caret::Underline];

    pub fn name(self) -> &'static str {
        match self {
            Self::Block => "block",
            Self::Bar => "bar",
            Self::Underline => "underline",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.name() == name)
    }
}

/// The `color_*` keys, without the `color_` start.
const COLOR_KEYS: [&str; 6] = ["bg", "main", "sub", "text", "error", "error_extra"];

impl Default for Config {
    fn default() -> Self {
        Self {
            mode: Mode::Time(std::time::Duration::from_secs(30)),
            options: Options::default(),
            theme: Theme::default(),
            live: false,
            pace: false,
            strict: Strict::Off,
            challenge: Challenge::Off,
            caret: Caret::Block,
            layout: Layout::Qwerty,
            custom_theme: None,
            weekly_goal: 0,
            update_check: true,
            ascii: false,
        }
    }
}

/// The key and value of a `key = value` line. A value in quotes can hold a
/// `#`; elsewhere `#` starts a comment.
fn parse_line(line: &str) -> Option<(&str, &str)> {
    let line = line.trim();
    if line.starts_with('#') {
        return None;
    }
    let (key, rest) = line.split_once('=')?;
    let rest = rest.trim_start();
    let value = match rest.strip_prefix('"') {
        Some(quoted) => quoted.split('"').next()?,
        None => rest.split(['#', ' ', '\t']).next()?,
    };
    Some((key.trim(), value))
}

impl Config {
    /// Read the settings. Unknown keys and bad values keep their defaults.
    pub fn parse(text: &str) -> Self {
        let mut config = Self::default();
        let mut theme_name = "";
        let mut colors: Vec<(&str, Option<Color>)> = Vec::new();
        for (key, value) in text.lines().filter_map(parse_line) {
            if let Some(color_key) = key.strip_prefix("color_") {
                let name = COLOR_KEYS.iter().find(|&&k| k == color_key);
                let color = match value {
                    "none" => Some(None),
                    _ => value.parse::<Color>().ok().map(Some),
                };
                if let (Some(name), Some(color)) = (name, color) {
                    colors.push((name, color));
                }
                continue;
            }
            match key {
                "mode" => config.mode = parse_mode(value).unwrap_or(config.mode),
                "theme" => theme_name = value,
                "live" => config.live = value == "true",
                "pace" => config.pace = value == "true",
                "strict" => config.strict = Strict::parse(value).unwrap_or(config.strict),
                "challenge" => {
                    config.challenge = Challenge::parse(value).unwrap_or(config.challenge)
                }
                "caret" => config.caret = Caret::parse(value).unwrap_or(config.caret),
                "layout" => config.layout = Layout::parse(value).unwrap_or(config.layout),
                "punctuation" => config.options.punctuation = value == "true",
                "weekly_goal" => config.weekly_goal = value.parse().unwrap_or(config.weekly_goal),
                "update_check" => config.update_check = value != "false",
                "symbols" => config.ascii = value == "ascii",
                "numbers" => config.options.numbers = value == "true",
                "words" => {
                    config.options.list = WordList::parse(value).unwrap_or(config.options.list)
                }
                _ => {}
            }
        }
        if !colors.is_empty() {
            config.custom_theme = Some(Theme::custom(&colors));
        }
        config.theme = match (theme_name, config.custom_theme) {
            ("custom", Some(custom)) => custom,
            (name, _) => Theme::find(name).unwrap_or_default(),
        };
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
             live = {}\n\
             pace = {}\n\
             strict = \"{}\"\n\
             challenge = \"{}\"\n\
             caret = \"{}\"\n\
             layout = \"{}\"\n\
             weekly_goal = {}\n\
             update_check = {}\n\
             symbols = \"{}\"\n",
            mode_label(self.mode),
            self.theme.name,
            self.options.punctuation,
            self.options.numbers,
            self.options.list.name(),
            self.live,
            self.pace,
            self.strict.name(),
            self.challenge.name(),
            self.caret.name(),
            self.layout.name(),
            self.weekly_goal,
            self.update_check,
            if self.ascii { "ascii" } else { "unicode" }
        ) + &self.custom_colors_text()
    }

    /// The `color_*` lines of the custom theme, so a save keeps them.
    fn custom_colors_text(self) -> String {
        let Some(t) = self.custom_theme else {
            return String::new();
        };
        let bg = t.bg.map_or("none".to_string(), |c| c.to_string());
        let values = [
            bg,
            t.main.to_string(),
            t.sub.to_string(),
            t.text.to_string(),
            t.error.to_string(),
            t.error_extra.to_string(),
        ];
        COLOR_KEYS
            .iter()
            .zip(values)
            .map(|(key, value)| format!("color_{key} = \"{value}\"\n"))
            .collect()
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
            pace: true,
            strict: Strict::Master,
            challenge: Challenge::Survival,
            caret: Caret::Underline,
            layout: Layout::Colemak,
            custom_theme: None,
            weekly_goal: 3000,
            update_check: false,
            ascii: true,
        };
        assert_eq!(Config::parse(&config.to_text()), config);
    }

    #[test]
    fn color_keys_make_a_custom_theme() {
        let text = "theme = \"custom\"\n\
                    color_main = \"#ff79c6\"  # pink\n\
                    color_sub = darkgray\n\
                    color_bg = \"none\"\n\
                    color_nope = \"red\"\n\
                    color_text = \"not a color\"\n";
        let config = Config::parse(text);
        let custom = config.custom_theme.expect("color keys give a custom theme");
        assert_eq!(config.theme, custom);
        assert_eq!(custom.main, Color::Rgb(0xff, 0x79, 0xc6));
        assert_eq!(custom.sub, Color::DarkGray);
        assert_eq!(
            custom.text,
            Theme::find("terminal").unwrap().text,
            "a bad color is skipped"
        );
        // Saving keeps the colors.
        assert_eq!(Config::parse(&config.to_text()), config);
        // Without color keys, "custom" is not a theme.
        assert_eq!(Config::parse("theme = custom").theme, Theme::default());
    }

    #[test]
    fn bad_lines_keep_the_defaults() {
        let config =
            Config::parse("mode = \"time x\"\ntheme = nope\nnot a setting\nnumbers = true # yes\n");
        assert_eq!(config.mode, Mode::Time(Duration::from_secs(30)));
        assert_eq!(config.theme, Theme::default());
        assert!(config.options.numbers);
    }
}
