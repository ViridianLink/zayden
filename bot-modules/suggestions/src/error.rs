use std::borrow::Cow;

use zayden_core::error::Respond;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    MissingSuggesionChannel,
    InvalidModalStructure,
    Zayden(zayden_core::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::MissingSuggesionChannel => {
                write!(f, "Please specify a channel to fetch suggestions from.")
            }
            Error::InvalidModalStructure => write!(f, "invalid suggestions modal structure"),
            Self::Zayden(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Zayden(e) => Some(e),
            _ => None,
        }
    }
}

impl Respond for Error {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Error::MissingSuggesionChannel => Some(Cow::Owned(self.to_string())),
            Error::InvalidModalStructure => None,
            Self::Zayden(e) => e.user_message(),
        }
    }
}

impl From<zayden_core::Error> for Error {
    fn from(value: zayden_core::Error) -> Self {
        Self::Zayden(value)
    }
}

impl From<serenity::Error> for Error {
    fn from(value: serenity::Error) -> Self {
        Self::Zayden(zayden_core::Error::Serenity(value))
    }
}
