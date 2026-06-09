use std::borrow::Cow;
use std::fmt::Display;

use zayden_core::error::{HandlerError, Respond};

pub type Result<T> = std::result::Result<T, GoldStarError>;

#[derive(Debug)]
pub enum GoldStarError {
    SelfStar,
    NoStars(i64),
    InvalidOptions,

    Internal(String),

    Serenity(serenity::Error),
    Sqlx(sqlx::Error),
}

impl Display for GoldStarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SelfStar => write!(f, "You can't give yourself a star."),
            Self::NoStars(timestamp) => write!(
                f,
                "You don't have any stars to give.\nNext free star <t:{timestamp}:R>"
            ),
            Self::InvalidOptions => write!(f, "Invalid command options."),
            Self::Internal(msg) => write!(f, "internal error: {msg}"),
            Self::Serenity(e) => write!(f, "serenity: {e:?}"),
            Self::Sqlx(e) => write!(f, "sqlx: {e:?}"),
        }
    }
}

impl std::error::Error for GoldStarError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Serenity(e) => Some(e),
            Self::Sqlx(e) => Some(e),
            Self::SelfStar
            | Self::NoStars(_)
            | Self::InvalidOptions
            | Self::Internal(_) => None,
        }
    }
}

impl Respond for GoldStarError {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::Internal(_) | Self::Serenity(_) | Self::Sqlx(_) => None,
            Self::SelfStar | Self::NoStars(_) | Self::InvalidOptions => {
                Some(Cow::Owned(self.to_string()))
            },
        }
    }
}

impl From<serenity::Error> for GoldStarError {
    fn from(value: serenity::Error) -> Self {
        Self::Serenity(value)
    }
}

impl From<sqlx::Error> for GoldStarError {
    fn from(value: sqlx::Error) -> Self {
        Self::Sqlx(value)
    }
}

impl From<HandlerError> for GoldStarError {
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

impl From<GoldStarError> for HandlerError {
    fn from(e: GoldStarError) -> Self {
        Self::from_respond(e)
    }
}
