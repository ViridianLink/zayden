use std::borrow::Cow;

use zayden_core::CoreError as ZaydenError;
use zayden_core::error::{HandlerError, Respond};

pub type Result<T> = std::result::Result<T, TicketError>;

#[derive(Debug)]
pub enum TicketError {
    NotInSupportChannel,
    SupportNotFound,
    Internal(String),

    ZaydenCore(ZaydenError),
}

impl std::fmt::Display for TicketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotInSupportChannel => {
                write!(f, "This command only works in the support channel.")
            },
            Self::SupportNotFound => write!(f, "Support message not found"),
            Self::Internal(msg) => write!(f, "internal error: {msg}"),
            Self::ZaydenCore(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for TicketError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ZaydenCore(e) => Some(e),
            Self::NotInSupportChannel
            | Self::SupportNotFound
            | Self::Internal(_) => None,
        }
    }
}

impl Respond for TicketError {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::NotInSupportChannel | Self::SupportNotFound => {
                Some(Cow::Owned(self.to_string()))
            },
            Self::Internal(_) => None,
            Self::ZaydenCore(e) => e.user_message(),
        }
    }
}

impl From<serenity::Error> for TicketError {
    fn from(value: serenity::Error) -> Self {
        Self::ZaydenCore(ZaydenError::Serenity(value))
    }
}

impl From<ZaydenError> for TicketError {
    fn from(value: ZaydenError) -> Self {
        Self::ZaydenCore(value)
    }
}

impl From<sqlx::Error> for TicketError {
    fn from(value: sqlx::Error) -> Self {
        Self::ZaydenCore(ZaydenError::Sqlx(value))
    }
}

impl From<TicketError> for HandlerError {
    fn from(e: TicketError) -> Self {
        Self::from_respond(e)
    }
}

impl From<HandlerError> for TicketError {
    fn from(e: HandlerError) -> Self {
        match e {
            HandlerError::Database(e) => Self::ZaydenCore(ZaydenError::Sqlx(e)),
            HandlerError::Discord(e) => Self::ZaydenCore(ZaydenError::Serenity(e)),
            HandlerError::Module { source, .. } => {
                Self::ZaydenCore(ZaydenError::InvalidOption(source.to_string()))
            },
        }
    }
}
