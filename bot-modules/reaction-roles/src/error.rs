use std::borrow::Cow;

use serenity::all::ReactionConversionError;
use zayden_core::error::{HandlerError, Respond};

pub type Result<T> = std::result::Result<T, Error>;

#[expect(
    clippy::error_impl_error,
    reason = "conventional error type name in domain crate"
)]
#[derive(Debug)]
pub enum Error {
    MissingGuildId,
    InvalidMessageId(String),
    ReactionConversionError(ReactionConversionError),
    Serenity(serenity::Error),
    Sqlx(sqlx::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingGuildId => zayden_core::Error::MissingGuildId.fmt(f),
            Self::InvalidMessageId(id) => write!(f, "Invalid message ID: {id}"),
            Self::ReactionConversionError(_) => {
                write!(f, "Failed to convert emoji to reaction")
            },
            Self::Serenity(e) => write!(f, "serenity: {e:?}"),
            Self::Sqlx(e) => write!(f, "sqlx: {e:?}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ReactionConversionError(e) => Some(e),
            Self::Serenity(e) => Some(e),
            Self::Sqlx(e) => Some(e),
            Self::MissingGuildId | Self::InvalidMessageId(_) => None,
        }
    }
}

impl Respond for Error {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::ReactionConversionError(_) | Self::Serenity(_) | Self::Sqlx(_) => {
                None
            },
            Self::MissingGuildId | Self::InvalidMessageId(_) => {
                Some(Cow::Owned(self.to_string()))
            },
        }
    }
}

impl From<ReactionConversionError> for Error {
    fn from(err: ReactionConversionError) -> Self {
        Self::ReactionConversionError(err)
    }
}

impl From<serenity::Error> for Error {
    fn from(value: serenity::Error) -> Self {
        Self::Serenity(value)
    }
}

impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        Self::Sqlx(value)
    }
}

impl From<Error> for HandlerError {
    fn from(e: Error) -> Self {
        Self::from_respond(e)
    }
}
