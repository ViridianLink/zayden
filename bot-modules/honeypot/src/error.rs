use std::borrow::Cow;

use zayden_core::error::{HandlerError, Respond};

pub type Result<T> = std::result::Result<T, HoneypotError>;

#[derive(Debug, thiserror::Error)]
pub enum HoneypotError {
    #[error(transparent)]
    Discord(#[from] serenity::Error),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("This command can only be used in a server.")]
    MissingGuildId,
    #[error("You need the Manage Server permission to configure the honeypot.")]
    NotPrivileged,
    #[error("Unknown honeypot subcommand: {0}")]
    UnknownSubcommand(String),
    #[error("internal error: {0}")]
    Internal(String),
}

impl Respond for HoneypotError {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::MissingGuildId
            | Self::NotPrivileged
            | Self::UnknownSubcommand(_) => Some(Cow::Owned(self.to_string())),
            Self::Discord(_) | Self::Database(_) | Self::Internal(_) => None,
        }
    }
}

impl From<HoneypotError> for HandlerError {
    fn from(e: HoneypotError) -> Self {
        Self::from_respond(e)
    }
}

impl From<HandlerError> for HoneypotError {
    fn from(e: HandlerError) -> Self {
        match e {
            HandlerError::Discord(e) => Self::Discord(e),
            HandlerError::Database(e) => Self::Database(e),
            HandlerError::Module { source, .. } => Self::Internal(source.to_string()),
        }
    }
}
