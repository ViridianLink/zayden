use std::borrow::Cow;

use zayden_core::Error as ZaydenError;
use zayden_core::error::Respond;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    NotInSupportChannel,
    SupportNotFound,

    ZaydenCore(ZaydenError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::NotInSupportChannel => {
                write!(f, "This command only works in the support channel.")
            }
            Error::SupportNotFound => write!(f, "Support message not found"),
            Self::ZaydenCore(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ZaydenCore(e) => Some(e),
            _ => None,
        }
    }
}

impl Respond for Error {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::NotInSupportChannel | Self::SupportNotFound => Some(Cow::Owned(self.to_string())),
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
