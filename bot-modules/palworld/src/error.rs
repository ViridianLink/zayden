use std::borrow::Cow;

use zayden_core::error::{HandlerError, Respond};

pub type Result<T> = std::result::Result<T, PalworldError>;

#[derive(Debug, thiserror::Error)]
pub enum PalworldError {
    #[error("Palworld data is temporarily unavailable. Please try again shortly.")]
    SourceUnavailable,
    #[error("Couldn't find `{query}` in {entity}.")]
    NotFound { entity: &'static str, query: String },
    #[error("`{0}` isn't a known Palworld element.")]
    UnknownElement(String),

    #[error("FlareSolverr error: {0}")]
    FlareSolverr(String),
    #[error("failed to parse source content: {0}")]
    Parse(String),

    #[error("network error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("discord error: {0}")]
    Serenity(#[from] serenity::Error),
}

impl Respond for PalworldError {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::SourceUnavailable
            | Self::NotFound { .. }
            | Self::UnknownElement(_) => Some(Cow::Owned(self.to_string())),
            Self::FlareSolverr(_)
            | Self::Parse(_)
            | Self::Reqwest(_)
            | Self::Serenity(_) => None,
        }
    }
}

impl From<PalworldError> for HandlerError {
    fn from(e: PalworldError) -> Self {
        Self::from_respond(e)
    }
}

impl From<HandlerError> for PalworldError {
    fn from(e: HandlerError) -> Self {
        match e {
            HandlerError::Discord(e) => Self::Serenity(e),
            HandlerError::Database(_) => Self::SourceUnavailable,
            HandlerError::Module { source, .. } => Self::Parse(source.to_string()),
        }
    }
}
