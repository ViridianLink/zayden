use std::borrow::Cow;

use zayden_core::error::{HandlerError, Respond};

pub type Result<T> = std::result::Result<T, VerifyError>;

#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    #[error("This command can only be used by server members.")]
    NotGuildMember,

    #[error(
        "No verification role is configured for this server. An admin can set one on the dashboard under Roles."
    )]
    RoleNotConfigured,

    #[error(transparent)]
    Discord(#[from] serenity::Error),

    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

impl Respond for VerifyError {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::NotGuildMember | Self::RoleNotConfigured => {
                Some(Cow::Owned(self.to_string()))
            },
            Self::Discord(_) | Self::Sqlx(_) => None,
        }
    }
}

impl From<VerifyError> for HandlerError {
    fn from(e: VerifyError) -> Self {
        Self::from_respond(e)
    }
}
