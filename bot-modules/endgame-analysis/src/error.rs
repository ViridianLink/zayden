use std::borrow::Cow;

use zayden_core::Error as ZaydenError;
use zayden_core::error::Respond;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    WeaponNotFound(String),

    ZaydenCore(ZaydenError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::WeaponNotFound(weapon) => write!(f, "Weapon {weapon} not found"),
            Self::ZaydenCore(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ZaydenCore(e) => Some(e),
            _ => None,
        }
    }
}

impl Respond for Error {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::WeaponNotFound(_) => Some(Cow::Owned(self.to_string())),
            Self::ZaydenCore(e) => e.user_message(),
        }
    }
}

impl From<serenity::Error> for Error {
    fn from(value: serenity::Error) -> Self {
        Self::ZaydenCore(ZaydenError::Serenity(value))
    }
}
