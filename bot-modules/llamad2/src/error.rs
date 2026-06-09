use std::borrow::Cow;

use zayden_core::error::{HandlerError, Respond};

pub type Result<T> = std::result::Result<T, LlamaD2Error>;

#[derive(Debug, thiserror::Error)]
pub enum LlamaD2Error {
    #[error("Incorrect codeword, please try again!")]
    IncorrectCodeword,

    #[error("internal error: {0}")]
    Internal(String),

    #[error(transparent)]
    Discord(#[from] serenity::Error),
}

impl Respond for LlamaD2Error {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::IncorrectCodeword => Some(Cow::Owned(self.to_string())),
            Self::Internal(_) | Self::Discord(_) => None,
        }
    }
}

impl From<LlamaD2Error> for HandlerError {
    fn from(e: LlamaD2Error) -> Self {
        Self::from_respond(e)
    }
}
