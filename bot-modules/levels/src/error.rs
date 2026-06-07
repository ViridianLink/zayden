use zayden_core::error::{HandlerError, Respond};

pub type Result<T> = std::result::Result<T, LevelsError>;

#[derive(Debug, thiserror::Error)]
pub enum LevelsError {
    #[error(transparent)]
    Discord(#[from] serenity::Error),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("internal error: {0}")]
    Internal(String),
}

impl Respond for LevelsError {}

impl From<LevelsError> for HandlerError {
    fn from(e: LevelsError) -> Self {
        Self::from_respond(e)
    }
}
