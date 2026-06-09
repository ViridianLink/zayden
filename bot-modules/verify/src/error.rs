use std::borrow::Cow;

use zayden_core::error::{HandlerError, Respond};

pub type Result<T> = std::result::Result<T, VerifyError>;

#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    #[error("This command can only be used by server members.")]
    NotGuildMember,

    #[error(transparent)]
    Discord(#[from] serenity::Error),
}

impl Respond for VerifyError {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::NotGuildMember => Some(Cow::Owned(self.to_string())),
            Self::Discord(_) => None,
        }
    }
}

impl From<VerifyError> for HandlerError {
    fn from(e: VerifyError) -> Self {
        Self::from_respond(e)
    }
}
