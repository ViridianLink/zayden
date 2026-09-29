use std::fmt;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "destiny2_mode", rename_all = "lowercase")]
pub enum Mode {
    All,
    PvE,
    PvP,
}

impl Mode {
    pub const ALL: [Self; 3] = [Self::All, Self::PvE, Self::PvP];
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

impl FromStr for Mode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL.into_iter().find(|m| m.to_string() == s).ok_or(())
    }
}
