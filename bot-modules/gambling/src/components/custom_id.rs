use std::fmt;
use std::str::FromStr;

use crate::GamblingError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlackjackCustomId {
    Hit,
    Stand,
    Double,
    Split,
    Surrender,
}

impl BlackjackCustomId {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Hit => "blackjack_hit",
            Self::Stand => "blackjack_stand",
            Self::Double => "blackjack_double",
            Self::Split => "blackjack_split",
            Self::Surrender => "blackjack_surrender",
        }
    }
}

impl FromStr for BlackjackCustomId {
    type Err = GamblingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "blackjack_hit" => Ok(Self::Hit),
            "blackjack_stand" => Ok(Self::Stand),
            "blackjack_double" => Ok(Self::Double),
            "blackjack_split" => Ok(Self::Split),
            "blackjack_surrender" => Ok(Self::Surrender),
            id => Err(GamblingError::internal(format!(
                "unrecognized blackjack component id: {id}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HigherLowerCustomId {
    Higher,
    Lower,
}

impl HigherLowerCustomId {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Higher => "hol_higher",
            Self::Lower => "hol_lower",
        }
    }
}

impl FromStr for HigherLowerCustomId {
    type Err = GamblingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "hol_higher" => Ok(Self::Higher),
            "hol_lower" => Ok(Self::Lower),
            id => Err(GamblingError::internal(format!(
                "unrecognized higher-lower component id: {id}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrestigeCustomId {
    Confirm,
    Cancel,
}

impl PrestigeCustomId {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Confirm => "prestige_confirm",
            Self::Cancel => "prestige_cancel",
        }
    }
}

impl FromStr for PrestigeCustomId {
    type Err = GamblingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "prestige_confirm" => Ok(Self::Confirm),
            "prestige_cancel" => Ok(Self::Cancel),
            id => Err(GamblingError::internal(format!(
                "unrecognized prestige component id: {id}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TicTacToeCustomId {
    Accept,
    Cancel,
    Cell { row: usize, col: usize },
}

impl fmt::Display for TicTacToeCustomId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Accept => f.write_str("ttt_accept"),
            Self::Cancel => f.write_str("ttt_cancel"),
            Self::Cell { row, col } => write!(f, "ttt_{row}{col}"),
        }
    }
}

impl FromStr for TicTacToeCustomId {
    type Err = GamblingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ttt_accept" => return Ok(Self::Accept),
            "ttt_cancel" => return Ok(Self::Cancel),
            _ => {},
        }

        let unrecognized = || {
            GamblingError::internal(format!(
                "unrecognized tictactoe component id: {s}"
            ))
        };

        let mut coords = s.strip_prefix("ttt_").ok_or_else(unrecognized)?.chars();

        let row =
            coords.next().and_then(|c| c.to_digit(10)).ok_or_else(unrecognized)?;
        let col =
            coords.next().and_then(|c| c.to_digit(10)).ok_or_else(unrecognized)?;

        if coords.next().is_some() {
            return Err(unrecognized());
        }

        Ok(Self::Cell { row: row as usize, col: col as usize })
    }
}
