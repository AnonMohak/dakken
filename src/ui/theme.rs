//! Colors from the terminal's own palette, so the test follows the shell theme.

use ratatui::style::Color;

pub const MAIN: Color = Color::Cyan;
pub const SUB: Color = Color::DarkGray;
/// The terminal's default text color.
pub const TEXT: Color = Color::Reset;
pub const ERROR: Color = Color::LightRed;
pub const ERROR_EXTRA: Color = Color::Red;
