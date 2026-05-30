use std::borrow::Cow;

use zayden_core::error::Respond;

pub type Result<T> = std::result::Result<T, Error>;

#[expect(
    clippy::error_impl_error,
    reason = "conventional error type name in domain crate"
)]
#[derive(Debug)]
pub enum Error {
    MissingSuggesionChannel,
    InvalidModalStructure,
    Zayden(zayden_core::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSuggesionChannel => {
                write!(f, "Please specify a channel to fetch suggestions from.")
            },
            Self::InvalidModalStructure => {
                write!(f, "invalid suggestions modal structure")
            },
            Self::Zayden(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Zayden(e) => Some(e),
            Self::MissingSuggesionChannel | Self::InvalidModalStructure => None,
        }
    }
}

impl Respond for Error {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::MissingSuggesionChannel => Some(Cow::Owned(self.to_string())),
            Self::InvalidModalStructure => None,
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

impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        Self::Zayden(zayden_core::Error::Sqlx(value))
    }
}
