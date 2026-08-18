use std::fmt;
use std::str::FromStr;

use crate::GamblingError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandState {
    Active,
    Waiting,
    Done,
}

impl HandState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Waiting => "waiting",
            Self::Done => "done",
        }
    }
}

impl FromStr for HandState {
    type Err = GamblingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "waiting" => Ok(Self::Waiting),
            "done" => Ok(Self::Done),
            state => Err(GamblingError::internal(format!(
                "unrecognized blackjack hand state: {state}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlackjackCustomId {
    Hit,
    Stand,
    Double,
    Split,
    Surrender,
    Hand { index: u8, state: HandState },
    Dealer { state: HandState },
}

impl fmt::Display for BlackjackCustomId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hit => f.write_str("blackjack_hit"),
            Self::Stand => f.write_str("blackjack_stand"),
            Self::Double => f.write_str("blackjack_double"),
            Self::Split => f.write_str("blackjack_split"),
            Self::Surrender => f.write_str("blackjack_surrender"),
            Self::Hand { index, state } => {
                write!(f, "blackjack_hand_{index}_{}", state.as_str())
            },
            Self::Dealer { state } => {
                write!(f, "blackjack_dealer_{}", state.as_str())
            },
        }
    }
}

impl FromStr for BlackjackCustomId {
    type Err = GamblingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "blackjack_hit" => return Ok(Self::Hit),
            "blackjack_stand" => return Ok(Self::Stand),
            "blackjack_double" => return Ok(Self::Double),
            "blackjack_split" => return Ok(Self::Split),
            "blackjack_surrender" => return Ok(Self::Surrender),
            _ => {},
        }

        let unrecognized = || {
            GamblingError::internal(format!(
                "unrecognized blackjack component id: {s}"
            ))
        };

        if let Some(state) = s.strip_prefix("blackjack_dealer_") {
            return Ok(Self::Dealer { state: state.parse()? });
        }

        let (index, state) = s
            .strip_prefix("blackjack_hand_")
            .ok_or_else(unrecognized)?
            .rsplit_once('_')
            .ok_or_else(unrecognized)?;

        Ok(Self::Hand {
            index: index.parse().map_err(|_e| unrecognized())?,
            state: state.parse()?,
        })
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
