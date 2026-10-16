use std::borrow::Cow;

use deadlock_api::DeadlockApiError;
use zayden_core::error::{HandlerError, Respond};

pub type Result<T> = std::result::Result<T, DeadlockError>;

#[derive(Debug, thiserror::Error)]
pub enum DeadlockError {
    #[error(transparent)]
    Api(#[from] DeadlockApiError),
    #[error("Unknown hero `{0}`.")]
    UnknownHero(String),
    #[error("Unknown rank `{0}`.")]
    UnknownRank(String),
    #[error("Pick two different heroes.")]
    SameHero,
    #[error("failed to decode cached deadlock-api data for {key}: {source}")]
    CacheDecode {
        key: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("invalid option: {0}")]
    InvalidOption(String),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Serenity(#[from] serenity::Error),
}

impl Respond for DeadlockError {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::Api(e)
                if matches!(
                    e,
                    DeadlockApiError::Busy { .. } | DeadlockApiError::NotFound
                ) =>
            {
                Some(Cow::Owned(e.to_string()))
            },
            Self::UnknownHero(_) | Self::UnknownRank(_) | Self::SameHero => {
                Some(Cow::Owned(self.to_string()))
            },
            Self::Api(_)
            | Self::CacheDecode { .. }
            | Self::InvalidOption(_)
            | Self::Sqlx(_)
            | Self::Serenity(_) => None,
        }
    }
}

impl From<DeadlockError> for HandlerError {
    fn from(e: DeadlockError) -> Self {
        Self::from_respond(e)
    }
}

impl From<HandlerError> for DeadlockError {
    fn from(e: HandlerError) -> Self {
        match e {
            HandlerError::Discord(e) => Self::Serenity(e),
            HandlerError::Database(e) => Self::Sqlx(e),
            HandlerError::Module { source, .. } => {
                Self::InvalidOption(source.to_string())
            },
        }
    }
}
