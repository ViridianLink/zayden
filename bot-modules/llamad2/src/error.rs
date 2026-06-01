use zayden_core::error::{HandlerError, Respond};

pub type Result<T> = std::result::Result<T, LlamaD2Error>;

#[derive(Debug, thiserror::Error)]
pub enum LlamaD2Error {
    #[error(transparent)]
    Discord(#[from] serenity::Error),
}

impl Respond for LlamaD2Error {}

impl From<LlamaD2Error> for HandlerError {
    fn from(e: LlamaD2Error) -> Self {
        Self::from_respond(e)
    }
}
