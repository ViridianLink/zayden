use zayden_core::error::Respond;

pub type Result<T> = std::result::Result<T, VerifyError>;

#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    #[error(transparent)]
    Discord(#[from] serenity::Error),
}

impl Respond for VerifyError {}
