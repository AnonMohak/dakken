//! 256 colors for terminals without 24-bit color, such as older versions of
//! macOS Terminal.app. The themes are RGB; in such a terminal, each RGB
//! color of the screen changes to the nearest of the 256 colors, in the
//! screen buffer just before it is drawn.

use ratatui::{buffer::Buffer, style::Color};

/// The terminal shows 24-bit color. Most terminals that do set
/// `COLORTERM`; Windows Terminal and the Windows console do not, but show
/// it.
pub fn truecolor() -> bool {
    let var = |name| std::env::var(name).unwrap_or_default();
    matches!(var("COLORTERM").as_str(), "truecolor" | "24bit")
        || cfg!(windows)
        || matches!(var("TERM_PROGRAM").as_str(), "WezTerm" | "iTerm.app")
}

/// Change each RGB color of the screen to the nearest of the 256 colors.
pub fn reduce(buffer: &mut Buffer) {
    for cell in &mut buffer.content {
        cell.fg = nearest(cell.fg);
        cell.bg = nearest(cell.bg);
    }
}

/// The levels of each channel in the 6 x 6 x 6 color cube (colors 16 to 231).
const CUBE: [u8; 6] = [0, 95, 135, 175, 215, 255];

/// The nearest of the 256 colors to an RGB color. Other colors stay.
pub fn nearest(color: Color) -> Color {
    let Color::Rgb(r, g, b) = color else {
        return color;
    };
    let level = |v: u8| {
        (0..6)
            .min_by_key(|&i| (i32::from(CUBE[i]) - i32::from(v)).abs())
            .unwrap_or(0)
    };
    let (ri, gi, bi) = (level(r), level(g), level(b));
    let cube = (CUBE[ri], CUBE[gi], CUBE[bi]);
    // The gray ramp: colors 232 to 255 are 8, 18, ... 238.
    let mean = (u32::from(r) + u32::from(g) + u32::from(b)) / 3;
    let step = (mean.saturating_sub(3) / 10).min(23) as u8;
    let gray = 8 + 10 * step;
    let dist = |(x, y, z): (u8, u8, u8)| {
        let d = |a: u8, b: u8| (i32::from(a) - i32::from(b)).pow(2);
        d(x, r) + d(y, g) + d(z, b)
    };
    if dist((gray, gray, gray)) < dist(cube) {
        Color::Indexed(232 + step)
    } else {
        Color::Indexed(16 + 36 * ri as u8 + 6 * gi as u8 + bi as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_colors_become_the_nearest_of_256() {
        assert_eq!(nearest(Color::Rgb(0, 0, 0)), Color::Indexed(16));
        assert_eq!(nearest(Color::Rgb(255, 255, 255)), Color::Indexed(231));
        assert_eq!(nearest(Color::Rgb(255, 0, 0)), Color::Indexed(196));
        // The tokyo-night background is a dark gray-blue: a gray.
        assert_eq!(nearest(Color::Rgb(0x1a, 0x1b, 0x26)), Color::Indexed(234));
        // The tokyo-night accent stays blue.
        assert_eq!(nearest(Color::Rgb(0x7a, 0xa2, 0xf7)), Color::Indexed(111));
        assert_eq!(nearest(Color::Cyan), Color::Cyan);
        assert_eq!(nearest(Color::Reset), Color::Reset);
    }
}
