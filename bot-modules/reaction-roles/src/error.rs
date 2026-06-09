use std::borrow::Cow;

use serenity::all::ReactionConversionError;
use zayden_core::CoreError;
use zayden_core::error::{HandlerError, Respond};

pub type Result<T> = std::result::Result<T, ReactionRoleError>;

#[derive(Debug)]
pub enum ReactionRoleError {
    MissingGuildId,
    MissingUserId,
    InvalidMessageId(String),
    Internal(String),
    ReactionConversionError(ReactionConversionError),
    Serenity(serenity::Error),
    Sqlx(sqlx::Error),
}

impl std::fmt::Display for ReactionRoleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingGuildId => CoreError::MissingGuildId.fmt(f),
            Self::MissingUserId => write!(f, "Reaction is missing user ID"),
            Self::InvalidMessageId(id) => write!(f, "Invalid message ID: {id}"),
            Self::ReactionConversionError(_) => {
                write!(f, "Failed to convert emoji to reaction")
            },
            Self::Internal(msg) => write!(f, "internal error: {msg}"),
            Self::Serenity(e) => write!(f, "serenity: {e:?}"),
            Self::Sqlx(e) => write!(f, "sqlx: {e:?}"),
        }
    }
}

impl std::error::Error for ReactionRoleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ReactionConversionError(e) => Some(e),
            Self::Serenity(e) => Some(e),
            Self::Sqlx(e) => Some(e),
            Self::MissingGuildId
            | Self::MissingUserId
            | Self::InvalidMessageId(_)
            | Self::Internal(_) => None,
        }
    }
}

impl Respond for ReactionRoleError {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::ReactionConversionError(_)
            | Self::Serenity(_)
            | Self::Sqlx(_)
            | Self::Internal(_) => None,
            Self::MissingGuildId
            | Self::MissingUserId
            | Self::InvalidMessageId(_) => Some(Cow::Owned(self.to_string())),
        }
    }
}

impl From<ReactionConversionError> for ReactionRoleError {
    fn from(err: ReactionConversionError) -> Self {
        Self::ReactionConversionError(err)
    }
}

impl From<serenity::Error> for ReactionRoleError {
    fn from(value: serenity::Error) -> Self {
        Self::Serenity(value)
    }
}

impl From<sqlx::Error> for ReactionRoleError {
    fn from(value: sqlx::Error) -> Self {
        Self::Sqlx(value)
    }
}

impl From<ReactionRoleError> for HandlerError {
    fn from(e: ReactionRoleError) -> Self {
        Self::from_respond(e)
    }
}

impl From<HandlerError> for ReactionRoleError {
    fn from(e: HandlerError) -> Self {
        match e {
            HandlerError::Database(e) => Self::Sqlx(e),
            HandlerError::Discord(e) => Self::Serenity(e),
            HandlerError::Module { .. } => Self::MissingGuildId,
        }
    }
}
