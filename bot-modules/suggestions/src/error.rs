use std::borrow::Cow;

use zayden_core::CoreError;
use zayden_core::error::{HandlerError, Respond};

pub type Result<T> = std::result::Result<T, SuggestionsError>;

#[derive(Debug)]
pub enum SuggestionsError {
    MissingSuggesionChannel,
    InvalidModalStructure,
    Internal(String),
    Zayden(CoreError),
}

impl std::fmt::Display for SuggestionsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSuggesionChannel => {
                write!(f, "Please specify a channel to fetch suggestions from.")
            },
            Self::InvalidModalStructure => {
                write!(f, "invalid suggestions modal structure")
            },
            Self::Internal(msg) => write!(f, "internal error: {msg}"),
            Self::Zayden(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for SuggestionsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Zayden(e) => Some(e),
            Self::MissingSuggesionChannel
            | Self::InvalidModalStructure
            | Self::Internal(_) => None,
        }
    }
}

impl Respond for SuggestionsError {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::MissingSuggesionChannel => Some(Cow::Owned(self.to_string())),
            Self::InvalidModalStructure | Self::Internal(_) => None,
            Self::Zayden(e) => e.user_message(),
        }
    }
}

impl From<CoreError> for SuggestionsError {
    fn from(value: CoreError) -> Self {
        Self::Zayden(value)
    }
}

impl From<serenity::Error> for SuggestionsError {
    fn from(value: serenity::Error) -> Self {
        Self::Zayden(CoreError::Serenity(value))
    }
}

impl From<sqlx::Error> for SuggestionsError {
    fn from(value: sqlx::Error) -> Self {
        Self::Zayden(CoreError::Sqlx(value))
    }
}

impl From<SuggestionsError> for HandlerError {
    fn from(e: SuggestionsError) -> Self {
        Self::from_respond(e)
    }
}
