use std::borrow::Cow;

use zayden_core::error::{HandlerError, Respond};

pub type Result<T> = std::result::Result<T, MarathonError>;

#[derive(Debug, thiserror::Error)]
pub enum MarathonError {
    #[error("Marathon data is temporarily unavailable. Please try again shortly.")]
    SourceUnavailable,
    #[error("Couldn't find `{query}` in {entity}.")]
    NotFound { entity: &'static str, query: String },
    #[error("This command can only be used within a server.")]
    MissingGuildId,
    #[error(
        "You need the Manage Server permission to configure Marathon announcements."
    )]
    NotPrivileged,

    #[error("Mobalytics returned a Cloudflare challenge instead of content.")]
    CloudflareChallenge,
    #[error("Mobalytics rejected the request (CSRF).")]
    Csrf,
    #[error("FlareSolverr error: {0}")]
    FlareSolverr(String),
    #[error("failed to parse widget-tree content: {0}")]
    Parse(String),

    #[error("network error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("discord error: {0}")]
    Serenity(#[from] serenity::Error),
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),
}

impl Respond for MarathonError {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::SourceUnavailable
            | Self::NotFound { .. }
            | Self::MissingGuildId
            | Self::NotPrivileged => Some(Cow::Owned(self.to_string())),
            Self::CloudflareChallenge
            | Self::Csrf
            | Self::FlareSolverr(_)
            | Self::Parse(_)
            | Self::Reqwest(_)
            | Self::Serenity(_)
            | Self::Sqlx(_) => None,
        }
    }
}

impl From<MarathonError> for HandlerError {
    fn from(e: MarathonError) -> Self {
        Self::from_respond(e)
    }
}

impl From<HandlerError> for MarathonError {
    fn from(e: HandlerError) -> Self {
        match e {
            HandlerError::Discord(e) => Self::Serenity(e),
            HandlerError::Database(e) => Self::Sqlx(e),
            HandlerError::Module { source, .. } => Self::Parse(source.to_string()),
        }
    }
}
