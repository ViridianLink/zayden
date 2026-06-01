use zayden_core::error::{HandlerError, Respond};

pub type Result<T> = std::result::Result<T, VerifyError>;

#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    #[error(transparent)]
    Discord(#[from] serenity::Error),
}

impl Respond for VerifyError {}

impl From<VerifyError> for HandlerError {
    fn from(e: VerifyError) -> Self {
        Self::from_respond(e)
    }
}
