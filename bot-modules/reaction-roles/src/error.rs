use std::borrow::Cow;

use serenity::all::ReactionConversionError;
use zayden_core::error::Respond;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    MissingGuildId,
    InvalidMessageId(String),
    ReactionConversionError(ReactionConversionError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::MissingGuildId => zayden_core::Error::MissingGuildId.fmt(f),
            Self::InvalidMessageId(id) => write!(f, "Invalid message ID: {id}"),
            Self::ReactionConversionError(_) => write!(f, "Failed to convert emoji to reaction"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ReactionConversionError(e) => Some(e),
            _ => None,
        }
    }
}

impl Respond for Error {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::ReactionConversionError(_) => None,
            _ => Some(Cow::Owned(self.to_string())),
        }
    }
}

impl From<ReactionConversionError> for Error {
    fn from(err: ReactionConversionError) -> Self {
        Self::ReactionConversionError(err)
    }
}
