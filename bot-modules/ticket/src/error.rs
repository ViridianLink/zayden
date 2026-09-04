use std::borrow::Cow;

use zayden_core::CoreError as ZaydenError;
use zayden_core::error::{HandlerError, Respond};

pub type Result<T> = std::result::Result<T, TicketError>;

#[derive(Debug)]
pub enum TicketError {
    NotInSupportChannel,
    SupportNotFound,
    ArticleNotFound,
    ForumChannelUnsupported,
    MissingPermissions,
    NotTicketParticipant,
    TicketAlreadyClosed,
    FaqNotConfigured,
    Wiki(String),
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
            Self::ArticleNotFound => write!(
                f,
                "That FAQ article no longer exists. Run the command again for \
                 an up to date list."
            ),
            Self::ForumChannelUnsupported => write!(
                f,
                "A ticket button cannot be posted in a forum channel. Post it \
                 in a text channel, or let people open posts in the forum \
                 directly - those become tickets on their own."
            ),
            Self::MissingPermissions => write!(
                f,
                "You need the Manage Messages permission to use that subcommand."
            ),
            Self::NotTicketParticipant => write!(
                f,
                "Only the person who opened this ticket or the support team can \
                 use these buttons."
            ),
            Self::TicketAlreadyClosed => {
                write!(f, "This ticket has already been solved or closed.")
            },
            Self::FaqNotConfigured => write!(
                f,
                "No wiki is configured for this server. Set one in the dashboard \
                 under Support."
            ),
            Self::Wiki(msg) => write!(f, "The wiki could not be reached: {msg}"),
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
            | Self::ArticleNotFound
            | Self::ForumChannelUnsupported
            | Self::MissingPermissions
            | Self::NotTicketParticipant
            | Self::TicketAlreadyClosed
            | Self::FaqNotConfigured
            | Self::Wiki(_)
            | Self::Internal(_) => None,
        }
    }
}

impl Respond for TicketError {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::NotInSupportChannel
            | Self::SupportNotFound
            | Self::ArticleNotFound
            | Self::ForumChannelUnsupported
            | Self::MissingPermissions
            | Self::NotTicketParticipant
            | Self::TicketAlreadyClosed
            | Self::FaqNotConfigured
            | Self::Wiki(_) => Some(Cow::Owned(self.to_string())),
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
