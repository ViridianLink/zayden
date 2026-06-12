use std::fmt;
use std::fmt::{Display, Formatter};

use serenity::all::{ButtonStyle, CreateButton};

#[derive(Clone, Copy)]
pub enum Mode {
    All,
    PvE,
    PvP,
}

impl Display for Mode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::All => write!(f, "All"),
            Self::PvE => write!(f, "PvE"),
            Self::PvP => write!(f, "PvP"),
        }
    }
}

impl From<Mode> for CreateButton<'_> {
    fn from(value: Mode) -> Self {
        CreateButton::new(format!("{value}"))
            .label(format!("{value}"))
            .style(ButtonStyle::Secondary)
    }
}
