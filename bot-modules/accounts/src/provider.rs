use std::fmt;
use std::str::FromStr;

use crate::error::AccountsError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Provider {
    Steam,
}

impl Provider {
    pub const ALL: [Self; 1] = [Self::Steam];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Steam => "steam",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Steam => "Steam",
        }
    }
}

impl FromStr for Provider {
    type Err = AccountsError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "steam" => Ok(Self::Steam),
            _ => Err(AccountsError::UnknownProvider(s.to_owned())),
        }
    }
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}
