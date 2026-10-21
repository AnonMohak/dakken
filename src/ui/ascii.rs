//! `symbols = "ascii"`: the app screens with ASCII only, for a terminal
//! whose font lacks the symbols. Hearts and moods become short ASCII words
//! where they are made; the other symbols change in the screen buffer just
//! before it is drawn. Letters, such as the "ü" of a German word, stay.

use ratatui::buffer::Buffer;

/// The ASCII for a mood symbol: love <3, peace ~, darkness x, sorrow v,
/// change >, resolve ^, dreams *.
pub fn mood(symbol: char, ascii: bool) -> String {
    if !ascii {
        return symbol.to_string();
    }
    match symbol {
        '♥' => "<3",
        '≈' => "~",
        '†' => "x",
        '▼' => "v",
        '↑' => ">",
        '▲' => "^",
        _ => "*",
    }
    .to_string()
}

/// The ASCII for a symbol of the screens, if it has one.
fn ascii_for(symbol: &str) -> Option<&'static str> {
    let mut chars = symbol.chars();
    let (Some(c), None) = (chars.next(), chars.next()) else {
        return None;
    };
    Some(match c {
        '·' | '─' | '—' | '–' => "-",
        '━' => "=",
        '│' | '┃' => "|",
        '└' | '┘' | '┌' | '┐' | '├' | '┤' | '┬' | '┴' | '┼' => "+",
        '•' => "*",
        '↑' => "^",
        '↓' => "v",
        '♥' => "<",
        // The graph: a Braille cell with dots is a point, an empty one is
        // a space.
        '\u{2800}' => " ",
        '\u{2801}'..='\u{28ff}' => "*",
        _ => return None,
    })
}

/// Replace the symbols of the screen with ASCII.
pub fn replace_symbols(buffer: &mut Buffer) {
    for cell in &mut buffer.content {
        if let Some(ascii) = ascii_for(cell.symbol()) {
            cell.set_symbol(ascii);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbols_and_moods_become_ascii() {
        assert_eq!(ascii_for("·"), Some("-"));
        assert_eq!(ascii_for("━"), Some("="));
        assert_eq!(ascii_for("└"), Some("+"));
        assert_eq!(ascii_for("\u{28ff}"), Some("*"));
        assert_eq!(ascii_for("ü"), None);
        assert_eq!(ascii_for("a"), None);
        assert_eq!(mood('♥', true), "<3");
        assert_eq!(mood('¤', true), "*");
        assert_eq!(mood('♥', false), "♥");
    }
}
