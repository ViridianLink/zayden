use std::borrow::Cow;
use std::fmt::Display;

use zayden_core::error::{HandlerError, Respond};

pub type Result<T> = std::result::Result<T, GreetingsError>;

#[derive(Debug)]
pub enum GreetingsError {
    GuildOnly,
    InvalidUrl(String),
    TooManyImages(i64),
    DuplicateImage,
    UnknownKind(String),
    MessageTooLong(usize),

    InvalidCooldown(String),
    UserCooldown(i64),
    GuildCooldown(i64),

    Internal(String),

    Serenity(serenity::Error),
    Sqlx(sqlx::Error),
}

impl Display for GreetingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GuildOnly => {
                write!(f, "This command can only be used in a server.")
            },
            Self::InvalidUrl(url) => write!(
                f,
                "`{url}` isn't a usable image link. Links must start with \
                 `https://`."
            ),
            Self::TooManyImages(max) => write!(
                f,
                "This server already has the maximum of {max} images for that \
                 greeting. Remove one before adding another."
            ),
            Self::DuplicateImage => {
                write!(f, "That image link is already in the list.")
            },
            Self::UnknownKind(kind) => write!(f, "Unknown greeting type `{kind}`."),
            Self::MessageTooLong(max) => write!(
                f,
                "Greeting messages are limited to {max} characters so the reply \
                 still fits once mentions are filled in."
            ),
            Self::InvalidCooldown(raw) => write!(
                f,
                "`{raw}` isn't a usable cooldown. Enter a whole number of \
                 seconds between 0 and 86400."
            ),
            Self::UserCooldown(secs) => write!(
                f,
                "You're greeting a little fast \u{2014} try `/good` again in {secs}s."
            ),
            Self::GuildCooldown(secs) => {
                write!(f, "Someone just used `/good` here. Try again in {secs}s.")
            },
            Self::Internal(msg) => write!(f, "internal error: {msg}"),
            Self::Serenity(e) => write!(f, "serenity: {e:?}"),
            Self::Sqlx(e) => write!(f, "sqlx: {e:?}"),
        }
    }
}

impl std::error::Error for GreetingsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Serenity(e) => Some(e),
            Self::Sqlx(e) => Some(e),
            Self::GuildOnly
            | Self::InvalidUrl(_)
            | Self::TooManyImages(_)
            | Self::DuplicateImage
            | Self::UnknownKind(_)
            | Self::MessageTooLong(_)
            | Self::InvalidCooldown(_)
            | Self::UserCooldown(_)
            | Self::GuildCooldown(_)
            | Self::Internal(_) => None,
        }
    }
}

impl Respond for GreetingsError {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::Internal(_) | Self::Serenity(_) | Self::Sqlx(_) => None,
            Self::GuildOnly
            | Self::InvalidUrl(_)
            | Self::TooManyImages(_)
            | Self::DuplicateImage
            | Self::UnknownKind(_)
            | Self::MessageTooLong(_)
            | Self::InvalidCooldown(_)
            | Self::UserCooldown(_)
            | Self::GuildCooldown(_) => Some(Cow::Owned(self.to_string())),
        }
    }
}

impl From<serenity::Error> for GreetingsError {
    fn from(value: serenity::Error) -> Self {
        Self::Serenity(value)
    }
}

impl From<sqlx::Error> for GreetingsError {
    fn from(value: sqlx::Error) -> Self {
        Self::Sqlx(value)
    }
}

impl From<HandlerError> for GreetingsError {
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

impl From<GreetingsError> for HandlerError {
    fn from(e: GreetingsError) -> Self {
        Self::from_respond(e)
    }
}
