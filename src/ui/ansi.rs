//! The colored reports of the command line (`--heatmap`, `--skills`, ...)
//! on the app screens: their ANSI color codes become ratatui styles, and
//! back to ANSI codes for the printed reports in the theme colors.

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// One line of text with ANSI color codes as styled spans. Text with no
/// code has the `base` style. Codes other than colors and bold are left out.
pub fn line(text: &str, base: Style) -> Line<'static> {
    let mut spans = Vec::new();
    let mut style = base;
    let mut rest = text;
    while let Some(start) = rest.find("\x1b[") {
        if start > 0 {
            spans.push(Span::styled(rest[..start].to_string(), style));
        }
        let after = &rest[start + 2..];
        let Some(end) = after.find(|c: char| c.is_ascii_alphabetic()) else {
            rest = "";
            break;
        };
        if after.as_bytes()[end] == b'm' {
            style = apply(style, base, &after[..end]);
        }
        rest = &after[end + 1..];
    }
    if !rest.is_empty() {
        spans.push(Span::styled(rest.to_string(), style));
    }
    Line::from(spans)
}

/// A styled line as text with ANSI codes, the opposite of `line`.
pub fn to_ansi(line: &Line) -> String {
    let mut out = String::new();
    for span in &line.spans {
        let mut codes = Vec::new();
        if span.style.add_modifier.contains(Modifier::BOLD) {
            codes.push("1".to_string());
        }
        if let Some(c) = span.style.fg.and_then(|c| sgr(c, 30)) {
            codes.push(c);
        }
        if let Some(c) = span.style.bg.and_then(|c| sgr(c, 40)) {
            codes.push(c);
        }
        if codes.is_empty() {
            out += &span.content;
        } else {
            out += &format!("\x1b[{}m{}\x1b[0m", codes.join(";"), span.content);
        }
    }
    out
}

/// The SGR parameters of a color: `base` is 30 for the text and 40 for the
/// background. `None` for the terminal's own color.
fn sgr(color: Color, base: u8) -> Option<String> {
    let named = |n: u8| Some((base + n).to_string());
    let bright = |n: u8| Some((base + 60 + n).to_string());
    match color {
        Color::Reset => None,
        Color::Black => named(0),
        Color::Red => named(1),
        Color::Green => named(2),
        Color::Yellow => named(3),
        Color::Blue => named(4),
        Color::Magenta => named(5),
        Color::Cyan => named(6),
        Color::Gray => named(7),
        Color::DarkGray => bright(0),
        Color::LightRed => bright(1),
        Color::LightGreen => bright(2),
        Color::LightYellow => bright(3),
        Color::LightBlue => bright(4),
        Color::LightMagenta => bright(5),
        Color::LightCyan => bright(6),
        Color::White => bright(7),
        Color::Indexed(n) => Some(format!("{};5;{n}", base + 8)),
        Color::Rgb(r, g, b) => Some(format!("{};2;{r};{g};{b}", base + 8)),
    }
}

/// The text with the ANSI codes taken out.
pub fn plain(text: &str) -> String {
    line(text, Style::new())
        .spans
        .iter()
        .map(|s| s.content.as_ref())
        .collect()
}

/// `style` after the SGR parameters `params`, such as "38;5;37" or "0".
fn apply(mut style: Style, base: Style, params: &str) -> Style {
    let mut codes = params.split(';').map(|p| p.parse::<u8>().unwrap_or(0));
    while let Some(code) = codes.next() {
        style = match code {
            0 => base,
            1 => style.add_modifier(Modifier::BOLD),
            30..=37 => style.fg(Color::Indexed(code - 30)),
            90..=97 => style.fg(Color::Indexed(code - 90 + 8)),
            40..=47 => style.bg(Color::Indexed(code - 40)),
            100..=107 => style.bg(Color::Indexed(code - 100 + 8)),
            39 => style.fg(base.fg.unwrap_or(Color::Reset)),
            49 => style.bg(base.bg.unwrap_or(Color::Reset)),
            38 | 48 => {
                let color = match codes.next() {
                    Some(5) => codes.next().map(Color::Indexed),
                    Some(2) => match (codes.next(), codes.next(), codes.next()) {
                        (Some(r), Some(g), Some(b)) => Some(Color::Rgb(r, g, b)),
                        _ => None,
                    },
                    _ => None,
                };
                match (code, color) {
                    (38, Some(c)) => style.fg(c),
                    (_, Some(c)) => style.bg(c),
                    _ => style,
                }
            }
            _ => style,
        };
    }
    style
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_ansi_reads_back_the_same_line() {
        let text = "a \x1b[1;38;2;1;2;3mbold\x1b[0m \x1b[30;48;5;43mkey\x1b[0m \x1b[96mcyan\x1b[0m";
        let parsed = line(text, Style::new());
        assert_eq!(line(&to_ansi(&parsed), Style::new()), parsed);
    }

    #[test]
    fn color_codes_become_styles() {
        let base = Style::new().fg(Color::White);
        let l = line("a \x1b[30;48;5;37m q 3 \x1b[0m b", base);
        assert_eq!(l.spans.len(), 3);
        assert_eq!(l.spans[0].style, base);
        assert_eq!(
            l.spans[1].style,
            base.fg(Color::Indexed(0)).bg(Color::Indexed(37))
        );
        assert_eq!(l.spans[1].content, " q 3 ");
        assert_eq!(l.spans[2].style, base);
        let rgb = line("\x1b[38;2;1;2;3mx", base);
        assert_eq!(rgb.spans[0].style.fg, Some(Color::Rgb(1, 2, 3)));
        assert_eq!(plain("\x1b[1mbold\x1b[0m text"), "bold text");
    }
}
