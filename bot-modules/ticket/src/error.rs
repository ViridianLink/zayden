use std::borrow::Cow;

use zayden_core::Error as ZaydenError;
use zayden_core::error::Respond;

pub type Result<T> = std::result::Result<T, Error>;

#[expect(
    clippy::error_impl_error,
    reason = "conventional error type name in domain crate"
)]
#[derive(Debug)]
pub enum Error {
    NotInSupportChannel,
    SupportNotFound,

    ZaydenCore(ZaydenError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotInSupportChannel => {
                write!(f, "This command only works in the support channel.")
            },
            Self::SupportNotFound => write!(f, "Support message not found"),
            Self::ZaydenCore(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ZaydenCore(e) => Some(e),
            Self::NotInSupportChannel | Self::SupportNotFound => None,
        }
    }
}

impl Respond for Error {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::NotInSupportChannel | Self::SupportNotFound => {
                Some(Cow::Owned(self.to_string()))
            },
            Self::ZaydenCore(e) => e.user_message(),
        }
    }
}

impl From<serenity::Error> for Error {
    fn from(value: serenity::Error) -> Self {
        Self::ZaydenCore(ZaydenError::Serenity(value))
    }
}

impl From<ZaydenError> for Error {
    fn from(value: ZaydenError) -> Self {
        Self::ZaydenCore(value)
    }
}

impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        Self::ZaydenCore(ZaydenError::Sqlx(value))
    }
}
