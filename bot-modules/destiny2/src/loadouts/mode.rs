use std::fmt;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "destiny2_mode", rename_all = "lowercase")]
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
