//! Challenges: game rules on top of a test. The menu changes them with `g`.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Challenge {
    #[default]
    Off,
    /// Start with `LIVES` lives. Each wrong word takes one, and the test
    /// ends with the last life.
    Survival,
}

/// The challenges in menu order.
pub const CHALLENGES: [Challenge; 2] = [Challenge::Off, Challenge::Survival];

pub const LIVES: u8 = 3;

impl Challenge {
    pub fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Survival => "survival",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        CHALLENGES.into_iter().find(|c| c.name() == name)
    }

    /// The challenge after this one in `CHALLENGES`, for the menu.
    pub fn next(self) -> Self {
        let i = CHALLENGES.iter().position(|&c| c == self).unwrap_or(0);
        CHALLENGES[(i + 1) % CHALLENGES.len()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_parse_back_and_next_cycles() {
        for c in CHALLENGES {
            assert_eq!(Challenge::parse(c.name()), Some(c));
        }
        assert_eq!(Challenge::parse("nope"), None);
        assert_eq!(Challenge::Survival.next(), Challenge::Off);
        assert_eq!(Challenge::Off.next(), Challenge::Survival);
    }
}
