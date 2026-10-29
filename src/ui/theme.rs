//! Color themes. The default is sumi. "terminal" uses the terminal's own
//! palette, so the test follows the shell theme.

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

/// The momo theme: night sakura. It is on in momo mode only, so it is not
/// in `THEMES`. Errors are coral, apart from the pink accent.
pub const MOMO: Theme = Theme {
    name: "momo",
    bg: Some(rgb(0x1e1420)),
    main: rgb(0xff8fb8),
    sub: rgb(0x6e5566),
    text: rgb(0xf3e4ec),
    error: rgb(0xff7a45),
    error_extra: rgb(0xa3462a),
};

/// The themes in menu order. The first is the default.
pub const THEMES: [Theme; 17] = [
    // The default: black ink on washi paper, with a red seal for errors.
    Theme {
        name: "sumi",
        bg: Some(rgb(0xf2ece0)),
        main: rgb(0x1c1c1c),
        sub: rgb(0x857c6f),
        text: rgb(0x3a3530),
        error: rgb(0xb7282e),
        error_extra: rgb(0x7a1a1f),
    },
    Theme {
        name: "tokyo-night",
        bg: Some(rgb(0x1a1b26)),
        main: rgb(0x7aa2f7),
        sub: rgb(0x565f89),
        text: rgb(0xc0caf5),
        error: rgb(0xf7768e),
        error_extra: rgb(0xa3485c),
    },
    Theme {
        name: "terminal",
        bg: None,
        main: Color::Cyan,
        sub: Color::DarkGray,
        text: Color::Reset,
        error: Color::LightRed,
        error_extra: Color::Red,
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
    // Dark gray with a gold accent.
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
        name: "catppuccin-mocha",
        bg: Some(rgb(0x1e1e2e)),
        main: rgb(0xcba6f7),
        sub: rgb(0x6c7086),
        text: rgb(0xcdd6f4),
        error: rgb(0xf38ba8),
        error_extra: rgb(0xa35d73),
    },
    Theme {
        name: "rose-pine",
        bg: Some(rgb(0x191724)),
        main: rgb(0xebbcba),
        sub: rgb(0x6e6a86),
        text: rgb(0xe0def4),
        error: rgb(0xeb6f92),
        error_extra: rgb(0xa14e66),
    },
    Theme {
        name: "one-dark",
        bg: Some(rgb(0x282c34)),
        main: rgb(0x61afef),
        sub: rgb(0x5c6370),
        text: rgb(0xabb2bf),
        error: rgb(0xe06c75),
        error_extra: rgb(0x9b4a50),
    },
    // White lines on blueprint blue; errors in yellow.
    Theme {
        name: "blueprint",
        bg: Some(rgb(0x0f3057)),
        main: rgb(0xffffff),
        sub: rgb(0x6f8fb4),
        text: rgb(0xdbe7f5),
        error: rgb(0xffcc4d),
        error_extra: rgb(0xa3832f),
    },
    // Tea green on dark moss.
    Theme {
        name: "matcha",
        bg: Some(rgb(0x1f241c)),
        main: rgb(0xa8c686),
        sub: rgb(0x5f6d53),
        text: rgb(0xe6ead8),
        error: rgb(0xe07a5f),
        error_extra: rgb(0x9b5442),
    },
    // The deep sea, with a glowing teal.
    Theme {
        name: "abyss",
        bg: Some(rgb(0x031926)),
        main: rgb(0x2ec4b6),
        sub: rgb(0x3d6f80),
        text: rgb(0xcfe8ef),
        error: rgb(0xff6b6b),
        error_extra: rgb(0xa34545),
    },
    // Neon pink on a purple night; errors in pale yellow.
    Theme {
        name: "synthwave",
        bg: Some(rgb(0x241734)),
        main: rgb(0xff71ce),
        sub: rgb(0x7a5c9c),
        text: rgb(0xe0d6ff),
        error: rgb(0xfffb96),
        error_extra: rgb(0xa3a060),
    },
    // Light themes.
    Theme {
        name: "catppuccin-latte",
        bg: Some(rgb(0xeff1f5)),
        main: rgb(0x8839ef),
        sub: rgb(0x7c7f93),
        text: rgb(0x4c4f69),
        error: rgb(0xd20f39),
        error_extra: rgb(0xa0102c),
    },
    Theme {
        name: "github-light",
        bg: Some(rgb(0xffffff)),
        main: rgb(0x0969da),
        sub: rgb(0x6e7781),
        text: rgb(0x1f2328),
        error: rgb(0xcf222e),
        error_extra: rgb(0x8a1820),
    },
    Theme {
        name: "rose-pine-dawn",
        bg: Some(rgb(0xfaf4ed)),
        main: rgb(0xd7827e),
        sub: rgb(0x9893a5),
        text: rgb(0x575279),
        error: rgb(0xb4637a),
        error_extra: rgb(0x7e4555),
    },
    Theme {
        name: "ayu-light",
        bg: Some(rgb(0xfcfcfc)),
        main: rgb(0xfa8d3e),
        sub: rgb(0x8a9199),
        text: rgb(0x5c6166),
        error: rgb(0xe65050),
        error_extra: rgb(0xa13838),
    },
];

/// The theme with the terminal's own palette.
const TERMINAL: Theme = THEMES[2];

impl Default for Theme {
    fn default() -> Self {
        THEMES[0]
    }
}

impl Theme {
    pub fn find(name: &str) -> Option<Self> {
        THEMES.iter().copied().find(|t| t.name == name)
    }

    /// The theme after this one in `THEMES`, and then `custom` if there is
    /// one, for the menu.
    pub fn next(self, custom: Option<Theme>) -> Self {
        let all: Vec<Theme> = THEMES.iter().copied().chain(custom).collect();
        let i = all.iter().position(|t| t.name == self.name).unwrap_or(0);
        all[(i + 1) % all.len()]
    }

    /// The `custom` theme: the terminal theme with the given colors on top.
    /// The keys are bg, main, sub, text, error and error_extra.
    pub fn custom(colors: &[(&str, Option<Color>)]) -> Self {
        let mut t = Self {
            name: "custom",
            ..TERMINAL
        };
        for &(key, color) in colors {
            match (key, color) {
                ("bg", c) => t.bg = c,
                ("main", Some(c)) => t.main = c,
                ("sub", Some(c)) => t.sub = c,
                ("text", Some(c)) => t.text = c,
                ("error", Some(c)) => t.error = c,
                ("error_extra", Some(c)) => t.error_extra = c,
                _ => {}
            }
        }
        t
    }
}

pub fn names() -> Vec<&'static str> {
    THEMES.iter().map(|t| t.name).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_theme_is_tokyo_night() {
        assert_eq!(Theme::default().name, "sumi");
        assert_eq!(TERMINAL.name, "terminal");
        let t = Theme::find("terminal").unwrap();
        assert_eq!((t.bg, t.text), (None, Color::Reset));
    }

    #[test]
    fn next_cycles_through_all_themes() {
        let mut t = Theme::default();
        for expected in THEMES.iter().skip(1).chain(&THEMES[..1]) {
            t = t.next(None);
            assert_eq!(t, *expected);
        }
        let custom = Theme::custom(&[("main", Some(Color::Magenta))]);
        assert_eq!(THEMES[THEMES.len() - 1].next(Some(custom)), custom);
        assert_eq!(custom.next(Some(custom)), THEMES[0]);
        assert_eq!(Theme::find("nord").map(|t| t.name), Some("nord"));
        assert_eq!(Theme::find("nope"), None);
    }
}
