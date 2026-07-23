use std::str::FromStr;

use crate::LevelsError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelsCustomId {
    Previous,
    User,
    Next,
}

impl LevelsCustomId {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Previous => "levels_previous",
            Self::User => "levels_user",
            Self::Next => "levels_next",
        }
    }
}

impl FromStr for LevelsCustomId {
    type Err = LevelsError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "levels_previous" => Ok(Self::Previous),
            "levels_user" => Ok(Self::User),
            "levels_next" => Ok(Self::Next),
            id => Err(LevelsError::Internal(format!(
                "unrecognized levels component id: {id}"
            ))),
        }
    }
}
