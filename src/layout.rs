//! Keyboard layouts to practise on a QWERTY keyboard: each key gives the
//! character at the same place on the other layout.

/// The keys of a US QWERTY keyboard, without and then with Shift.
const QWERTY: &str = "-=qwertyuiop[]asdfghjkl;'zxcvbnm,./_+QWERTYUIOP{}ASDFGHJKL:\"ZXCVBNM<>?";
/// The Dvorak characters at the same places.
const DVORAK: &str = "[]',.pyfgcrl/=aoeuidhtns-;qjkxbmwvz{}\"<>PYFGCRL?+AOEUIDHTNS_:QJKXBMWVZ";
/// The Colemak characters at the same places.
const COLEMAK: &str = "-=qwfpgjluy;[]arstdhneio'zxcvbkm,./_+QWFPGJLUY:{}ARSTDHNEIO\"ZXCVBKM<>?";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Layout {
    #[default]
    Qwerty,
    Dvorak,
    Colemak,
}

impl Layout {
    pub const ALL: [Layout; 3] = [Layout::Qwerty, Layout::Dvorak, Layout::Colemak];

    pub fn name(self) -> &'static str {
        match self {
            Self::Qwerty => "qwerty",
            Self::Dvorak => "dvorak",
            Self::Colemak => "colemak",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|l| l.name() == name)
    }

    /// The character for the QWERTY key `c` on this layout.
    pub fn map(self, c: char) -> char {
        let target = match self {
            Self::Qwerty => return c,
            Self::Dvorak => DVORAK,
            Self::Colemak => COLEMAK,
        };
        QWERTY
            .chars()
            .position(|q| q == c)
            .and_then(|i| target.chars().nth(i))
            .unwrap_or(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_have_one_character_for_each_key() {
        let n = QWERTY.chars().count();
        assert_eq!(n, 70);
        for table in [DVORAK, COLEMAK] {
            assert_eq!(table.chars().count(), n);
            let mut chars: Vec<char> = table.chars().collect();
            chars.sort();
            chars.dedup();
            assert_eq!(chars.len(), n, "each character comes from one key");
        }
    }

    #[test]
    fn keys_map_to_the_same_place_on_the_layout() {
        // The home row.
        let home = |l: Layout| "asdfjkl;".chars().map(|c| l.map(c)).collect::<String>();
        assert_eq!(home(Layout::Dvorak), "aoeuhtns");
        assert_eq!(home(Layout::Colemak), "arstneio");
        assert_eq!(Layout::Dvorak.map('Q'), '"');
        assert_eq!(Layout::Colemak.map('P'), ':');
        assert_eq!(Layout::Qwerty.map('q'), 'q');
        assert_eq!(Layout::Dvorak.map('1'), '1', "keys not in the table stay");
    }
}
