//! The score card: a short text to paste in a chat, and the escape code that
//! puts text on the clipboard.

use crate::history::format_time;

/// Squares on each row of the card, one for each word.
const ROW: usize = 10;
/// Squares at most: a long test shows its first 100 words.
const MAX_SQUARES: usize = 100;

/// The card for the daily challenge of `day` (days since 1970-01-01 UTC): the
/// score, the streak, then a blue square for each correct word and a black
/// square for each wrong word.
pub fn score_card(day: i64, wpm: f64, accuracy: f64, words: &[bool], streak: usize) -> String {
    let date = &format_time(day.max(0) as u64 * 86_400)[..10];
    let s = if streak == 1 { "" } else { "s" };
    format!("dakken daily {date}\n{wpm:.0} wpm · {accuracy:.0}% acc · {streak} day{s} in a row\n")
        + &squares(words)
}

/// The card for any other test, with its label (such as "time 30") and
/// the `day` it was typed.
pub fn test_card(label: &str, day: i64, wpm: f64, accuracy: f64, words: &[bool]) -> String {
    let date = &format_time(day.max(0) as u64 * 86_400)[..10];
    format!("dakken {label} {date}\n{wpm:.0} wpm · {accuracy:.0}% acc\n") + &squares(words)
}

/// A blue square for each correct word and a black square for each wrong
/// word, 10 on each row.
fn squares(words: &[bool]) -> String {
    let mut out = String::new();
    for row in words[..words.len().min(MAX_SQUARES)].chunks(ROW) {
        for &correct in row {
            out += if correct { "🟦" } else { "⬛" };
        }
        out += "\n";
    }
    out
}

/// The OSC 52 escape code that asks the terminal to put `text` on the
/// clipboard. WezTerm, Windows Terminal, kitty and iTerm2 support it.
pub fn osc52(text: &str) -> String {
    format!("\x1b]52;c;{}\x07", base64(text.as_bytes()))
}

fn base64(bytes: &[u8]) -> String {
    const CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = u32::from(b[0]) << 16 | u32::from(b[1]) << 8 | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(CHARS[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_shows_score_streak_and_words() {
        let mut words = vec![true; 12];
        words[3] = false;
        let card = score_card(20_381, 92.4, 98.0, &words, 5);
        assert_eq!(
            card,
            "dakken daily 2025-10-20\n92 wpm · 98% acc · 5 days in a row\n\
             🟦🟦🟦⬛🟦🟦🟦🟦🟦🟦\n🟦🟦\n"
        );
        assert!(score_card(0, 1.0, 1.0, &[], 1).contains("1 day in a row"));
    }

    #[test]
    fn other_tests_get_a_card_with_their_label() {
        let card = test_card("time 30 · punctuation", 20_381, 71.6, 95.2, &[true; 150]);
        assert!(
            card.starts_with("dakken time 30 · punctuation 2025-10-20\n72 wpm · 95% acc\n"),
            "{card}"
        );
        assert_eq!(card.matches('🟦').count(), 100, "at most 100 squares");
    }

    #[test]
    fn base64_pads_the_end() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(osc52("hi"), "\x1b]52;c;aGk=\x07");
    }
}
