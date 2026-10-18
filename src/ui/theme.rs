//! Color themes. The default, "terminal", uses the terminal's own palette, so
//! the test follows the shell theme.

use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub name: &'static str,
    /// `None` keeps the terminal background.
    pub bg: Option<Color>,
    pub main: Color,
    pub sub: Color,
    pub text: Color,
    pub error: Color,
    pub error_extra: Color,
}

const fn rgb(hex: u32) -> Color {
    Color::Rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

/// The themes in menu order. The first is the default.
pub const THEMES: [Theme; 4] = [
    Theme {
        name: "terminal",
        bg: None,
        main: Color::Cyan,
        sub: Color::DarkGray,
        text: Color::Reset,
        error: Color::LightRed,
        error_extra: Color::Red,
    },
    // Monkeytype's default theme.
    Theme {
        name: "serika-dark",
        bg: Some(rgb(0x323437)),
        main: rgb(0xe2b714),
        sub: rgb(0x646669),
        text: rgb(0xd1d0c5),
        error: rgb(0xca4754),
        error_extra: rgb(0x7e2a33),
    },
    Theme {
        name: "nord",
        bg: Some(rgb(0x2e3440)),
        main: rgb(0x88c0d0),
        sub: rgb(0x616e88),
        text: rgb(0xd8dee9),
        error: rgb(0xbf616a),
        error_extra: rgb(0x8c4a52),
    },
    Theme {
        name: "dracula",
        bg: Some(rgb(0x282a36)),
        main: rgb(0xbd93f9),
        sub: rgb(0x6272a4),
        text: rgb(0xf8f8f2),
        error: rgb(0xff5555),
        error_extra: rgb(0xa33d3d),
    },
];

impl Default for Theme {
    fn default() -> Self {
        THEMES[0]
    }
}

impl Theme {
    pub fn find(name: &str) -> Option<Self> {
        THEMES.iter().copied().find(|t| t.name == name)
    }

    /// The theme after this one in `THEMES`, for the menu.
    pub fn next(self) -> Self {
        let i = THEMES.iter().position(|t| *t == self).unwrap_or(0);
        THEMES[(i + 1) % THEMES.len()]
    }
}

pub fn names() -> Vec<&'static str> {
    THEMES.iter().map(|t| t.name).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_theme_uses_the_terminal_palette() {
        let t = Theme::default();
        assert_eq!((t.name, t.bg, t.text), ("terminal", None, Color::Reset));
    }

    #[test]
    fn next_cycles_through_all_themes() {
        let mut t = Theme::default();
        for expected in THEMES.iter().skip(1).chain(&THEMES[..1]) {
            t = t.next();
            assert_eq!(t, *expected);
        }
        assert_eq!(Theme::find("nord").map(|t| t.name), Some("nord"));
        assert_eq!(Theme::find("nope"), None);
    }
}
