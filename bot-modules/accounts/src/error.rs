use std::borrow::Cow;

use zayden_core::error::{HandlerError, Respond};

pub type Result<T> = std::result::Result<T, AccountsError>;

#[derive(Debug, thiserror::Error)]
pub enum AccountsError {
    #[error("Unknown account provider `{0}`.")]
    UnknownProvider(String),
    #[error("You haven't linked a {0} account.")]
    NotLinked(&'static str),
    #[error("invalid OpenID assertion: {0}")]
    InvalidAssertion(&'static str),
    #[error("Steam rejected the OpenID assertion")]
    VerificationRejected,
    #[error("{0}")]
    Internal(String),
    #[error(transparent)]
    Url(#[from] url::ParseError),
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    Serenity(#[from] serenity::Error),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

impl Respond for AccountsError {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::UnknownProvider(_) | Self::NotLinked(_) => {
                Some(Cow::Owned(self.to_string()))
            },
            Self::InvalidAssertion(_)
            | Self::VerificationRejected
            | Self::Internal(_)
            | Self::Url(_)
            | Self::Reqwest(_)
            | Self::Serenity(_)
            | Self::Sqlx(_) => None,
        }
    }
}

impl From<AccountsError> for HandlerError {
    fn from(e: AccountsError) -> Self {
        Self::from_respond(e)
    }
}

impl From<HandlerError> for AccountsError {
    fn from(e: HandlerError) -> Self {
        match e {
            HandlerError::Discord(e) => Self::Serenity(e),
            HandlerError::Database(e) => Self::Sqlx(e),
            HandlerError::Module { source, .. } => {
                Self::Internal(source.to_string())
            },
        }
    }
}
