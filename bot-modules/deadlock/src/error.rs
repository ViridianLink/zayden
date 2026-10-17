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
    #[error("Use this in a server to change server sharing.")]
    GuildOnly,
    #[error("failed to decode cached deadlock-api data for {key}: {source}")]
    CacheDecode {
        key: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("invalid option: {0}")]
    InvalidOption(String),
    #[error("invalid patch date `{0}`")]
    PatchDate(String),
    #[error("failed to encode {what}")]
    Encode {
        what: &'static str,
        #[source]
        source: serde_json::Error,
    },
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Serenity(#[from] serenity::Error),
}

impl DeadlockError {
    #[must_use]
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            Self::Api(DeadlockApiError::Request(e)) if e.is_timeout() || e.is_connect()
        ) || matches!(
            self,
            Self::Api(DeadlockApiError::Http { status }) if status.is_server_error()
        )
    }
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
            Self::UnknownHero(_)
            | Self::UnknownRank(_)
            | Self::SameHero
            | Self::GuildOnly => Some(Cow::Owned(self.to_string())),
            Self::Api(_)
            | Self::CacheDecode { .. }
            | Self::InvalidOption(_)
            | Self::PatchDate(_)
            | Self::Encode { .. }
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
