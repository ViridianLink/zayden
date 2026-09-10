use std::borrow::Cow;

use jellyfin::JellyfinError;
use zayden_core::error::{HandlerError, Respond};

pub type Result<T> = std::result::Result<T, WatchError>;

#[derive(Debug, thiserror::Error)]
pub enum WatchError {
    #[error(transparent)]
    Jellyfin(#[from] JellyfinError),
    #[error(transparent)]
    Discord(#[from] serenity::Error),
    #[error(transparent)]
    Database(#[from] sqlx::Error),

    #[error("Unknown subcommand: {0}")]
    UnknownSubcommand(String),
    #[error("This command can only be used in a server.")]
    MissingGuildId,
    #[error("That round has already been answered.")]
    RoundClosed,
    #[error("That round has expired.")]
    RoundExpired,
    #[error("internal error: {0}")]
    Internal(String),
}

impl Respond for WatchError {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::Jellyfin(e) => e.user_message(),
            Self::UnknownSubcommand(_)
            | Self::MissingGuildId
            | Self::RoundClosed
            | Self::RoundExpired => Some(Cow::Owned(self.to_string())),
            Self::Discord(_) | Self::Database(_) | Self::Internal(_) => None,
        }
    }
}

impl From<WatchError> for HandlerError {
    fn from(e: WatchError) -> Self {
        Self::from_respond(e)
    }
}

impl From<HandlerError> for WatchError {
    fn from(e: HandlerError) -> Self {
        match e {
            HandlerError::Discord(e) => Self::Discord(e),
            HandlerError::Database(e) => Self::Database(e),
            HandlerError::Module { source, .. } => {
                Self::Internal(source.to_string())
            },
        }
    }
}
