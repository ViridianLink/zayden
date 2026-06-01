use zayden_core::error::Respond;

pub type Result<T> = std::result::Result<T, LevelsError>;

#[derive(Debug, thiserror::Error)]
pub enum LevelsError {
    #[error(transparent)]
    Discord(#[from] serenity::Error),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

impl Respond for LevelsError {}
